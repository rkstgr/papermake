use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    response::Response,
    routing::get,
};

use crate::{
    AppState,
    error::{ApiError, Result as ApiResult},
};

pub fn router() -> Router<AppState> {
    Router::new().route("/{render_id}/pdf", get(get_render_pdf))
}

#[axum::debug_handler]
pub async fn get_render_pdf(
    State(state): State<AppState>,
    Path(render_id): Path<String>,
) -> ApiResult<Response<Body>> {
    let pdf_bytes = state
        .registry
        .get_render_pdf(&render_id)
        .await
        .map_err(|e| match e {
            papermake_registry::RegistryError::RenderStorage(_) => {
                ApiError::render_not_found(&render_id)
            }
            _ => ApiError::Internal(e.to_string()),
        })?;

    let filename = format!("render-{}.pdf", render_id);

    Ok(Response::builder()
        .header(CONTENT_TYPE, "application/pdf")
        .header(
            CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(pdf_bytes))
        .unwrap())
}
