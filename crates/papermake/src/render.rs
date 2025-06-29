//! PDF rendering functionality

use std::sync::Arc;

use serde::Serialize;
use typst::World;
use typst::WorldExt;
use typst_pdf::PdfOptions;

use crate::PapermakeError;
use crate::TypstFileSystem;
use crate::error::Result;
use crate::typst::TypstWorld;

#[derive(Debug, Serialize)]
pub struct RenderError {
    pub message: String,
    pub start: usize,
    pub end: usize,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}:{})", self.message, self.start, self.end)
    }
}

#[derive(Debug, Serialize)]
pub struct RenderResult {
    pub pdf: Option<Vec<u8>>,
    pub errors: Vec<RenderError>,
}

/// Render a Typst template to PDF
///
/// This is the main public API for template compilation.
pub fn render_template(
    main_typ: String,
    file_system: Arc<dyn TypstFileSystem>,
    data: serde_json::Value,
) -> Result<Vec<u8>, CompileError> {
    let data_str = serde_json::to_string(&data)?;
    let world = TypstWorld::with_file_system(main_typ, data_str, file_system);

    let compile_result = typst::compile(&world);

    let mut errors = Vec::new();
    let mut pdf = None;

    match compile_result.output {
        Ok(document) => {
            pdf = Some(typst_pdf::pdf(&document, &PdfOptions::default()).unwrap());
        }
        Err(diagnostics) => {
            for diagnostic in diagnostics {
                let span = diagnostic.span;
                if let Some(id) = span.id() {
                    if let Ok(_file) = world.source(id) {
                        if let Some(range) = world.range(span) {
                            errors.push(RenderError {
                                message: diagnostic.message.to_string(),
                                start: range.start,
                                end: range.end,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(RenderResult { pdf, errors })
}

pub fn render_template_with_cache(
    main_typ: String,
    file_system: Arc<dyn TypstFileSystem>,
    data: serde_json::Value,
    world_cache: Option<&mut TypstWorld>,
) -> Result<RenderResult> {
    todo!("Implement this")
    // Either use the cached world or create a new one
    // let world = match world_cache {
    //     Some(cached_world) => {
    //         // Update the inputs in the existing world
    //         cached_world
    //             .update_data(
    //                 serde_json::to_string(&data)
    //                     .map_err(|e| PapermakeError::Rendering(e.to_string()))?,
    //             )
    //             .map_err(|e| PapermakeError::Rendering(e.to_string()))?;
    //         // Make sure to reset tracking state
    //         // cached_world.reset(); TODO: Implement this
    //         cached_world
    //     }
    //     None => &mut TypstWorld::new(
    //         template.main_typ.clone(),
    //         serde_json::to_string(&data).map_err(|e| PapermakeError::Rendering(e.to_string()))?,
    //     ),
    // };

    // let compile_result = typst::compile(world as &dyn World);

    // let mut errors = Vec::new();
    // let mut pdf = None;

    // match compile_result.output {
    //     Ok(document) => {
    //         pdf = Some(typst_pdf::pdf(&document, &PdfOptions::default()).unwrap());
    //     }
    //     Err(diagnostics) => {
    //         for diagnostic in diagnostics {
    //             let span = diagnostic.span;
    //             if let Some(id) = span.id() {
    //                 if let Ok(_file) = world.source(id) {
    //                     if let Some(range) = world.range(span) {
    //                         errors.push(RenderError {
    //                             message: diagnostic.message.to_string(),
    //                             start: range.start,
    //                             end: range.end,
    //                         });
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    // Ok(RenderResult { pdf, errors })
}
