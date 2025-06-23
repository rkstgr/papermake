# Papermake

Papermake is a fast PDF generation service built in Rust using [Typst](https://github.com/typst/typst) as the rendering engine. Generate PDFs via HTTP API or embed the core library directly in your applications.

**🚀 Fast**: From request to PDF in under 20ms. That's 100x faster than traditional PDF generation methods.
**📐 Type-safe**: Schema validation with JSON data binding
**🔧 Production-ready**: Template versioning, background workers, and S3 storage

⚠️ **Early Development**: APIs and features are subject to change.

## Quick Start - HTTP API

Get a PDF generated in under 2 minutes:

### 1. Start the Services

```bash
git clone https://github.com/rkstgr/papermake-rs
cd papermake-rs

# Start MinIO (S3-compatible storage)
docker-compose up -d

# Start the Papermake server
cargo run -p papermake-server
```

Server starts at `http://localhost:3000`

### 2. Create a Template

```bash
curl -X POST http://localhost:3000/api/templates \
  -H "Content-Type: application/json" \
  -d '{
    "id": "hello-world",
    "name": "Hello World Template",
    "content": "Hello #data.name!",
    "author": "you",
    "schema": {
      "type": "object",
      "properties": {
        "name": {"type": "string"}
      }
    }
  }'
```

### 3. Generate a PDF

```bash
curl -X POST http://localhost:3000/api/renders \
  -H "Content-Type: application/json" \
  -d '{
    "template_id": "hello-world",
    "template_version": 1,
    "data": {"name": "World"}
  }'
```

Returns a render job ID like:
```json
{
  "data": {
    "id": "b2d56b44-0431-438a-b88e-16841b87cbf8",
    "status": "queued",
    "created_at": "2025-06-23T20:53:53.291987Z",
    "estimated_completion": null
  }
}
```

### 4. Download the PDF

```bash
# Check status
curl http://localhost:3000/api/renders/b2d56b44-0431-438a-b88e-16841b87cbf8

# Download PDF when completed
curl http://localhost:3000/api/renders/b2d56b44-0431-438a-b88e-16841b87cbf8/pdf -o hello.pdf
```

## More Examples

### Batch Rendering

Generate multiple PDFs at once:

```bash
curl -X POST http://localhost:3000/api/renders/batch \
  -H "Content-Type: application/json" \
  -d '{
    "requests": [
      {
        "template_id": "hello-world",
        "template_version": 1,
        "data": {"name": "Alice"}
      },
      {
        "template_id": "hello-world",
        "template_version": 1,
        "data": {"name": "Bob"}
      }
    ]
  }'
```

### Template Management

```bash
# List all templates
curl http://localhost:3000/api/templates

# Get specific template
curl http://localhost:3000/api/templates/invoice

# List template versions
curl http://localhost:3000/api/templates/invoice/versions
```

## Using the Core Library

For embedding Papermake directly in your Rust applications:

```rust
use papermake::{TemplateBuilder, Schema};
use serde_json::json;

// Create a template with builder pattern
let template = TemplateBuilder::new("invoice".into())
    .name("Invoice Template")
    .content("= Invoice\n\nHello #data.name!\nTotal: $#data.amount")
    .schema(Schema::from_value(json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "amount": {"type": "number"}
        }
    })))
    .build()?;

// Render with data
let data = json!({
    "name": "John Doe",
    "amount": 1250.50
});

let result = template.render(&data)?;
if let Some(pdf_bytes) = result.pdf {
    std::fs::write("invoice.pdf", pdf_bytes)?;
}
```

### High-Performance Rendering

For high-volume scenarios, use template caching:

```rust
// Build with caching for better performance
let cached_template = TemplateBuilder::new("report".into())
    .content(include_str!("templates/report.typ"))
    .build_cached()?;

// Multiple renders reuse the compiled template
for customer in customers {
    let data = json!({"customer": customer});
    let pdf = cached_template.render(&data)?;
    // Template compilation is cached automatically
}
```

### Environment Configuration

Copy `.env.example` to `.env` and configure:

```bash
# Database
DATABASE_URL=sqlite:./data/papermake.db

# S3 Storage
S3_ACCESS_KEY_ID=minioadmin
S3_SECRET_ACCESS_KEY=minioadmin
S3_ENDPOINT_URL=http://localhost:9000
S3_BUCKET=papermake-dev

# Server
HOST=0.0.0.0
PORT=3000
```

## Project Structure

- **`papermake/`** - Core PDF generation library
- **`papermake-registry/`** - Template versioning and storage
- **`papermake-server/`** - HTTP API server with background workers

## Contributing

Contributions welcome! Please submit a Pull Request.
