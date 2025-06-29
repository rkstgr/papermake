//! Storage abstraction for registry data

use async_trait::async_trait;

// Re-export for convenience
pub use papermake::FileError;

pub mod blob_storage;

pub use blob_storage::BlobStorage;

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
