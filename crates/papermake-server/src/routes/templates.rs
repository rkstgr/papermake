//! Template management routes

use crate::{
    AppState,
    error::{ApiError, Result},
    models::{
        ApiResponse, CreateTemplateRequest, PaginatedResponse, SearchQuery, TemplateDetails,
        TemplatePreviewRequest, TemplateSummary, TemplateValidationRequest,
        TemplateValidationResponse,
    },
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use papermake::TemplateBuilder;
use papermake_registry::{TemplateRegistry, template_ref::TemplateRef};
use tracing::{debug, error, info};

/// Helper function to parse TemplateRef from string
fn parse_template_ref(ref_str: &str) -> Result<TemplateRef> {
    ref_str
        .parse()
        .map_err(|e| ApiError::validation(&format!("Invalid template reference format: {}", e)))
}

/// Create template routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_templates).post(create_template))
        // Special endpoints that need to come before catch-all template routes
        .route("/preview", post(preview_template))
        .route("/validate", post(validate_template))
        // Docker-style template reference routes (supports org/name:tag[@digest])
        .route("/{*template_ref}", get(get_template_by_ref))
        // Template management by name
        .route("/{*template_name}/tags", get(list_template_tags_by_name))
        // Draft endpoints (by template name)
        .route(
            "/{*template_name}/draft",
            get(get_draft).put(save_draft).delete(delete_draft),
        )
        .route("/{*template_name}/draft/publish", post(publish_draft))
}

/// List all templates with pagination and search
async fn list_templates(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<PaginatedResponse<TemplateSummary>>> {
    debug!("Listing templates with query: {:?}", query);

    // For now, get all templates and apply basic filtering
    // In production, this should be done at the database level
    let templates = state.registry.list_templates().await.map_err(|e| {
        error!("Failed to list templates: {}", e);
        e
    })?;

    // Apply search filter if provided
    let filtered_templates: Vec<_> = if let Some(search_term) = &query.search {
        templates
            .into_iter()
            .filter(|t| {
                t.template_ref
                    .name
                    .to_lowercase()
                    .contains(&search_term.to_lowercase())
                    || t.template_ref
                        .to_string()
                        .to_lowercase()
                        .contains(&search_term.to_lowercase())
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
        .map(|te| TemplateSummary {
            template_ref: te.template_ref.to_string(),
            name: te.template_ref.name.clone(),
            tag: te.template_ref.tag.clone(),
            org: te.template_ref.org.clone(),
            uses_24h: 0, // TODO: Get from analytics
            published_at: te.published_at,
            author: te.author,
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

/// Get a template by Docker-style reference (org/name:tag[@digest])
async fn get_template_by_ref(
    State(state): State<AppState>,
    Path(template_ref_str): Path<String>,
) -> Result<Json<ApiResponse<TemplateDetails>>> {
    debug!("Getting template by reference: {}", template_ref_str);

    // URL decode the template reference
    let decoded_ref = urlencoding::decode(&template_ref_str)
        .map_err(|_| ApiError::validation("Invalid URL encoding in template reference"))?;

    let template = state.registry.get_template(&decoded_ref).await?;
    let details = TemplateDetails::from(template);

    Ok(Json(ApiResponse::new(details)))
}

/// Create a new template
async fn create_template(
    State(state): State<AppState>,
    Json(request): Json<CreateTemplateRequest>,
) -> Result<impl IntoResponse> {
    info!("Creating new template: {}", request.template_ref);

    // Parse template reference
    let template_ref = parse_template_ref(&request.template_ref)?;

    // Build template using papermake's builder
    let template = TemplateBuilder::new()
        .description(request.description.unwrap_or_default())
        .content(request.content)
        .schema(papermake::Schema::from_value(
            request.schema.unwrap_or(serde_json::Value::Null),
        ))
        .build()
        .map_err(|e| ApiError::validation(&e.to_string()))?;

    // Publish template to registry
    let template_entry = state
        .registry
        .publish_template(template, template_ref, request.author)
        .await?;

    let details = TemplateDetails::from(template_entry);

    Ok((StatusCode::CREATED, Json(ApiResponse::new(details))))
}

/// List all tags of a template by name
async fn list_template_tags_by_name(
    State(state): State<AppState>,
    Path(template_name): Path<String>,
) -> Result<Json<ApiResponse<Vec<String>>>> {
    debug!("Listing tags for template: {}", template_name);

    let tags = state.registry.list_tags(&template_name).await?;

    Ok(Json(ApiResponse::new(tags)))
}

/// Preview a template without storing it
async fn preview_template(
    State(_state): State<AppState>,
    Json(request): Json<TemplatePreviewRequest>,
) -> Result<Response> {
    debug!("Previewing template");

    // Create temporary template
    let template = TemplateBuilder::new()
        .content(request.content)
        .schema(papermake::Schema::from_value(
            request.schema.unwrap_or(serde_json::Value::Null),
        ))
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
    let mut builder = TemplateBuilder::new().content(request.content);

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

// === Draft Management Endpoints ===

/// Get a draft template by name
async fn get_draft(
    State(state): State<AppState>,
    Path(template_name): Path<String>,
) -> Result<Json<ApiResponse<Option<TemplateDetails>>>> {
    debug!("Getting draft for template: {}", template_name);

    let draft = state.registry.get_draft_template(&template_name).await?;
    let details = draft.map(TemplateDetails::from);

    Ok(Json(ApiResponse::new(details)))
}

/// Save a draft template
async fn save_draft(
    State(state): State<AppState>,
    Path(template_name): Path<String>,
    Json(request): Json<CreateTemplateRequest>,
) -> Result<impl IntoResponse> {
    info!("Saving draft for template: {}", template_name);

    // Build template using papermake's builder
    let template = TemplateBuilder::new()
        .description(request.description.unwrap_or_default())
        .content(request.content)
        .schema(papermake::Schema::from_value(
            request.schema.unwrap_or(serde_json::Value::Null),
        ))
        .build()
        .map_err(|e| ApiError::validation(&e.to_string()))?;

    // Save as draft
    let draft_template = state
        .registry
        .save_draft_template(template, template_name, request.author)
        .await?;

    let details = TemplateDetails::from(draft_template);

    Ok((StatusCode::OK, Json(ApiResponse::new(details))))
}

/// Delete a draft template
async fn delete_draft(
    State(state): State<AppState>,
    Path(template_name): Path<String>,
) -> Result<impl IntoResponse> {
    info!("Deleting draft for template: {}", template_name);

    state.registry.delete_draft_template(&template_name).await?;

    Ok((StatusCode::NO_CONTENT, ()))
}

/// Publish a draft as a new version
async fn publish_draft(
    State(state): State<AppState>,
    Path(template_name): Path<String>,
) -> Result<impl IntoResponse> {
    info!("Publishing draft for template: {}", template_name);

    let published_template = state.registry.publish_draft(&template_name).await?;
    let details = TemplateDetails::from(published_template);

    Ok((StatusCode::CREATED, Json(ApiResponse::new(details))))
}
