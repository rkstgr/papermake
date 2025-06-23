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