//! Storage abstraction for templates
//! 

#[cfg(feature = "fs")]
mod file;
#[cfg(feature = "fs")]
pub use file::FileStorage;

use async_trait::async_trait;
use crate::error::Result;
use crate::template::{Template, TemplateId};

/// Storage trait for template persistence
#[async_trait]
pub trait Storage: 'static + Sync + Send {
    /// Save a template
    async fn save_template(&self, template: &Template) -> Result<()>;
    
    /// Get a template by ID
    async fn get_template(&self, id: &TemplateId) -> Result<Template>;
    
    /// List all templates
    async fn list_templates(&self) -> Result<Vec<Template>>;
    
    /// Delete a template
    async fn delete_template(&self, id: &TemplateId) -> Result<()>;
    
    /// Save an additional file associated with a template
    async fn save_template_file(&self, template_id: &TemplateId, path: &str, content: &[u8]) -> Result<()>;
    
    /// Get a file associated with a template
    async fn get_template_file(&self, template_id: &TemplateId, path: &str) -> Result<Vec<u8>>;
    
    /// List all files associated with a template
    async fn list_template_files(&self, template_id: &TemplateId) -> Result<Vec<String>>;
}

// A concrete implementation would be provided elsewhere,
// e.g. a FileStorage or DatabaseStorage