# Papermake - Typst Template Registry

A content-addressable registry for Typst templates with server-side rendering capabilities.

## Project Overview

Papermake consists of three main crates:
- **`papermake`**: Core Typst compilation engine with virtual filesystem
- **`papermake-registry`**: Content-addressable template storage and publishing
- **`papermake-server`**: HTTP API and web interface

## Architecture

```
User → Web UI → Server API → Registry Library → Typst Engine
                     ↓              ↓
                Cache Layer    Blob Storage (S3)
```

### Key Design Principles
- **Content-addressable storage** using SHA-256 hashes
- **Merkle tree approach** for efficient deduplication
- **Mutable tags** pointing to immutable content
- **Server-side rendering** with `template.render(data) → PDF`
- **Library-first architecture** with clean separation

## Current Implementation Status

### ✅ Phase 1: Core Typst Engine (`papermake` crate)
- [x] `TypstWorld` with virtual filesystem support
- [x] `TypstFileSystem` trait for async file resolution
- [x] Font caching via `CACHED_FONTS` static
- [x] Data injection through `sys.inputs.data`
- [x] Basic template rendering functionality

### ✅ Phase 2: Registry Foundation (`papermake-registry` crate)
- [x] `BlobStorage` trait with async operations
- [x] `S3Storage` implementation for AWS S3 compatibility
- [x] `ContentAddress` utilities for SHA-256 hashing and key generation
- [x] `TemplateBundle` and `TemplateMetadata` structs with validation
- [x] `Manifest` serialization/deserialization
- [x] Integration tests for storage layer

### 🚧 Phase 3: Registry File System Integration (In Progress)
- [x] `RegistryFileSystem<S: BlobStorage>` implementing `TypstFileSystem`
- [x] `TemplateReference` parsing (`namespace:tag@hash` format)
- [x] `Registry::publish()` method (store files → create manifest → update refs)
- [x] `Registry::resolve()` method (tag → manifest hash lookup)
- [ ] `Registry::render()` method using `RegistryFileSystem` ← **NEXT**

### 📋 Phase 4: Caching Layer (Planned)
- [ ] `Cache` struct with LRU for blobs, manifests, and refs
- [ ] Cache integration in `Registry` methods
- [ ] Cache invalidation API for webhooks
- [ ] Performance testing

### 📋 Phase 5: Server Layer (Planned)
- [ ] HTTP server with `/render/{reference}` endpoint
- [ ] `/publish` endpoint for template uploads
- [ ] Authentication and authorization
- [ ] Version tag immutability enforcement

## Code Structure

```
papermake/
├── papermake/                 # Core Typst engine
│   ├── src/
│   │   ├── lib.rs
│   │   ├── typst_world.rs     # TypstWorld implementation
│   │   └── filesystem.rs      # TypstFileSystem trait
│   └── Cargo.toml
├── papermake-registry/        # Registry core
│   ├── src/
│   │   ├── lib.rs
│   │   ├── storage/           # BlobStorage implementations
│   │   ├── bundle.rs          # TemplateBundle & TemplateMetadata
│   │   ├── manifest.rs        # Manifest format ← IMPLEMENT NEXT
│   │   ├── address.rs         # ContentAddress utilities
│   │   ├── registry.rs        # Registry core logic
│   │   └── reference.rs       # TemplateReference parsing
│   └── Cargo.toml
└── papermake-server/          # HTTP API (future)
    ├── src/lib.rs
    └── Cargo.toml
```

## Reference Format

Templates are referenced using: `[org/user]/name:tag@sha256:hash`

**Examples:**
- `invoice:latest` - Official template
- `john/invoice:latest` - User template
- `acme-corp/letterhead:stable` - Organization template
- `john/invoice:latest@sha256:abc123` - Tag with hash verification

## Storage Layout

```
storage/
├── blobs/sha256/{hash}           # Individual files
├── manifests/sha256/{hash}       # Template manifests
└── refs/
    ├── invoice/latest            # Official: name/tag
    ├── john/invoice/latest       # User: user/name/tag
    └── acme-corp/invoice/stable  # Org: org/name/tag
```

## Manifest Format

```json
{
  "entrypoint": "main.typ",
  "files": {
    "main.typ": "sha256:abc123...",
    "schema.json": "sha256:def456...",
    "components/header.typ": "sha256:ghi789...",
    "assets/logo.png": "sha256:jkl012..."
  },
  "metadata": {
    "name": "invoice-template",
    "author": "john@example.com"
  }
}
```

## Implementation Tasks

### Immediate Next Steps

1. **Implement `Manifest` struct** (`papermake-registry/src/manifest.rs`)
   ```rust
   #[derive(Serialize, Deserialize)]
   pub struct Manifest {
       pub entrypoint: String,
       pub files: HashMap<String, String>, // filename -> hash
       pub metadata: TemplateMetadata,
   }

   pub struct TemplateMetadata {
       pub name: String,
       pub author: String,
   }
   ```

2. **Add manifest serialization tests**
   - JSON roundtrip testing
   - Validate required fields
   - Error handling for malformed manifests

3. **Implement `TemplateReference` parsing** (`papermake-registry/src/reference.rs`)
   ```rust
   pub struct TemplateReference {
       pub namespace: String,
       pub tag: Option<String>,
       pub hash: Option<String>,
   }

   impl std::str::FromStr for TemplateReference { /* ... */ }
   ```

### Current Development Focus

**Working on:** Registry file system integration to resolve Typst imports directly through blob storage

**Key insight:** Instead of materializing full template bundles, resolve individual file imports on-demand through the `TypstFileSystem` trait, enabling:
- Memory efficiency (lazy loading)
- Natural caching at the blob level
- Consistent content-addressable access

## Testing Strategy

### Unit Tests
- Content addressing consistency (same content = same hash)
- Template bundle validation
- Reference parsing edge cases
- Storage backend operations

### Integration Tests
- End-to-end publish workflow
- Template resolution and rendering
- Cache behavior and invalidation
- S3 storage with mocked backend

### Performance Tests
- Template rendering latency
- Cache hit/miss ratios
- Concurrent access patterns
- Large template handling

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Invalid reference format: {0}")]
    InvalidReference(String),

    #[error("Compilation error: {0}")]
    CompileError(#[from] papermake::CompileError),

    #[error("Hash verification failed")]
    HashMismatch,
}
```

## Usage Examples

### Publishing a Template
```rust
let metadata = TemplateMetadata::new(
    "Invoice Template",
    "john@example.com"
);

let bundle = TemplateBundle::new(main_typ_content, metadata)
    .with_schema(schema_json)
    .add_file("assets/logo.png", logo_data);

let manifest_hash = registry.publish(bundle, "john/invoice", "latest").await?;
```

### Rendering a Template
```rust
let pdf_bytes = registry.render(
    "john/invoice:latest",
    json!({
        "from": "Acme Corp",
        "to": "Client Name",
        "items": [{"description": "Service", "amount": "$100"}],
        "total": "$100"
    })
).await?;
```

## Development Workflow

1. **Run tests**: `cargo test --workspace`
2. **Check formatting**: `cargo fmt --all`
3. **Run clippy**: `cargo clippy --workspace --all-targets`
4. **Build all crates**: `cargo build --workspace`
5. **Run integration tests**: `cargo test --workspace --test integration`

## Contributing Guidelines

1. **Code Style**: Use `cargo fmt` and `cargo clippy`
2. **Testing**: All new features must include comprehensive tests
3. **Documentation**: Public APIs must be documented
4. **Error Handling**: Use `thiserror` for custom error types
5. **Async**: Use `async-trait` for async trait methods

---

*This document is probably not up-to-date. If there are differences between the code and the document, please refer to the source code for the most accurate information.*


# Papermake Server Implementation Todo List

## Tech Stack & Decisions
- **Framework**: Axum/Tokio for async HTTP server
- **Storage**: S3 for blobs (templates, data, PDFs) + ClickHouse for render analytics
- **Auth**: None initially, optional API keys later
- **Namespaces**: Simplified `name:tag` format (no user/org prefixes)
- **Config**: Environment variables only
- **Deployment**: Docker container

## Phase 1: Basic HTTP Server
- [x] Set up Axum server with basic routing
- [x] Add environment variable configuration (S3 + ClickHouse credentials)
- [x] Implement health check endpoint `GET /health`
- [x] Add startup validation (required S3/ClickHouse connectivity)
- [x] Basic error handling and JSON response structure

## Phase 2: Template Management
- [x] Implement `POST /templates/{name}/publish?tag=latest` with multipart form
- [x] Implement `POST /templates/{name}/publish-simple` with json body
- [x] Implement `GET /templates` - list all templates with metadata
- [x] Implement `GET /templates/{name}/tags` - list tags for template
- [x] Implement `GET /templates/{reference}` - get template metadata

## Phase 3: Render Functionality
- [x] Set up ClickHouse connection and table creation
```sql
CREATE TABLE renders (
    render_id UUID,
    timestamp DateTime64(3),
    template_ref String,
    manifest_hash String,
    data_hash String,
    pdf_hash String,
    success Bool,
    duration_ms UInt32,
    pdf_size_bytes UInt32,
    error String,
    template_name String,
    template_tag String
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (timestamp, template_name);
```
- [x] Implement `POST /render/{reference}` endpoint

## Phase 4: Render History & Analytics
- [ ] Implement `GET /renders?limit=10` - latest N renders from ClickHouse
- [ ] Implement `GET /renders/{render_id}/data` - fetch original input data from S3
- [x] Implement `GET /renders/{render_id}/pdf` - fetch rendered PDF from S3
- [ ] Add basic analytics endpoints:
  - [ ] `GET /analytics/volume?days=30` - render volume over time
  - [ ] `GET /analytics/templates` - total renders per template
  - [ ] `GET /analytics/performance?days=30` - average duration over time

## Phase 5: Docker & Deployment
- [ ] Create Dockerfile with multi-stage build
- [x] Add docker-compose.yml with ClickHouse and MinIO for local dev
- [x] Document required environment variables:
- [ ] Add startup scripts and health checks

## Phase 6: Error Handling & Observability
- [x] Comprehensive error responses (404, 400, 500 with JSON structure)
- [ ] Request/response logging
- [ ] Metrics collection (render counts, durations, errors)
- [ ] Version tag immutability enforcement (409 Conflict for duplicate versions)

## Phase 7: Testing & Documentation
- [x] Integration tests with test ClickHouse and S3 (partially in papermake-registry)
- [ ] API documentation (OpenAPI/Swagger)
- [ ] Example templates and curl commands
- [ ] Performance testing under load

**Dependencies:**
- `papermake-registry` crate (completed phases 1-4)
- ClickHouse instance
- S3-compatible storage (MinIO for dev)

## HTTP API Endpoints

### Server Configuration
- **Framework**: Axum with Tokio
- **Base URL**: All API routes under `/api`
- **CORS**: Permissive CORS enabled
- **Body Limit**: 50MB for large PDF uploads

### Route Structure
```
/health (GET)
/api/
├── templates/
│   ├── / (GET) - List all templates
│   ├── /{name}/publish (POST) - Publish template (multipart)
│   ├── /{name}/publish-simple (POST) - Publish template (JSON)
│   ├── /{name}/tags (GET) - List template tags
│   └── /{reference} (GET) - Get template metadata
├── render/
│   └── /{reference} (POST) - Render template to PDF
├── renders/
│   └── /{render_id}/pdf (GET) - Download rendered PDF
└── analytics/
    (empty - no endpoints implemented)
```

### 1. Health Check
- **`GET /health`**
  - Returns server status, version, and timestamp
  - Response: JSON with health information

### 2. Template Management (`/api/templates`)

#### 2.1 List Templates
- **`GET /api/templates`**
  - Query parameters: `limit` (default: 50), `offset` (default: 0), `search`
  - Returns paginated list of templates
  - Response: `PaginatedResponse<TemplateInfo>`

#### 2.2 Publish Template (Multipart)
- **`POST /api/templates/{name}/publish?tag=latest`**
  - Content-Type: `multipart/form-data`
  - Form fields:
    - `main_typ`: Main template file (required)
    - `metadata`: JSON metadata with name and author (required)
    - `schema`: Optional JSON schema file
    - `files[]`: Additional template files (optional, multiple)
  - Response: `ApiResponse<PublishResponse>`

#### 2.3 Publish Template (JSON)
- **`POST /api/templates/{name}/publish-simple?tag=latest`**
  - Content-Type: `application/json`
  - JSON body:
    - `main_typ`: Template content as string
    - `schema`: Optional JSON schema object
    - `metadata`: Template metadata object
  - Response: `ApiResponse<PublishResponse>`

#### 2.4 List Template Tags
- **`GET /api/templates/{name}/tags`**
  - Returns all available tags for a template
  - Response: `ApiResponse<Vec<String>>`

#### 2.5 Get Template Metadata
- **`GET /api/templates/{reference}`**
  - Reference formats: `name`, `name:tag`, `namespace/name`, `namespace/name:tag`
  - Returns template metadata
  - Response: `ApiResponse<TemplateMetadataResponse>`

### 3. Template Rendering (`/api/render`)

#### 3.1 Render Template
- **`POST /api/render/{reference}`**
  - Content-Type: `application/json`
  - JSON body: `{"data": {...}}` - Data to inject into template
  - Returns PDF metadata with render_id
  - Response: `ApiResponse<RenderResponse>` with render_id, pdf_hash, duration_ms

### 4. Render History (`/api/renders`)

#### 4.1 Download Rendered PDF
- **`GET /api/renders/{render_id}/pdf`**
  - Downloads PDF file for specific render
  - Response: PDF file with `application/pdf` content-type

### 5. Analytics (`/api/analytics`)
- **Status**: Placeholder router - no endpoints implemented yet
- **Planned**: Volume metrics, template analytics, performance metrics

### Error Handling
- **400 Bad Request**: Invalid input or malformed requests
- **404 Not Found**: Template or resource not found
- **409 Conflict**: Version tag conflicts (planned)
- **500 Internal Server Error**: Server or storage errors
- All errors return JSON with error details

### Content-Addressable Features
- Templates stored with SHA-256 hashes
- Immutable content with mutable tag references
- Support for namespaced template references
- Server-side rendering with Typst engine
- PDF storage and retrieval by render ID
