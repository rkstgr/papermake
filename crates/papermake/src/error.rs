//! Error types for the papermake library

use thiserror::Error;

/// Main error type for the papermake library
#[derive(Error, Debug)]
pub enum PapermakeError {
    #[error("Template error: {0}")]
    Template(String),

    #[error("Schema validation error: {0}")]
    SchemaValidation(String),

    #[error("Rendering error: {0}")]
    Rendering(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Shorthand result type for papermake operations
pub type Result<T> = std::result::Result<T, PapermakeError>;
