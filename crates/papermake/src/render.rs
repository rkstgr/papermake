//! PDF rendering functionality
//!
//! This module provides the main template rendering functionality,
//! converting Typst templates with JSON data into PDF documents.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use typst::World;
use typst::WorldExt;
use typst_pdf::{PdfOptions, PdfStandards};

use crate::RenderFileSystem;
use crate::error::{CompilationError, ConfigError, PapermakeError, Result};
use crate::typst::PapermakeWorld;

/// Individual rendering error with location information
///
/// This struct captures detailed information about a single rendering error,
/// including its location in the source and a descriptive message.
#[derive(Debug, Serialize, Clone)]
pub struct RenderError {
    /// The error message
    pub message: String,
    /// Starting position in the source
    pub start: usize,
    /// Ending position in the source
    pub end: usize,
    /// Optional file path where the error occurred
    pub file: Option<String>,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.file {
            Some(file) => write!(f, "{}:{}-{}: {}", file, self.start, self.end, self.message),
            None => write!(f, "{}:{}: {}", self.start, self.end, self.message),
        }
    }
}

/// Result of template rendering operation
///
/// Contains either the successfully generated PDF bytes or detailed error information.
/// Even when PDF generation succeeds, there may be warnings in the errors vector.
#[derive(Debug, Serialize)]
pub struct RenderResult {
    /// The generated PDF bytes (None if compilation failed)
    pub pdf: Option<Vec<u8>>,
    /// List of compilation errors and warnings
    pub errors: Vec<RenderError>,
    /// Whether the rendering was successful (PDF was generated)
    pub success: bool,
}

/// PDF standard the exported document should conform to
///
/// Standards are enforced by Typst's PDF exporter. The PDF/A variants produce
/// archivable output; `A3b` additionally allows arbitrary embedded files, which
/// is the basis for hybrid e-invoice formats such as ZUGFeRD/Factur-X.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PdfStandard {
    /// PDF 1.7 (the default)
    #[serde(rename = "1.7")]
    V1_7,
    /// PDF/A-2b
    #[serde(rename = "a-2b")]
    A2b,
    /// PDF/A-3b
    #[serde(rename = "a-3b")]
    A3b,
}

impl From<PdfStandard> for typst_pdf::PdfStandard {
    fn from(standard: PdfStandard) -> Self {
        match standard {
            PdfStandard::V1_7 => typst_pdf::PdfStandard::V_1_7,
            PdfStandard::A2b => typst_pdf::PdfStandard::A_2b,
            PdfStandard::A3b => typst_pdf::PdfStandard::A_3b,
        }
    }
}

/// Options controlling PDF export
#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    /// PDF standards the output must conform to (empty = plain PDF 1.7)
    pub pdf_standards: Vec<PdfStandard>,
}

impl RenderOptions {
    /// Options for PDF/A-3b output (e.g. as the base for ZUGFeRD/Factur-X e-invoices)
    pub fn pdf_a3b() -> Self {
        Self {
            pdf_standards: vec![PdfStandard::A3b],
        }
    }
}

/// Build typst-pdf export options from render options
fn pdf_options(options: &RenderOptions) -> Result<PdfOptions<'static>> {
    let standards: Vec<typst_pdf::PdfStandard> = options
        .pdf_standards
        .iter()
        .copied()
        .map(Into::into)
        .collect();
    let standards = PdfStandards::new(&standards).map_err(|e| {
        PapermakeError::Config(ConfigError::InvalidConfig {
            setting: "pdf_standards".to_string(),
            reason: e.to_string(),
        })
    })?;
    Ok(PdfOptions {
        standards,
        ..Default::default()
    })
}

/// Render a Typst template to PDF
///
/// This is the main public API for template compilation. It takes a template string,
/// a file system for resolving imports, and JSON data to inject into the template.
///
/// # Arguments
///
/// * `main_typ` - The main Typst template content as a string
/// * `file_system` - File system abstraction for resolving imports and assets
/// * `data` - JSON data to inject into the template
///
/// # Returns
///
/// Returns a `RenderResult` containing either the PDF bytes (on success) or
/// detailed error information (on failure).
///
/// # Errors
///
/// This function can return various errors:
/// - `DataError` - JSON serialization issues
/// - `CompilationError` - Typst compilation failures
/// - `FileSystemError` - File access issues during import resolution
///
/// # Example
///
/// ```rust,no_run
/// use papermake::{render_template, typst::InMemoryFileSystem};
/// use std::sync::Arc;
///
/// let template = "Hello #data.name!";
/// let fs = Arc::new(InMemoryFileSystem::new());
/// let data = serde_json::json!({ "name": "World" });
///
/// let result = render_template(template.to_string(), fs, &data).unwrap();
/// if result.success {
///     println!("PDF generated: {} bytes", result.pdf.unwrap().len());
/// } else {
///     for error in result.errors {
///         println!("Error: {}", error);
///     }
/// }
/// ```
pub fn render_template(
    main_typ: String,
    file_system: Arc<dyn RenderFileSystem>,
    data: &serde_json::Value,
) -> Result<RenderResult> {
    render_template_with_options(main_typ, file_system, data, &RenderOptions::default())
}

/// Render a Typst template to PDF with explicit export options
///
/// Behaves like [`render_template`], but lets the caller control PDF export,
/// e.g. requesting PDF/A-3b conformant output:
///
/// ```rust,no_run
/// use papermake::{render_template_with_options, RenderOptions, typst::InMemoryFileSystem};
/// use std::sync::Arc;
///
/// let result = render_template_with_options(
///     "Hello #data.name!".to_string(),
///     Arc::new(InMemoryFileSystem::new()),
///     &serde_json::json!({ "name": "World" }),
///     &RenderOptions::pdf_a3b(),
/// ).unwrap();
/// ```
pub fn render_template_with_options(
    main_typ: String,
    file_system: Arc<dyn RenderFileSystem>,
    data: &serde_json::Value,
    options: &RenderOptions,
) -> Result<RenderResult> {
    let pdf_opts = pdf_options(options)?;
    let data_str = serde_json::to_string(&data)?;

    let world = PapermakeWorld::with_file_system(main_typ, data_str, file_system);

    let compile_result = typst::compile(&world);

    let mut errors = Vec::new();
    let mut pdf = None;
    let mut success = false;

    match compile_result.output {
        Ok(document) => {
            // Compilation succeeded, generate PDF
            match typst_pdf::pdf(&document, &pdf_opts) {
                Ok(pdf_bytes) => {
                    pdf = Some(pdf_bytes);
                    success = true;
                }
                Err(pdf_error) => {
                    errors.push(RenderError {
                        message: format!("PDF generation failed: {:?}", pdf_error),
                        start: 0,
                        end: 0,
                        file: None,
                    });
                }
            }
        }
        Err(diagnostics) => {
            // Compilation failed, collect diagnostic information
            for diagnostic in diagnostics {
                let span = diagnostic.span;
                let mut render_error = RenderError {
                    message: diagnostic.message.to_string(),
                    start: 0,
                    end: 0,
                    file: None,
                };

                // Try to get source location information
                if let Some(id) = span.id() {
                    if let Ok(_source) = world.source(id) {
                        render_error.file = Some(format!("{:?}", id));
                        if let Some(range) = world.range(span) {
                            render_error.start = range.start;
                            render_error.end = range.end;
                        }
                    }
                }

                errors.push(render_error);
            }
        }
    }

    Ok(RenderResult {
        pdf,
        errors,
        success,
    })
}

/// Render a template with caching support
///
/// This function allows reusing a compiled world for multiple renders with different data,
/// which can improve performance when rendering the same template multiple times.
///
/// # Arguments
///
/// * `main_typ` - The main Typst template content as a string
/// * `file_system` - File system abstraction for resolving imports and assets
/// * `data` - JSON data to inject into the template
/// * `world_cache` - Optional cached world to reuse (will be updated with new data)
///
/// # Returns
///
/// Returns a `RenderResult` containing either the PDF bytes or error information.
///
/// # Performance Note
///
/// When providing a cached world, make sure the template content hasn't changed,
/// as this function only updates the data, not the template structure.
pub fn render_template_with_cache(
    main_typ: String,
    file_system: Arc<dyn RenderFileSystem>,
    data: serde_json::Value,
    world_cache: Option<&mut PapermakeWorld>,
) -> Result<RenderResult> {
    let data_str = serde_json::to_string(&data)?;

    let world = match world_cache {
        Some(cached_world) => {
            // Update the data in the existing world
            cached_world.update_data(data_str).map_err(|e| {
                PapermakeError::Compilation(CompilationError::DataInjection {
                    reason: format!("Failed to update cached world data: {}", e),
                })
            })?;
            cached_world
        }
        None => {
            // Create a new world if no cache is provided
            return render_template(main_typ, file_system, &data);
        }
    };

    // Compile with the updated world
    let compile_result = typst::compile(world as &dyn World);

    let mut errors = Vec::new();
    let mut pdf = None;
    let mut success = false;

    match compile_result.output {
        Ok(document) => match typst_pdf::pdf(&document, &PdfOptions::default()) {
            Ok(pdf_bytes) => {
                pdf = Some(pdf_bytes);
                success = true;
            }
            Err(pdf_error) => {
                errors.push(RenderError {
                    message: format!("PDF generation failed: {:?}", pdf_error),
                    start: 0,
                    end: 0,
                    file: None,
                });
            }
        },
        Err(diagnostics) => {
            for diagnostic in diagnostics {
                let span = diagnostic.span;
                let mut render_error = RenderError {
                    message: diagnostic.message.to_string(),
                    start: 0,
                    end: 0,
                    file: None,
                };

                if let Some(id) = span.id() {
                    if let Ok(_source) = world.source(id) {
                        render_error.file = Some(format!("{:?}", id));
                        if let Some(range) = world.range(span) {
                            render_error.start = range.start;
                            render_error.end = range.end;
                        }
                    }
                }

                errors.push(render_error);
            }
        }
    }

    Ok(RenderResult {
        pdf,
        errors,
        success,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typst::InMemoryFileSystem;

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    fn render_with(options: &RenderOptions) -> RenderResult {
        render_template_with_options(
            "Hello #data.name!".to_string(),
            Arc::new(InMemoryFileSystem::new()),
            &serde_json::json!({ "name": "World" }),
            options,
        )
        .unwrap()
    }

    #[test]
    fn default_render_is_plain_pdf() {
        let result = render_with(&RenderOptions::default());
        assert!(result.success, "render failed: {:?}", result.errors);
        let pdf = result.pdf.unwrap();
        assert!(pdf.starts_with(b"%PDF"));
        // No PDF/A conformance metadata without a requested standard
        assert!(!contains(&pdf, b"pdfaid"));
    }

    #[test]
    fn pdf_a3b_render_declares_conformance() {
        let result = render_with(&RenderOptions::pdf_a3b());
        assert!(
            result.success,
            "PDF/A-3b render failed: {:?}",
            result.errors
        );
        let pdf = result.pdf.unwrap();
        assert!(pdf.starts_with(b"%PDF"));
        // typst-pdf writes pdfaid:part / pdfaid:conformance into the XMP metadata
        assert!(contains(&pdf, b"pdfaid"));
    }

    #[test]
    fn conflicting_standards_are_rejected() {
        let options = RenderOptions {
            pdf_standards: vec![PdfStandard::A2b, PdfStandard::A3b],
        };
        let result = render_template_with_options(
            "Hello".to_string(),
            Arc::new(InMemoryFileSystem::new()),
            &serde_json::json!({}),
            &options,
        );
        assert!(matches!(result, Err(PapermakeError::Config(_))));
    }

    #[test]
    fn pdf_standard_serde_uses_typst_cli_names() {
        assert_eq!(
            serde_json::to_string(&PdfStandard::A3b).unwrap(),
            "\"a-3b\""
        );
        assert_eq!(
            serde_json::from_str::<PdfStandard>("\"a-2b\"").unwrap(),
            PdfStandard::A2b
        );
        assert_eq!(
            serde_json::from_str::<PdfStandard>("\"1.7\"").unwrap(),
            PdfStandard::V1_7
        );
    }
}
