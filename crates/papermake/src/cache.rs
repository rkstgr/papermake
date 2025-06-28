//! Template-level caching for improved performance

use crate::render::{RenderOptions, RenderResult};
use crate::typst::TypstWorld;
use crate::{PapermakeError, Result, Template};
use std::sync::{Arc, Mutex};

/// A cached template that reuses the compiled world for faster rendering
#[derive(Debug)]
pub struct CachedTemplate {
    template: Template,
    world_cache: Arc<Mutex<Option<TypstWorld>>>,
}

impl CachedTemplate {
    /// Create a new cached template from an existing template
    pub fn new(template: Template) -> Self {
        Self {
            template,
            world_cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Render the template with data, using the cached world for performance
    pub fn render(&self, data: &serde_json::Value) -> Result<RenderResult> {
        self.render_with_options(data, RenderOptions::default())
    }

    /// Render the template with data and options, using the cached world
    pub fn render_with_options(
        &self,
        data: &serde_json::Value,
        options: RenderOptions,
    ) -> Result<RenderResult> {
        let mut cache_guard = self
            .world_cache
            .lock()
            .map_err(|_| PapermakeError::Rendering("Failed to acquire cache lock".to_string()))?;

        // Initialize cache if empty or update existing cache
        match cache_guard.as_mut() {
            Some(cached_world) => {
                // Use existing cached world
                crate::render::render_pdf_with_cache(
                    &self.template,
                    data,
                    Some(cached_world),
                    Some(options),
                )
            }
            None => {
                // Create new world and cache it
                let mut new_world = TypstWorld::new(
                    self.template.content.clone(),
                    serde_json::to_string(data)
                        .map_err(|e| PapermakeError::Rendering(e.to_string()))?,
                );
                let result = crate::render::render_pdf_with_cache(
                    &self.template,
                    data,
                    Some(&mut new_world),
                    Some(options),
                )?;
                *cache_guard = Some(new_world);
                Ok(result)
            }
        }
    }

    /// Clear the cached world, forcing recompilation on next render
    pub fn clear_cache(&self) -> Result<()> {
        let mut cache_guard = self
            .world_cache
            .lock()
            .map_err(|_| PapermakeError::Rendering("Failed to acquire cache lock".to_string()))?;
        *cache_guard = None;
        Ok(())
    }

    /// Check if the template has a cached world
    pub fn is_cached(&self) -> bool {
        self.world_cache
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    /// Get a reference to the underlying template
    pub fn template(&self) -> &Template {
        &self.template
    }

    /// Validate data against the template's schema
    pub fn validate_data(&self, data: &serde_json::Value) -> Result<()> {
        self.template.validate_data(data)
    }
}

impl Clone for CachedTemplate {
    fn clone(&self) -> Self {
        Self {
            template: self.template.clone(),
            world_cache: Arc::new(Mutex::new(None)), // New cache for clone
        }
    }
}

/// Extension trait to add caching capabilities to Template
pub trait TemplateCache {
    /// Convert this template into a cached template for better performance
    fn with_cache(self) -> CachedTemplate;
}

impl TemplateCache for Template {
    fn with_cache(self) -> CachedTemplate {
        CachedTemplate::new(self)
    }
}
