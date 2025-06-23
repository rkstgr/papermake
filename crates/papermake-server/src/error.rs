//! Error handling for the API server

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use papermake_registry::RegistryError;
use serde_json::json;
use thiserror::Error;

/// Result type for API operations
pub type Result<T> = std::result::Result<T, ApiError>;

/// API error types
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),
    
    #[error("Render job not found: {0}")]
    RenderNotFound(String),
    
    #[error("Render job failed: {0}")]
    RenderFailed(String),
    
    #[error("Registry error: {0}")]
    Registry(#[from] RegistryError),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Internal server error: {0}")]
    Internal(String),
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Papermake error: {0}")]
    Papermake(#[from] papermake::PapermakeError),
    
    #[error("Timeout error: operation timed out")]
    Timeout,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::TemplateNotFound(_) | ApiError::RenderNotFound(_) => {
                (StatusCode::NOT_FOUND, self.to_string())
            }
            ApiError::Validation(_) | ApiError::BadRequest(_) => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            ApiError::Config(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error".to_string())
            }
            ApiError::Registry(ref e) => match e {
                RegistryError::TemplateNotFound(_) => {
                    (StatusCode::NOT_FOUND, self.to_string())
                }
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "Registry error".to_string())
            }
            ApiError::RenderFailed(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.to_string())
            }
            ApiError::Timeout => {
                (StatusCode::REQUEST_TIMEOUT, "Request timed out".to_string())
            }
            ApiError::Serialization(_) => {
                (StatusCode::BAD_REQUEST, "Invalid JSON format".to_string())
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

// Convenience functions for common errors
impl ApiError {
    pub fn template_not_found(id: &str) -> Self {
        Self::TemplateNotFound(id.to_string())
    }
    
    pub fn render_not_found(id: &str) -> Self {
        Self::RenderNotFound(id.to_string())
    }
    
    pub fn bad_request(msg: &str) -> Self {
        Self::BadRequest(msg.to_string())
    }
    
    pub fn internal(msg: &str) -> Self {
        Self::Internal(msg.to_string())
    }
    
    pub fn validation(msg: &str) -> Self {
        Self::Validation(msg.to_string())
    }
}