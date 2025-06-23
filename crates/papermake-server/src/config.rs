//! Server configuration management

use crate::error::{ApiError, Result};
use serde::{Deserialize, Serialize};

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host to bind to
    pub host: String,
    
    /// Port to bind to
    pub port: u16,
    
    /// Maximum number of concurrent render jobs
    pub max_concurrent_renders: usize,
    
    /// Timeout for render jobs in seconds
    pub render_timeout_seconds: u64,
    
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
    
    /// Whether to enable debug logging
    pub debug: bool,
}

impl ServerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .map_err(|_| ApiError::Config("Invalid PORT value".to_string()))?,
            max_concurrent_renders: std::env::var("MAX_CONCURRENT_RENDERS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .map_err(|_| ApiError::Config("Invalid MAX_CONCURRENT_RENDERS value".to_string()))?,
            render_timeout_seconds: std::env::var("RENDER_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "300".to_string()) // 5 minutes default
                .parse()
                .map_err(|_| ApiError::Config("Invalid RENDER_TIMEOUT_SECONDS value".to_string()))?,
            cors_origins: std::env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "*".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            debug: std::env::var("DEBUG")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(false),
        })
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            max_concurrent_renders: 10,
            render_timeout_seconds: 300,
            cors_origins: vec!["*".to_string()],
            debug: false,
        }
    }
}