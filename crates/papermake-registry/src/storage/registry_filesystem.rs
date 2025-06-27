//! Registry file system implementation for Typst integration
//!
//! This module provides a bridge between papermake's TypstFileSystem trait
//! and the registry's file storage, allowing TypstWorld to load template files
//! and assets from S3 during rendering.

use super::{FileStorage, s3_storage::S3Storage};
use crate::{error::{RegistryError, Result}, template_ref::TemplateRef};
use async_trait::async_trait;
use papermake::TypstFileSystem;
// FileError is used in map_err closure
use std::sync::Arc;

/// File system implementation that loads files from registry storage
/// for use with TypstWorld during template rendering
pub struct RegistryFileSystem {
    /// File storage backend (S3)
    file_storage: Arc<dyn FileStorage>,

    /// Template reference for scoping file access
    template_ref: TemplateRef,
}

impl RegistryFileSystem {
    /// Create a new registry file system for a specific template
    pub fn new(file_storage: Arc<dyn FileStorage>, template_ref: TemplateRef) -> Self {
        Self {
            file_storage,
            template_ref,
        }
    }
}

#[async_trait]
impl TypstFileSystem for RegistryFileSystem {
    async fn get_file(&self, path: &str) -> std::result::Result<Vec<u8>, papermake::FileError> {
        // Determine the S3 key based on file type and path
        let s3_key = if path.ends_with(".typ") {
            // Typst source files - could be imports or includes
            S3Storage::template_asset_key(&self.template_ref, path)
        } else {
            // Other assets (fonts, images, etc.)
            S3Storage::template_asset_key(&self.template_ref, path)
        };

        // Load from S3
        self.file_storage
            .get_file(&s3_key)
            .await
            .map_err(|e| match e {
                RegistryError::Storage(msg) if msg.contains("not found") => {
                    papermake::FileError::NotFound(path.into())
                }
                _ => papermake::FileError::Other(Some(format!("Storage error: {}", e).into())),
            })
    }
}

/// Convenience functions for working with template files
impl RegistryFileSystem {
    /// Store a template source file
    pub async fn store_template_source(&self, content: &str) -> Result<()> {
        let key = S3Storage::template_source_key(&self.template_ref);
        self.file_storage.put_file(&key, content.as_bytes()).await
    }

    /// Store a template asset
    pub async fn store_template_asset(&self, asset_path: &str, content: &[u8]) -> Result<()> {
        let key = S3Storage::template_asset_key(&self.template_ref, asset_path);
        self.file_storage.put_file(&key, content).await
    }

    /// List all assets for this template
    pub async fn list_template_assets(&self) -> Result<Vec<String>> {
        let prefix = S3Storage::template_prefix(&self.template_ref);
        let keys = self.file_storage.list_files(&prefix).await?;

        // Filter to only assets (exclude source files) and strip prefix
        let assets: Vec<String> = keys
            .into_iter()
            .filter(|key| key.contains("/assets/") && !key.ends_with("/source.typ"))
            .filter_map(|key| {
                // Extract the asset path from the full S3 key
                // Format: "templates/{org}/{name}/assets/{asset_path}" or "templates/{name}/assets/{asset_path}"
                let parts: Vec<&str> = key.split('/').collect();
                if let Some(assets_index) = parts.iter().position(|&part| part == "assets") {
                    if assets_index + 1 < parts.len() {
                        Some(parts[assets_index + 1..].join("/"))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        Ok(assets)
    }

    /// Delete all files for this template
    pub async fn delete_template_files(&self) -> Result<()> {
        let prefix = S3Storage::template_prefix(&self.template_ref);
        let keys = self.file_storage.list_files(&prefix).await?;

        for key in keys {
            self.file_storage.delete_file(&key).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::FileStorage;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // Mock file storage for testing
    struct MockFileStorage {
        files: Mutex<HashMap<String, Vec<u8>>>,
    }

    impl MockFileStorage {
        fn new() -> Self {
            Self {
                files: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl FileStorage for MockFileStorage {
        async fn put_file(&self, key: &str, content: &[u8]) -> Result<()> {
            self.files
                .lock()
                .unwrap()
                .insert(key.to_string(), content.to_vec());
            Ok(())
        }

        async fn get_file(&self, key: &str) -> Result<Vec<u8>> {
            self.files
                .lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or_else(|| RegistryError::Storage(format!("File {} not found", key)))
        }

        async fn file_exists(&self, key: &str) -> Result<bool> {
            Ok(self.files.lock().unwrap().contains_key(key))
        }

        async fn delete_file(&self, key: &str) -> Result<()> {
            self.files.lock().unwrap().remove(key);
            Ok(())
        }

        async fn list_files(&self, prefix: &str) -> Result<Vec<String>> {
            let files = self.files.lock().unwrap();
            let matching_keys: Vec<String> = files
                .keys()
                .filter(|key| key.starts_with(prefix))
                .cloned()
                .collect();
            Ok(matching_keys)
        }
    }

    #[tokio::test]
    async fn test_registry_filesystem() {
        let storage = Arc::new(MockFileStorage::new());
        let template_ref = TemplateRef::new("test-template").with_tag("v1");
        let fs = RegistryFileSystem::new(storage.clone(), template_ref);

        // Store a template asset
        let asset_content = b"Hello, World!";
        fs.store_template_asset("fonts/Arial.ttf", asset_content)
            .await
            .unwrap();

        // Retrieve via TypstFileSystem interface
        let retrieved = fs.get_file("fonts/Arial.ttf").await.unwrap();
        assert_eq!(retrieved, asset_content);

        // Test file not found
        let result = fs.get_file("nonexistent.txt").await;
        assert!(matches!(result, Err(papermake::FileError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_template_operations() {
        let storage = Arc::new(MockFileStorage::new());
        let template_ref = TemplateRef::new("test-template").with_tag("v1");
        let fs = RegistryFileSystem::new(storage.clone(), template_ref);

        // Store template source
        fs.store_template_source("= Hello World")
            .await
            .unwrap();

        // Store multiple assets
        fs.store_template_asset("fonts/Arial.ttf", b"font data")
            .await
            .unwrap();
        fs.store_template_asset("images/logo.png", b"image data")
            .await
            .unwrap();

        // List assets
        let assets = fs.list_template_assets().await.unwrap();
        assert_eq!(assets.len(), 2);
        assert!(assets.contains(&"fonts/Arial.ttf".to_string()));
        assert!(assets.contains(&"images/logo.png".to_string()));

        // Delete all template files
        fs.delete_template_files().await.unwrap();

        // Verify files are gone
        let assets_after = fs.list_template_assets().await.unwrap();
        assert!(assets_after.is_empty());
    }
}
