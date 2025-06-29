use std::{sync::Arc};

use papermake::{FileError, RenderFileSystem};

use crate::{
    BlobStorage,
    address::ContentAddress,
    error::{RegistryError, StorageError},
    manifest::Manifest,
};

pub struct RegistryFileSystem<S: BlobStorage> {
    storage: Arc<S>,
    manifest: Manifest,
    runtime: tokio::runtime::Handle,
}

impl<S: BlobStorage> RegistryFileSystem<S> {
    pub fn new(storage: Arc<S>, manifest: Manifest) -> Result<Self, RegistryError> {
        let runtime = tokio::runtime::Handle::try_current().map_err(|_| {
            RegistryError::Storage(StorageError::configuration(
                "No tokio runtime available for async operations",
            ))
        })?;

        Ok(Self {
            storage,
            manifest,
            runtime,
        })
    }

    fn normalize_path(&self, path: &str) -> String {
        // Remove leading slash if present
        let path = path.strip_prefix('/').unwrap_or(path);
        
        path.to_string()
    }
}

impl<S: BlobStorage + 'static> RenderFileSystem for RegistryFileSystem<S> {
    fn get_file(&self, path: &str) -> Result<Vec<u8>, FileError> {
        let normalized_path = self.normalize_path(path);

        let file_hash = self
            .manifest
            .files
            .get(&normalized_path)
            .ok_or_else(|| FileError::NotFound(path.into()))?;

        let blob_key = ContentAddress::blob_key(file_hash);

        let storage = self.storage.clone(); // Ensure storage is cloneable or use Arc
        let blob_key = blob_key.clone();
        let handle = self.runtime.clone();

        std::thread::spawn(move || {
            handle.block_on(storage.get(&blob_key))
        })
        .join()
        .map_err(|_| FileError::NotFound(path.into()))?
        .map_err(|_| FileError::NotFound(path.into()))
    }
}
