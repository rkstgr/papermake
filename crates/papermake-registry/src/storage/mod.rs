//! Storage abstraction for registry data

use crate::{entities::*, error::Result};
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

    /// Save a versioned template
    async fn save_versioned_template(&self, template: &VersionedTemplate) -> Result<()>;

    /// Get a specific version of a template by name:version
    async fn get_versioned_template_by_name(
        &self,
        template_name: &str,
        version: &str,
    ) -> Result<VersionedTemplate>;

    /// Get a specific version of a template by ID (for backward compatibility)
    async fn get_versioned_template(
        &self,
        id: &papermake::TemplateId,
        version: u64,
    ) -> Result<VersionedTemplate>;

    /// List all versions for a template by name
    async fn list_template_versions_by_name(&self, template_name: &str) -> Result<Vec<String>>;

    /// List all versions for a template by ID (for backward compatibility)
    async fn list_template_versions(&self, id: &papermake::TemplateId) -> Result<Vec<u64>>;

    /// Delete a specific template version by name:version
    async fn delete_template_version_by_name(&self, template_name: &str, version: &str) -> Result<()>;

    /// Delete a specific template version by ID (for backward compatibility)
    async fn delete_template_version(&self, id: &papermake::TemplateId, version: u64) -> Result<()>;

    /// Search templates by name/description
    async fn search_templates(&self, query: &str) -> Result<Vec<(papermake::TemplateId, u64)>>;

    // === Draft Management ===

    /// Save a draft template
    async fn save_draft(&self, template: &VersionedTemplate) -> Result<()>;

    /// Get a draft template by name
    async fn get_draft(&self, template_name: &str) -> Result<Option<VersionedTemplate>>;

    /// Delete a draft template
    async fn delete_draft(&self, template_name: &str) -> Result<()>;

    /// Check if a template has a draft
    async fn has_draft(&self, template_name: &str) -> Result<bool>;

    /// Get the latest published version number for auto-incrementing
    async fn get_next_version_number(&self, template_name: &str) -> Result<u64>;

    // === Render Job Management ===

    /// Save a render job
    async fn save_render_job(&self, job: &RenderJob) -> Result<()>;

    /// Get a render job by ID
    async fn get_render_job(&self, job_id: &str) -> Result<RenderJob>;

    /// Find render job by template name and data hash (for caching)
    async fn find_render_job_by_hash_name(
        &self,
        template_name: &str,
        version: &str,
        data_hash: &str,
    ) -> Result<Option<RenderJob>>;

    /// Find render job by template ID and data hash (for backward compatibility)
    async fn find_render_job_by_hash(
        &self,
        template_id: &papermake::TemplateId,
        version: u64,
        data_hash: &str,
    ) -> Result<Option<RenderJob>>;

    /// List render jobs for a template by name
    async fn list_render_jobs_by_name(
        &self,
        template_name: &str,
        version: Option<&str>,
    ) -> Result<Vec<RenderJob>>;

    /// List render jobs for a template by ID (for backward compatibility)
    async fn list_render_jobs(
        &self,
        template_id: &papermake::TemplateId,
        version: Option<u64>,
    ) -> Result<Vec<RenderJob>>;

    /// List all templates (latest version of each)
    async fn list_all_templates(&self) -> Result<Vec<VersionedTemplate>>;

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

/// Legacy trait for backward compatibility
/// TODO: Remove after migration
#[async_trait]
pub trait RegistryStorage {
    // Template operations
    async fn save_versioned_template(&self, template: &VersionedTemplate) -> Result<()>;
    async fn get_versioned_template(&self, id: &papermake::TemplateId, version: u64) -> Result<VersionedTemplate>;
    async fn list_template_versions(&self, id: &papermake::TemplateId) -> Result<Vec<u64>>;
    async fn delete_template_version(&self, id: &papermake::TemplateId, version: u64) -> Result<()>;
    
    // Asset operations
    async fn save_template_asset(&self, template_id: &papermake::TemplateId, path: &str, content: &[u8]) -> Result<()>;
    async fn get_template_asset(&self, template_id: &papermake::TemplateId, path: &str) -> Result<Vec<u8>>;
    async fn list_template_assets(&self, template_id: &papermake::TemplateId) -> Result<Vec<String>>;
}
