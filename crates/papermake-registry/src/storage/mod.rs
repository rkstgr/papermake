//! Storage abstraction for registry data

use crate::{entities::*, error::Result};
use async_trait::async_trait;

// File system implementation
#[cfg(feature = "fs")]
pub mod file_storage;

// PostgreSQL implementation  
#[cfg(feature = "postgres")]
pub mod postgres;

/// Storage trait for registry operations
///
/// This trait defines all storage operations needed for the template registry,
/// including templates, users, organizations, and marketplace data.
#[async_trait]
pub trait RegistryStorage {
    // === Template Management ===

    /// Save a versioned template
    async fn save_versioned_template(&self, template: &VersionedTemplate) -> Result<()>;

    /// Get a specific version of a template
    async fn get_versioned_template(
        &self,
        id: &papermake::TemplateId,
        version: u64,
    ) -> Result<VersionedTemplate>;

    /// List all versions for a template
    async fn list_template_versions(&self, id: &papermake::TemplateId) -> Result<Vec<u64>>;

    /// Delete a specific template version
    async fn delete_template_version(&self, id: &papermake::TemplateId, version: u64)
    -> Result<()>;

    /// Save additional files for a template (fonts, images, etc.)
    async fn save_template_asset(
        &self,
        template_id: &papermake::TemplateId,
        path: &str,
        content: &[u8],
    ) -> Result<()>;

    /// Get a template asset
    async fn get_template_asset(
        &self,
        template_id: &papermake::TemplateId,
        path: &str,
    ) -> Result<Vec<u8>>;

    /// List all assets for a template
    async fn list_template_assets(
        &self,
        template_id: &papermake::TemplateId,
    ) -> Result<Vec<String>>;

    // === User Management ===

    /// Save a user
    async fn save_user(&self, user: &User) -> Result<()>;

    /// Get a user by ID
    async fn get_user(&self, id: &UserId) -> Result<User>;

    /// Get a user by username (must be unique)
    async fn get_user_by_username(&self, username: &str) -> Result<User>;

    /// List all users (admin operation)
    async fn list_users(&self) -> Result<Vec<User>>;

    /// Delete a user
    async fn delete_user(&self, id: &UserId) -> Result<()>;

    // === Organization Management ===

    /// Save an organization
    async fn save_organization(&self, org: &Organization) -> Result<()>;

    /// Get an organization by ID
    async fn get_organization(&self, id: &OrgId) -> Result<Organization>;

    /// List all organizations
    async fn list_organizations(&self) -> Result<Vec<Organization>>;

    /// Delete an organization
    async fn delete_organization(&self, id: &OrgId) -> Result<()>;

    // === Discovery and Access Control ===

    /// List templates by scope (for discovery)
    async fn list_templates_by_scope(
        &self,
        scope: &TemplateScope,
    ) -> Result<Vec<(papermake::TemplateId, u64)>>;

    /// Search templates by name/description
    async fn search_templates(
        &self,
        query: &str,
        user_id: &UserId,
    ) -> Result<Vec<(papermake::TemplateId, u64)>>;

    /// Check if a user can access a specific template version
    async fn can_user_access(
        &self,
        template_id: &papermake::TemplateId,
        version: u64,
        user_id: &UserId,
    ) -> Result<bool>;

    // === Marketplace Operations ===

    /// Update marketplace metadata for a template
    async fn update_marketplace_metadata(
        &self,
        template_id: &papermake::TemplateId,
        version: u64,
        metadata: &MarketplaceMetadata,
    ) -> Result<()>;

    /// Get marketplace metadata
    async fn get_marketplace_metadata(
        &self,
        template_id: &papermake::TemplateId,
        version: u64,
    ) -> Result<MarketplaceMetadata>;
}
