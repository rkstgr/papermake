//! Core data structures for the papermake registry

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;
use papermake::{Template, TemplateId};
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

/// A versioned template with registry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedTemplate {
    /// The core template from papermake
    pub template: Template,
    
    /// Auto-incrementing version number (1, 2, 3...)
    pub version: u64,
    
    /// Author of this version (simple string identifier)
    pub author: String,
    
    /// If this template was forked, track the source
    pub forked_from: Option<(TemplateId, u64)>,
    
    /// When this version was published
    #[serde(with = "time::serde::rfc3339")]
    pub published_at: OffsetDateTime,
    
    /// Whether this version is immutable (true once published)
    pub immutable: bool,
    
    /// Schema definition embedded in the template metadata
    pub schema: Option<HashMap<String, serde_json::Value>>,
}

impl VersionedTemplate {
    /// Create a new versioned template
    pub fn new(
        template: Template,
        version: u64,
        author: String,
    ) -> Self {
        Self {
            template,
            version,
            author,
            forked_from: None,
            published_at: OffsetDateTime::now_utc(),
            immutable: true,
            schema: None,
        }
    }
    
    /// Create a forked template
    pub fn forked_from(
        template: Template,
        version: u64,
        author: String,
        source: (TemplateId, u64),
    ) -> Self {
        Self {
            template,
            version,
            author,
            forked_from: Some(source),
            published_at: OffsetDateTime::now_utc(),
            immutable: true,
            schema: None,
        }
    }
    
    /// Get the template ID
    pub fn id(&self) -> &TemplateId {
        &self.template.id
    }
    
    /// Add schema definition
    pub fn with_schema(mut self, schema: HashMap<String, serde_json::Value>) -> Self {
        self.schema = Some(schema);
        self
    }
}

/// Render job tracking a PDF generation process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderJob {
    /// Unique job identifier
    pub id: String,
    
    /// Template ID and version used for rendering
    pub template_id: TemplateId,
    pub template_version: u64,
    
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
    /// Create a new render job
    pub fn new(
        template_id: TemplateId,
        template_version: u64,
        data: serde_json::Value,
    ) -> Self {
        // Generate data hash for caching
        let data_string = serde_json::to_string(&data).unwrap_or_default();
        let mut hasher = DefaultHasher::new();
        data_string.hash(&mut hasher);
        let data_hash = format!("{:x}", hasher.finish());
        
        Self {
            id: Uuid::new_v4().to_string(),
            template_id,
            template_version,
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