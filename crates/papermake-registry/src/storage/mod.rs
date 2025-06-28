//! Storage abstraction for registry data

use crate::TemplateRef;
use crate::{entities::*, error::Result};
use async_trait::async_trait;
// Re-export for convenience
pub use papermake::FileError;

// SQLite implementation
#[cfg(feature = "sqlite")]
pub mod sqlite_storage;

// S3 implementation
#[cfg(feature = "s3")]
pub mod s3_storage;

// Registry file system for Typst integration
pub mod registry_filesystem;

/// Metadata storage trait for template and render job data
#[async_trait]
pub trait MetadataStorage: Send + Sync {
    // === Template Management ===

    /// Save a template entry
    async fn save_template(&self, template: &TemplateEntry) -> Result<()>;

    /// Get a template entry by Docker-style reference (org/name:tag[@digest])
    async fn get_template(&self, template_ref: &TemplateRef) -> Result<TemplateEntry>;

    /// Delete a template entry by reference
    async fn delete_template(&self, template_ref: &TemplateRef) -> Result<()>;

    /// List all tags for a template by name
    async fn list_template_tags(&self, name: &str) -> Result<Vec<String>>;

    /// Search templates by name/description
    async fn search_templates(&self, query: &str) -> Result<Vec<TemplateEntry>>;

    /// Get the next version number for auto-incrementing (e.g., 3 after v2)
    async fn get_next_version_number(&self, name: &str) -> Result<u64>;

    // === Render Job Management ===

    /// Save a render job
    async fn save_render_job(&self, job: &RenderJob) -> Result<()>;

    /// Get a render job by ID
    async fn get_render_job(&self, job_id: &str) -> Result<RenderJob>;

    /// Find render job by template reference and data hash (for caching)
    async fn find_cached_render(
        &self,
        template_ref: &str,
        data_hash: &str,
    ) -> Result<Option<RenderJob>>;

    /// List render jobs for a template by reference
    async fn list_render_jobs(&self, template_ref: &str) -> Result<Vec<RenderJob>>;

    /// List all templates
    async fn list_all_templates(&self) -> Result<Vec<TemplateEntry>>;

    /// List all render jobs
    async fn list_all_render_jobs(&self) -> Result<Vec<RenderJob>>;
}

/// File storage trait for binary data
///
/// This trait handles file operations in S3-compatible storage
#[async_trait]
pub trait FileStorage: Send + Sync {
    /// Store a file
    async fn put_file(&self, key: &str, content: &[u8]) -> Result<()>;

    /// Retrieve a file
    async fn get_file(&self, key: &str) -> Result<Vec<u8>>;

    /// Check if a file exists
    async fn file_exists(&self, key: &str) -> Result<bool>;

    /// Delete a file
    async fn delete_file(&self, key: &str) -> Result<()>;

    /// List files with a prefix
    async fn list_files(&self, prefix: &str) -> Result<Vec<String>>;

    // === Template Content Storage ===

    /// Save template content (Typst markup) to storage and return S3 key
    async fn save_template_content(&self, template_ref: &TemplateRef, content: &str) -> Result<String>;

    /// Save template schema (JSON) to storage and return S3 key
    async fn save_template_schema(&self, template_ref: &TemplateRef, schema: &papermake::Schema) -> Result<String>;

    /// Get template content from storage by S3 key
    async fn get_template_content(&self, s3_key: &str) -> Result<String>;

    /// Get template schema from storage by S3 key
    async fn get_template_schema(&self, s3_key: &str) -> Result<papermake::Schema>;

    /// Delete template files (both content and schema)
    async fn delete_template_files(&self, content_key: &str, schema_key: &str) -> Result<()>;
}

/// File system abstraction for Typst rendering
///
/// This trait provides file access to TypstWorld during rendering
#[async_trait]
pub trait TypstFileSystem: Send + Sync {
    /// Get file content by path
    async fn get_file(&self, path: &str) -> std::result::Result<Vec<u8>, papermake::FileError>;
}
