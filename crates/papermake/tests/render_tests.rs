use papermake::{Schema, Template, render_pdf};
use pdf::object::MaybeRef;
use serde_json::json;

#[test]
fn test_render_pdf() {
    // Create a template with the schema
    let template = Template::new(
        "test",
        "Test Template",
        "#let data = json.decode(sys.inputs.data)\n#set text(font: \"Arial\")\nHello #data.name!",
        Schema::new(),
    );

    // Valid data
    let data = json!({
        "name": "World"
    });

    // Render
    let result = render_pdf(&template, &data);
    assert!(result.is_ok());

    let pdf_bytes = result.unwrap();
    assert!(pdf_bytes.pdf.is_some());

    // Write PDF to temp file for manual inspection if needed
    let temp_dir = std::env::temp_dir();
    let pdf_path = temp_dir.join("test_system_font.pdf");
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
    let mut found_arial = false;

    // Check each page's resources for fonts
    if let Ok(page) = file.get_page(0) {
        if let Ok(resources) = page.resources() {
            for (_, font_ref) in resources.fonts.iter() {
                match font_ref {
                    MaybeRef::Direct(font) => {
                        if let Some(name) = &font.name {
                            if name.to_string().to_lowercase().contains("arial") {
                                found_arial = true;
                                break;
                            }
                        }
                    }
                    MaybeRef::Indirect(r) => {
                        let font = r.data();
                        if let Some(name) = &font.name {
                            if name.to_string().to_lowercase().contains("arial") {
                                found_arial = true;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    assert!(found_arial, "PDF should contain Arial font");
}
