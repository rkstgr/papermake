//! Error types for the papermake library
//!
//! This module provides a comprehensive error hierarchy for all papermake operations.
//! Errors are organized by domain to provide clear context and actionable information.

use std::fmt;
use thiserror::Error;
use typst::diag::{FileError, SourceDiagnostic};

/// Main error type for the papermake library
///
/// This is the root error type that encompasses all possible errors that can occur
/// during papermake operations. Each variant represents a different domain of errors.
#[derive(Error, Debug)]
pub enum PapermakeError {
    /// Template-related errors (structure, validation, loading)
    #[error("Template error: {0}")]
    Template(#[from] TemplateError),

    /// Typst compilation errors with rich diagnostics
    #[error("Compilation error: {0}")]
    Compilation(#[from] CompilationError),

    /// File system operations (reading, writing, permissions)
    #[error("File system error: {0}")]
    FileSystem(#[from] FileSystemError),

    /// Data serialization and validation errors
    #[error("Data error: {0}")]
    Data(#[from] DataError),

    /// Configuration and initialization errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
}

/// Template-related errors
///
/// These errors occur during template loading, validation, or structure analysis.
#[derive(Error, Debug)]
pub enum TemplateError {
    #[error("Template not found: {path}")]
    NotFound { path: String },

    #[error("Invalid template structure: {message}")]
    InvalidStructure { message: String },

    #[error("Missing required file: {file}")]
    MissingFile { file: String },

    #[error("Invalid template content: {reason}")]
    InvalidContent { reason: String },

    #[error("Template dependency error: {dependency} - {reason}")]
    DependencyError { dependency: String, reason: String },
}

/// Typst compilation errors with rich diagnostics
///
/// These errors occur during the Typst compilation process and include
/// detailed diagnostic information when available.
#[derive(Error, Debug)]
pub enum CompilationError {
    #[error("Typst compilation failed with {error_count} error(s)")]
    TypstError {
        error_count: usize,
        diagnostics: Vec<DiagnosticInfo>,
    },

    #[error("Template compilation failed: {message}")]
    TemplateCompilation { message: String },

    #[error("Data injection failed: {reason}")]
    DataInjection { reason: String },

    #[error("Syntax error in template: {message}")]
    SyntaxError { message: String },

    #[error("Import resolution failed: {import_path} - {reason}")]
    ImportResolution { import_path: String, reason: String },
}

/// File system related errors
///
/// These errors occur during file operations and provide context about
/// what operation failed and why.
#[derive(Error, Debug)]
pub enum FileSystemError {
    #[error("File not found: {path}")]
    NotFound { path: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Invalid file path: {path}")]
    InvalidPath { path: String },

    #[error("File read error: {path} - {reason}")]
    ReadError { path: String, reason: String },

    #[error("File write error: {path} - {reason}")]
    WriteError { path: String, reason: String },

    #[error("Invalid UTF-8 content in file: {path}")]
    InvalidUtf8 { path: String },
}

/// Data serialization and validation errors
///
/// These errors occur during JSON serialization/deserialization and
/// schema validation operations.
#[derive(Error, Debug)]
pub enum DataError {
    #[error("JSON serialization failed: {reason}")]
    Serialization { reason: String },

    #[error("JSON deserialization failed: {reason}")]
    Deserialization { reason: String },

    #[error("Schema validation failed: {message}")]
    SchemaValidation { message: String },

    #[error("Invalid data format: {expected}, got {actual}")]
    InvalidFormat { expected: String, actual: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid field value: {field} - {reason}")]
    InvalidFieldValue { field: String, reason: String },
}

/// Configuration and initialization errors
///
/// These errors occur during system configuration, font loading,
/// and other initialization tasks.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Font loading failed: {reason}")]
    FontLoading { reason: String },

    #[error("Cache initialization failed: {reason}")]
    CacheInit { reason: String },

    #[error("Invalid configuration: {setting} - {reason}")]
    InvalidConfig { setting: String, reason: String },

    #[error("Environment variable error: {var} - {reason}")]
    Environment { var: String, reason: String },

    #[error("Runtime error: {message}")]
    Runtime { message: String },
}

/// Rich diagnostic information from Typst compilation
///
/// This struct captures detailed information about compilation errors
/// including source location, severity, and helpful hints.
#[derive(Debug, Clone)]
pub struct DiagnosticInfo {
    /// The error message
    pub message: String,
    /// Severity level (error, warning, info)
    pub severity: DiagnosticSeverity,
    /// Source location information
    pub location: Option<SourceLocation>,
    /// Helpful hints for fixing the error
    pub hints: Vec<String>,
}

/// Diagnostic severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

/// Source location information for diagnostics
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// File path or identifier
    pub file: String,
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Character range in the source
    pub range: Option<(usize, usize)>,
}

// Implement Display for DiagnosticInfo to provide user-friendly error messages
impl fmt::Display for DiagnosticInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.location {
            Some(loc) => write!(f, "{}:{}: {}", loc.file, loc.line, self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

// Implement Display for DiagnosticSeverity
impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticSeverity::Error => write!(f, "error"),
            DiagnosticSeverity::Warning => write!(f, "warning"),
            DiagnosticSeverity::Info => write!(f, "info"),
        }
    }
}

/// Shorthand result type for papermake operations
pub type Result<T> = std::result::Result<T, PapermakeError>;

/// Utility function to convert Typst diagnostics to our diagnostic format
pub fn convert_typst_diagnostic(diagnostic: SourceDiagnostic) -> DiagnosticInfo {
    DiagnosticInfo {
        message: diagnostic.message.to_string(),
        severity: DiagnosticSeverity::Error, // Typst diagnostics are typically errors
        location: None, // Will be filled in by the caller with file context
        hints: diagnostic.hints.into_iter().map(|h| h.to_string()).collect(),
    }
}

/// Create a template error for missing files
pub fn template_missing_file<S: Into<String>>(file: S) -> PapermakeError {
    PapermakeError::Template(TemplateError::MissingFile { file: file.into() })
}

/// Create a compilation error from Typst diagnostics
pub fn compilation_error_from_diagnostics(diagnostics: Vec<SourceDiagnostic>) -> PapermakeError {
    let diagnostic_infos: Vec<DiagnosticInfo> = diagnostics
        .into_iter()
        .map(convert_typst_diagnostic)
        .collect();
    
    let error_count = diagnostic_infos.len();
    
    PapermakeError::Compilation(CompilationError::TypstError {
        error_count,
        diagnostics: diagnostic_infos,
    })
}

// ============================================================================
// From Implementations for External Error Types
// ============================================================================

/// Convert std::io::Error to PapermakeError
impl From<std::io::Error> for PapermakeError {
    fn from(error: std::io::Error) -> Self {
        let reason = error.to_string();
        match error.kind() {
            std::io::ErrorKind::NotFound => {
                PapermakeError::FileSystem(FileSystemError::NotFound {
                    path: "<unknown>".to_string(),
                })
            }
            std::io::ErrorKind::PermissionDenied => {
                PapermakeError::FileSystem(FileSystemError::PermissionDenied {
                    path: "<unknown>".to_string(),
                })
            }
            _ => PapermakeError::FileSystem(FileSystemError::ReadError {
                path: "<unknown>".to_string(),
                reason,
            }),
        }
    }
}

/// Convert serde_json::Error to PapermakeError
impl From<serde_json::Error> for PapermakeError {
    fn from(error: serde_json::Error) -> Self {
        let reason = error.to_string();
        if error.is_syntax() || error.is_data() {
            PapermakeError::Data(DataError::Deserialization { reason })
        } else {
            PapermakeError::Data(DataError::Serialization { reason })
        }
    }
}

/// Convert typst::diag::FileError to PapermakeError
impl From<FileError> for PapermakeError {
    fn from(error: FileError) -> Self {
        match error {
            FileError::NotFound(path) => {
                PapermakeError::FileSystem(FileSystemError::NotFound {
                    path: path.display().to_string(),
                })
            }
            FileError::AccessDenied => {
                PapermakeError::FileSystem(FileSystemError::PermissionDenied {
                    path: "<unknown>".to_string(),
                })
            }
            FileError::InvalidUtf8 => {
                PapermakeError::FileSystem(FileSystemError::InvalidUtf8 {
                    path: "<unknown>".to_string(),
                })
            }
            FileError::Other(msg) => {
                PapermakeError::FileSystem(FileSystemError::ReadError {
                    path: "<unknown>".to_string(),
                    reason: msg.map(|m| m.to_string()).unwrap_or_else(|| "Unknown error".to_string()),
                })
            }
            FileError::IsDirectory => {
                PapermakeError::FileSystem(FileSystemError::InvalidPath {
                    path: "<directory>".to_string(),
                })
            }
            FileError::NotSource => {
                PapermakeError::FileSystem(FileSystemError::ReadError {
                    path: "<unknown>".to_string(),
                    reason: "File is not a Typst source file".to_string(),
                })
            }
            FileError::Package(pkg_error) => {
                PapermakeError::FileSystem(FileSystemError::ReadError {
                    path: "<package>".to_string(),
                    reason: format!("Package error: {:?}", pkg_error),
                })
            }
        }
    }
}

/// Convert std::string::FromUtf8Error to PapermakeError
impl From<std::string::FromUtf8Error> for PapermakeError {
    fn from(_error: std::string::FromUtf8Error) -> Self {
        PapermakeError::FileSystem(FileSystemError::InvalidUtf8 {
            path: "<unknown>".to_string(),
        })
    }
}

// ============================================================================
// Error Helper Functions
// ============================================================================

impl PapermakeError {
    /// Get a user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            PapermakeError::Template(e) => match e {
                TemplateError::NotFound { path } => {
                    format!("Template not found: {}", path)
                }
                TemplateError::InvalidStructure { message } => {
                    format!("Invalid template structure: {}", message)
                }
                TemplateError::MissingFile { file } => {
                    format!("Template is missing required file: {}", file)
                }
                _ => format!("Template error: {}", e),
            },
            PapermakeError::Compilation(e) => match e {
                CompilationError::TypstError { error_count, .. } => {
                    format!("Template compilation failed with {} error(s)", error_count)
                }
                _ => format!("Compilation error: {}", e),
            },
            PapermakeError::FileSystem(e) => match e {
                FileSystemError::NotFound { path } => {
                    format!("File not found: {}", path)
                }
                FileSystemError::PermissionDenied { path } => {
                    format!("Permission denied accessing: {}", path)
                }
                _ => format!("File system error: {}", e),
            },
            PapermakeError::Data(e) => match e {
                DataError::Serialization { .. } => {
                    "Failed to serialize data. Please check your data format.".to_string()
                }
                DataError::Deserialization { .. } => {
                    "Failed to parse data. Please check your JSON format.".to_string()
                }
                _ => format!("Data error: {}", e),
            },
            PapermakeError::Config(e) => {
                format!("Configuration error: {}", e)
            }
        }
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            PapermakeError::Template(TemplateError::NotFound { .. }) => false,
            PapermakeError::FileSystem(FileSystemError::NotFound { .. }) => false,
            PapermakeError::FileSystem(FileSystemError::PermissionDenied { .. }) => false,
            PapermakeError::Config(_) => false,
            _ => true,
        }
    }

    /// Get error suggestions for common problems
    pub fn suggestions(&self) -> Vec<String> {
        match self {
            PapermakeError::Template(TemplateError::NotFound { .. }) => {
                vec![
                    "Check if the template path is correct".to_string(),
                    "Verify the template exists in the expected location".to_string(),
                ]
            }
            PapermakeError::Data(DataError::Deserialization { .. }) => {
                vec![
                    "Verify your JSON syntax is valid".to_string(),
                    "Check for missing quotes or trailing commas".to_string(),
                    "Validate your data against the template schema".to_string(),
                ]
            }
            PapermakeError::Compilation(CompilationError::DataInjection { .. }) => {
                vec![
                    "Ensure your data matches the expected structure".to_string(),
                    "Check if required fields are present".to_string(),
                ]
            }
            _ => vec![],
        }
    }
}
