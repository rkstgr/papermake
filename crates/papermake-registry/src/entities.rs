//! Core data structures for the papermake registry

use papermake::{Template, TemplateId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use time::OffsetDateTime;
use uuid::Uuid;

/// A versioned template with registry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedTemplate {
    /// The core template from papermake
    pub template: Template,

    /// Machine-readable template name (e.g. "invoice-template")
    pub template_name: String,

    /// Human-readable display name (e.g. "Monthly Invoice Template")
    pub display_name: String,

    /// Version string (e.g. "v1", "v2", "latest", "draft")
    pub version: String,

    /// Author of this version (simple string identifier)
    pub author: String,

    /// If this template was forked, track the source
    pub forked_from: Option<(String, String)>, // Changed to (template_name, version)

    /// When this version was published
    #[serde(with = "time::serde::rfc3339")]
    pub published_at: OffsetDateTime,

    /// Whether this version is immutable (true once published)
    pub immutable: bool,

    /// Whether this is a draft (vs published version)
    pub is_draft: bool,

    /// Schema definition embedded in the template metadata
    pub schema: Option<HashMap<String, serde_json::Value>>,
}

impl VersionedTemplate {
    /// Create a new published versioned template
    pub fn new(template: Template, template_name: String, display_name: String, version: String, author: String) -> Self {
        Self {
            template,
            template_name,
            display_name,
            version,
            author,
            forked_from: None,
            published_at: OffsetDateTime::now_utc(),
            immutable: true,
            is_draft: false,
            schema: None,
        }
    }

    /// Create a new draft template
    pub fn new_draft(template: Template, template_name: String, display_name: String, author: String) -> Self {
        Self {
            template,
            template_name,
            display_name,
            version: "draft".to_string(),
            author,
            forked_from: None,
            published_at: OffsetDateTime::now_utc(),
            immutable: false,
            is_draft: true,
            schema: None,
        }
    }

    /// Create a forked template
    pub fn forked_from(
        template: Template,
        template_name: String,
        display_name: String,
        version: String,
        author: String,
        source: (String, String), // (template_name, version)
    ) -> Self {
        Self {
            template,
            template_name,
            display_name,
            version,
            author,
            forked_from: Some(source),
            published_at: OffsetDateTime::now_utc(),
            immutable: true,
            is_draft: false,
            schema: None,
        }
    }

    /// Get the template ID (for backward compatibility)
    pub fn id(&self) -> &TemplateId {
        &self.template.id
    }

    /// Get the template name:version identifier
    pub fn name_version(&self) -> String {
        format!("{}:{}", self.template_name, self.version)
    }

    /// Check if this is the latest version tag
    pub fn is_latest(&self) -> bool {
        self.version == "latest"
    }

    /// Check if this is a draft
    pub fn is_draft(&self) -> bool {
        self.is_draft
    }

    /// Promote draft to published version
    pub fn publish(mut self, new_version: String) -> Self {
        self.version = new_version;
        self.is_draft = false;
        self.immutable = true;
        self.published_at = OffsetDateTime::now_utc();
        self
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

    /// Template identifier - supports both UUID (for backward compatibility) and name
    pub template_id: TemplateId,
    /// Template name (e.g. "invoice-template")
    pub template_name: String,
    /// Template version (e.g. "v1", "v2", "latest")
    pub template_version: String,

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
    /// Create a new render job with template name and version
    pub fn new(template_id: TemplateId, template_name: String, template_version: String, data: serde_json::Value) -> Self {
        // Generate data hash for caching
        let data_string = serde_json::to_string(&data).unwrap_or_default();
        let mut hasher = DefaultHasher::new();
        data_string.hash(&mut hasher);
        let data_hash = format!("{:x}", hasher.finish());

        Self {
            id: Uuid::new_v4().to_string(),
            template_id,
            template_name,
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

    /// Create a new render job (backward compatibility with u64 version)
    #[deprecated(note = "Use new() with String version instead")]
    pub fn new_legacy(template_id: TemplateId, template_version: u64, data: serde_json::Value) -> Self {
        Self::new(template_id.clone(), template_id.as_ref().to_string(), format!("v{}", template_version), data)
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
