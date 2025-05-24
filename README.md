# Papermake

Papermake is a fast PDF generation library built in Rust that uses [Typst](https://github.com/typst/typst) as its rendering engine. It's designed for high-volume, low-latency document processing with a focus on ergonomics and testability.

## Why Papermake?

Papermake was born from experiences in finance where existing PDF generation tools couldn't meet the demands of high-volume, low-latency document processing at scale. Legacy tools like Crystal Reports have legacy dependencies, are difficult to version control, and lacked proper testing capabilities. Papermake solves these challenges by providing a fast, cloud-ready PDF generator with a focus on ergonomics, testability, and debuggability - all while remaining platform-independent.

⚠️ **Please Note:** This project is in its early stages of development. Features, APIs, and documentation are subject to change.

## Key Features

-   **🚀 High Performance**: Built with Rust and Typst for fast, low-latency PDF generation. Automatic template compilation caching for high-volume document processing
-   **🔧 Ergonomic API**: Builder patterns and fluent interfaces for ease of use, with seamless data binding from JSON
-   **📐 Schema Validation**: Type-safe data binding with compile-time schema macros

## Planned Features

-   Template versioning and management system
-   HTTP API server for microservice deployments
-   Enhanced debugging and error reporting
-   Additional output formats beyond PDF

## Quick Start

Here's a simple example of creating and rendering a template:

```rust
use papermake::{schema, Template, TemplateCache};
use serde_json::json;

// Define your document schema with macro
let schema = schema! {
    name: String,
    age?: Number
};

// Create a template with builder pattern
let template = Template::builder("invoice")
    .name("Invoice Template")
    .content("#let data = json.decode(sys.inputs.data)\nHello #data.name!")
    .schema(schema)
    .build()
    .unwrap();

// For better performance, use caching
let cached_template = template.with_cache();

// Render with data - first render compiles, subsequent renders use cache
let data = json!({
    "name": "John Doe"
});

let result = cached_template.render(&data).unwrap();
```

## Performance with Caching

For high-performance scenarios where you're rendering the same template multiple times with different data, use the caching API:

```rust
use papermake::Template;

// Build with caching directly for convenience
let cached_template = Template::builder("report")
    .name("Monthly Report")
    .content(include_str!("templates/report.typ"))
    .schema(schema)
    .build_cached()?;

// Multiple renders reuse the compiled template
for customer in customers {
    let data = json!({"customer": customer});
    let pdf = cached_template.render(&data)?;
    // Cache is automatically managed
}

// Clear cache if needed
cached_template.clear_cache()?;
```

## Documentation

For more detailed documentation and examples, please visit our documentation (coming soon).

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
