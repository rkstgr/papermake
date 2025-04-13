//! PDF rendering functionality

use typst_pdf::PdfOptions;

use crate::error::Result;
use crate::template::Template;
use crate::typst::TypstWorld;
use crate::PapermakeError;

/// Options for PDF rendering
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Paper size (e.g., "a4", "letter")
    pub paper_size: String,
    
    /// Whether to compress the output PDF
    pub compress: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            paper_size: "a4".to_string(),
            compress: true,
        }
    }
}

/// Render a template with data to a PDF
pub fn render_pdf(
    template: &Template,
    data: &serde_json::Value,
    _options: Option<RenderOptions>,
) -> Result<Vec<u8>> {
    // Validate data against schema
    template.validate_data(data)?;
    
    let world = TypstWorld::new(
        template.content.clone(),
        serde_json::to_string(&data).map_err(|e| PapermakeError::Rendering(e.to_string()))?,
    );

    let document = typst::compile(&world)
        .output
        .map_err(|e| PapermakeError::Rendering(format!("compile error: {:?}", e)))?;

    
    let pdf_bytes = typst_pdf::pdf(&document, &PdfOptions::default()).unwrap();
    
    // Return content as bytes
    Ok(pdf_bytes)
}