//! Template-related API models

use papermake_registry::{TemplateId, entities::VersionedTemplate};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Template summary for listing endpoints
#[derive(Debug, Serialize)]
pub struct TemplateSummary {
    pub id: TemplateId,
    pub name: String,
    pub latest_version: String,
    pub uses_24h: u64,
    #[serde(with = "time::serde::rfc3339")]
    pub published_at: OffsetDateTime,
    pub author: String,
}

/// Full template details
#[derive(Debug, Serialize)]
pub struct TemplateDetails {
    pub id: TemplateId,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub schema: Option<serde_json::Value>,
    pub tag: String,
    pub author: String,
    #[serde(with = "time::serde::rfc3339")]
    pub published_at: OffsetDateTime,
    pub uses_total: u64,
    pub uses_24h: u64,
}

impl From<VersionedTemplate> for TemplateDetails {
    fn from(vt: VersionedTemplate) -> Self {
        // Convert papermake Schema to JSON Value
        let schema = match serde_json::to_value(&vt.template.schema) {
            Ok(serde_json::Value::Null) => None,
            Ok(value) => Some(value),
            Err(_) => None,
        };

        Self {
            id: vt.template.id,
            name: vt.template.name,
            description: vt.template.description,
            content: vt.template.content,
            schema,
            tag: vt.tag,
            author: vt.author,
            published_at: vt.published_at,
            uses_total: 0, // Will be populated by analytics
            uses_24h: 0,   // Will be populated by analytics
        }
    }
}

/// Template version information
#[derive(Debug, Serialize)]
pub struct TemplateVersion {
    pub version: String,
    #[serde(with = "time::serde::rfc3339")]
    pub published_at: OffsetDateTime,
    pub author: String,
    pub uses_total: u64,
}

/// Request to create a new template
#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub id: TemplateId,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub schema: Option<serde_json::Value>,
    pub author: String,
}

/// Request to update a template (creates new version)
#[derive(Debug, Deserialize)]
pub struct UpdateTemplateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub schema: Option<serde_json::Value>,
    pub author: String,
}

/// Template preview request (no storage)
#[derive(Debug, Deserialize)]
pub struct TemplatePreviewRequest {
    pub content: String,
    pub data: serde_json::Value,
    pub schema: Option<serde_json::Value>,
}

/// Template validation request
#[derive(Debug, Deserialize)]
pub struct TemplateValidationRequest {
    pub content: String,
    pub schema: Option<serde_json::Value>,
    pub data: Option<serde_json::Value>,
}

/// Template validation response
#[derive(Debug, Serialize)]
pub struct TemplateValidationResponse {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

#[derive(Debug, Serialize)]
pub struct ValidationError {
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ValidationWarning {
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}
