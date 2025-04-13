use papermake::{Schema, SchemaField, FieldType, Template, render_pdf};
use serde_json::json;

#[test]
fn test_render_pdf() {
    // Create a simple schema
    let mut schema = Schema::new();
    schema.add_field(SchemaField {
        name: "name".to_string(),
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
    assert!(!pdf_bytes.is_empty());

    // Save to file
    let file_path = "test_output.pdf";
    std::fs::write(file_path, &pdf_bytes).unwrap();
    println!("PDF saved to {}", file_path);
}