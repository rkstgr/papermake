//! Render-related API models

use papermake_registry::entities::RenderJob;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Render job summary for listing endpoints
#[derive(Debug, Serialize)]
pub struct RenderJobSummary {
    pub id: String,
    pub template_ref: String,
    pub status: RenderStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub completed_at: Option<OffsetDateTime>,
    pub rendering_latency: Option<i64>,
    pub pdf_url: Option<String>,
}

impl From<RenderJob> for RenderJobSummary {
    fn from(job: RenderJob) -> Self {
        let status = match job.status {
            papermake_registry::entities::RenderStatus::Pending => RenderStatus::Queued,
            papermake_registry::entities::RenderStatus::InProgress => RenderStatus::Processing,
            papermake_registry::entities::RenderStatus::Completed => RenderStatus::Completed,
            papermake_registry::entities::RenderStatus::Failed => RenderStatus::Failed,
        };

        let job_id = job.id.clone();

        Self {
            id: job.id,
            template_ref: job.template_ref.to_string(),
            status,
            created_at: job.created_at,
            completed_at: job.completed_at,
            rendering_latency: job.rendering_latency.map(|l| l as i64),
            pdf_url: job
                .pdf_s3_key
                .map(|_key| format!("/api/renders/{}/pdf", job_id)),
        }
    }
}

/// Full render job details
#[derive(Debug, Serialize)]
pub struct RenderJobDetails {
    pub id: String,
    pub template_ref: String,
    pub data: serde_json::Value,
    pub data_hash: String,
    pub status: RenderStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub completed_at: Option<OffsetDateTime>,
    pub rendering_latency: Option<i64>,
    pub pdf_url: Option<String>,
    pub error_message: Option<String>,
}

/// Render job status
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RenderStatus {
    Queued,
    Processing,
    Completed,
    Failed,
}

/// Request to create a render job
#[derive(Debug, Deserialize)]
pub struct CreateRenderRequest {
    pub template_ref: String,
    pub data: serde_json::Value,
    pub options: Option<RenderOptions>,
}

/// Render options
#[derive(Debug, Deserialize)]
pub struct RenderOptions {
    /// Paper size (e.g., "a4", "letter")
    pub paper_size: Option<String>,

    /// Whether to compress the output PDF
    pub compress: Option<bool>,

    /// Priority level (higher = more priority)
    pub priority: Option<i32>,
}

/// Response when creating a render job
#[derive(Debug, Serialize)]
pub struct CreateRenderResponse {
    pub id: String,
    pub status: RenderStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub estimated_completion: Option<OffsetDateTime>,
}

/// Batch render request
#[derive(Debug, Deserialize)]
pub struct BatchRenderRequest {
    pub requests: Vec<CreateRenderRequest>,
}

/// Batch render response
#[derive(Debug, Serialize)]
pub struct BatchRenderResponse {
    pub batch_id: String,
    pub render_jobs: Vec<CreateRenderResponse>,
    pub total_jobs: usize,
}

/// Render job query parameters
#[derive(Debug, Deserialize)]
pub struct RenderJobQuery {
    #[serde(flatten)]
    pub pagination: super::PaginationQuery,

    /// Filter by template reference
    pub template_ref: Option<String>,

    /// Filter by status
    pub status: Option<RenderStatus>,

    /// Filter by date range (start)
    pub date_from: Option<OffsetDateTime>,

    /// Filter by date range (end)
    pub date_to: Option<OffsetDateTime>,
}

/// WebSocket message for render job updates
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct RenderJobUpdate {
    pub job_id: String,
    pub status: RenderStatus,
    pub progress: Option<f32>, // 0.0 to 1.0
    pub message: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub completed_at: Option<OffsetDateTime>,
    pub pdf_url: Option<String>,
}
