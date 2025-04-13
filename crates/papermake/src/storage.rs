//! Storage abstraction for templates

use async_trait::async_trait;
use crate::error::Result;
use crate::template::{Template, TemplateId};

/// Storage trait for template persistence
#[async_trait]
pub trait Storage: Send + Sync + 'static {
    /// Save a template
    async fn save_template(&self, template: &Template) -> Result<()>;
    
    /// Get a template by ID
    async fn get_template(&self, id: &TemplateId) -> Result<Template>;
    
    /// List all templates
    async fn list_templates(&self) -> Result<Vec<Template>>;
    
    /// Delete a template
    async fn delete_template(&self, id: &TemplateId) -> Result<()>;
}

// A concrete implementation would be provided elsewhere,
// e.g. a FileStorage or DatabaseStorage