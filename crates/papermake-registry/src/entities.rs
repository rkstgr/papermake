//! Core data structures for the papermake registry

use crate::template_ref::TemplateRef;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use time::OffsetDateTime;
use uuid::Uuid;

/// A template entry in the registry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateEntry {
    /// S3 key for the template content (Typst markup)
    pub content_s3_key: String,

    /// S3 key for the template schema (JSON)
    pub schema_s3_key: String,

    /// Docker-style template reference (org/name:tag@digest)
    pub template_ref: TemplateRef,

    /// Author of this version (simple string identifier)
    pub author: String,

    /// If this template was forked, track the source
    pub forked_from: Option<TemplateRef>,

    /// When this version was published
    #[serde(with = "time::serde::rfc3339")]
    pub published_at: OffsetDateTime,
}

impl TemplateEntry {
    /// Create a new published template entry
    pub fn new(
        content_s3_key: String,
        schema_s3_key: String,
        template_ref: TemplateRef,
        author: String,
    ) -> Self {
        Self {
            content_s3_key,
            schema_s3_key,
            template_ref,
            author,
            forked_from: None,
            published_at: OffsetDateTime::now_utc(),
        }
    }

    /// Create a forked template entry
    pub fn forked_from(
        content_s3_key: String,
        schema_s3_key: String,
        template_ref: TemplateRef,
        author: String,
        source: TemplateRef,
    ) -> Self {
        Self {
            content_s3_key,
            schema_s3_key,
            template_ref,
            author,
            forked_from: Some(source),
            published_at: OffsetDateTime::now_utc(),
        }
    }

    /// Get the template reference string
    pub fn reference(&self) -> String {
        self.template_ref.to_string()
    }

    /// Get the name:tag portion
    pub fn name_tag(&self) -> String {
        self.template_ref.name_tag()
    }

    /// Check if this is the latest tag
    pub fn is_latest(&self) -> bool {
        self.template_ref.is_latest()
    }
    // TODO: how is new_tag derived
    /// Promote draft to published tag
    pub fn publish(mut self, new_tag: String) -> Self {
        self.template_ref = self.template_ref.with_different_tag(new_tag);
        self.published_at = OffsetDateTime::now_utc();
        self
    }

    /// Generate and set the content digest from S3 keys
    pub fn generate_digest(&mut self) {
        // Use S3 keys as the basis for digest generation
        let content = format!("{}|{}", self.content_s3_key, self.schema_s3_key);

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let digest = format!("sha256:{:x}", hasher.finalize());

        self.template_ref = self.template_ref.with_content_digest(digest);
    }
}

/// Render job tracking a PDF generation process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderJob {
    /// Unique job identifier (render_id)
    pub id: String,

    /// Template reference for this render job
    pub template_ref: TemplateRef,

    /// Input data for rendering
    pub data: serde_json::Value,

    /// Hash of the input data (for caching)
    pub data_hash: String,

    /// Job status
    pub status: RenderStatus,

    /// S3 key where the resulting PDF is stored
    pub pdf_s3_key: Option<String>,

    /// Time taken to render (in milliseconds)
    pub rendering_latency: Option<u64>,

    /// When the job was created
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,

    /// When the job was completed (if finished)
    #[serde(with = "time::serde::rfc3339::option")]
    pub completed_at: Option<OffsetDateTime>,

    /// Error message if the job failed
    pub error_message: Option<String>,
}

/// Status of a render job
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RenderStatus {
    /// Job is pending execution
    Pending,
    /// Job is currently being processed
    InProgress,
    /// Job completed successfully
    Completed,
    /// Job failed with an error
    Failed,
}

impl RenderJob {
    /// Create a new render job with template reference
    pub fn new(template_ref: TemplateRef, data: serde_json::Value) -> Self {
        // Generate data hash for caching
        let data_string = serde_json::to_string(&data).unwrap_or_default();
        let mut hasher = DefaultHasher::new();
        data_string.hash(&mut hasher);
        let data_hash = format!("{:x}", hasher.finish());

        Self {
            id: Uuid::new_v4().to_string(),
            template_ref,
            data,
            data_hash,
            status: RenderStatus::Pending,
            pdf_s3_key: None,
            rendering_latency: None,
            created_at: OffsetDateTime::now_utc(),
            completed_at: None,
            error_message: None,
        }
    }

    /// Mark job as in progress
    pub fn start(&mut self) {
        self.status = RenderStatus::InProgress;
    }

    /// Mark job as completed successfully
    pub fn complete(&mut self, pdf_s3_key: String, latency_ms: u64) {
        self.status = RenderStatus::Completed;
        self.pdf_s3_key = Some(pdf_s3_key);
        self.rendering_latency = Some(latency_ms);
        self.completed_at = Some(OffsetDateTime::now_utc());
    }

    /// Mark job as failed
    pub fn fail(&mut self, error: String) {
        self.status = RenderStatus::Failed;
        self.error_message = Some(error);
        self.completed_at = Some(OffsetDateTime::now_utc());
    }
}
