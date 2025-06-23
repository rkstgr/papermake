//! Analytics service business logic

use papermake_registry::DefaultRegistry;
use std::sync::Arc;

/// Service for analytics-related business logic
#[derive(Clone)]
pub struct AnalyticsService {
    registry: Arc<DefaultRegistry>,
}

impl AnalyticsService {
    pub fn new(registry: Arc<DefaultRegistry>) -> Self {
        Self { registry }
    }
}