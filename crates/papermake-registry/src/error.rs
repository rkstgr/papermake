//! Error types for the papermake registry

use thiserror::Error;

/// Registry-specific errors
#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),
    
    #[error("Render job not found: {0}")]
    RenderJobNotFound(String),
    
    #[error("Version {version} not found for template {template_id}")]
    VersionNotFound { template_id: String, version: u64 },
    
    #[error("Access denied: {0}")]
    AccessDenied(String),
    
    #[error("User not found: {0}")]
    UserNotFound(String),
    
    #[error("Organization not found: {0}")]
    OrganizationNotFound(String),
    
    #[error("Template already exists: {0}")]
    TemplateAlreadyExists(String),
    
    #[error("Cannot modify immutable template version {version} of {template_id}")]
    ImmutableVersion { template_id: String, version: u64 },
    
    #[error("Invalid template scope: {0}")]
    InvalidScope(String),
    
    #[error("Fork source not found: {template_id} version {version}")]
    ForkSourceNotFound { template_id: String, version: u64 },
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Papermake error: {0}")]
    Papermake(#[from] papermake::PapermakeError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Time error: {0}")]
    Time(#[from] time::error::ComponentRange),
}

/// Result type for registry operations
pub type Result<T> = std::result::Result<T, RegistryError>;