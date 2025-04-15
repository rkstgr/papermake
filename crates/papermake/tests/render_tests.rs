use papermake::{Schema, SchemaField, FieldType, Template, render_pdf};
use serde_json::json;

#[test]
fn test_render_pdf() {
    // Create a simple schema
    let mut schema = Schema::new();
    schema.add_field(SchemaField {
        key: "name".to_string(),
        label: Some("Name".to_string()),
        field_type: FieldType::String,
        required: true,
        description: None,
        default: None,
    });
    
    // Create a template with the schema
    let template = Template::new(
        "test",
        "Test Template",
        "#let data = json.decode(sys.inputs.data)\nHello #data.name!",
        schema,
    );
    
    // Valid data
    let data = json!({
        "name": "World"
    });
    
    // Render
    let result = render_pdf(&template, &data, None);
    assert!(result.is_ok());
    
    let pdf_bytes = result.unwrap();
    assert!(pdf_bytes.pdf.is_some());

    // Verify PDF structure instead of saving to file
    // 1. Check for PDF header
    let header = &pdf_bytes.pdf.as_ref().unwrap()[0..8];
    assert!(header == b"%PDF-1.7" || header == b"%PDF-1.6" || header == b"%PDF-1.5", 
               "PDF should start with a valid header");

    // 2. Check for basic PDF structure markers
    let pdf_str = String::from_utf8_lossy(&pdf_bytes.pdf.as_ref().unwrap());
    assert!(pdf_str.contains("obj"), "PDF should contain object definitions");
    assert!(pdf_str.contains("endobj"), "PDF should contain object endings");
    assert!(pdf_str.contains("/Type"), "PDF should contain type definitions");
}