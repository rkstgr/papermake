use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{
    address::ContentAddress,
    bundle::{TemplateBundle, TemplateInfo},
    error::{RegistryError, StorageError},
    manifest::Manifest,
    reference::Reference,
    render_storage::{AnalyticsQuery, AnalyticsResult, RenderRecord, RenderStorage},
    storage::{BlobStorage, filesystem::RegistryFileSystem},
};

/// Core registry for template publishing and resolution
pub struct Registry<S: BlobStorage, R: RenderStorage> {
    storage: Arc<S>,
    render_storage: Option<Arc<R>>,
}

/// Result of a render operation with tracking
#[derive(Debug)]
pub struct RenderResult {
    /// UUIDv7 for the render operation
    pub render_id: String,
    /// Generated PDF bytes
    pub pdf_bytes: Vec<u8>,
    /// SHA-256 hash of the PDF
    pub pdf_hash: String,
    /// Render duration in milliseconds
    pub duration_ms: u32,
}

// Implementation for Registry with blob storage only
impl<S: BlobStorage + 'static, R: RenderStorage> Registry<S, R> {
    /// Create a new registry with the given storage backend
    pub fn new(storage: S, render_storage: R) -> Self {
        Self {
            storage: Arc::new(storage),
            render_storage: Some(Arc::new(render_storage)),
        }
    }
}

// Implementation for Registry with both blob and render storage
impl<S: BlobStorage + 'static, R: RenderStorage + 'static> Registry<S, R> {
    /// Create a new registry with both blob and render storage
    pub fn new_with_render_storage(storage: S, render_storage: R) -> Self {
        Self {
            storage: Arc::new(storage),
            render_storage: Some(Arc::new(render_storage)),
        }
    }
}

// Shared implementation for all registry types
impl<S: BlobStorage + 'static, R: RenderStorage + 'static> Registry<S, R> {
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

    /// Resolve a template reference to its manifest hash
    ///
    /// This method implements the "tag → manifest hash lookup" workflow:
    /// 1. Parses the reference string (namespace/name:tag[@hash])
    /// 2. Looks up the reference in storage to get the manifest hash
    /// 3. Optionally verifies the hash if provided in the reference
    /// 4. Returns the manifest hash for content-addressable access
    ///
    /// # Examples
    /// - `"invoice:latest"` → resolves official template
    /// - `"john/invoice:v1.0.0"` → resolves user template
    /// - `"john/invoice:latest@sha256:abc123"` → resolves with hash verification
    pub async fn resolve(&self, reference: &str) -> Result<String, RegistryError> {
        // Step 1: Parse the reference
        let parsed_ref = Reference::parse(reference)?;

        // Step 2: Build the namespace/tag path for storage lookup
        let namespace_path = match &parsed_ref.namespace {
            Some(ns) => format!("{}/{}", ns, parsed_ref.name),
            None => parsed_ref.name.clone(),
        };
        let tag = parsed_ref.tag_or_default();
        let ref_key = ContentAddress::ref_key(&namespace_path, tag);

        // Step 3: Look up the manifest hash from storage
        let manifest_hash_bytes = self.storage.get(&ref_key).await.map_err(|e| match e {
            crate::storage::blob_storage::StorageError::NotFound(_) => {
                RegistryError::Template(crate::error::TemplateError::not_found(reference))
            }
            _ => RegistryError::Storage(StorageError::backend(e.to_string())),
        })?;

        let manifest_hash = String::from_utf8(manifest_hash_bytes).map_err(|e| {
            RegistryError::Storage(StorageError::backend(format!(
                "Invalid UTF-8 in stored manifest hash: {}",
                e
            )))
        })?;

        // Step 4: Verify hash if provided in reference
        if let Some(expected_hash) = &parsed_ref.hash {
            if &manifest_hash != expected_hash {
                return Err(RegistryError::Reference(
                    crate::error::ReferenceError::hash_mismatch(
                        reference.to_string(),
                        expected_hash.clone(),
                        manifest_hash,
                    ),
                ));
            }
        }

        // Return the manifest hash
        Ok(manifest_hash)
    }

    /// Render a template to PDF using JSON data
    ///
    /// This method implements the end-to-end template rendering workflow:
    /// 1. Resolves the template reference to get the manifest hash
    /// 2. Loads the manifest from storage to get file mappings
    /// 3. Creates a RegistryFileSystem that resolves files through blob storage
    /// 4. Uses papermake to render the template with the provided data
    ///
    /// # Arguments
    /// * `reference` - Template reference (e.g., "john/invoice:latest")
    /// * `data` - JSON data to inject into the template
    ///
    /// # Returns
    /// Returns the PDF bytes on successful rendering
    ///
    /// # Examples
    /// ```rust,no_run
    /// use papermake_registry::Registry;
    /// use papermake_registry::storage::blob_storage::MemoryStorage;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = MemoryStorage::new();
    /// let registry = Registry::new(storage);
    ///
    /// let pdf_bytes = registry.render(
    ///     "john/invoice:latest",
    ///     &json!({
    ///         "customer_name": "Acme Corp",
    ///         "total": "$1,000.00"
    ///     })
    /// ).await?;
    ///
    /// println!("Generated PDF: {} bytes", pdf_bytes.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn render(
        &self,
        reference: &str,
        data: &serde_json::Value,
    ) -> Result<Vec<u8>, RegistryError> {
        // Step 1: Resolve the template reference to get manifest hash
        let manifest_hash = self.resolve(reference).await?;

        // Step 2: Load the manifest from storage
        let manifest_key = ContentAddress::manifest_key(&manifest_hash);
        let manifest_bytes = self.storage.get(&manifest_key).await.map_err(|e| {
            RegistryError::Storage(StorageError::backend(format!(
                "Failed to load manifest {}: {}",
                manifest_hash, e
            )))
        })?;

        let manifest = Manifest::from_bytes(&manifest_bytes).map_err(|e| {
            RegistryError::ContentAddressing(crate::error::ContentAddressingError::manifest_error(
                e.to_string(),
            ))
        })?;

        // Step 3: Get the entrypoint content
        let entrypoint_hash = manifest.entrypoint_hash().ok_or_else(|| {
            RegistryError::Template(crate::error::TemplateError::invalid(
                "Manifest missing entrypoint hash",
            ))
        })?;

        let entrypoint_key = ContentAddress::blob_key(entrypoint_hash);
        let entrypoint_bytes = self.storage.get(&entrypoint_key).await.map_err(|e| {
            RegistryError::Storage(StorageError::backend(format!(
                "Failed to load entrypoint file: {}",
                e
            )))
        })?;

        let entrypoint_content = String::from_utf8(entrypoint_bytes).map_err(|e| {
            RegistryError::Template(crate::error::TemplateError::invalid(format!(
                "Entrypoint file is not valid UTF-8: {}",
                e
            )))
        })?;

        // Step 4: Create RegistryFileSystem for resolving imports
        let file_system = RegistryFileSystem::new(self.storage.clone(), manifest)?;

        // Step 5: Render the template using papermake
        let render_result =
            papermake::render_template(entrypoint_content, Arc::new(file_system), data)
                .map_err(RegistryError::Compilation)?;

        // Check if rendering was successful
        if render_result.success {
            render_result.pdf.ok_or_else(|| {
                RegistryError::Template(crate::error::TemplateError::invalid(
                    "Rendering succeeded but no PDF was generated",
                ))
            })
        } else {
            // Collect error messages
            let error_messages: Vec<String> =
                render_result.errors.iter().map(|e| e.to_string()).collect();

            Err(RegistryError::Template(
                crate::error::TemplateError::invalid(format!(
                    "Template rendering failed: {}",
                    error_messages.join("; ")
                )),
            ))
        }
    }

    /// List all templates in the registry
    ///
    /// This method scans all references in storage and groups them by template
    /// to provide a comprehensive list of available templates with their metadata.
    ///
    /// # Returns
    /// Returns a vector of `TemplateInfo` structs containing:
    /// - Template name and namespace
    /// - Available tags
    /// - Latest manifest hash (from "latest" tag or newest tag)
    /// - Template metadata from the manifest
    ///
    /// # Examples
    /// ```rust,no_run
    /// use papermake_registry::Registry;
    /// use papermake_registry::storage::blob_storage::MemoryStorage;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = MemoryStorage::new();
    /// let registry = Registry::new(storage);
    ///
    /// let templates = registry.list_templates().await?;
    /// for template in templates {
    ///     println!("Template: {} ({})", template.name, template.full_name());
    ///     println!("  Tags: {:?}", template.tags);
    ///     println!("  Author: {}", template.metadata.author);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_templates(&self) -> Result<Vec<TemplateInfo>, RegistryError> {
        // Step 1: List all reference keys with "refs/" prefix
        let ref_keys = self
            .storage
            .list_keys("refs/")
            .await
            .map_err(|e| RegistryError::Storage(StorageError::backend(e.to_string())))?;

        // Step 2: Parse reference keys to extract template information
        let mut templates_map: BTreeMap<String, (Vec<String>, Option<String>)> = BTreeMap::new();

        for ref_key in ref_keys {
            // Parse reference key: "refs/{namespace}/{tag}" or "refs/{namespace}/{name}/{tag}"
            if let Some(parsed) = Self::parse_ref_key(&ref_key) {
                let (namespace_path, tag) = parsed;

                // Add this tag to the template's tag list
                let entry = templates_map
                    .entry(namespace_path.clone())
                    .or_insert((Vec::new(), None));
                entry.0.push(tag.clone());

                // If this is the "latest" tag, remember it for getting metadata
                if tag == "latest" {
                    entry.1 = Some(ref_key.clone());
                }
            }
        }

        // Step 3: For each unique template, resolve metadata
        let mut template_infos = Vec::new();

        for (namespace_path, (mut tags, latest_ref_key)) in templates_map {
            // Sort tags for consistent output
            tags.sort();

            // Use "latest" tag if available, otherwise use the first tag alphabetically
            let ref_key_to_use = latest_ref_key.unwrap_or_else(|| {
                format!(
                    "refs/{}/{}",
                    namespace_path,
                    tags.first().unwrap_or(&"latest".to_string())
                )
            });

            // Get the manifest hash for this reference
            match self.storage.get(&ref_key_to_use).await {
                Ok(manifest_hash_bytes) => {
                    let manifest_hash = match String::from_utf8(manifest_hash_bytes) {
                        Ok(hash) => hash,
                        Err(_) => continue, // Skip invalid UTF-8
                    };

                    // Load the manifest to get metadata
                    let manifest_key = ContentAddress::manifest_key(&manifest_hash);
                    match self.storage.get(&manifest_key).await {
                        Ok(manifest_bytes) => {
                            match Manifest::from_bytes(&manifest_bytes) {
                                Ok(manifest) => {
                                    // Parse namespace and name from namespace_path
                                    let (namespace, name) =
                                        Self::parse_namespace_path(&namespace_path);

                                    let template_info = TemplateInfo::new(
                                        name,
                                        namespace,
                                        tags,
                                        manifest_hash,
                                        manifest.metadata.clone(),
                                    );

                                    template_infos.push(template_info);
                                }
                                Err(_) => {
                                    // Skip templates with invalid manifests
                                    continue;
                                }
                            }
                        }
                        Err(_) => {
                            // Skip templates with missing manifests
                            continue;
                        }
                    }
                }
                Err(_) => {
                    // Skip invalid references
                    continue;
                }
            }
        }

        // Sort templates by full name for consistent output
        template_infos.sort_by(|a, b| a.full_name().cmp(&b.full_name()));

        Ok(template_infos)
    }

    /// Parse a reference key to extract namespace/name path and tag
    ///
    /// Examples:
    /// - "refs/invoice/latest" -> Some(("invoice", "latest"))
    /// - "refs/john/invoice/v1.0.0" -> Some(("john/invoice", "v1.0.0"))
    /// - "invalid/key" -> None
    fn parse_ref_key(ref_key: &str) -> Option<(String, String)> {
        if !ref_key.starts_with("refs/") {
            return None;
        }

        let path = &ref_key[5..]; // Remove "refs/" prefix
        let parts: Vec<&str> = path.split('/').collect();

        if parts.len() < 2 {
            return None;
        }

        // Last part is always the tag
        let tag = parts.last().unwrap().to_string();

        // Everything else is the namespace path
        let namespace_path = parts[..parts.len() - 1].join("/");

        Some((namespace_path, tag))
    }

    /// Parse namespace path to extract namespace and name
    ///
    /// Examples:
    /// - "invoice" -> (None, "invoice")
    /// - "john/invoice" -> (Some("john"), "invoice")
    /// - "acme-corp/letterhead" -> (Some("acme-corp"), "letterhead")
    fn parse_namespace_path(namespace_path: &str) -> (Option<String>, String) {
        let parts: Vec<&str> = namespace_path.split('/').collect();

        if parts.len() == 1 {
            // No namespace, just name
            (None, parts[0].to_string())
        } else if parts.len() == 2 {
            // namespace/name
            (Some(parts[0].to_string()), parts[1].to_string())
        } else {
            // Multiple slashes - treat as namespace/name where namespace includes slashes
            let name = parts.last().unwrap().to_string();
            let namespace = parts[..parts.len() - 1].join("/");
            (Some(namespace), name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{S3Storage, bundle::TemplateMetadata, storage::blob_storage::MemoryStorage};

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

    #[tokio::test]
    async fn test_registry_resolve_basic() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);
        let bundle = create_test_bundle();

        // First publish a template
        let manifest_hash = registry
            .publish(bundle, "john/invoice", "latest")
            .await
            .unwrap();

        // Then resolve it back
        let resolved_hash = registry.resolve("john/invoice:latest").await.unwrap();

        assert_eq!(manifest_hash, resolved_hash);
    }

    #[tokio::test]
    async fn test_registry_resolve_different_reference_formats() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);
        let bundle = create_test_bundle();

        // Publish template
        let manifest_hash = registry
            .publish(bundle, "john/invoice", "v1.0.0")
            .await
            .unwrap();

        // Test different ways to resolve the same template

        // With explicit tag
        let resolved1 = registry.resolve("john/invoice:v1.0.0").await.unwrap();
        assert_eq!(manifest_hash, resolved1);

        // Without namespace (should fail since we published with namespace)
        let result = registry.resolve("invoice:v1.0.0").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::Template(_)));
    }

    #[tokio::test]
    async fn test_registry_resolve_with_hash_verification() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);
        let bundle = create_test_bundle();

        // Publish template
        let manifest_hash = registry
            .publish(bundle, "john/invoice", "latest")
            .await
            .unwrap();

        // Resolve with correct hash verification
        let reference_with_hash = format!("john/invoice:latest@{}", manifest_hash);
        let resolved_hash = registry.resolve(&reference_with_hash).await.unwrap();
        assert_eq!(manifest_hash, resolved_hash);

        // Resolve with incorrect hash verification (should fail)
        let wrong_hash = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
        let reference_with_wrong_hash = format!("john/invoice:latest@{}", wrong_hash);
        let result = registry.resolve(&reference_with_wrong_hash).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::Reference(_)));
    }

    #[tokio::test]
    async fn test_registry_resolve_default_tag() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);
        let bundle = create_test_bundle();

        // Publish with explicit "latest" tag
        let manifest_hash = registry
            .publish(bundle, "john/invoice", "latest")
            .await
            .unwrap();

        // Resolve without tag (should default to "latest")
        let resolved_hash = registry.resolve("john/invoice").await.unwrap();
        assert_eq!(manifest_hash, resolved_hash);
    }

    #[tokio::test]
    async fn test_registry_resolve_nonexistent_template() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);

        // Try to resolve a template that doesn't exist
        let result = registry.resolve("nonexistent/template:latest").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::Template(_)));
    }

    #[tokio::test]
    async fn test_registry_resolve_invalid_reference_format() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);

        // Try to resolve with invalid reference format
        let result = registry.resolve("").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::Reference(_)));

        // Try to resolve with hash only
        let result = registry.resolve("@sha256:abc123").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::Reference(_)));
    }

    #[tokio::test]
    async fn test_registry_resolve_official_template() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);
        let bundle = create_test_bundle();

        // Publish an official template (no namespace)
        let manifest_hash = registry.publish(bundle, "invoice", "latest").await.unwrap();

        // Resolve official template
        let resolved_hash = registry.resolve("invoice:latest").await.unwrap();
        assert_eq!(manifest_hash, resolved_hash);

        // Also test without explicit tag
        let resolved_hash2 = registry.resolve("invoice").await.unwrap();
        assert_eq!(manifest_hash, resolved_hash2);
    }

    #[tokio::test]
    async fn test_registry_publish_resolve_integration() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);

        // Create multiple templates with different namespaces and tags
        let metadata1 = TemplateMetadata::new("Invoice Template", "john@example.com");
        let metadata2 = TemplateMetadata::new("Invoice Template", "alice@example.com");
        let metadata3 = TemplateMetadata::new("Official Invoice", "admin@example.com");

        let content1 = b"Invoice v1 content".to_vec();
        let content2 = b"Invoice v2 content".to_vec();
        let content3 = b"Official invoice content".to_vec();

        let bundle1 = TemplateBundle::new(content1, metadata1);
        let bundle2 = TemplateBundle::new(content2, metadata2);
        let bundle3 = TemplateBundle::new(content3, metadata3);

        // Publish multiple versions and namespaces
        let hash1 = registry
            .publish(bundle1, "john/invoice", "v1.0.0")
            .await
            .unwrap();
        let hash2 = registry
            .publish(bundle2, "alice/invoice", "latest")
            .await
            .unwrap();
        let hash3 = registry
            .publish(bundle3, "invoice", "official")
            .await
            .unwrap();

        // Resolve each template
        let resolved1 = registry.resolve("john/invoice:v1.0.0").await.unwrap();
        let resolved2 = registry.resolve("alice/invoice:latest").await.unwrap();
        let resolved3 = registry.resolve("invoice:official").await.unwrap();

        assert_eq!(hash1, resolved1);
        assert_eq!(hash2, resolved2);
        assert_eq!(hash3, resolved3);

        // Test cross-namespace isolation (these should fail)
        assert!(registry.resolve("john/invoice:latest").await.is_err());
        assert!(registry.resolve("alice/invoice:v1.0.0").await.is_err());
        assert!(registry.resolve("invoice:v1.0.0").await.is_err());

        // Test with hash verification
        let reference_with_hash = format!("john/invoice:v1.0.0@{}", hash1);
        let verified_hash = registry.resolve(&reference_with_hash).await.unwrap();
        assert_eq!(hash1, verified_hash);
    }

    #[tokio::test]
    async fn test_registry_render_basic() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);
        let bundle = create_test_bundle();

        // Publish template
        let _manifest_hash = registry
            .publish(bundle, "john/invoice", "latest")
            .await
            .unwrap();

        // Render template
        let data = serde_json::json!({
            "name": "Test Customer"
        });

        let pdf_bytes = registry.render("john/invoice:latest", &data).await.unwrap();

        assert!(!pdf_bytes.is_empty());
        // PDF should start with PDF header
        assert!(pdf_bytes.starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn test_registry_render_nonexistent_template() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);

        let data = serde_json::json!({
            "name": "Test Customer"
        });

        let result = registry.render("nonexistent/template:latest", &data).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::Template(_)));
    }

    #[tokio::test]
    async fn test_registry_render_with_hash_verification() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);
        let bundle = create_test_bundle();

        // Publish template
        let manifest_hash = registry
            .publish(bundle, "john/invoice", "latest")
            .await
            .unwrap();

        // Render with hash verification
        let data = serde_json::json!({
            "name": "Test Customer"
        });

        let reference_with_hash = format!("john/invoice:latest@{}", manifest_hash);
        let pdf_bytes = registry.render(&reference_with_hash, &data).await.unwrap();

        assert!(!pdf_bytes.is_empty());
        assert!(pdf_bytes.starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn test_registry_render_with_wrong_hash() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);
        let bundle = create_test_bundle();

        // Publish template
        let _manifest_hash = registry
            .publish(bundle, "john/invoice", "latest")
            .await
            .unwrap();

        // Try to render with wrong hash
        let data = serde_json::json!({
            "name": "Test Customer"
        });

        let wrong_hash = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
        let reference_with_wrong_hash = format!("john/invoice:latest@{}", wrong_hash);

        let result = registry.render(&reference_with_wrong_hash, &data).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::Reference(_)));
    }

    #[tokio::test]
    async fn test_registry_render_template_with_imports() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);

        // Create a template with imports
        let metadata = TemplateMetadata::new("Template with Imports", "test@example.com");
        let main_content = br#"#import "header.typ": header

#header(data.title)

= Template Body
Content: #data.content"#
            .to_vec();

        let header_content = br#"#let header(title) = [
  = #title
  #line(length: 100%)
]"#
        .to_vec();

        let bundle =
            TemplateBundle::new(main_content, metadata).add_file("header.typ", header_content);

        // Publish template
        let _manifest_hash = registry
            .publish(bundle, "john/complex-template", "latest")
            .await
            .unwrap();

        // Render template
        let data = serde_json::json!({
            "title": "Invoice Template",
            "content": "This is a test invoice"
        });

        let pdf_bytes = registry
            .render("john/complex-template:latest", &data)
            .await
            .unwrap();

        assert!(!pdf_bytes.is_empty());
        assert!(pdf_bytes.starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn test_registry_render_different_data() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);
        let bundle = create_test_bundle();

        // Publish template
        let _manifest_hash = registry
            .publish(bundle, "john/invoice", "latest")
            .await
            .unwrap();

        // Render with different data sets
        let data1 = serde_json::json!({
            "name": "Customer A"
        });

        let data2 = serde_json::json!({
            "name": "Customer B"
        });

        let pdf1 = registry
            .render("john/invoice:latest", &data1)
            .await
            .unwrap();

        let pdf2 = registry
            .render("john/invoice:latest", &data2)
            .await
            .unwrap();

        // Both should be valid PDFs
        assert!(pdf1.starts_with(b"%PDF"));
        assert!(pdf2.starts_with(b"%PDF"));

        // PDFs should be different (different content)
        assert_ne!(pdf1, pdf2);
    }

    #[tokio::test]
    async fn test_registry_list_templates_empty() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);

        let templates = registry.list_templates().await.unwrap();
        assert!(templates.is_empty());
    }

    #[tokio::test]
    async fn test_registry_list_templates_single() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);
        let bundle = create_test_bundle();

        // Publish a template
        let _manifest_hash = registry
            .publish(bundle, "john/invoice", "latest")
            .await
            .unwrap();

        let templates = registry.list_templates().await.unwrap();
        assert_eq!(templates.len(), 1);

        let template = &templates[0];
        assert_eq!(template.name, "invoice");
        assert_eq!(template.namespace, Some("john".to_string()));
        assert_eq!(template.tags, vec!["latest"]);
        assert_eq!(template.metadata.name, "Test Template");
        assert_eq!(template.metadata.author, "test@example.com");
        assert_eq!(template.full_name(), "john/invoice");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_registry_list_templates_no_namespace() {
        unsafe {
            std::env::set_var("S3_ENDPOINT_URL", "http://localhost:9000");
            std::env::set_var("S3_ACCESS_KEY_ID", "minioadmin");
            std::env::set_var("S3_SECRET_ACCESS_KEY", "minioadmin");
            std::env::set_var("S3_BUCKET", "papermake-registry-test");
            std::env::set_var("S3_REGION", "us-east-1");
        }
        let storage = S3Storage::from_env().await.unwrap();
        storage.ensure_bucket().await.unwrap();
        let registry = Registry::new(storage);
        let bundle = create_test_bundle();

        // Publish a template
        let _manifest_hash = registry.publish(bundle, "invoice", "latest").await.unwrap();

        let templates = registry.list_templates().await.unwrap();
        assert_eq!(templates.len(), 1);

        let template = &templates[0];
        assert_eq!(template.name, "invoice");
        assert_eq!(template.namespace, None);
        assert_eq!(template.tags, vec!["latest"]);
        assert_eq!(template.metadata.name, "Test Template");
        assert_eq!(template.metadata.author, "test@example.com");
        assert_eq!(template.full_name(), "invoice");
    }

    #[tokio::test]
    async fn test_registry_list_templates_multiple_tags() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);
        let bundle1 = create_test_bundle();
        let bundle2 = create_test_bundle();

        // Publish same template with different tags
        registry
            .publish(bundle1, "john/invoice", "latest")
            .await
            .unwrap();
        registry
            .publish(bundle2, "john/invoice", "v1.0.0")
            .await
            .unwrap();

        let templates = registry.list_templates().await.unwrap();
        assert_eq!(templates.len(), 1);

        let template = &templates[0];
        assert_eq!(template.name, "invoice");
        assert_eq!(template.namespace, Some("john".to_string()));

        // Tags should be sorted
        let mut expected_tags = vec!["latest", "v1.0.0"];
        expected_tags.sort();
        assert_eq!(template.tags, expected_tags);
    }

    #[tokio::test]
    async fn test_registry_list_templates_multiple_templates() {
        let storage = MemoryStorage::new();
        let registry = Registry::new(storage);

        // Create different bundles
        let metadata1 = TemplateMetadata::new("Invoice Template", "john@example.com");
        let metadata2 = TemplateMetadata::new("Letter Template", "alice@example.com");
        let metadata3 = TemplateMetadata::new("Official Invoice", "admin@example.com");

        let bundle1 = TemplateBundle::new(b"invoice content".to_vec(), metadata1);
        let bundle2 = TemplateBundle::new(b"letter content".to_vec(), metadata2);
        let bundle3 = TemplateBundle::new(b"official content".to_vec(), metadata3);

        // Publish templates in different namespaces
        registry
            .publish(bundle1, "john/invoice", "latest")
            .await
            .unwrap();
        registry
            .publish(bundle2, "alice/letter", "latest")
            .await
            .unwrap();
        registry
            .publish(bundle3, "invoice", "official")
            .await
            .unwrap(); // No namespace

        let templates = registry.list_templates().await.unwrap();
        assert_eq!(templates.len(), 3);

        // Templates should be sorted by full name
        assert_eq!(templates[0].full_name(), "alice/letter");
        assert_eq!(templates[1].full_name(), "invoice");
        assert_eq!(templates[2].full_name(), "john/invoice");

        // Check individual templates
        assert_eq!(templates[0].namespace, Some("alice".to_string()));
        assert_eq!(templates[0].name, "letter");
        assert_eq!(templates[0].metadata.author, "alice@example.com");

        assert_eq!(templates[1].namespace, None);
        assert_eq!(templates[1].name, "invoice");
        assert_eq!(templates[1].metadata.author, "admin@example.com");

        assert_eq!(templates[2].namespace, Some("john".to_string()));
        assert_eq!(templates[2].name, "invoice");
        assert_eq!(templates[2].metadata.author, "john@example.com");
    }

    #[tokio::test]
    async fn test_parse_ref_key() {
        // Test valid reference keys
        assert_eq!(
            Registry::<MemoryStorage>::parse_ref_key("refs/invoice/latest"),
            Some(("invoice".to_string(), "latest".to_string()))
        );

        assert_eq!(
            Registry::<MemoryStorage>::parse_ref_key("refs/john/invoice/v1.0.0"),
            Some(("john/invoice".to_string(), "v1.0.0".to_string()))
        );

        assert_eq!(
            Registry::<MemoryStorage>::parse_ref_key("refs/org/user/template/stable"),
            Some(("org/user/template".to_string(), "stable".to_string()))
        );

        // Test invalid reference keys
        assert_eq!(
            Registry::<MemoryStorage>::parse_ref_key("invalid/key"),
            None
        );

        assert_eq!(Registry::<MemoryStorage>::parse_ref_key("refs/"), None);

        assert_eq!(
            Registry::<MemoryStorage>::parse_ref_key("refs/onlyname"),
            None
        );
    }

    #[tokio::test]
    async fn test_parse_namespace_path() {
        // Test different namespace path formats
        assert_eq!(
            Registry::<MemoryStorage>::parse_namespace_path("invoice"),
            (None, "invoice".to_string())
        );

        assert_eq!(
            Registry::<MemoryStorage>::parse_namespace_path("john/invoice"),
            (Some("john".to_string()), "invoice".to_string())
        );

        assert_eq!(
            Registry::<MemoryStorage>::parse_namespace_path("org/user/template"),
            (Some("org/user".to_string()), "template".to_string())
        );
    }
}
