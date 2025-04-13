//! Template handling for Typst documents

use std::path::Path;
use serde::{Serialize, Deserialize};
use crate::error::{PapermakeError, Result};
use crate::schema::Schema;

/// Unique identifier for a template
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TemplateId(pub String);

impl From<String> for TemplateId {
    fn from(s: String) -> Self {
        TemplateId(s)
    }
}

impl From<&str> for TemplateId {
    fn from(s: &str) -> Self {
        TemplateId(s.to_string())
    }
}

impl AsRef<str> for TemplateId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A template for PDF generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// Unique identifier
    pub id: TemplateId,
    
    /// Human-readable name
    pub name: String,
    
    /// Typst markdown content
    pub content: String,
    
    /// Associated schema
    pub schema: Schema,
    
    /// Optional description
    pub description: Option<String>,
    
    /// Creation timestamp
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    
    /// Last update timestamp
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: time::OffsetDateTime,
}

impl Template {
    /// Create a new template
    pub fn new(id: impl Into<TemplateId>, name: impl Into<String>, content: impl Into<String>, schema: Schema) -> Self {
        let now = time::OffsetDateTime::now_utc();
        Template {
            id: id.into(),
            name: name.into(),
            content: content.into(),
            schema,
            description: None,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    
    /// Validate data against the template's schema
    pub fn validate_data(&self, data: &serde_json::Value) -> Result<()> {
        self.schema.validate(data)
    }
    
    /// Parse a template from a file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| PapermakeError::Io(e))?;
            
        // This is a simplified implementation
        // In a real implementation, you'd parse frontmatter for metadata
        // and extract the schema definition
        
        // For now, just extract content after "---" markers
        let parts: Vec<&str> = content.split("---").collect();
        if parts.len() < 3 {
            return Err(PapermakeError::Template(
                "Invalid template format. Expected frontmatter between '---' markers.".to_string()
            ));
        }
        
        // Very naive parsing for demo purposes
        let template_content = parts[2].trim().to_string();
        
        // In a real implementation, you'd parse the frontmatter (parts[1])
        // to extract metadata and schema
        let schema = Schema::default();
        
        Ok(Template {
            id: TemplateId(path.as_ref().file_stem().unwrap().to_string_lossy().to_string()),
            name: path.as_ref().file_stem().unwrap().to_string_lossy().to_string(),
            content: template_content,
            schema,
            description: None,
            created_at: time::OffsetDateTime::now_utc(),
            updated_at: time::OffsetDateTime::now_utc(),
        })
    }
}