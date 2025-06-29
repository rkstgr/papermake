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
//! use papermake::TemplateBuilder;
//!
//! # async fn example() -> Result<()> {
//! // Create a registry
//! let storage = FileSystemStorage::new("./templates").await?;
//! let registry = DefaultRegistry::new(storage);
//!
//! // Create a user
//! let user = User::new("alice", "alice@company.com");
//! registry.save_user(&user).await?;
//!
//! // Publish a template
//! let template = TemplateBuilder::new("invoice-template".into())
//!     .name("Invoice Template")
//!     .content("Invoice for: #data.customer_name")
//!     .build()?;
//!
//! let version = registry.publish_template(
//!     template,
//!     &user.id,
//!     TemplateScope::User(user.id.clone())
//! ).await?;
//!
//! // Render with access control
//! let data = serde_json::json!({"customer_name": "ACME Corp"});
//! let result = registry.render(
//!     &"invoice-template".into(),
//!     Some(version),
//!     &data,
//!     &user.id
//! ).await?;
//! # Ok(())
//! # }
//! ```

pub mod address;
pub mod bundle;
pub mod error;
pub mod manifest;
pub mod reference;
pub mod storage;

pub use storage::{BlobStorage, TypstFileSystem};

#[cfg(feature = "s3")]
pub use storage::s3_storage::S3Storage;
