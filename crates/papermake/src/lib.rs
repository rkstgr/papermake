//! Papermake is a PDF generation library that uses Typst templates
//! with associated schemas to render PDFs from structured data.

pub mod error;
pub mod schema;
pub mod template;
pub mod render;
pub mod typst;
pub mod macros;
pub mod cache;
// Re-export core types
pub use error::{PapermakeError, Result};
pub use schema::{Schema, SchemaField, FieldType, SchemaBuilder};
pub use template::{Template, TemplateId, TemplateBuilder};
pub use render::{render_pdf, RenderOptions, RenderResult};
pub use cache::{CachedTemplate, TemplateCache};

/// Get the library version
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}