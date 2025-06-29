//! # Papermake Registry
//!
//! A template registry and versioning system for papermake that provides:
//! - Template versioning with immutable versions
//! - Multi-level access control (user, organization, public, marketplace)
//! - Fork workflows with clean slate approach
//! - Auto-incrementing version numbers
//! - Enterprise-grade template management
//!
//! ## Core Concepts
//!
//! - **Templates** are versioned immutable assets once published
//! - **Scopes** define visibility: User → Organization → Public → Marketplace
//! - **Forks** create independent copies with attribution to source
//! - **Versions** auto-increment (1, 2, 3...) for simplicity
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use papermake_registry::*;
//! use papermake_registry::bundle::{TemplateBundle, TemplateMetadata};
//! use papermake_registry::storage::blob_storage::MemoryStorage;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a registry with memory storage
//! let storage = MemoryStorage::new();
//! let registry = Registry::new(storage);
//!
//! // Create a template bundle
//! let metadata = TemplateMetadata::new("Invoice Template", "alice@company.com");
//! let main_content = b"#let data = json.decode(sys.inputs.data)\n= Invoice\nFor: #data.customer_name".to_vec();
//! let bundle = TemplateBundle::new(main_content, metadata);
//!
//! // Publish the template
//! let manifest_hash = registry.publish(
//!     bundle,
//!     "alice/invoice-template",
//!     "latest"
//! ).await?;
//!
//! println!("Published template with manifest hash: {}", manifest_hash);
//! # Ok(())
//! # }
//! ```

pub mod address;
pub mod bundle;
pub mod error;
pub mod manifest;
pub mod reference;
pub mod registry;
pub mod storage;

pub use registry::Registry;
pub use storage::{BlobStorage, TypstFileSystem};

#[cfg(feature = "s3")]
pub use storage::s3_storage::S3Storage;
