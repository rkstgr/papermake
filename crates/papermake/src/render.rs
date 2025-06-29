//! PDF rendering functionality
//!
//! This module provides the main template rendering functionality,
//! converting Typst templates with JSON data into PDF documents.

use std::sync::Arc;

use serde::Serialize;
use typst::World;
use typst::WorldExt;
use typst_pdf::PdfOptions;

use crate::TypstFileSystem;
use crate::error::{CompilationError, PapermakeError, Result};
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
    file_system: Arc<dyn TypstFileSystem>,
    data: &serde_json::Value,
) -> Result<RenderResult> {
    // Serialize the data to JSON string for injection into Typst
    let data_str = serde_json::to_string(&data)?;

    // Create the Typst world with the template and data
    let world = PapermakeWorld::with_file_system(main_typ, data_str, file_system);

    // Compile the template
    let compile_result = typst::compile(&world);

    let mut errors = Vec::new();
    let mut pdf = None;
    let mut success = false;

    match compile_result.output {
        Ok(document) => {
            // Compilation succeeded, generate PDF
            match typst_pdf::pdf(&document, &PdfOptions::default()) {
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
    file_system: Arc<dyn TypstFileSystem>,
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
