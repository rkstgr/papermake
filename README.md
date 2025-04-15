# Papermake

Papermake is a fast & ergonomic PDF generation library, built in Rust. The core rendering engine is based on [Typst](https://github.com/typst/typst), which is a powerful typesetting system.

## Why Papermake?

Papermake was born from experiences in finance where existing PDF generation tools couldn't meet the demands of high-volume, low-latency document processing at scale. Legacy tools like Crystal Reports have legacy dependencies, are difficult to version control, and lacked proper testing capabilities. Papermake solves these challenges by providing a fast, cloud-ready PDF generator with a focus on ergonomics, testability, and debuggability - all while remaining platform-independent.

## Features

-   ⚡️ **High Performance**: Built with Rust for optimal performance
-   📝 **Typst Template Support**: Use Typst's powerful markup language for document templates
-   🔍 **Schema Validation**: Define and validate input data structure using JSON schemas
-   🚀 **HTTP API**: Ready-to-use server with RESTful endpoints
-   📦 **File Management**: Built-in template and asset management
-   🔒 **Type Safety**: Strong typing and validation throughout the pipeline

## Quick Start

Here's a simple example of creating and rendering a template:

```rust
use papermake::{Schema, SchemaField, FieldType, Template, render_pdf};
use serde_json::json;

// Define your document schema
let mut schema = Schema::new();
schema.add_field(SchemaField {
    key: "name".to_string(),
    label: Some("Name".to_string()),
    field_type: FieldType::String,
    required: true,
    description: Some("Customer name".to_string()),
    default: None,
});

// Create a template
let template = Template::new(
    "invoice",
    "Invoice Template",
    "#let data = json.decode(sys.inputs.data)\nHello #data.name!",
    schema,
);

// Render with data
let data = json!({
    "name": "John Doe"
});

let result = render_pdf(&template, &data, None).unwrap();
```

## Using the HTTP API

The HTTP server provides RESTful endpoints for template management and rendering. Run the server with:

```bash
cargo run -p papermake-server
```

Here's an example using curl:

```bash
# Create a new template
curl -X POST http://localhost:3000/templates \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Invoice",
    "content": "#let data = json.decode(sys.inputs.data)\n= Invoice\nBill to: #data.name",
    "schema": {
      "fields": [
        {
          "key": "name",
          "label": "Customer Name",
          "field_type": "string",
          "required": true
        }
      ]
    }
  }'

# Render the template
curl -X POST http://localhost:3000/templates/invoice/render \
  -H "Content-Type: application/json" \
  -d '{
    "name": "John Doe"
  }' \
  --output invoice.pdf
```

## Documentation

For more detailed documentation and examples, please visit our documentation (coming soon).

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
