//! Papermake is a PDF generation library that uses Typst templates
//! with associated schemas to render PDFs from structured data.

pub mod error;
pub mod render;
pub mod typst;
// Re-export core types
pub use error::{
    CompilationError, ConfigError, DataError, DiagnosticInfo, DiagnosticSeverity, FileSystemError,
    PapermakeError, Result, SourceLocation, TemplateError, compilation_error_from_diagnostics,
    convert_typst_diagnostic, template_missing_file,
};
pub use render::{RenderError, RenderResult, render_template, render_template_with_cache};
pub use typst::{InMemoryFileSystem, PapermakeWorld, TypstFileSystem};

// Re-export typst types needed by papermake-registry
pub use ::typst::diag::FileError;

/// Get the library version
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
