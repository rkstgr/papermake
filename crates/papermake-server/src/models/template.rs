//! Template-related API models

use papermake_registry::entities::TemplateEntry;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Template summary for listing endpoints
#[derive(Debug, Serialize)]
pub struct TemplateSummary {
    pub template_ref: String,
    pub name: String,
    pub tag: String,
    pub org: Option<String>,
    pub uses_24h: u64,
    #[serde(with = "time::serde::rfc3339")]
    pub published_at: OffsetDateTime,
    pub author: String,
}

/// Full template details
#[derive(Debug, Serialize)]
pub struct TemplateDetails {
    pub template_ref: String,
    pub name: String,
    pub tag: String,
    pub org: Option<String>,
    pub digest: Option<String>,
    pub description: Option<String>,
    pub content: String,
    pub schema: Option<serde_json::Value>,
    pub author: String,
    #[serde(with = "time::serde::rfc3339")]
    pub published_at: OffsetDateTime,
    pub uses_total: u64,
    pub uses_24h: u64,
}

impl From<TemplateEntry> for TemplateDetails {
    fn from(te: TemplateEntry) -> Self {
        // Convert papermake Schema to JSON Value
        let schema = match serde_json::to_value(&te.template.schema) {
            Ok(serde_json::Value::Null) => None,
            Ok(value) => Some(value),
            Err(_) => None,
        };

        Self {
            template_ref: te.template_ref.to_string(),
            name: te.template_ref.name.clone(),
            tag: te.template_ref.tag.clone(),
            org: te.template_ref.org.clone(),
            digest: te.template_ref.digest.clone(),
            description: te.template.description,
            content: te.template.content,
            schema,
            author: te.author,
            published_at: te.published_at,
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
    pub template_ref: String,
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
