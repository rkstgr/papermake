//! Papermake is a PDF generation library that uses Typst templates
//! with associated schemas to render PDFs from structured data.

pub mod error;
pub mod macros;
pub mod render;
pub mod typst;
// Re-export core types
pub use error::{PapermakeError, Result};
pub use render::{RenderResult, render_template};
pub use typst::{TypstFileSystem, TypstWorld};

// Re-export typst types needed by papermake-registry
pub use ::typst::diag::FileError;

/// Get the library version
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
