//! Render job management routes

use crate::{
    error::{ApiError, Result},
    models::{
        ApiResponse, PaginatedResponse, RenderJobSummary, RenderJobDetails, CreateRenderRequest,
        CreateRenderResponse, RenderJobQuery, RenderStatus, BatchRenderRequest, BatchRenderResponse,
    },
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use papermake_registry::{TemplateId, TemplateRegistry};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Create render routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_renders).post(create_render))
        .route("/batch", post(create_batch_render))
        .route("/:render_id", get(get_render))
        .route("/:render_id/pdf", get(download_pdf))
        .route("/:render_id/retry", post(retry_render))
}

/// List render jobs with pagination and filtering
async fn list_renders(
    State(state): State<AppState>,
    Query(query): Query<RenderJobQuery>,
) -> Result<Json<PaginatedResponse<RenderJobSummary>>> {
    debug!("Listing render jobs with query: {:?}", query);

    // Get render jobs from registry
    // Note: This is a simplified implementation - in production we'd want 
    // more sophisticated filtering at the database level
    let jobs = state.registry.list_render_jobs().await?;

    // Apply filters
    let filtered_jobs: Vec<_> = jobs
        .into_iter()
        .filter(|job| {
            // Filter by template_id if specified
            if let Some(ref template_id) = query.template_id {
                if &job.template_id != template_id {
                    return false;
                }
            }

            // Filter by date range if specified
            if let Some(date_from) = query.date_from {
                if job.created_at < date_from {
                    return false;
                }
            }

            if let Some(date_to) = query.date_to {
                if job.created_at > date_to {
                    return false;
                }
            }

            // Filter by status if specified
            if let Some(ref status) = query.status {
                let job_status = if job.completed_at.is_some() {
                    if job.pdf_s3_key.is_some() {
                        RenderStatus::Completed
                    } else {
                        RenderStatus::Failed
                    }
                } else {
                    RenderStatus::Processing
                };

                if std::mem::discriminant(&job_status) != std::mem::discriminant(status) {
                    return false;
                }
            }

            true
        })
        .collect();

    // Apply pagination
    let total = filtered_jobs.len() as u32;
    let start = query.pagination.offset as usize;
    let end = (start + query.pagination.limit as usize).min(filtered_jobs.len());
    let page_jobs = filtered_jobs[start..end].to_vec();

    // Convert to summary format
    let summaries: Vec<RenderJobSummary> = page_jobs
        .into_iter()
        .map(RenderJobSummary::from)
        .collect();

    let response = PaginatedResponse::new(
        summaries,
        query.pagination.limit,
        query.pagination.offset,
        Some(total),
    );

    Ok(Json(response))
}

/// Create a new render job
async fn create_render(
    State(state): State<AppState>,
    Json(request): Json<CreateRenderRequest>,
) -> Result<impl IntoResponse> {
    info!(
        "Creating render job for template {:?} v{}",
        request.template_id, request.template_version
    );

    // Create render job through registry
    let render_job = state
        .registry
        .render_template(&request.template_id, request.template_version, &request.data)
        .await?;

    let response = CreateRenderResponse {
        id: render_job.id.clone(),
        status: RenderStatus::Queued,
        created_at: render_job.created_at,
        estimated_completion: None, // Could calculate based on queue depth
    };

    Ok((StatusCode::CREATED, Json(ApiResponse::new(response))))
}

/// Get specific render job details
async fn get_render(
    State(state): State<AppState>,
    Path(render_id): Path<String>,
) -> Result<Json<ApiResponse<RenderJobDetails>>> {
    debug!("Getting render job: {}", render_id);

    let job = state.registry.get_render_job(&render_id).await?;

    let status = if job.completed_at.is_some() {
        if job.pdf_s3_key.is_some() {
            RenderStatus::Completed
        } else {
            RenderStatus::Failed
        }
    } else {
        RenderStatus::Processing
    };

    let details = RenderJobDetails {
        id: job.id.clone(),
        template_id: job.template_id,
        template_version: job.template_version,
        data: job.data,
        data_hash: job.data_hash,
        status,
        created_at: job.created_at,
        completed_at: job.completed_at,
        rendering_latency: job.rendering_latency,
        pdf_url: job.pdf_s3_key.as_ref().map(|_| format!("/api/renders/{}/pdf", job.id)),
        error_message: None, // Could be stored in the future
    };

    Ok(Json(ApiResponse::new(details)))
}

/// Download PDF for a completed render job
async fn download_pdf(
    State(state): State<AppState>,
    Path(render_id): Path<String>,
) -> Result<Response> {
    debug!("Downloading PDF for render job: {}", render_id);

    let job = state.registry.get_render_job(&render_id).await?;

    let pdf_key = job.pdf_s3_key.ok_or_else(|| {
        if job.completed_at.is_some() {
            ApiError::RenderFailed("Render job failed".to_string())
        } else {
            ApiError::bad_request("Render job not yet completed")
        }
    })?;

    // Get PDF data from S3 storage
    // Note: We need to access the file storage directly here
    // This is a bit of a hack - in production we might want a dedicated service
    let file_storage = &state.registry.file_storage;
    let pdf_data = file_storage.get_file(&pdf_key).await?;

    // Generate filename
    let filename = format!("render_{}.pdf", render_id);

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/pdf")
        .header(header::CONTENT_LENGTH, pdf_data.len())
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .header(header::CACHE_CONTROL, "private, max-age=3600")
        .body(axum::body::Body::from(pdf_data))
        .unwrap())
}


/// Retry a failed render job
async fn retry_render(
    State(state): State<AppState>,
    Path(render_id): Path<String>,
) -> Result<Json<ApiResponse<CreateRenderResponse>>> {
    info!("Retrying render job: {}", render_id);

    let job = state.registry.get_render_job(&render_id).await?;

    // Only allow retry for failed jobs
    if job.completed_at.is_some() && job.pdf_s3_key.is_some() {
        return Err(ApiError::bad_request("Render job already completed successfully"));
    }

    // Create a new render job with the same parameters
    let new_job = state
        .registry
        .render_template(&job.template_id, job.template_version, &job.data)
        .await?;

    let response = CreateRenderResponse {
        id: new_job.id,
        status: RenderStatus::Queued,
        created_at: new_job.created_at,
        estimated_completion: None,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// Create multiple render jobs in batch
async fn create_batch_render(
    State(state): State<AppState>,
    Json(request): Json<BatchRenderRequest>,
) -> Result<impl IntoResponse> {
    info!("Creating batch of {} render jobs", request.requests.len());

    if request.requests.is_empty() {
        return Err(ApiError::bad_request("No render requests provided"));
    }

    if request.requests.len() > 100 {
        return Err(ApiError::bad_request("Too many requests in batch (max 100)"));
    }

    let batch_id = Uuid::new_v4().to_string();
    let mut render_jobs = Vec::new();

    // Process each request
    for req in request.requests {
        match state
            .registry
            .render_template(&req.template_id, req.template_version, &req.data)
            .await
        {
            Ok(job) => {
                render_jobs.push(CreateRenderResponse {
                    id: job.id,
                    status: RenderStatus::Queued,
                    created_at: job.created_at,
                    estimated_completion: None,
                });
            }
            Err(e) => {
                error!("Failed to create render job in batch: {}", e);
                // Continue with other jobs rather than failing the entire batch
                // In production, you might want different behavior
            }
        }
    }

    let response = BatchRenderResponse {
        batch_id,
        total_jobs: render_jobs.len(),
        render_jobs,
    };

    Ok((StatusCode::CREATED, Json(ApiResponse::new(response))))
}