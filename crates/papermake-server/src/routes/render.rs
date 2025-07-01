use axum::{
    Json, Router,
    extract::{Path, State},
    routing::post,
};
use papermake::RenderError;
use papermake_registry::registry::RenderResult;
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    error::{ApiError, Result},
    models::ApiResponse,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/{reference}", post(render_template))
}

#[derive(Debug, Deserialize)]
pub struct RenderRequest {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct RenderResponse {
    pub render_id: String,
    pub pdf_hash: String,
    pub duration_ms: u32,
}

#[axum::debug_handler]
pub async fn render_template(
    State(state): State<AppState>,
    Path(reference): Path<String>,
    Json(request): Json<RenderRequest>,
) -> Result<Json<ApiResponse<RenderResult>>> {
    let result = state
        .registry
        .render_and_store(&reference, &request.data)
        .await
        .map_err(|e| ApiError::RenderFailed(e.to_string()))?;

    Ok(Json(ApiResponse::new(result)))
}
