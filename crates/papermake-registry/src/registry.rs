//! High-level registry interface for template management

use crate::{
    entities::*,
    error::Result,
    storage::{FileStorage, MetadataStorage},
    template_ref::TemplateRef,
};
use async_trait::async_trait;
use papermake::Template;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// High-level template registry interface
#[async_trait]
pub trait TemplateRegistry {
    /// Get a template by Docker-style reference (org/name:tag[@digest])
    async fn get_template(&self, template_ref: &TemplateRef) -> Result<TemplateEntry>;

    /// Get a template by name and tag
    async fn get_template_by_name(&self, name: &str, tag: &str) -> Result<TemplateEntry>;

    /// List all tags for a template by name
    async fn list_tags(&self, name: &str) -> Result<Vec<String>>;

    /// Delete a template by reference
    async fn delete_template(&self, template_ref: &TemplateRef) -> Result<()>;

    /// Search templates by name/description
    async fn search_templates(&self, query: &str) -> Result<Vec<TemplateEntry>>;

    /// Render a template by Docker-style reference
    async fn render_template(
        &self,
        template_ref: &TemplateRef,
        data: &serde_json::Value,
    ) -> Result<RenderJob>;

    /// Get render job status
    async fn get_render_job(&self, job_id: &str) -> Result<RenderJob>;

    /// Find cached render result by template reference
    async fn find_cached_render(
        &self,
        template_ref: &str,
        data: &serde_json::Value,
    ) -> Result<Option<RenderJob>>;

    /// Get the latest version of a template by name
    async fn get_latest_template(&self, name: &str) -> Result<TemplateEntry>;

    /// List all templates (all latest versions)
    async fn list_templates(&self) -> Result<Vec<TemplateEntry>>;

    /// List all render jobs
    async fn list_render_jobs(&self) -> Result<Vec<RenderJob>>;

    /// Publish a new template with Docker-style reference
    async fn publish_template(
        &self,
        template: Template,
        template_ref: TemplateRef,
        author: String,
    ) -> Result<TemplateEntry>;

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
    async fn get_template(&self, template_ref: &TemplateRef) -> Result<TemplateEntry> {
        self.metadata_storage.get_template(template_ref).await
    }

    async fn get_template_by_name(&self, name: &str, tag: &str) -> Result<TemplateEntry> {
        let template_ref = TemplateRef::new(name).with_tag(tag);
        self.metadata_storage.get_template(&template_ref).await
    }

    async fn list_tags(&self, name: &str) -> Result<Vec<String>> {
        self.metadata_storage.list_template_tags(name).await
    }

    async fn delete_template(&self, template_ref: &TemplateRef) -> Result<()> {
        self.metadata_storage.delete_template(template_ref).await
    }

    async fn search_templates(&self, query: &str) -> Result<Vec<TemplateEntry>> {
        self.metadata_storage.search_templates(query).await
    }

    async fn render_template(
        &self,
        template_ref: &TemplateRef,
        data: &serde_json::Value,
    ) -> Result<RenderJob> {
        let job = RenderJob::new(template_ref.clone(), data.clone());

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
        template_ref: &str,
        data: &serde_json::Value,
    ) -> Result<Option<RenderJob>> {
        // Generate data hash for lookup
        let data_string = serde_json::to_string(data).unwrap_or_default();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        data_string.hash(&mut hasher);
        let data_hash = format!("{:x}", hasher.finish());

        self.metadata_storage
            .find_cached_render(template_ref, &data_hash)
            .await
    }

    async fn get_latest_template(&self, name: &str) -> Result<TemplateEntry> {
        let template_ref = TemplateRef::new(name).with_tag("latest");
        self.metadata_storage.get_template(&template_ref).await
    }

    async fn list_templates(&self) -> Result<Vec<TemplateEntry>> {
        self.metadata_storage.list_all_templates().await
    }

    async fn list_render_jobs(&self) -> Result<Vec<RenderJob>> {
        self.metadata_storage.list_all_render_jobs().await
    }

    async fn publish_template(
        &self,
        template: Template,
        template_ref: TemplateRef,
        author: String,
    ) -> Result<TemplateEntry> {
        // Create template entry
        let template_entry = TemplateEntry::new(template, template_ref, author);

        // Save to storage
        self.metadata_storage.save_template(&template_entry).await?;

        Ok(template_entry)
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

    /// Access to metadata storage (for draft operations)
    pub fn metadata_storage(&self) -> &Arc<dyn MetadataStorage> {
        &self.metadata_storage
    }
}
