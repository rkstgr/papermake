# Papermake Registry

A template registry and versioning system for papermake that provides enterprise-grade template management with multi-level access control, immutable versioning, and marketplace capabilities.

## Features

- **🔒 Access Control**: User, organization, public, and marketplace scopes
- **📦 Immutable Versioning**: Auto-incrementing versions (1, 2, 3...) that can't be modified once published
- **🔀 Fork Workflows**: Clean slate template forking with attribution
- **🏢 Enterprise Ready**: Organization-based sharing and compliance-friendly audit trails
- **🛒 Marketplace Support**: Built-in marketplace metadata and discovery
- **💾 Flexible Storage**: File system and PostgreSQL database backends

## Core Concepts

### Templates
Templates are versioned immutable assets once published. Each version contains:
- Template content (Typst markup)
- Schema definition for data validation
- Metadata (name, description, timestamps)
- Access scope and author information

### Scopes
Templates have four visibility levels:
- **User**: Private to the template owner
- **Organization**: Shared within an organization
- **Public**: Accessible to all users
- **Marketplace**: Available for purchase/download with enhanced metadata

### Versioning
- Auto-incrementing version numbers (1, 2, 3...)
- Immutable once published (compliance-friendly)
- Clean slate forking (independent evolution with attribution)

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
papermake-registry = "0.1"
papermake = "0.1"

# Optional features
papermake-registry = { version = "0.1", features = ["postgres"] }
```

### Basic Usage

```rust
use papermake_registry::*;
use papermake::TemplateBuilder;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a registry with file system storage
    let storage = FileSystemStorage::new("./templates").await?;
    let registry = DefaultRegistry::new(storage);

    // Create a user
    let user = User::new("alice", "alice@company.com");
    registry.save_user(&user).await?;

    // Publish a template
    let template = TemplateBuilder::new("invoice-template".into())
        .name("Invoice Template")
        .content("#let data = json(sys.inputs.data)\nInvoice for: #data.customer_name")
        .build()?;
        
    let version = registry.publish_template(
        template, 
        &user.id, 
        TemplateScope::User(user.id.clone())
    ).await?;

    // Render with access control
    let data = serde_json::json!({"customer_name": "ACME Corp"});
    let result = registry.render(
        &"invoice-template".into(), 
        Some(version), 
        &data, 
        &user.id
    ).await?;

    println!("Generated PDF: {} bytes", result.pdf.as_ref().unwrap().len());
    Ok(())
}
```

### Template Versioning

```rust
// Publish version 1
let template_v1 = TemplateBuilder::new("my-template".into())
    .name("My Template v1")
    .content("Version 1 content")
    .build()?;

let v1 = registry.publish_template(
    template_v1, 
    &user.id, 
    TemplateScope::User(user.id.clone())
).await?; // Returns version 1

// Publish version 2 (auto-increments)
let template_v2 = TemplateBuilder::new("my-template".into())
    .name("My Template v2")
    .content("Version 2 content")
    .build()?;

let v2 = registry.publish_template(
    template_v2, 
    &user.id, 
    TemplateScope::User(user.id.clone())
).await?; // Returns version 2

// Get specific version
let v1_template = registry.get_template(&"my-template".into(), Some(1)).await?;
let latest = registry.get_template(&"my-template".into(), None).await?; // Gets v2
```

### Forking Templates

```rust
// Fork a public template
let forked_version = registry.fork_template(
    &"original-template".into(),
    1, // source version
    &"my-fork".into(), // new template id
    &user.id,
    TemplateScope::User(user.id.clone())
).await?;

// Forked template starts at version 1 and evolves independently
let forked = registry.get_template(&"my-fork".into(), Some(1)).await?;
assert_eq!(forked.forked_from, Some(("original-template".into(), 1)));
```

### Access Control

```rust
// Create organization
let org = Organization::new("ACME Corp");
registry.save_organization(&org).await?;

// Add user to organization
let mut user = registry.get_user(&user.id).await?;
user.add_to_organization(org.id.clone());
registry.save_user(&user).await?;

// Publish template for organization
let org_template = TemplateBuilder::new("org-template".into())
    .name("Organization Template")
    .content("Shared template content")
    .build()?;

registry.publish_template(
    org_template,
    &user.id,
    TemplateScope::Organization(org.id)
).await?;

// List templates by scope
let user_templates = registry.list_user_templates(&user.id).await?;
let org_templates = registry.list_org_templates(&org.id, &user.id).await?;
let public_templates = registry.list_public_templates().await?;
```

### Marketplace Integration

```rust
// Create marketplace metadata
let marketplace_metadata = MarketplaceMetadata::new(
    "Professional Invoice Template",
    "Beautiful, professional invoice template with custom styling",
    "Business",
    "Template Studio"
)
.with_tags(vec!["invoice".to_string(), "business".to_string(), "professional".to_string()])
.with_price(999) // $9.99 in cents
.with_previews(vec!["https://example.com/preview1.png".to_string()]);

// Submit to marketplace
registry.submit_to_marketplace(
    &"invoice-template".into(),
    1,
    marketplace_metadata,
    &user.id
).await?;

// List marketplace templates
let marketplace_templates = registry.list_marketplace_templates().await?;
```

## Storage Backends

### File System Storage

```rust
let storage = FileSystemStorage::new("./templates").await?;
```

Directory structure:
```
./templates/
├── templates/
│   └── template_id/
│       ├── versions/
│       │   ├── 1.json
│       │   ├── 2.json
│       │   └── ...
│       └── assets/
│           ├── fonts/
│           └── images/
├── users/
│   └── user_id.json
└── organizations/
    └── org_id.json
```

### PostgreSQL Storage

Enable the `postgres` feature:

```toml
[dependencies]
papermake-registry = { version = "0.1", features = ["postgres"] }
```

```rust
use papermake_registry::storage::postgres::PostgresStorage;

// Create from database URL
let storage = PostgresStorage::from_url("postgresql://localhost/papermake").await?;

// Run migrations to create tables
storage.migrate().await?;

// Use with registry
let registry = DefaultRegistry::new(storage);
```

**Database Setup:**

```sql
-- Create database
CREATE DATABASE papermake_registry;

-- The storage will automatically create these tables:
-- - users (with organization arrays)
-- - organizations 
-- - templates (with JSONB for template data and marketplace metadata)
-- - template_assets (for fonts, images, etc.)
```

**Features:**
- ACID compliance with PostgreSQL transactions
- Efficient JSON storage for template data and marketplace metadata
- Full-text search across template names, descriptions, and marketplace data
- Native array support for user organization membership
- Proper indexes for high-performance queries
- Connection pooling with sqlx

## API Reference

### Registry Operations

```rust
// Template lifecycle
async fn publish_template(&self, template: Template, author: &UserId, scope: TemplateScope) -> Result<u64>
async fn get_template(&self, id: &TemplateId, version: Option<u64>) -> Result<VersionedTemplate>
async fn list_versions(&self, id: &TemplateId) -> Result<Vec<u64>>
async fn fork_template(&self, source_id: &TemplateId, source_version: u64, new_id: &TemplateId, user: &UserId, new_scope: TemplateScope) -> Result<u64>

// Access control
async fn can_access(&self, id: &TemplateId, version: u64, user: &UserId) -> Result<bool>
async fn share_with_org(&self, id: &TemplateId, org: &OrgId, user: &UserId) -> Result<()>
async fn make_public(&self, id: &TemplateId, user: &UserId) -> Result<()>

// Discovery
async fn list_user_templates(&self, user: &UserId) -> Result<Vec<(TemplateId, u64)>>
async fn list_org_templates(&self, org: &OrgId, user: &UserId) -> Result<Vec<(TemplateId, u64)>>
async fn list_public_templates(&self) -> Result<Vec<(TemplateId, u64)>>
async fn search_templates(&self, query: &str, user: &UserId) -> Result<Vec<(TemplateId, u64)>>

// Rendering
async fn render(&self, id: &TemplateId, version: Option<u64>, data: &serde_json::Value, user: &UserId) -> Result<RenderResult>
```

### Storage Operations

```rust
// Template management
async fn save_versioned_template(&self, template: &VersionedTemplate) -> Result<()>
async fn get_versioned_template(&self, id: &TemplateId, version: u64) -> Result<VersionedTemplate>
async fn list_template_versions(&self, id: &TemplateId) -> Result<Vec<u64>>

// Asset management
async fn save_template_asset(&self, template_id: &TemplateId, path: &str, content: &[u8]) -> Result<()>
async fn get_template_asset(&self, template_id: &TemplateId, path: &str) -> Result<Vec<u8>>
async fn list_template_assets(&self, template_id: &TemplateId) -> Result<Vec<String>>

// User/organization management
async fn save_user(&self, user: &User) -> Result<()>
async fn get_user(&self, id: &UserId) -> Result<User>
async fn save_organization(&self, org: &Organization) -> Result<()>
async fn get_organization(&self, id: &OrgId) -> Result<Organization>
```

## Features

- `fs` (default): File system storage backend
- `postgres`: PostgreSQL storage backend with full ACID compliance

## Contributing

See the main [papermake repository](https://github.com/rkstgr/papermake) for contribution guidelines.

## License

Licensed under the Apache License, Version 2.0.