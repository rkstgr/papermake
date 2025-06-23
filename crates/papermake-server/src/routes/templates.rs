//! Template management routes

use crate::{
    error::{ApiError, Result},
    models::{
        ApiResponse, PaginatedResponse, TemplateDetails, TemplateSummary, CreateTemplateRequest,
        TemplatePreviewRequest, TemplateValidationRequest,
        TemplateValidationResponse, SearchQuery,
    },
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use papermake::TemplateBuilder;
use papermake_registry::{TemplateId, TemplateRegistry};
use tracing::{debug, error, info};

/// Create template routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_templates).post(create_template))
        .route("/{template_id}", get(get_template))
        .route("/{template_id}/versions", get(list_template_versions))
        .route("/{template_id}/versions/{version}", get(get_template_version))
        .route("/preview", post(preview_template))
        .route("/validate", post(validate_template))
}

/// List all templates with pagination and search
async fn list_templates(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<PaginatedResponse<TemplateSummary>>> {
    debug!("Listing templates with query: {:?}", query);

    // For now, get all templates and apply basic filtering
    // In production, this should be done at the database level
    let templates = state.registry.list_templates().await
        .map_err(|e| {
            error!("Failed to list templates: {}", e);
            e
        })?;
    
    // Apply search filter if provided
    let filtered_templates: Vec<_> = if let Some(search_term) = &query.search {
        templates
            .into_iter()
            .filter(|t| {
                t.template.name.to_lowercase().contains(&search_term.to_lowercase())
                    || t.template.id.to_string().to_lowercase().contains(&search_term.to_lowercase())
            })
            .collect()
    } else {
        templates
    };

    // Apply pagination
    let total = filtered_templates.len() as u32;
    let start = query.pagination.offset as usize;
    let end = (start + query.pagination.limit as usize).min(filtered_templates.len());
    let page_templates = filtered_templates[start..end].to_vec();

    // Convert to summary format
    // Note: In production, we'd get usage stats from analytics
    let summaries: Vec<TemplateSummary> = page_templates
        .into_iter()
        .map(|vt| TemplateSummary {
            id: vt.template.id,
            name: vt.template.name,
            latest_version: vt.version,
            uses_24h: 0, // TODO: Get from analytics
            published_at: vt.published_at,
            author: vt.author,
        })
        .collect();

    let response = PaginatedResponse::new(
        summaries,
        query.pagination.limit,
        query.pagination.offset,
        Some(total),
    );

    Ok(Json(response))
}

/// Get a specific template (latest version)
async fn get_template(
    State(state): State<AppState>,
    Path(template_id): Path<TemplateId>,
) -> Result<Json<ApiResponse<TemplateDetails>>> {
    debug!("Getting template: {:?}", template_id);

    let template = state.registry.get_latest_template(&template_id).await?;
    let details = TemplateDetails::from(template);

    Ok(Json(ApiResponse::new(details)))
}

/// Create a new template
async fn create_template(
    State(state): State<AppState>,
    Json(request): Json<CreateTemplateRequest>,
) -> Result<impl IntoResponse> {
    info!("Creating new template: {:?}", request.id);

    // Build template using papermake's builder
    let template = TemplateBuilder::new(request.id.clone())
        .name(request.name)
        .description(request.description.unwrap_or_default())
        .content(request.content)
        .schema(papermake::Schema::from_value(request.schema.unwrap_or(serde_json::Value::Null)))
        .build()
        .map_err(|e| ApiError::validation(&e.to_string()))?;

    // Publish template to registry
    let versioned_template = state
        .registry
        .publish_template(template, request.author)
        .await?;

    let details = TemplateDetails::from(versioned_template);

    Ok((StatusCode::CREATED, Json(ApiResponse::new(details))))
}

/// List all versions of a template
async fn list_template_versions(
    State(state): State<AppState>,
    Path(template_id): Path<TemplateId>,
) -> Result<Json<ApiResponse<Vec<u64>>>> {
    debug!("Listing versions for template: {:?}", template_id);

    let versions = state.registry.list_versions(&template_id).await?;

    Ok(Json(ApiResponse::new(versions)))
}

/// Get a specific version of a template
async fn get_template_version(
    State(state): State<AppState>,
    Path((template_id, version)): Path<(TemplateId, u64)>,
) -> Result<Json<ApiResponse<TemplateDetails>>> {
    debug!("Getting template {:?} version {}", template_id, version);

    let template = state.registry.get_template(&template_id, version).await?;
    let details = TemplateDetails::from(template);

    Ok(Json(ApiResponse::new(details)))
}


/// Preview a template without storing it
async fn preview_template(
    State(_state): State<AppState>,
    Json(request): Json<TemplatePreviewRequest>,
) -> Result<Response> {
    debug!("Previewing template");

    // Create temporary template
    let template = TemplateBuilder::new("preview".into())
        .name("Preview")
        .content(request.content)
        .schema(papermake::Schema::from_value(request.schema.unwrap_or(serde_json::Value::Null)))
        .build()
        .map_err(|e| ApiError::validation(&e.to_string()))?;

    // Render directly without creating a render job
    let render_result = papermake::render::render_pdf(&template, &request.data, None)
        .map_err(|e| ApiError::RenderFailed(e.to_string()))?;

    if let Some(pdf_data) = render_result.pdf {
        Ok(Response::builder()
            .header("content-type", "application/pdf")
            .header("content-length", pdf_data.len())
            .header("cache-control", "no-cache")
            .body(axum::body::Body::from(pdf_data))
            .unwrap())
    } else {
        error!("Template preview failed: {:?}", render_result.errors);
        Err(ApiError::RenderFailed(format!(
            "Template compilation failed: {:?}",
            render_result.errors
        )))
    }
}

/// Validate a template
async fn validate_template(
    Json(request): Json<TemplateValidationRequest>,
) -> Result<Json<ApiResponse<TemplateValidationResponse>>> {
    debug!("Validating template");

    // Try to build the template to validate syntax
    let mut builder = TemplateBuilder::new("validation".into())
        .name("Validation")
        .content(request.content);

    if let Some(schema) = request.schema {
        builder = builder.schema(papermake::Schema::from_value(schema));
    }

    let template_result = builder.build();

    match template_result {
        Ok(template) => {
            // If data is provided, try to validate against schema
            let mut errors = Vec::new();
            
            if let Some(data) = request.data {
                if let Err(e) = template.validate_data(&data) {
                    errors.push(crate::models::ValidationError {
                        message: e.to_string(),
                        line: None,
                        column: None,
                    });
                }
            }

            let response = TemplateValidationResponse {
                valid: errors.is_empty(),
                errors,
                warnings: Vec::new(), // Could add warnings for best practices
            };

            Ok(Json(ApiResponse::new(response)))
        }
        Err(e) => {
            let response = TemplateValidationResponse {
                valid: false,
                errors: vec![crate::models::ValidationError {
                    message: e.to_string(),
                    line: None,
                    column: None,
                }],
                warnings: Vec::new(),
            };

            Ok(Json(ApiResponse::new(response)))
        }
    }
}