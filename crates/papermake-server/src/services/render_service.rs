//! Render service business logic

use papermake_registry::DefaultRegistry;
use std::sync::Arc;

/// Service for render-related business logic
#[derive(Clone)]
pub struct RenderService {
    registry: Arc<DefaultRegistry>,
}

impl RenderService {
    pub fn new(registry: Arc<DefaultRegistry>) -> Self {
        Self { registry }
    }
}