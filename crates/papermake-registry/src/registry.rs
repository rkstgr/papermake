//! High-level registry interface for template management

use crate::{entities::*, error::Result, storage::{MetadataStorage, FileStorage}};
use papermake::{TemplateId, Template};
use async_trait::async_trait;
use std::sync::Arc;
use std::hash::{Hash, Hasher};

/// High-level template registry interface
#[async_trait]
pub trait TemplateRegistry {

    /// Get a specific template version
    async fn get_template(&self, id: &TemplateId, version: u64) -> Result<VersionedTemplate>;

    /// List all versions of a template
    async fn list_versions(&self, id: &TemplateId) -> Result<Vec<u64>>;

    /// Delete a template version
    async fn delete_template(&self, id: &TemplateId, version: u64) -> Result<()>;

    /// Search templates by name/description
    async fn search_templates(&self, query: &str) -> Result<Vec<(TemplateId, u64)>>;

    /// Render a template to PDF
    async fn render_template(
        &self,
        template_id: &TemplateId,
        version: u64,
        data: &serde_json::Value,
    ) -> Result<RenderJob>;

    /// Get render job status
    async fn get_render_job(&self, job_id: &str) -> Result<RenderJob>;

    /// Find cached render result
    async fn find_cached_render(
        &self,
        template_id: &TemplateId,
        version: u64,
        data: &serde_json::Value,
    ) -> Result<Option<RenderJob>>;

    /// Get the latest version of a template
    async fn get_latest_template(&self, id: &TemplateId) -> Result<VersionedTemplate>;

    /// List all templates (all latest versions)
    async fn list_templates(&self) -> Result<Vec<VersionedTemplate>>;

    /// List all render jobs
    async fn list_render_jobs(&self) -> Result<Vec<RenderJob>>;

    /// Publish a new template (auto-increment version)
    async fn publish_template(&self, template: Template, author: String) -> Result<VersionedTemplate>;

    /// Update render job status
    async fn update_render_job(&self, job: &RenderJob) -> Result<()>;
}

/// Default implementation of the template registry
pub struct DefaultRegistry {
    metadata_storage: Arc<dyn MetadataStorage>,
    file_storage: Arc<dyn FileStorage>,
}

impl DefaultRegistry {
    /// Create a new registry with the given storage backends
    pub fn new(
        metadata_storage: Arc<dyn MetadataStorage>,
        file_storage: Arc<dyn FileStorage>,
    ) -> Self {
        Self {
            metadata_storage,
            file_storage,
        }
    }
}

#[async_trait]
impl TemplateRegistry for DefaultRegistry {

    async fn get_template(&self, id: &TemplateId, version: u64) -> Result<VersionedTemplate> {
        self.metadata_storage.get_versioned_template(id, version).await
    }

    async fn list_versions(&self, id: &TemplateId) -> Result<Vec<u64>> {
        self.metadata_storage.list_template_versions(id).await
    }

    async fn delete_template(&self, id: &TemplateId, version: u64) -> Result<()> {
        self.metadata_storage.delete_template_version(id, version).await
    }

    async fn search_templates(&self, query: &str) -> Result<Vec<(TemplateId, u64)>> {
        self.metadata_storage.search_templates(query).await
    }

    async fn render_template(
        &self,
        template_id: &TemplateId,
        version: u64,
        data: &serde_json::Value,
    ) -> Result<RenderJob> {
        let job = RenderJob::new(template_id.clone(), version, data.clone());
        
        // Save the job
        self.metadata_storage.save_render_job(&job).await?;
        
        // TODO: Implement actual rendering logic
        // For now, just return the pending job
        Ok(job)
    }

    async fn get_render_job(&self, job_id: &str) -> Result<RenderJob> {
        self.metadata_storage.get_render_job(job_id).await
    }

    async fn find_cached_render(
        &self,
        template_id: &TemplateId,
        version: u64,
        data: &serde_json::Value,
    ) -> Result<Option<RenderJob>> {
        // Generate data hash for lookup
        let data_string = serde_json::to_string(data).unwrap_or_default();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        data_string.hash(&mut hasher);
        let data_hash = format!("{:x}", hasher.finish());
        
        self.metadata_storage.find_render_job_by_hash(template_id, version, &data_hash).await
    }

    async fn get_latest_template(&self, id: &TemplateId) -> Result<VersionedTemplate> {
        let versions = self.metadata_storage.list_template_versions(id).await?;
        let latest_version = versions.into_iter().max().ok_or_else(|| {
            crate::RegistryError::TemplateNotFound(format!("No versions found for template {:?}", id))
        })?;
        self.metadata_storage.get_versioned_template(id, latest_version).await
    }

    async fn list_templates(&self) -> Result<Vec<VersionedTemplate>> {
        self.metadata_storage.list_all_templates().await
    }

    async fn list_render_jobs(&self) -> Result<Vec<RenderJob>> {
        self.metadata_storage.list_all_render_jobs().await
    }

    async fn publish_template(&self, template: Template, author: String) -> Result<VersionedTemplate> {
        // Get next version number
        let versions = self.metadata_storage.list_template_versions(&template.id).await.unwrap_or_default();
        let next_version = versions.into_iter().max().unwrap_or(0) + 1;
        
        // Create versioned template
        let versioned_template = VersionedTemplate::new(template, next_version, author);
        
        // Save to storage
        self.metadata_storage.save_versioned_template(&versioned_template).await?;
        
        Ok(versioned_template)
    }

    async fn update_render_job(&self, job: &RenderJob) -> Result<()> {
        self.metadata_storage.save_render_job(job).await
    }
}

impl DefaultRegistry {
    /// Access to file storage (for direct operations like PDF downloads)
    pub fn file_storage(&self) -> &Arc<dyn FileStorage> {
        &self.file_storage
    }
}