//! Papermake is a PDF generation library that uses Typst templates
//! with associated schemas to render PDFs from structured data.

pub mod cache;
pub mod error;
pub mod macros;
pub mod render;
pub mod schema;
pub mod template;
pub mod typst;
// Re-export core types
pub use cache::{CachedTemplate, TemplateCache};
pub use error::{PapermakeError, Result};
pub use render::{RenderOptions, RenderResult, render_pdf};
pub use schema::{FieldType, Schema, SchemaBuilder, SchemaField};
pub use template::{Template, TemplateBuilder};
pub use typst::{TypstFileSystem, TypstWorld};

// Re-export typst types needed by papermake-registry
pub use ::typst::diag::FileError;

/// Get the library version
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
