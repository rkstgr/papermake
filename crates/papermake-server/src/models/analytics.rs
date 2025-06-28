//! Analytics and dashboard API models

use super::{RenderJobSummary, TemplateSummary};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Dashboard metrics overview
#[derive(Debug, Serialize)]
pub struct DashboardMetrics {
    /// Current queue depth (jobs waiting to be processed)
    pub queue_depth: i64,

    /// P90 latency in milliseconds (last 1000 jobs)
    pub p90_latency_ms: Option<i64>,

    /// Total renders in last 24 hours
    pub total_renders_24h: i64,

    /// Success rate in last 24 hours (0.0 to 1.0)
    pub success_rate_24h: f64,

    /// Most recent render jobs
    pub recent_renders: Vec<RenderJobSummary>,

    /// Most popular templates (last 24h)
    pub popular_templates: Vec<TemplateUsage>,

    /// Recently published templates
    pub new_templates: Vec<TemplateSummary>,
}

/// Template usage statistics
#[derive(Debug, Serialize)]
pub struct TemplateUsage {
    pub template_ref: String,
    pub template_name: String,
    pub version: String,
    pub uses_24h: i64,
    pub uses_7d: i64,
    pub uses_30d: i64,
    pub uses_total: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub published_at: OffsetDateTime,
    pub avg_render_time_ms: Option<f64>,
}

/// Performance metrics over time
#[derive(Debug, Serialize)]
pub struct PerformanceMetrics {
    /// Time period these metrics cover
    pub period: TimePeriod,

    /// Data points over time
    pub data_points: Vec<PerformanceDataPoint>,

    /// Summary statistics
    pub summary: PerformanceSummary,
}

/// Performance data point for time series
#[derive(Debug, Serialize)]
pub struct PerformanceDataPoint {
    pub timestamp: OffsetDateTime,
    pub total_renders: i64,
    pub successful_renders: i64,
    pub failed_renders: i64,
    pub avg_latency_ms: Option<f64>,
    pub p90_latency_ms: Option<f64>,
    pub queue_depth: Option<i64>,
}

/// Performance summary statistics
#[derive(Debug, Serialize)]
pub struct PerformanceSummary {
    pub total_renders: i64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p90_latency_ms: f64,
    pub p99_latency_ms: f64,
}

/// Time period for analytics queries
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TimePeriod {
    Hour,
    Day,
    Week,
    Month,
}

/// Analytics query parameters
#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    /// Time period to analyze
    pub period: Option<TimePeriod>,

    /// Start date for analysis
    pub date_from: Option<OffsetDateTime>,

    /// End date for analysis
    pub date_to: Option<OffsetDateTime>,

    /// Filter by specific template reference
    pub template_ref: Option<String>,

    /// Limit number of results
    pub limit: Option<u32>,
}

/// Template analytics details
#[derive(Debug, Serialize)]
pub struct TemplateAnalytics {
    pub template_ref: String,
    pub template_name: String,
    pub total_versions: u64,
    pub latest_version: String,
    pub usage_over_time: Vec<UsageDataPoint>,
    pub performance_metrics: TemplatePerformanceMetrics,
}

/// Usage data point for template analytics
#[derive(Debug, Serialize)]
pub struct UsageDataPoint {
    pub timestamp: OffsetDateTime,
    pub renders: i64,
    pub unique_data_hashes: i64,
    pub avg_render_time_ms: Option<f64>,
}

/// Performance metrics specific to a template
#[derive(Debug, Serialize)]
pub struct TemplatePerformanceMetrics {
    pub total_renders: i64,
    pub successful_renders: i64,
    pub failed_renders: i64,
    pub success_rate: f64,
    pub avg_render_time_ms: f64,
    pub fastest_render_ms: Option<i64>,
    pub slowest_render_ms: Option<i64>,
    pub cache_hit_rate: f64, // Based on data_hash duplicates
}

/// System health metrics
#[derive(Debug, Serialize)]
pub struct SystemHealth {
    pub status: HealthStatus,
    pub uptime_seconds: u64,
    pub queue_health: QueueHealth,
    pub storage_health: StorageHealth,
    pub last_updated: OffsetDateTime,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

#[derive(Debug, Serialize)]
pub struct QueueHealth {
    pub status: HealthStatus,
    pub current_depth: i64,
    pub max_depth_24h: i64,
    pub processing_rate: f64, // jobs per minute
    pub avg_wait_time_ms: f64,
}

#[derive(Debug, Serialize)]
pub struct StorageHealth {
    pub status: HealthStatus,
    pub database_connected: bool,
    pub s3_connected: bool,
    pub database_response_time_ms: Option<i64>,
    pub s3_response_time_ms: Option<i64>,
}
