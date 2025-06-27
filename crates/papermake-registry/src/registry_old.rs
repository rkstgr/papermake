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
    async fn get_template(&self, template_ref: &str) -> Result<TemplateEntry>;

    /// Get a template by name and tag
    async fn get_template_by_name(
        &self,
        name: &str,
        tag: &str,
    ) -> Result<TemplateEntry>;

    /// List all tags for a template by name
    async fn list_tags(&self, name: &str) -> Result<Vec<String>>;

    /// Delete a template by reference
    async fn delete_template(&self, template_ref: &str) -> Result<()>;

    /// Search templates by name/description
    async fn search_templates(&self, query: &str) -> Result<Vec<TemplateEntry>>;

    /// Render a template by Docker-style reference
    async fn render_template(
        &self,
        template_ref: &str,
        data: &serde_json::Value,
    ) -> Result<RenderJob>;

    /// Render a template by name and tag
    async fn render_template_by_name(
        &self,
        name: &str,
        tag: &str,
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

    // === Draft Management ===

    /// Save a draft template
    async fn save_draft(
        &self,
        template: Template,
        template_ref: TemplateRef,
        author: String,
    ) -> Result<TemplateEntry>;

    /// Get a draft template by name
    async fn get_draft(&self, name: &str) -> Result<Option<TemplateEntry>>;

    /// Delete a draft template
    async fn delete_draft(&self, name: &str) -> Result<()>;

    /// Check if a template has a draft
    async fn has_draft(&self, name: &str) -> Result<bool>;

    /// Publish a draft as a new version
    async fn publish_draft(&self, name: &str) -> Result<TemplateEntry>;
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
    async fn get_template(&self, name: &str, tag: &str) -> Result<VersionedTemplate> {
        self.metadata_storage
            .get_versioned_template_by_name(name, tag)
            .await
    }

    async fn get_template_by_name(
        &self,
        template_name: &str,
        tag: &str,
    ) -> Result<VersionedTemplate> {
        self.metadata_storage
            .get_versioned_template_by_name(template_name, tag)
            .await
    }


    async fn delete_template(&self, id: &TemplateId, version: u64) -> Result<()> {
        self.metadata_storage
            .delete_template_version(id, version)
            .await
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
        // Convert legacy template_id to template_name for job creation
        let template_name = template_id.as_ref().to_string();
        let job = RenderJob::new(template_name, format!("v{}", version), data.clone());

        // Save the job
        self.metadata_storage.save_render_job(&job).await?;

        // TODO: Implement actual rendering logic
        // For now, just return the pending job
        Ok(job)
    }

    async fn render_template_by_name(
        &self,
        template_name: &str,
        tag: &str,
        data: &serde_json::Value,
    ) -> Result<RenderJob> {
        let job = RenderJob::new(template_name.to_string(), tag.to_string(), data.clone());

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

        let template_name = template_id.as_ref().to_string();
        let template_tag = format!("v{}", version);
        self.metadata_storage
            .find_render_job_by_hash_name(&template_name, &template_tag, &data_hash)
            .await
    }

    async fn get_latest_template(&self, id: &TemplateId) -> Result<VersionedTemplate> {
        let versions = self.metadata_storage.list_template_versions(id).await?;
        let latest_version = versions.into_iter().max().ok_or_else(|| {
            crate::RegistryError::TemplateNotFound(format!(
                "No versions found for template {:?}",
                id
            ))
        })?;
        self.metadata_storage
            .get_versioned_template(id, latest_version)
            .await
    }


    async fn list_templates(&self) -> Result<Vec<VersionedTemplate>> {
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
        self.metadata_storage
            .save_template_entry(&template_entry)
            .await?;

        Ok(template_entry)
    }

    async fn update_render_job(&self, job: &RenderJob) -> Result<()> {
        self.metadata_storage.save_render_job(job).await
    }

    // === Draft Management ===

    async fn save_draft(
        &self,
        template: Template,
        template_ref: TemplateRef,
        author: String,
    ) -> Result<TemplateEntry> {
        let draft_template = TemplateEntry::new_draft(template, template_ref, author);
        self.metadata_storage.save_draft(&draft_template).await?;
        Ok(draft_template)
    }

    async fn get_draft(&self, name: &str) -> Result<Option<TemplateEntry>> {
        self.metadata_storage.get_draft(name).await
    }

    async fn delete_draft(&self, name: &str) -> Result<()> {
        self.metadata_storage.delete_draft(name).await
    }

    async fn has_draft(&self, name: &str) -> Result<bool> {
        self.metadata_storage.has_draft(name).await
    }

    async fn publish_draft(&self, name: &str) -> Result<TemplateEntry> {
        // Get the draft
        let draft = self
            .metadata_storage
            .get_draft(name)
            .await?
            .ok_or_else(|| {
                crate::RegistryError::TemplateNotFound(format!(
                    "No draft found for template {}",
                    name
                ))
            })?;

        // Get next tag number
        let next_tag = self
            .metadata_storage
            .get_next_tag_number(name)
            .await?;

        // Create published version from draft
        let published_template = draft.publish(format!("v{}", next_tag));

        // Save the published version
        self.metadata_storage
            .save_template_entry(&published_template)
            .await?;

        // Delete the draft
        self.metadata_storage.delete_draft(name).await?;

        Ok(published_template)
    }
}

impl DefaultRegistry {
    /// Access to file storage (for direct operations like PDF downloads)
    pub fn file_storage(&self) -> &Arc<dyn FileStorage> {
        &self.file_storage
    }
}
