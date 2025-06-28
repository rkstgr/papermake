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

    /// Update an existing template with new content/schema
    async fn update_template(
        &self,
        template_ref: &TemplateRef,
        template: Template,
    ) -> Result<TemplateEntry>;

    /// Update render job status
    async fn update_render_job(&self, job: &RenderJob) -> Result<()>;

    /// Reconstruct a Template from a TemplateEntry by fetching content and schema from storage
    async fn reconstruct_template(&self, template_entry: &TemplateEntry) -> Result<Template>;

    /// Get a template entry along with its reconstructed content in one call
    async fn get_template_with_content(&self, template_ref: &TemplateRef) -> Result<(TemplateEntry, Template)>;
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
        // Get template entry to find S3 keys before deletion
        let template_entry = self.metadata_storage.get_template(template_ref).await?;
        
        // Delete from metadata storage first
        self.metadata_storage.delete_template(template_ref).await?;
        
        // Then delete S3 files (best effort - don't fail if S3 deletion fails)
        let _ = self.file_storage.delete_template_files(
            &template_entry.content_s3_key,
            &template_entry.schema_s3_key
        ).await;
        
        Ok(())
    }

    async fn search_templates(&self, query: &str) -> Result<Vec<TemplateEntry>> {
        self.metadata_storage.search_templates(query).await
    }

    async fn render_template(
        &self,
        template_ref: &TemplateRef,
        data: &serde_json::Value,
    ) -> Result<RenderJob> {
        // Get template entry and content in one call
        let (_template_entry, template) = self.get_template_with_content(template_ref).await?;

        // Validate data against template schema before rendering
        template.validate_data(data)?;

        // Create render job
        let job = RenderJob::new(template_ref.clone(), data.clone());

        // Save the job as pending
        self.metadata_storage.save_render_job(&job).await?;

        // TODO: Implement actual rendering logic
        // This would typically:
        // 1. Render the template to PDF using papermake
        // 2. Store the PDF in S3
        // 3. Update the job status to completed with the S3 key
        // For now, we just mark it as pending
        
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
        // Save template content and schema to S3
        let content_s3_key = self.file_storage.save_template_content(&template_ref, &template.content).await?;
        let schema_s3_key = self.file_storage.save_template_schema(&template_ref, &template.schema).await?;

        // Create template entry with S3 keys
        let mut template_entry = TemplateEntry::new(content_s3_key, schema_s3_key, template_ref, author);
        
        // Generate content digest based on S3 keys
        template_entry.generate_digest();

        // Save metadata to storage
        self.metadata_storage.save_template(&template_entry).await?;

        Ok(template_entry)
    }

    async fn update_template(
        &self,
        template_ref: &TemplateRef,
        template: Template,
    ) -> Result<TemplateEntry> {
        // Get existing template entry to preserve metadata
        let existing_entry = self.metadata_storage.get_template(template_ref).await?;

        // Save updated content and schema to S3 (overwrites existing files)
        let content_s3_key = self.file_storage.save_template_content(template_ref, &template.content).await?;
        let schema_s3_key = self.file_storage.save_template_schema(template_ref, &template.schema).await?;

        // Create updated template entry preserving original metadata
        let mut updated_entry = TemplateEntry {
            content_s3_key,
            schema_s3_key,
            template_ref: template_ref.clone(),
            author: existing_entry.author,
            forked_from: existing_entry.forked_from,
            published_at: existing_entry.published_at, // Keep original publish time
        };

        // Generate new content digest
        updated_entry.generate_digest();

        // Save updated metadata to storage
        self.metadata_storage.save_template(&updated_entry).await?;

        Ok(updated_entry)
    }

    async fn update_render_job(&self, job: &RenderJob) -> Result<()> {
        self.metadata_storage.save_render_job(job).await
    }

    async fn reconstruct_template(&self, template_entry: &TemplateEntry) -> Result<Template> {
        // Fetch content and schema from S3
        let content = self.file_storage.get_template_content(&template_entry.content_s3_key).await?;
        let schema = self.file_storage.get_template_schema(&template_entry.schema_s3_key).await?;

        Ok(Template::new(content, schema))
    }

    async fn get_template_with_content(&self, template_ref: &TemplateRef) -> Result<(TemplateEntry, Template)> {
        // Get template entry
        let template_entry = self.metadata_storage.get_template(template_ref).await?;
        
        // Reconstruct the template
        let template = self.reconstruct_template(&template_entry).await?;
        
        Ok((template_entry, template))
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
