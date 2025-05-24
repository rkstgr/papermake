//! Core registry service trait and implementations

use async_trait::async_trait;
use papermake::{Template, TemplateId, RenderResult};
use crate::{
    entities::*,
    error::{RegistryError, Result},
    storage::RegistryStorage,
};

/// Core registry service for template management
#[async_trait]
pub trait TemplateRegistry {
    // === Template Lifecycle ===
    
    /// Publish a new template version (auto-increments version number)
    async fn publish_template(
        &self,
        template: Template,
        author: &UserId,
        scope: TemplateScope,
    ) -> Result<u64>;
    
    /// Get a specific template version
    async fn get_template(&self, id: &TemplateId, version: Option<u64>) -> Result<VersionedTemplate>;
    
    /// Get the latest version number for a template
    async fn get_latest_version(&self, id: &TemplateId) -> Result<u64>;
    
    /// List all versions of a template
    async fn list_versions(&self, id: &TemplateId) -> Result<Vec<u64>>;
    
    /// Delete a template (only if user owns it and it's not public/marketplace)
    async fn delete_template(&self, id: &TemplateId, user: &UserId) -> Result<()>;
    
    // === Access Control ===
    
    /// Check if a user can access a specific template version
    async fn can_access(&self, id: &TemplateId, version: u64, user: &UserId) -> Result<bool>;
    
    /// Share a template with an organization (must be template owner)
    async fn share_with_org(&self, id: &TemplateId, org: &OrgId, user: &UserId) -> Result<()>;
    
    /// Make a template public (must be template owner, irreversible)
    async fn make_public(&self, id: &TemplateId, user: &UserId) -> Result<()>;
    
    // === Fork Workflow ===
    
    /// Fork a template to create an independent copy with attribution
    async fn fork_template(
        &self,
        source_id: &TemplateId,
        source_version: u64,
        new_id: &TemplateId,
        user: &UserId,
        new_scope: TemplateScope,
    ) -> Result<u64>;
    
    // === Discovery ===
    
    /// List templates owned by a user
    async fn list_user_templates(&self, user: &UserId) -> Result<Vec<(TemplateId, u64)>>;
    
    /// List templates shared within an organization
    async fn list_org_templates(&self, org: &OrgId, user: &UserId) -> Result<Vec<(TemplateId, u64)>>;
    
    /// List all public templates
    async fn list_public_templates(&self) -> Result<Vec<(TemplateId, u64)>>;
    
    /// List marketplace templates
    async fn list_marketplace_templates(&self) -> Result<Vec<(TemplateId, u64)>>;
    
    /// Search templates by name/description
    async fn search_templates(&self, query: &str, user: &UserId) -> Result<Vec<(TemplateId, u64)>>;
    
    // === Rendering ===
    
    /// Render a template with access control
    async fn render(
        &self,
        id: &TemplateId,
        version: Option<u64>,
        data: &serde_json::Value,
        user: &UserId,
    ) -> Result<RenderResult>;
    
    /// Render a template with custom options
    async fn render_with_options(
        &self,
        id: &TemplateId,
        version: Option<u64>,
        data: &serde_json::Value,
        options: papermake::RenderOptions,
        user: &UserId,
    ) -> Result<RenderResult>;
    
    // === User Management ===
    
    /// Save a user to the registry
    async fn save_user(&self, user: &User) -> Result<()>;
    
    /// Get a user by ID
    async fn get_user(&self, id: &UserId) -> Result<User>;
    
    /// Get a user by username
    async fn get_user_by_username(&self, username: &str) -> Result<User>;
    
    /// Add a user to an organization
    async fn add_user_to_org(&self, user_id: &UserId, org_id: &OrgId) -> Result<()>;
    
    // === Organization Management ===
    
    /// Save an organization
    async fn save_organization(&self, org: &Organization) -> Result<()>;
    
    /// Get an organization by ID
    async fn get_organization(&self, id: &OrgId) -> Result<Organization>;
    
    // === Marketplace ===
    
    /// Submit a template to the marketplace
    async fn submit_to_marketplace(
        &self,
        id: &TemplateId,
        version: u64,
        metadata: MarketplaceMetadata,
        user: &UserId,
    ) -> Result<()>;
    
    /// Get marketplace metadata for a template
    async fn get_marketplace_metadata(&self, id: &TemplateId, version: u64) -> Result<MarketplaceMetadata>;
}

/// Default implementation of the registry using any RegistryStorage backend
pub struct DefaultRegistry<S: RegistryStorage> {
    storage: S,
}

impl<S: RegistryStorage> DefaultRegistry<S> {
    /// Create a new registry with the given storage backend
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
    
    /// Get storage reference
    pub fn storage(&self) -> &S {
        &self.storage
    }
}

#[async_trait]
impl<S: RegistryStorage + Send + Sync> TemplateRegistry for DefaultRegistry<S> {
    async fn publish_template(
        &self,
        template: Template,
        author: &UserId,
        scope: TemplateScope,
    ) -> Result<u64> {
        // Check if this is the first version or increment
        let next_version = match self.get_latest_version(&template.id).await {
            Ok(latest) => latest + 1,
            Err(RegistryError::TemplateNotFound(_)) => 1, // First version
            Err(e) => return Err(e),
        };
        
        let versioned_template = VersionedTemplate::new(template, next_version, scope, author.clone());
        self.storage.save_versioned_template(&versioned_template).await?;
        
        Ok(next_version)
    }
    
    async fn get_template(&self, id: &TemplateId, version: Option<u64>) -> Result<VersionedTemplate> {
        let version = match version {
            Some(v) => v,
            None => self.get_latest_version(id).await?,
        };
        
        self.storage.get_versioned_template(id, version).await
    }
    
    async fn get_latest_version(&self, id: &TemplateId) -> Result<u64> {
        let versions = self.storage.list_template_versions(id).await?;
        versions.into_iter().max().ok_or_else(|| {
            RegistryError::TemplateNotFound(id.as_ref().to_string())
        })
    }
    
    async fn list_versions(&self, id: &TemplateId) -> Result<Vec<u64>> {
        self.storage.list_template_versions(id).await
    }
    
    async fn can_access(&self, id: &TemplateId, version: u64, user: &UserId) -> Result<bool> {
        let template = self.storage.get_versioned_template(id, version).await?;
        let user = self.storage.get_user(user).await?;
        Ok(template.can_access(&user))
    }
    
    async fn fork_template(
        &self,
        source_id: &TemplateId,
        source_version: u64,
        new_id: &TemplateId,
        user: &UserId,
        new_scope: TemplateScope,
    ) -> Result<u64> {
        // Check access to source template
        if !self.can_access(source_id, source_version, user).await? {
            return Err(RegistryError::AccessDenied(
                format!("Cannot access template {} version {}", source_id.as_ref(), source_version)
            ));
        }
        
        // Get source template
        let source_template = self.storage.get_versioned_template(source_id, source_version).await?;
        
        // Create new template with modified ID
        let mut new_template = source_template.template.clone();
        new_template.id = new_id.clone();
        new_template.updated_at = time::OffsetDateTime::now_utc();
        
        // Create versioned template with fork attribution
        let versioned_template = VersionedTemplate::forked_from(
            new_template,
            1, // Always start at version 1 for forks
            new_scope,
            user.clone(),
            (source_id.clone(), source_version),
        );
        
        self.storage.save_versioned_template(&versioned_template).await?;
        Ok(1)
    }
    
    async fn render(
        &self,
        id: &TemplateId,
        version: Option<u64>,
        data: &serde_json::Value,
        user: &UserId,
    ) -> Result<RenderResult> {
        self.render_with_options(id, version, data, papermake::RenderOptions::default(), user).await
    }
    
    async fn render_with_options(
        &self,
        id: &TemplateId,
        version: Option<u64>,
        data: &serde_json::Value,
        options: papermake::RenderOptions,
        user: &UserId,
    ) -> Result<RenderResult> {
        let template = self.get_template(id, version).await?;
        
        // Check access
        let user_obj = self.storage.get_user(user).await?;
        if !template.can_access(&user_obj) {
            return Err(RegistryError::AccessDenied(
                format!("Cannot access template {} version {}", id.as_ref(), template.version)
            ));
        }
        
        // Render using papermake
        Ok(template.template.render_with_options(data, options)?)
    }
    
    async fn save_user(&self, user: &User) -> Result<()> {
        self.storage.save_user(user).await
    }
    
    async fn get_user(&self, id: &UserId) -> Result<User> {
        self.storage.get_user(id).await
    }
    
    async fn get_user_by_username(&self, username: &str) -> Result<User> {
        self.storage.get_user_by_username(username).await
    }
    
    async fn list_user_templates(&self, user: &UserId) -> Result<Vec<(TemplateId, u64)>> {
        self.storage.list_templates_by_scope(&TemplateScope::User(user.clone())).await
    }
    
    async fn list_public_templates(&self) -> Result<Vec<(TemplateId, u64)>> {
        self.storage.list_templates_by_scope(&TemplateScope::Public).await
    }
    
    async fn list_marketplace_templates(&self) -> Result<Vec<(TemplateId, u64)>> {
        self.storage.list_templates_by_scope(&TemplateScope::Marketplace).await
    }
    
    // Simplified implementations for remaining methods
    async fn delete_template(&self, _id: &TemplateId, _user: &UserId) -> Result<()> {
        todo!("Delete template implementation")
    }
    
    async fn share_with_org(&self, _id: &TemplateId, _org: &OrgId, _user: &UserId) -> Result<()> {
        todo!("Share with org implementation")
    }
    
    async fn make_public(&self, _id: &TemplateId, _user: &UserId) -> Result<()> {
        todo!("Make public implementation")
    }
    
    async fn list_org_templates(&self, org: &OrgId, _user: &UserId) -> Result<Vec<(TemplateId, u64)>> {
        self.storage.list_templates_by_scope(&TemplateScope::Organization(org.clone())).await
    }
    
    async fn search_templates(&self, _query: &str, _user: &UserId) -> Result<Vec<(TemplateId, u64)>> {
        todo!("Search implementation")
    }
    
    async fn add_user_to_org(&self, _user_id: &UserId, _org_id: &OrgId) -> Result<()> {
        todo!("Add user to org implementation")
    }
    
    async fn save_organization(&self, _org: &Organization) -> Result<()> {
        todo!("Save organization implementation")
    }
    
    async fn get_organization(&self, _id: &OrgId) -> Result<Organization> {
        todo!("Get organization implementation")
    }
    
    async fn submit_to_marketplace(&self, _id: &TemplateId, _version: u64, _metadata: MarketplaceMetadata, _user: &UserId) -> Result<()> {
        todo!("Submit to marketplace implementation")
    }
    
    async fn get_marketplace_metadata(&self, _id: &TemplateId, _version: u64) -> Result<MarketplaceMetadata> {
        todo!("Get marketplace metadata implementation")
    }
}