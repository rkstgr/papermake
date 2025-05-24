use papermake::{schema, Template, TemplateCache};
use serde_json::json;

#[test]
fn test_cached_template_basic_usage() {
    let schema = schema! {
        name: String,
        age?: Number
    };

    let template = Template::builder("test")
        .name("Test Template")
        .content("#let data = json.decode(sys.inputs.data)\nHello #data.name!")
        .schema(schema)
        .build()
        .unwrap();

    let cached_template = template.with_cache();
    
    let data = json!({
        "name": "John Doe",
        "age": 30
    });

    // First render should create cache
    assert!(!cached_template.is_cached());
    let result1 = cached_template.render(&data).unwrap();
    assert!(cached_template.is_cached());
    assert!(result1.pdf.is_some());

    // Second render should use cache
    let result2 = cached_template.render(&data).unwrap();
    assert!(result2.pdf.is_some());
}

#[test]
fn test_cached_template_clear_cache() {
    let schema = schema! {
        name: String
    };

    let template = Template::builder("test")
        .name("Test Template")
        .content("#let data = json.decode(sys.inputs.data)\nHello #data.name!")
        .schema(schema)
        .build()
        .unwrap();

    let cached_template = template.with_cache();
    
    let data = json!({
        "name": "John Doe"
    });

    // Render to create cache
    let _result = cached_template.render(&data).unwrap();
    assert!(cached_template.is_cached());

    // Clear cache
    cached_template.clear_cache().unwrap();
    assert!(!cached_template.is_cached());
}

#[test]
fn test_cached_template_validation() {
    let schema = schema! {
        name: String,
        age: Number
    };

    let template = Template::builder("test")
        .name("Test Template")
        .content("#let data = json.decode(sys.inputs.data)\nHello #data.name!")
        .schema(schema)
        .build()
        .unwrap();

    let cached_template = template.with_cache();
    
    // Valid data
    let valid_data = json!({
        "name": "John Doe",
        "age": 30
    });
    assert!(cached_template.validate_data(&valid_data).is_ok());

    // Invalid data (missing required field)
    let invalid_data = json!({
        "name": "John Doe"
        // missing required age
    });
    assert!(cached_template.validate_data(&invalid_data).is_err());
}

#[test]
fn test_cached_template_clone() {
    let schema = schema! {
        name: String
    };

    let template = Template::builder("test")
        .name("Test Template")
        .content("#let data = json.decode(sys.inputs.data)\nHello #data.name!")
        .schema(schema)
        .build()
        .unwrap();

    let cached_template1 = template.with_cache();
    let cached_template2 = cached_template1.clone();
    
    let data = json!({
        "name": "John Doe"
    });

    // Render with first cached template
    let _result1 = cached_template1.render(&data).unwrap();
    assert!(cached_template1.is_cached());
    
    // Clone should have its own cache (not cached yet)
    assert!(!cached_template2.is_cached());
    
    // Render with cloned template
    let _result2 = cached_template2.render(&data).unwrap();
    assert!(cached_template2.is_cached());
}