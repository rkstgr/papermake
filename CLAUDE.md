# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Papermake is a high-performance PDF generation library built in Rust using Typst as the rendering engine. The project consists of a workspace with multiple crates designed for different use cases:

- **papermake** (core): PDF generation library with template caching and schema validation
- **papermake-registry**: Enterprise template versioning and management system  
- **papermake-server**: HTTP API server (planned)
- **papermake-worker**: Background worker service (planned)

## Architecture

### Core Library (`crates/papermake`)
The main abstraction layers are:
- **Template**: Immutable template definitions with schema validation
- **TemplateBuilder**: Ergonomic builder pattern for template creation
- **CachedTemplate**: Performance-optimized cached compilation for high-volume rendering
- **TemplateCache**: Thread-safe caching layer using `once_cell` for template compilation
- **Schema**: Type-safe data validation with compile-time macros
- **Typst integration**: Font caching and world management for Typst rendering

Key patterns:
- Templates are immutable once created
- Builder pattern with fluent API design
- Caching is automatic but can be manually managed for performance
- Schema validation happens before rendering

### Registry System (`crates/papermake-registry`)
Enterprise-grade template management with:
- **Entities**: User, Organization, VersionedTemplate with multi-level access control
- **Storage abstraction**: File system and PostgreSQL backends with feature flags
- **Registry service**: High-level API for template lifecycle management
- **Versioning**: Auto-incrementing immutable versions (1, 2, 3...)
- **Scopes**: User → Organization → Public → Marketplace visibility levels
- **Forking**: Clean slate template copies with attribution

## Common Development Commands

### Building and Testing
```bash
# Build entire workspace
cargo build

# Build with all features
cargo build --all-features

# Build specific crate
cargo build -p papermake
cargo build -p papermake-registry

# Run all tests
cargo test

# Run tests for specific crate
cargo test -p papermake
cargo test -p papermake-registry

# Run integration tests
cargo test --test integration_tests

# Test with PostgreSQL feature
cargo test -p papermake-registry --features postgres
```

### Feature Flags
- **papermake**: `fs` (default) - file system operations
- **papermake-registry**: `fs` (default), `postgres` - storage backends

### Publishing
Both crates are published to crates.io. When publishing:
```bash
# Check package before publishing
cargo package -p papermake
cargo publish -p papermake --dry-run

# Publish (from papermake-rs root)
cargo publish -p papermake
cargo publish -p papermake-registry
```

## Development Patterns

### Template Creation
Use the builder pattern consistently:
```rust
let template = TemplateBuilder::new("template-id".into())
    .name("Display Name")
    .content(typst_content)
    .schema(schema)
    .build()?;
```

### Performance-Critical Code
For high-volume scenarios, always use cached templates:
```rust
let cached = template.with_cache();
// or
let cached = TemplateBuilder::new(id).build_cached()?;
```

### Error Handling
- Use `thiserror` for custom error types
- All public APIs return `Result<T, Error>`
- Storage errors are wrapped in domain-specific error types

### Testing Strategy
- Unit tests in `src/` modules test individual components
- Integration tests in `tests/` directory test full workflows
- Use `tempfile` for file system tests
- PostgreSQL tests require `TEST_DATABASE_URL` environment variable

### Storage Backends
Registry supports multiple storage backends via feature flags:
- File system storage: organized directory structure with JSON files
- PostgreSQL storage: ACID-compliant with JSONB for flexible data
- Storage trait allows adding new backends

## Key Dependencies

- **typst**: Core rendering engine (v0.13)
- **serde**: Serialization throughout the codebase
- **sqlx**: PostgreSQL integration (optional)
- **tokio**: Async runtime for file operations (optional)
- **time**: Timestamp handling with RFC3339 serialization
- **uuid**: ID generation for registry entities

## Lambda Integration

There's a separate AWS Lambda renderer at `/Users/erik/Dev/rkstgr/papermake/papermake-aws/lambda_functions/renderer/` that uses papermake. It demonstrates:
- SQS event processing
- S3 template loading and PDF output
- Template caching optimization for serverless

## Registry Database Schema

When using PostgreSQL storage:
- `users` table with organization arrays
- `templates` table with JSONB for template data and marketplace metadata
- `template_assets` table for binary assets (fonts, images)
- Proper indexes on scope, author, and published date fields

## Common Issues

- Font loading: Ensure fonts are properly embedded or use system fonts
- Template compilation: First render is slower due to compilation
- PostgreSQL tests: Require a test database connection
- Workspace dependencies: Use path dependencies for local development

## Git Commit Guidelines

- **NEVER** include Claude or AI assistant attributions in commit messages
- Keep commit messages focused on the technical changes made
- Use conventional commit format: `type: description`
- Prefer concise, less verbose commit messages that explain the core change

## Current Development Status

### Draft-Based Template Editing (In Progress)

**Completed Tasks:**
- ✅ **Database Schema Updates**: Updated SQLite storage to support name:version identification with new columns:
  - `template_name` (e.g., "invoice-template") 
  - `display_name` (e.g., "Monthly Invoice Template")
  - `version` (e.g., "v1", "v2", "draft")
  - `is_draft` boolean flag
  - Primary key changed to `(template_name, version)`

- ✅ **Entity Model Restructuring**: Completely updated `VersionedTemplate` and `RenderJob` entities to use String versions instead of u64, added draft support fields

- ✅ **Storage Layer Extensions**: Added comprehensive draft management methods to `MetadataStorage` trait:
  - `save_draft()`, `get_draft()`, `delete_draft()`, `has_draft()`
  - `get_next_version_number()` for auto-incrementing
  - `get_versioned_template_by_name()` for name:version lookups

- ✅ **Registry Interface Updates**: Extended `TemplateRegistry` trait with draft workflow methods and implemented in `DefaultRegistry`

- ✅ **Server API Endpoints**: Added complete draft management endpoints:
  - `GET /api/templates/{template_name}/draft` - Retrieve draft
  - `PUT /api/templates/{template_name}/draft` - Save/update draft
  - `DELETE /api/templates/{template_name}/draft` - Delete draft
  - `POST /api/templates/{template_name}/draft/publish` - Publish draft as new version

- ✅ **Compilation Fixes**: Resolved all type conversion issues when changing from u64 to String versions, added `parse_version_to_u64()` helper functions for backward compatibility

**Pending Tasks:**
- 🔄 **Update existing server API routes** to support name:version URL structure (e.g., `/api/templates/invoice-template:v1` instead of `/api/templates/{uuid}`)
- 🔄 **Update frontend API client** to support draft methods and new URL patterns
- 🔄 **Overhaul template editor** to implement draft-first workflow where editing creates drafts automatically
- 🔄 **Update frontend URL routing** from UUID-based to name-based
- 🔄 **Create data migration scripts** for existing templates to convert UUID-based to name:version format

**Technical Implementation Details:**
- **Version Format**: Following Docker-style versioning (e.g., "v1", "v2", "latest", "draft")
- **Database Schema**: Uses `(template_name, version)` as primary key with backward compatibility via `template_id` UUID
- **Draft Workflow**: Users can edit templates (creating drafts), navigate away, return to continue editing, discard changes, or publish when ready
- **Auto-versioning**: Published templates auto-increment version numbers (v1 → v2 → v3)
- **Backward Compatibility**: Legacy UUID-based APIs still work via conversion helpers

**Key Files Modified:**
- `crates/papermake-registry/src/storage/sqlite_storage.rs` - Database schema and draft methods
- `crates/papermake-registry/src/entities.rs` - Entity model restructuring
- `crates/papermake-registry/src/storage/mod.rs` - Storage trait extensions
- `crates/papermake-registry/src/registry.rs` - Registry interface implementation
- `crates/papermake-server/src/routes/templates.rs` - Draft API endpoints
- `crates/papermake-server/src/models/` - API model updates for String versions

**Current State**: Backend foundation complete, server compiles successfully with draft endpoints functional. Ready for frontend integration or additional backend route updates.

# User Thoughts
I am thinking about the function of Template in the core library and the VersionedTemplate inside registry.
- Template in crates/papermake/src/template probably doesn't need template_id and name, because this is handled by the registry