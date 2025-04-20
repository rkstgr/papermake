# Papermake

Papermake is a fast & ergonomic PDF generation library, built in Rust. The core rendering engine is based on [Typst](https://github.com/typst/typst), which is a powerful typesetting system.

## Why Papermake?

Papermake was born from experiences in finance where existing PDF generation tools couldn't meet the demands of high-volume, low-latency document processing at scale. Legacy tools like Crystal Reports have legacy dependencies, are difficult to version control, and lacked proper testing capabilities. Papermake solves these challenges by providing a fast, cloud-ready PDF generator with a focus on ergonomics, testability, and debuggability - all while remaining platform-independent.

⚠️ **Please Note:** This project is in its early stages of development. Features, APIs, and documentation are subject to change.

## Roadmap & Key Features

Papermake aims to provide the following core capabilities:

-   🚀 **High Performance & Scalability**: Built with Rust and leveraging Typst for high-throughput, low-latency PDF generation suitable for demanding applications.
-   🔗 **Seamless Data Integration**: Easily bind JSON data to your Typst templates.
-   ✅ **Schema-Driven Data Validation**: Define and enforce input data structures using JSON schemas, ensuring data integrity before rendering.
-   🏛️ **Robust Template Management**: Features designed to support template versioning, auditing, and compliance needs, crucial for managed documents like certificates or reports.
-   💡 **Enhanced Debuggability**: Focused on providing clear feedback and tools to simplify the process of developing and troubleshooting templates.

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
