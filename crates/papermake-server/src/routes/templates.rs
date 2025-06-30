//! Template management routes

use crate::{
    AppState,
    error::{ApiError, Result},
    models::api::{ApiResponse, PaginatedResponse, SearchQuery},
};
use axum::{
    Json, Router,
    extract::{Multipart, Path, Query, State},
    routing::{get, post},
};
use papermake_registry::{
    TemplateInfo,
    bundle::{TemplateBundle, TemplateMetadata},
    reference::Reference,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Query parameters for publishing a template
#[derive(Debug, Deserialize)]
pub struct PublishParams {
    /// Tag for the template (defaults to "latest")
    #[serde(default = "default_tag")]
    pub tag: String,
}

fn default_tag() -> String {
    "latest".to_string()
}

/// Response after successfully publishing a template
#[derive(Debug, Serialize)]
pub struct PublishResponse {
    /// Success message
    pub message: String,
    /// The manifest hash of the published template
    pub manifest_hash: String,
    /// Template reference for future use
    pub reference: String,
}

/// Simplified request for publishing a template with JSON payload
#[derive(Debug, Deserialize)]
pub struct PublishSimpleRequest {
    /// Main template file content (UTF-8 string)
    pub main_typ: String,
    /// Optional JSON schema as object
    pub schema: Option<serde_json::Value>,
    /// Template metadata
    pub metadata: TemplateMetadata,
}

/// Template metadata response for API
#[derive(Debug, Serialize)]
pub struct TemplateMetadataResponse {
    /// Template name
    pub name: String,
    /// Optional namespace
    pub namespace: Option<String>,
    /// Current tag being viewed
    pub tag: String,
    /// Available tags
    pub tags: Vec<String>,
    /// Manifest hash
    pub manifest_hash: String,
    /// Template metadata
    pub metadata: TemplateMetadata,
    /// Full template reference
    pub reference: String,
}

/// Create template routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_templates))
        .route("/{name}/publish", post(publish_template))
        .route("/{name}/publish-simple", post(publish_template_simple))
        .route("/{name}/tags", get(list_template_tags))
        .route("/{reference}", get(get_template_metadata))
}

/// List all templates in the registry
///
/// GET /api/templates
/// Query parameters:
/// - limit: Maximum number of templates to return (default: 50)
/// - offset: Number of templates to skip (default: 0)
/// - search: Search term to filter templates by name
pub async fn list_templates(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<PaginatedResponse<TemplateInfo>>> {
    let templates = state.registry.list_templates().await?;

    // Apply search filter if provided
    // let filtered_templates: Vec<TemplateInfo> = if let Some(search_term) = &query.search {
    //     let search_lower = search_term.to_lowercase();
    //     templates
    //         .into_iter()
    //         .filter(|template| template.full_name().to_lowercase().contains(&search_lower))
    //         .collect()
    // } else {
    //     templates
    // };

    let filtered_templates = templates;

    // Apply pagination
    let total = filtered_templates.len() as u32;
    let offset = query.pagination.offset as usize;
    let limit = query.pagination.limit as usize;

    let paginated_templates: Vec<TemplateInfo> = filtered_templates
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    let response = PaginatedResponse::new(
        paginated_templates,
        query.pagination.limit,
        query.pagination.offset,
        Some(total),
    );

    Ok(Json(response))
}

/// Publish a new template or update an existing template with a new tag
///
/// POST /api/templates/{name}/publish?tag=latest
/// Content-Type: multipart/form-data
///
/// Form fields:
/// - main_typ: The main template file (required)
/// - metadata: JSON metadata with name and author (required)
/// - schema: Optional JSON schema file
/// - files[]: Additional template files (optional, multiple)
pub async fn publish_template(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<PublishParams>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<PublishResponse>>> {
    let mut main_typ: Option<Vec<u8>> = None;
    let mut metadata: Option<TemplateMetadata> = None;
    let mut schema: Option<Vec<u8>> = None;
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();

    // Parse multipart form data
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(&format!("Failed to parse multipart data: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        let data = field.bytes().await.map_err(|e| {
            ApiError::bad_request(&format!("Failed to read field '{}': {}", field_name, e))
        })?;

        match field_name.as_str() {
            "main_typ" => {
                main_typ = Some(data.to_vec());
            }
            "metadata" => {
                let metadata_str = String::from_utf8(data.to_vec())
                    .map_err(|_| ApiError::bad_request("Metadata must be valid UTF-8"))?;
                metadata = Some(serde_json::from_str(&metadata_str).map_err(|e| {
                    ApiError::bad_request(&format!("Invalid metadata JSON: {}", e))
                })?);
            }
            "schema" => {
                schema = Some(data.to_vec());
            }
            field_name if field_name.starts_with("files[") => {
                // Extract filename from field name like "files[components/header.typ]"
                if let Some(filename) = extract_filename_from_field(&field_name) {
                    files.insert(filename, data.to_vec());
                }
            }
            _ => {
                // Ignore unknown fields
            }
        }
    }

    // Validate required fields
    let main_typ =
        main_typ.ok_or_else(|| ApiError::bad_request("Missing required field: main_typ"))?;

    let metadata =
        metadata.ok_or_else(|| ApiError::bad_request("Missing required field: metadata"))?;

    // Create template bundle
    let mut bundle = TemplateBundle::new(main_typ, metadata);

    // Add schema if provided
    if let Some(schema_data) = schema {
        bundle = bundle.with_schema(schema_data);
    }

    // Add additional files
    for (filename, file_data) in files {
        bundle = bundle.add_file(filename, file_data);
    }

    // Validate bundle before publishing
    bundle
        .validate()
        .map_err(|e| ApiError::bad_request(&format!("Template validation failed: {}", e)))?;

    // Publish the template
    let manifest_hash = state.registry.publish(bundle, &name, &params.tag).await?;

    let reference = format!("{}:{}", name, params.tag);
    let response_data = PublishResponse {
        message: format!("Template '{}' published successfully", reference),
        manifest_hash,
        reference: reference.clone(),
    };

    Ok(Json(ApiResponse::with_message(
        response_data,
        format!("Template published with reference '{}'", reference),
    )))
}

/// Publish a template with simplified JSON payload (no multipart)
///
/// POST /api/templates/{name}/publish-simple?tag=latest
/// Content-Type: application/json
///
/// JSON body:
/// {
///   "main_typ": "template content as string",
///   "schema": { "optional": "json schema object" },
///   "metadata": { "name": "template name", "author": "author email" }
/// }
pub async fn publish_template_simple(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<PublishParams>,
    Json(request): Json<PublishSimpleRequest>,
) -> Result<Json<ApiResponse<PublishResponse>>> {
    // Convert main_typ string to bytes
    let main_typ = request.main_typ.into_bytes();

    // Create template bundle
    let mut bundle = TemplateBundle::new(main_typ, request.metadata);

    // Add schema if provided
    if let Some(schema_value) = request.schema {
        let schema_bytes = serde_json::to_vec(&schema_value)
            .map_err(|e| ApiError::bad_request(&format!("Failed to serialize schema: {}", e)))?;
        bundle = bundle.with_schema(schema_bytes);
    }

    // Validate bundle before publishing
    bundle
        .validate()
        .map_err(|e| ApiError::bad_request(&format!("Template validation failed: {}", e)))?;

    // Publish the template
    let manifest_hash = state.registry.publish(bundle, &name, &params.tag).await?;

    let reference = format!("{}:{}", name, params.tag);
    let response_data = PublishResponse {
        message: format!("Template '{}' published successfully", reference),
        manifest_hash,
        reference: reference.clone(),
    };

    Ok(Json(ApiResponse::with_message(
        response_data,
        format!("Template published with reference '{}'", reference),
    )))
}

/// List all tags for a specific template
///
/// GET /api/templates/{name}/tags
pub async fn list_template_tags(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<Vec<String>>>> {
    let templates = state.registry.list_templates().await?;

    // Find the template by name (considering both namespaced and non-namespaced)
    let template = templates
        .iter()
        .find(|t| t.name == name || t.full_name() == name)
        .ok_or_else(|| ApiError::template_not_found(&name))?;

    Ok(Json(ApiResponse::new(template.tags.clone())))
}

/// Get metadata for a specific template reference
///
/// GET /api/templates/{reference}
///
/// The reference can be:
/// - name (defaults to latest tag)
/// - name:tag
/// - namespace/name
/// - namespace/name:tag
pub async fn get_template_metadata(
    State(state): State<AppState>,
    Path(reference): Path<String>,
) -> Result<Json<ApiResponse<TemplateMetadataResponse>>> {
    // Parse the reference
    let parsed_ref = reference.parse::<Reference>().map_err(|e| {
        ApiError::bad_request(&format!(
            "Invalid template reference '{}': {}",
            reference, e
        ))
    })?;

    // Resolve the template to get manifest hash
    let manifest_hash = state.registry.resolve(&reference).await?;

    // Get all templates to find the one that matches
    let templates = state.registry.list_templates().await?;
    let template = templates
        .iter()
        .find(|t| {
            let template_matches = t.name == parsed_ref.name && t.namespace == parsed_ref.namespace;
            template_matches
        })
        .ok_or_else(|| ApiError::template_not_found(&reference))?;

    let tag = parsed_ref.tag_or_default();
    let response_data = TemplateMetadataResponse {
        name: template.name.clone(),
        namespace: template.namespace.clone(),
        tag: tag.to_string(),
        tags: template.tags.clone(),
        manifest_hash,
        metadata: template.metadata.clone(),
        reference: format!("{}:{}", template.full_name(), tag),
    };

    Ok(Json(ApiResponse::new(response_data)))
}

/// Extract filename from multipart field name like "files[components/header.typ]"
fn extract_filename_from_field(field_name: &str) -> Option<String> {
    if field_name.starts_with("files[") && field_name.ends_with(']') {
        let filename = &field_name[6..field_name.len() - 1]; // Remove "files[" and "]"
        if !filename.is_empty() {
            return Some(filename.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_filename_from_field() {
        assert_eq!(
            extract_filename_from_field("files[main.typ]"),
            Some("main.typ".to_string())
        );

        assert_eq!(
            extract_filename_from_field("files[components/header.typ]"),
            Some("components/header.typ".to_string())
        );

        assert_eq!(
            extract_filename_from_field("files[assets/images/logo.png]"),
            Some("assets/images/logo.png".to_string())
        );

        assert_eq!(extract_filename_from_field("files[]"), None);
        assert_eq!(extract_filename_from_field("other_field"), None);
        assert_eq!(extract_filename_from_field("files["), None);
    }

    #[test]
    fn test_default_tag() {
        assert_eq!(default_tag(), "latest");
    }
}
