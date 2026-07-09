# 📄 Papermake

**Content-addressable template registry with server-side rendering for [Typst](https://typst.app/) documents.**

Turn your Typst templates into APIs. Publish once, render anywhere.

```bash
# Publish a template
curl -X POST http://localhost:8080/templates/invoice/publish?tag=latest \
  -F "main_typ=@invoice.typ" \
  -F "schema=@schema.json" \
  -F "metadata={\"name\":\"Invoice\",\"author\":\"you@company.com\"}"

# Render with data → PDF
curl -X POST http://localhost:8080/render/invoice:latest \
  -H "Content-Type: application/json" \
  -d '{"company": "Acme Corp", "amount": 1500}' \
  --output invoice.pdf
```

## 🚀 Why Papermake?

- **🏗️ Template as Code** - Version your document templates like software
- **⚡ Server-side Rendering** - No local Typst installation needed
- **🔒 Content-Addressable** - Immutable, deduplicated storage (like Git for documents)
- **📊 Built-in Analytics** - Track usage, performance, and errors
- **🐳 Self-hostable** - Deploy anywhere with Docker

## 🏃‍♂️ Quick Start

### Using Docker Compose

```bash
git clone https://github.com/rkstgr/papermake
cd papermake
docker-compose up -d
```

This starts:
- **Papermake Server** on `localhost:8080`
- **MinIO** (S3-compatible storage) on `localhost:9000`, inspectable at `http://localhost:9001`
- **ClickHouse** (analytics) on `localhost:8123`

### Manual Setup

```bash
# Copy and update .env with your S3 and Clickhouse credentials
cp .env.example .env

# Run the server
cargo run -r -p papermake-server
```

## 🦀 Using the Library

You can also use `papermake` directly as a Rust library, without the server:

```toml
[dependencies]
papermake = "0.2"
```

```rust
use papermake::{render_template, InMemoryFileSystem};
use std::sync::Arc;

let template = "Hello #data.name!";
let fs = Arc::new(InMemoryFileSystem::new());
let data = serde_json::json!({ "name": "World" });

let result = render_template(template.to_string(), fs, &data)?;
if result.success {
    std::fs::write("hello.pdf", result.pdf.unwrap())?;
}
```

### Images & other files

Rendering happens against a **virtual file system** — there is no disk access and no
`--root` directory like the Typst CLI has. Every file your template references
(images, imports, data files) must be provided through the file system you pass in:

```rust
let mut fs = InMemoryFileSystem::new();
fs.add_file("logo.png", std::fs::read("assets/logo.png")?);
fs.add_file("header.typ", std::fs::read("templates/header.typ")?);
```

```typst
#import "header.typ": make_header
#image("logo.png", width: 40pt)
```

Paths are matched with or without a leading `/` (`logo.png` and `/logo.png` are
equivalent), so Typst's rooted paths resolve to the files you added. When publishing
to the registry, the same applies: upload assets as additional `files[]` and reference
them by their file name.

### Fonts

System fonts are discovered automatically. To bundle additional fonts (e.g. in a
container without system fonts), point the optional `FONTS_DIR` environment variable
at a directory of font files before the first render:

```bash
FONTS_DIR=./fonts cargo run
```

## 📚 Usage

### Publishing Templates

Templates consist of:
- `main.typ` - Your Typst template file
- files: Images, fonts, other files (optional)
- Metadata - Name, author, description

```typst
// invoice.typ
// #data is automatically populated with the input data
= Invoice #data.number

*Bill To:* #data.customer.name
*Amount:* $#data.amount
```

```bash
curl -X POST localhost:8080/templates/invoice/publish?tag=latest \
  -F "main_typ=@invoice.typ" \
  -F "files[]=@logo.png" \
  -F "metadata={\"name\":\"Professional Invoice\",\"author\":\"finance@company.com\"}"

# simple publish endpoint
curl -X POST http://localhost:3000/api/templates/invoice/publish-simple \
  -H 'Content-Type: application/json' \
  -d '{
  "main_typ": "#set text(font: \"Arial\")\nhello #data.name",
  "metadata": {
    "author": "dev@bigbank.com"
    "name": "Customer Invoice",
  }
}'
```

Returns
```json
{
  "data": {
    "message": "Template 'invoice:latest' published successfully",
    "manifest_hash": "sha256:8e0e58437230ce87a69a77edec3a24412a2f656bc42456f7f87c61d5de1ad5f9",
    "reference": "invoice:latest"
  },
  "message": "Template published with reference 'invoice:latest'"
}
```

### Rendering Documents

```bash
# Render to PDF
curl -X POST localhost:8080/render/invoice:latest \
  -H "Content-Type: application/json" \
  -d '{
    "number": "INV-001",
    "customer": {"name": "Acme Corp"},
    "amount": 1500
  }' \
  --output invoice.pdf
```

### Analytics & History

```bash
# Recent renders
curl localhost:8080/renders?limit=10

# Template usage stats
curl localhost:8080/analytics/templates

# Performance over time
curl localhost:8080/analytics/duration?days=30
```

## 🏗️ Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌────────────────┐
│   Templates     │───▶│   Papermake      │───▶│    Registry    │
│   (Multipart)   │    │   Server         │    │   (S3 + CH)    │
└─────────────────┘    └──────────────────┘    └────────────────┘
                                │
                                ▼
                       ┌──────────────────┐
                       │   Typst Engine   │
                       │   (Rendering)    │
                       └──────────────────┘
```

- **Content-Addressable Storage** - Templates stored by hash, deduplicated automatically
- **Immutable Versions** - `invoice:v1.0.0` never changes, `invoice:latest` is mutable
- **Render Tracking** - Every render logged with input/output hashes for full auditability

## 🛠️ API Reference

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/templates/{name}/publish?tag={tag}` | Upload template |
| `GET` | `/templates` | List all templates |
| `GET` | `/templates/{name}/tags` | List template versions |
| `POST` | `/render/{name}:{tag}` | Render template to PDF |
| `GET` | `/renders?limit=N` | Recent render history |
| `GET` | `/renders/{id}/pdf` | Download rendered PDF |
| `GET` | `/analytics/volume?days=N` | Render volume over time |


## 🎯 Use Cases

- **📑 Document Generation APIs** - Invoices, contracts, reports
- **📧 Email Templates** - Marketing campaigns, notifications
- **📋 Form Processing** - Applications, certificates, labels
- **📊 Report Automation** - Analytics dashboards, financial reports


## 🤝 Contributing

```bash
git clone https://github.com/rkstgr/papermake
cd papermake
cargo test
```

Built with Rust 🦀 • Powered by [Typst](https://typst.app/) • Inspired by Docker registry & Git's content addressing

---

**Documentation** (coming soon)
