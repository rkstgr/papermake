use papermake::{schema, Template};
use serde_json::json;

#[test]
fn test_builder_build_cached() {
    let schema = schema! {
        name: String,
        age?: Number
    };

    // Test building cached template directly from builder
    let cached_template = Template::builder("test")
        .name("Test Template")
        .content("#let data = json.decode(sys.inputs.data)\nHello #data.name!")
        .schema(schema)
        .build_cached()
        .unwrap();
    
    let data = json!({
        "name": "John Doe",
        "age": 30
    });

    // Should work just like regular cached template
    assert!(!cached_template.is_cached());
    let result = cached_template.render(&data).unwrap();
    assert!(cached_template.is_cached());
    assert!(result.pdf.is_some());
}

#[test]
fn test_multiple_renders_performance() {
    let schema = schema! {
        name: String,
        count: Number
    };

    let cached_template = Template::builder("performance_test")
        .name("Performance Test")
        .content("#let data = json.decode(sys.inputs.data)\nReport #data.count for #data.name")
        .schema(schema)
        .build_cached()
        .unwrap();
    
    // Simulate multiple renders
    for i in 1..=5 {
        let data = json!({
            "name": format!("User {}", i),
            "count": i
        });

        let result = cached_template.render(&data).unwrap();
        assert!(result.pdf.is_some());
        
        // After first render, should be cached
        if i > 1 {
            assert!(cached_template.is_cached());
        }
    }
}