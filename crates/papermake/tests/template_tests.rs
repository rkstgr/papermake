use papermake::{Schema, SchemaField, FieldType, Template};
use serde_json::json;

#[test]
fn test_template_schema_validation() {
    // Create a simple schema
    let mut schema = Schema::new();
    schema.add_field(SchemaField {
        name: "name".to_string(),
        field_type: FieldType::String,
        required: true,
        description: Some("Customer name".to_string()),
        default: None,
    }).add_field(SchemaField {
        name: "age".to_string(),
        field_type: FieldType::Number,
        required: false,
        description: Some("Customer age".to_string()),
        default: None,
    });
    
    // Create a template with the schema
    let template = Template::new(
        "invoice",
        "Invoice Template",
        "#let title = [Invoice]\n#title\nCustomer: #name\n",
        schema,
    );
    
    // Valid data should validate successfully
    let valid_data = json!({
        "name": "John Doe",
        "age": 30
    });
    
    assert!(template.validate_data(&valid_data).is_ok());
    
    // Missing required field should fail validation
    let invalid_data = json!({
        "age": 30
    });
    
    assert!(template.validate_data(&invalid_data).is_err());
    
    // Wrong type should fail validation
    let invalid_type_data = json!({
        "name": "John Doe",
        "age": "thirty"
    });
    
    assert!(template.validate_data(&invalid_type_data).is_err());
}