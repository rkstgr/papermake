//! Template service business logic

use papermake_registry::DefaultRegistry;
use std::sync::Arc;

/// Service for template-related business logic
#[derive(Clone)]
pub struct TemplateService {
    registry: Arc<DefaultRegistry>,
}

impl TemplateService {
    pub fn new(registry: Arc<DefaultRegistry>) -> Self {
        Self { registry }
    }
}