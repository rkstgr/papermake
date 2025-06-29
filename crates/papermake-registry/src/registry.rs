use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{
    address::ContentAddress,
    bundle::TemplateBundle,
    error::{RegistryError, StorageError},
    manifest::Manifest,
    storage::BlobStorage,
};

/// Core registry for template publishing and resolution
pub struct Registry<S: BlobStorage> {
    storage: Arc<S>,
}

impl<S: BlobStorage> Registry<S> {
    /// Create a new registry with the given storage backend
    pub fn new(storage: S) -> Self {
        Self {
            storage: Arc::new(storage),
        }
    }

    /// Publish a template bundle to the registry
    ///
    /// This method implements the "store files → create manifest → update refs" workflow:
    /// 1. Validates the template bundle
    /// 2. Stores all files as content-addressed blobs
    /// 3. Creates a manifest mapping file paths to their hashes
    /// 4. Stores the manifest as a content-addressed blob
    /// 5. Updates the reference (tag) to point to the manifest hash
    ///
    /// Returns the manifest hash for content-addressable access
    pub async fn publish(
        &self,
        bundle: TemplateBundle,
        namespace: &str,
        tag: &str,
    ) -> Result<String, RegistryError> {
        // Step 1: Validate the bundle
        bundle.validate().map_err(|e| {
            RegistryError::Template(crate::error::TemplateError::invalid(e.to_string()))
        })?;

        // Step 2: Store individual files as blobs
        let mut file_hashes = BTreeMap::new();

        // Store main.typ
        let main_hash = ContentAddress::hash(bundle.main_typ());
        let main_blob_key = ContentAddress::blob_key(&main_hash);
        self.storage
            .put(&main_blob_key, bundle.main_typ().to_vec())
            .await
            .map_err(|e| RegistryError::Storage(StorageError::backend(e.to_string())))?;
        file_hashes.insert("main.typ".to_string(), main_hash);

        // Store additional files
        for (file_path, file_content) in bundle.files() {
            let file_hash = ContentAddress::hash(file_content);
            let file_blob_key = ContentAddress::blob_key(&file_hash);
            self.storage
                .put(&file_blob_key, file_content.clone())
                .await
                .map_err(|e| RegistryError::Storage(StorageError::backend(e.to_string())))?;
            file_hashes.insert(file_path.clone(), file_hash);
        }

        // Step 3: Create manifest
        let manifest = Manifest::new(file_hashes, bundle.metadata().clone()).map_err(|e| {
            RegistryError::ContentAddressing(crate::error::ContentAddressingError::manifest_error(
                e.to_string(),
            ))
        })?;

        // Step 4: Store manifest
        let manifest_bytes = manifest.to_bytes().map_err(|e| {
            RegistryError::ContentAddressing(crate::error::ContentAddressingError::manifest_error(
                e.to_string(),
            ))
        })?;
        let manifest_hash = ContentAddress::hash(&manifest_bytes);
        let manifest_key = ContentAddress::manifest_key(&manifest_hash);
        self.storage
            .put(&manifest_key, manifest_bytes)
            .await
            .map_err(|e| RegistryError::Storage(StorageError::backend(e.to_string())))?;

        // Step 5: Update reference (tag)
        let ref_key = ContentAddress::ref_key(namespace, tag);
        self.storage
            .put(&ref_key, manifest_hash.as_bytes().to_vec())
            .await
            .map_err(|e| RegistryError::Storage(StorageError::backend(e.to_string())))?;

        // Return the manifest hash for content-addressable access
        Ok(manifest_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bundle::TemplateMetadata, storage::blob_storage::MemoryStorage};

    fn create_test_bundle() -> TemplateBundle {
        let metadata = TemplateMetadata::new("Test Template", "test@example.com");
        let main_content = br#"#let data = json.decode(sys.inputs.data)
= Test Template
Hello #data.name"#
            .to_vec();

        TemplateBundle::new(main_content, metadata)
            .add_file("assets/logo.png", b"fake_png_data".to_vec())
            .with_schema(
                br#"{"type": "object", "properties": {"name": {"type": "string"}}}"#.to_vec(),
            )
    }

    #[tokio::test]
    async fn test_registry_publish() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);
        let bundle = create_test_bundle();

        let manifest_hash = registry
            .publish(bundle, "test-user/test-template", "latest")
            .await
            .unwrap();

        assert!(manifest_hash.starts_with("sha256:"));
        assert_eq!(manifest_hash.len(), 71); // "sha256:" + 64 hex chars
    }

    #[tokio::test]
    async fn test_registry_publish_stores_all_components() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);
        let bundle = create_test_bundle();

        let manifest_hash = registry
            .publish(bundle, "test-user/test-template", "latest")
            .await
            .unwrap();

        // Check that all components were stored
        let storage_ref = &registry.storage;

        // Should have stored 3 blobs (main.typ, assets/logo.png, schema.json)
        // Plus 1 manifest, plus 1 reference
        // Total: 5 items
        assert_eq!(storage_ref.len(), 5);

        // Verify reference points to manifest hash
        let ref_key = ContentAddress::ref_key("test-user/test-template", "latest");
        let stored_manifest_hash = storage_ref.get(&ref_key).await.unwrap();
        assert_eq!(
            String::from_utf8(stored_manifest_hash).unwrap(),
            manifest_hash
        );
    }

    #[tokio::test]
    async fn test_registry_publish_content_addressable() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);

        // Create identical bundles
        let metadata1 = TemplateMetadata::new("Test Template", "test@example.com");
        let metadata2 = TemplateMetadata::new("Test Template", "test@example.com");
        let main_content = br#"#let data = json.decode(sys.inputs.data)
= Test Template
Hello #data.name"#
            .to_vec();

        let bundle1 = TemplateBundle::new(main_content.clone(), metadata1)
            .add_file("assets/logo.png", b"fake_png_data".to_vec())
            .with_schema(
                br#"{"type": "object", "properties": {"name": {"type": "string"}}}"#.to_vec(),
            );

        let bundle2 = TemplateBundle::new(main_content, metadata2)
            .add_file("assets/logo.png", b"fake_png_data".to_vec())
            .with_schema(
                br#"{"type": "object", "properties": {"name": {"type": "string"}}}"#.to_vec(),
            );

        let hash1 = registry
            .publish(bundle1, "user1/template", "v1")
            .await
            .unwrap();

        let hash2 = registry
            .publish(bundle2, "user2/template", "v1")
            .await
            .unwrap();

        // Same content should produce same manifest hash
        // The namespace doesn't affect the manifest content, only where the reference is stored
        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_registry_publish_invalid_bundle() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);

        // Create bundle with empty metadata (should fail validation)
        let metadata = TemplateMetadata::new("", "test@example.com");
        let bundle = TemplateBundle::new(b"test content".to_vec(), metadata);

        let result = registry
            .publish(bundle, "test-user/test-template", "latest")
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::Template(_)));
    }
}
