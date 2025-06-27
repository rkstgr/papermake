//! Template handling for Typst documents

use std::path::Path;
use serde::{Serialize, Deserialize};
use crate::error::{PapermakeError, Result};
use crate::schema::Schema;

/// A template for PDF generation (core content only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// Typst markdown content
    pub content: String,
    
    /// Associated schema
    pub schema: Schema,
    
    /// Optional description
    pub description: Option<String>,
}

impl Template {
    /// Create a new template
    pub fn new(content: impl Into<String>, schema: Schema) -> Self {
        Template {
            content: content.into(),
            schema,
            description: None,
        }
    }
    
    /// Create a new template builder
    pub fn builder() -> TemplateBuilder {
        TemplateBuilder::new()
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
    
    pub fn from_file_content(content: &str) -> Result<Self> {
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
            content: template_content,
            schema,
            description: None,
        })
    }
    

    /// Parse a template from a file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| PapermakeError::Io(e))?;
        
        Self::from_file_content(&content)
    }
}

/// Builder for creating templates with a fluent API
#[derive(Debug)]
pub struct TemplateBuilder {
    content: Option<String>,
    schema: Option<Schema>,
    description: Option<String>,
}

impl TemplateBuilder {
    /// Create a new template builder
    pub fn new() -> Self {
        TemplateBuilder {
            content: None,
            schema: None,
            description: None,
        }
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
        let content = self.content.ok_or_else(|| PapermakeError::Template("Template content is required".to_string()))?;
        let schema = self.schema.unwrap_or_default();
        
        Ok(Template {
            content,
            schema,
            description: self.description,
        })
    }
    
    /// Build the template with caching enabled for better performance
    pub fn build_cached(self) -> Result<crate::cache::CachedTemplate> {
        use crate::cache::TemplateCache;
        Ok(self.build()?.with_cache())
    }
    
    /// Convenience method for AWS Lambda and similar scenarios: parse raw content directly into a cached template
    /// This is useful when you have template content from S3, databases, etc. and want immediate caching
    pub fn from_raw_content_cached(content: impl Into<String>) -> Result<crate::cache::CachedTemplate> {
        use crate::cache::TemplateCache;
        let template = Self::new()
            .content(content)
            .build()?;
        Ok(template.with_cache())
    }
}