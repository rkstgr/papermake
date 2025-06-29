use std::sync::Arc;

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
}

impl<S: BlobStorage> RenderFileSystem for RegistryFileSystem<S> {
    fn get_file(&self, path: &str) -> Result<Vec<u8>, FileError> {
        let file_hash = self
            .manifest
            .files
            .get(path)
            .ok_or_else(|| FileError::NotFound(path.into()))?;

        let blob_key = ContentAddress::blob_key(file_hash);

        // Block on async call
        self.runtime
            .block_on(self.storage.get(&blob_key))
            .map_err(|_| FileError::NotFound(path.into()))
    }
}
