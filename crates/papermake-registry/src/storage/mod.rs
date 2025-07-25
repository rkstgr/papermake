//! Storage abstraction for registry data

use async_trait::async_trait;

pub mod blob_storage;
pub mod filesystem;

// Re-export for convenience
pub use blob_storage::BlobStorage;
pub use papermake::FileError;

// S3 implementation
#[cfg(feature = "s3")]
pub mod s3_storage;

/// File system abstraction for Typst rendering
///
/// This trait provides file access to TypstWorld during rendering
#[async_trait]
pub trait TypstFileSystem: Send + Sync {
    /// Get file content by path
    async fn get_file(&self, path: &str) -> std::result::Result<Vec<u8>, papermake::FileError>;
}
