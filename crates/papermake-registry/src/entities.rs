//! Core data structures for the papermake registry

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;
use papermake::{Template, TemplateId};

/// Unique identifier for a user
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

impl From<String> for UserId {
    fn from(s: String) -> Self {
        UserId(s)
    }
}

impl From<&str> for UserId {
    fn from(s: &str) -> Self {
        UserId(s.to_string())
    }
}

impl AsRef<str> for UserId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Unique identifier for an organization
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrgId(pub String);

impl From<String> for OrgId {
    fn from(s: String) -> Self {
        OrgId(s)
    }
}

impl From<&str> for OrgId {
    fn from(s: &str) -> Self {
        OrgId(s.to_string())
    }
}

impl AsRef<str> for OrgId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A user in the registry system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user identifier
    pub id: UserId,
    
    /// Human-readable username
    pub username: String,
    
    /// User's email address
    pub email: String,
    
    /// Organizations the user belongs to
    pub organizations: Vec<OrgId>,
    
    /// When the user account was created
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    
    /// Last time user information was updated
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl User {
    /// Create a new user with generated ID
    pub fn new(username: impl Into<String>, email: impl Into<String>) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: UserId(Uuid::new_v4().to_string()),
            username: username.into(),
            email: email.into(),
            organizations: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Create a user with specific ID (useful for testing)
    pub fn with_id(id: impl Into<UserId>, username: impl Into<String>, email: impl Into<String>) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: id.into(),
            username: username.into(),
            email: email.into(),
            organizations: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Add user to an organization
    pub fn add_to_organization(&mut self, org_id: OrgId) {
        if !self.organizations.contains(&org_id) {
            self.organizations.push(org_id);
            self.updated_at = OffsetDateTime::now_utc();
        }
    }
    
    /// Check if user belongs to an organization
    pub fn is_member_of(&self, org_id: &OrgId) -> bool {
        self.organizations.contains(org_id)
    }
}

/// An organization in the registry system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    /// Unique organization identifier
    pub id: OrgId,
    
    /// Organization name
    pub name: String,
    
    /// Optional description
    pub description: Option<String>,
    
    /// When the organization was created
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl Organization {
    /// Create a new organization with generated ID
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: OrgId(Uuid::new_v4().to_string()),
            name: name.into(),
            description: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }
    
    /// Create an organization with specific ID
    pub fn with_id(id: impl Into<OrgId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }
    
    /// Set organization description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Template visibility and access scope
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateScope {
    /// Private to a specific user
    User(UserId),
    
    /// Shared within an organization
    Organization(OrgId),
    
    /// Publicly accessible to all users
    Public,
    
    /// Available in the marketplace (potentially paid)
    Marketplace,
}

impl TemplateScope {
    /// Check if a user can access templates in this scope
    pub fn can_access(&self, user: &User) -> bool {
        match self {
            TemplateScope::User(owner_id) => &user.id == owner_id,
            TemplateScope::Organization(org_id) => user.is_member_of(org_id),
            TemplateScope::Public | TemplateScope::Marketplace => true,
        }
    }
    
    /// Check if a user can modify the scope (e.g., share with org, make public)
    pub fn can_modify(&self, user: &User) -> bool {
        match self {
            TemplateScope::User(owner_id) => &user.id == owner_id,
            TemplateScope::Organization(org_id) => user.is_member_of(org_id),
            TemplateScope::Public | TemplateScope::Marketplace => false, // Immutable once public
        }
    }
}

/// A versioned template with registry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedTemplate {
    /// The core template from papermake
    pub template: Template,
    
    /// Auto-incrementing version number (1, 2, 3...)
    pub version: u64,
    
    /// Access scope and visibility
    pub scope: TemplateScope,
    
    /// User who published this version
    pub author: UserId,
    
    /// If this template was forked, track the source (clean slate approach)
    pub forked_from: Option<(TemplateId, u64)>,
    
    /// When this version was published
    #[serde(with = "time::serde::rfc3339")]
    pub published_at: OffsetDateTime,
    
    /// Whether this version is immutable (true once published)
    pub immutable: bool,
    
    /// Optional marketplace metadata
    pub marketplace_metadata: Option<MarketplaceMetadata>,
}

impl VersionedTemplate {
    /// Create a new versioned template
    pub fn new(
        template: Template,
        version: u64,
        scope: TemplateScope,
        author: UserId,
    ) -> Self {
        Self {
            template,
            version,
            scope,
            author,
            forked_from: None,
            published_at: OffsetDateTime::now_utc(),
            immutable: true, // Templates are immutable once published
            marketplace_metadata: None,
        }
    }
    
    /// Create a forked template
    pub fn forked_from(
        template: Template,
        version: u64,
        scope: TemplateScope,
        author: UserId,
        source: (TemplateId, u64),
    ) -> Self {
        Self {
            template,
            version,
            scope,
            author,
            forked_from: Some(source),
            published_at: OffsetDateTime::now_utc(),
            immutable: true,
            marketplace_metadata: None,
        }
    }
    
    /// Check if a user can access this template version
    pub fn can_access(&self, user: &User) -> bool {
        self.scope.can_access(user)
    }
    
    /// Get the template ID
    pub fn id(&self) -> &TemplateId {
        &self.template.id
    }
    
    /// Add marketplace metadata
    pub fn with_marketplace_metadata(mut self, metadata: MarketplaceMetadata) -> Self {
        self.marketplace_metadata = Some(metadata);
        self
    }
}

/// Metadata for marketplace templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceMetadata {
    /// Display name for the marketplace
    pub title: String,
    
    /// Marketing description
    pub description: String,
    
    /// Template category
    pub category: String,
    
    /// Tags for search
    pub tags: Vec<String>,
    
    /// Price in cents (0 for free)
    pub price_cents: u64,
    
    /// Preview images or screenshots
    pub preview_urls: Vec<String>,
    
    /// Author attribution for marketplace
    pub author_name: String,
    
    /// License information
    pub license: String,
}

impl MarketplaceMetadata {
    /// Create new marketplace metadata
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        category: impl Into<String>,
        author_name: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            category: category.into(),
            tags: Vec::new(),
            price_cents: 0,
            preview_urls: Vec::new(),
            author_name: author_name.into(),
            license: "MIT".to_string(), // Default license
        }
    }
    
    /// Add tags to the template
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
    
    /// Set the price
    pub fn with_price(mut self, price_cents: u64) -> Self {
        self.price_cents = price_cents;
        self
    }
    
    /// Add preview URLs
    pub fn with_previews(mut self, urls: Vec<String>) -> Self {
        self.preview_urls = urls;
        self
    }
}