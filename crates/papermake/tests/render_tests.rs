use std::sync::Arc;

use papermake::{InMemoryFileSystem, render_template};
use serde_json::json;

#[test]
fn test_render_pdf() {
    // Use the fonts bundled in the repo so the test doesn't depend on system fonts
    unsafe {
        std::env::set_var(
            "FONTS_DIR",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../fonts"),
        );
    }

    // Valid data
    let data = json!({
        "name": "World"
    });

    // Render
    let result = render_template(
        "#set text(font: \"Linux Libertine\")\nHello #data.name!".to_string(),
        Arc::new(InMemoryFileSystem::new()),
        &data,
    );
    assert!(result.is_ok());

    let pdf_bytes = result.unwrap();
    assert!(pdf_bytes.pdf.is_some());

    // Write PDF to temp file for manual inspection if needed
    let temp_dir = std::env::temp_dir();
    let pdf_path = temp_dir.join("test_bundled_font.pdf");
    std::fs::write(&pdf_path, pdf_bytes.pdf.as_ref().unwrap()).unwrap();
    println!("PDF written to: {}", pdf_path.display());

    // Verify PDF structure instead of saving to file
    // 1. Check for PDF header
    let header = &pdf_bytes.pdf.as_ref().unwrap()[0..8];
    assert!(
        header == b"%PDF-1.7" || header == b"%PDF-1.6" || header == b"%PDF-1.5",
        "PDF should start with a valid header"
    );

    // Parse PDF and check for font
    let file = pdf::file::FileOptions::cached().open(&pdf_path).unwrap();
    let mut found_libertine = false;

    // Check each page's resources for fonts
    if let Ok(page) = file.get_page(0) {
        if let Ok(resources) = page.resources() {
            for (_, font_ref) in resources.fonts.iter() {
                if let Ok(font) = font_ref.load(&file.resolver()) {
                    if let Some(name) = &font.name {
                        if name.to_string().to_lowercase().contains("libertine") {
                            found_libertine = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    assert!(found_libertine, "PDF should contain the Linux Libertine font");
}
