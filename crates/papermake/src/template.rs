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
    
    /// Create a new template builder
    pub fn builder(id: impl Into<TemplateId>) -> TemplateBuilder {
        TemplateBuilder::new(id.into())
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
    
    /// Render the template with data to a PDF
    pub fn render(&self, data: &serde_json::Value) -> Result<crate::render::RenderResult> {
        crate::render::render_pdf(self, data, None)
    }
    
    /// Render the template with data and options to a PDF
    pub fn render_with_options(&self, data: &serde_json::Value, options: crate::render::RenderOptions) -> Result<crate::render::RenderResult> {
        crate::render::render_pdf(self, data, Some(options))
    }
    
    /// Render the template with data using a cached world
    pub fn render_with_cache(&self, data: &serde_json::Value, world_cache: Option<&mut crate::typst::TypstWorld>) -> Result<crate::render::RenderResult> {
        crate::render::render_pdf_with_cache(self, data, world_cache, None)
    }
    
    /// Render the template with data, options, and cached world
    pub fn render_with_cache_and_options(&self, data: &serde_json::Value, world_cache: Option<&mut crate::typst::TypstWorld>, options: crate::render::RenderOptions) -> Result<crate::render::RenderResult> {
        crate::render::render_pdf_with_cache(self, data, world_cache, Some(options))
    }
    
    pub fn from_file_content(id: impl Into<TemplateId>, content: &str) -> Result<Self> {
        // This is a simplified implementation
        // In a real implementation, you'd parse frontmatter for metadata
        // and extract the schema definition

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
            id: id.into(),
            name: "".to_string(), // TODO: extract from frontmatter
            content: template_content,
            schema,
            description: None,
            created_at: time::OffsetDateTime::now_utc(),
            updated_at: time::OffsetDateTime::now_utc(),
        })
    }
    

    /// Parse a template from a file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| PapermakeError::Io(e))?;
        
        let id = path.as_ref().file_stem().unwrap().to_string_lossy().to_string();
        Self::from_file_content(id, &content)
    }
}

/// Builder for creating templates with a fluent API
#[derive(Debug)]
pub struct TemplateBuilder {
    id: TemplateId,
    name: Option<String>,
    content: Option<String>,
    schema: Option<Schema>,
    description: Option<String>,
}

impl TemplateBuilder {
    /// Create a new template builder
    pub fn new(id: TemplateId) -> Self {
        TemplateBuilder {
            id,
            name: None,
            content: None,
            schema: None,
            description: None,
        }
    }
    
    /// Set the template name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    
    /// Set the template content
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }
    
    /// Set the template content from a file
    pub fn content_from_file(mut self, path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| PapermakeError::Io(e))?;
        self.content = Some(content);
        Ok(self)
    }
    
    /// Set the schema
    pub fn schema(mut self, schema: Schema) -> Self {
        self.schema = Some(schema);
        self
    }
    
    /// Set the description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    
    /// Build the template
    pub fn build(self) -> Result<Template> {
        let name = self.name.ok_or_else(|| PapermakeError::Template("Template name is required".to_string()))?;
        let content = self.content.ok_or_else(|| PapermakeError::Template("Template content is required".to_string()))?;
        let schema = self.schema.unwrap_or_default();
        
        let now = time::OffsetDateTime::now_utc();
        Ok(Template {
            id: self.id,
            name,
            content,
            schema,
            description: self.description,
            created_at: now,
            updated_at: now,
        })
    }
    
    /// Build the template with caching enabled for better performance
    pub fn build_cached(self) -> Result<crate::cache::CachedTemplate> {
        use crate::cache::TemplateCache;
        Ok(self.build()?.with_cache())
    }
}