//! Storage abstraction for registry data

use crate::{entities::*, error::Result, template_ref::TemplateRef};
use async_trait::async_trait;
// Re-export for convenience
pub use papermake::FileError;

// SQLite implementation
#[cfg(feature = "sqlite")]
pub mod sqlite_storage;

// TiKV implementation
#[cfg(feature = "tikv")]
pub mod tikv_storage;

// S3 implementation  
#[cfg(feature = "s3")]
pub mod s3_storage;

// Registry file system for Typst integration
pub mod registry_filesystem;

// Legacy implementations (will be removed)
#[cfg(feature = "fs")]
pub mod file_storage;
#[cfg(feature = "postgres")]
pub mod postgres;

/// Metadata storage trait for template and render job data
///
/// This trait handles structured data stored in TiKV
#[async_trait]
pub trait MetadataStorage: Send + Sync {
    // === Template Management ===

    /// Save a template entry
    async fn save_template_entry(&self, template: &TemplateEntry) -> Result<()>;

    /// Get a template entry by reference string
    async fn get_template_entry(
        &self,
        template_ref: &str,
    ) -> Result<TemplateEntry>;

    /// Get a template entry by name and tag
    async fn get_template_entry_by_name_tag(
        &self,
        name: &str,
        tag: &str,
    ) -> Result<TemplateEntry>;

    /// List all tags for a template by name
    async fn list_template_tags(&self, name: &str) -> Result<Vec<String>>;

    /// List all template entries for a given name
    async fn list_template_entries(&self, name: &str) -> Result<Vec<TemplateEntry>>;

    /// Delete a specific template entry by reference
    async fn delete_template_entry(&self, template_ref: &str) -> Result<()>;

    /// Delete a template entry by name and tag
    async fn delete_template_entry_by_name_tag(&self, name: &str, tag: &str) -> Result<()>;

    /// Search templates by name/description  
    async fn search_templates(&self, query: &str) -> Result<Vec<TemplateEntry>>;

    // === Draft Management ===

    /// Save a draft template entry
    async fn save_draft(&self, template: &TemplateEntry) -> Result<()>;

    /// Get a draft template entry by name
    async fn get_draft(&self, name: &str) -> Result<Option<TemplateEntry>>;

    /// Delete a draft template entry
    async fn delete_draft(&self, name: &str) -> Result<()>;

    /// Check if a template has a draft
    async fn has_draft(&self, name: &str) -> Result<bool>;

    /// Get the next tag number for auto-incrementing (e.g., "v3" after "v2")
    async fn get_next_tag_number(&self, name: &str) -> Result<u64>;

    // === Render Job Management ===

    /// Save a render job
    async fn save_render_job(&self, job: &RenderJob) -> Result<()>;

    /// Get a render job by ID
    async fn get_render_job(&self, job_id: &str) -> Result<RenderJob>;

    /// Find render job by template reference and data hash (for caching)
    async fn find_render_job_by_hash(
        &self,
        template_ref: &str,
        data_hash: &str,
    ) -> Result<Option<RenderJob>>;

    /// Find render job by template name, tag and data hash
    async fn find_render_job_by_name_tag_hash(
        &self,
        name: &str,
        tag: &str,
        data_hash: &str,
    ) -> Result<Option<RenderJob>>;

    /// List render jobs for a template by reference
    async fn list_render_jobs_by_template(
        &self,
        template_ref: &str,
    ) -> Result<Vec<RenderJob>>;

    /// List render jobs for a template by name and optional tag
    async fn list_render_jobs_by_name(
        &self,
        name: &str,
        tag: Option<&str>,
    ) -> Result<Vec<RenderJob>>;

    /// List all templates (latest tag of each)
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
}

/// File system abstraction for Typst rendering
///
/// This trait provides file access to TypstWorld during rendering
#[async_trait]
pub trait TypstFileSystem: Send + Sync {
    /// Get file content by path
    async fn get_file(&self, path: &str) -> std::result::Result<Vec<u8>, papermake::FileError>;
}

// Legacy trait removed - no backward compatibility needed
