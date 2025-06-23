//! Analytics and dashboard routes

use crate::{
    error::{ApiError, Result},
    models::{
        ApiResponse, DashboardMetrics, TemplateUsage, PerformanceMetrics, AnalyticsQuery,
        TemplateAnalytics, SystemHealth, RenderJobSummary, TemplateSummary, HealthStatus,
        QueueHealth, StorageHealth,
    },
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::get,
    Router,
};
use papermake_registry::TemplateId;
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::debug;

/// Create analytics routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dashboard", get(get_dashboard_metrics))
        .route("/templates/usage", get(get_template_usage))
        .route("/templates/:template_id/analytics", get(get_template_analytics))
        .route("/performance", get(get_performance_metrics))
        .route("/health", get(get_system_health))
}

/// Get dashboard overview metrics
async fn get_dashboard_metrics(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<DashboardMetrics>>> {
    debug!("Getting dashboard metrics");

    // Get current queue depth
    let queue_depth = calculate_queue_depth(&state).await?;

    // Get recent render jobs
    let all_jobs = state.registry.list_render_jobs().await?;
    let recent_renders: Vec<RenderJobSummary> = all_jobs
        .iter()
        .take(10)
        .cloned()
        .map(RenderJobSummary::from)
        .collect();

    // Calculate 24h metrics
    let now = OffsetDateTime::now_utc();
    let yesterday = now - time::Duration::hours(24);
    
    let jobs_24h: Vec<_> = all_jobs
        .iter()
        .filter(|job| job.created_at >= yesterday)
        .collect();

    let total_renders_24h = jobs_24h.len() as i64;
    let successful_renders_24h = jobs_24h
        .iter()
        .filter(|job| job.completed_at.is_some() && job.pdf_s3_key.is_some())
        .count() as i64;

    let success_rate_24h = if total_renders_24h > 0 {
        successful_renders_24h as f64 / total_renders_24h as f64
    } else {
        1.0
    };

    // Calculate P90 latency from last 1000 completed jobs
    let completed_jobs: Vec<_> = all_jobs
        .iter()
        .filter(|job| job.rendering_latency.is_some())
        .collect();

    let p90_latency_ms = if !completed_jobs.is_empty() {
        let mut latencies: Vec<i64> = completed_jobs
            .iter()
            .filter_map(|job| job.rendering_latency)
            .collect();
        latencies.sort();
        
        if !latencies.is_empty() {
            let p90_index = (latencies.len() as f64 * 0.9) as usize;
            Some(latencies.get(p90_index).copied().unwrap_or(0))
        } else {
            None
        }
    } else {
        None
    };

    // Get popular templates (simplified - count by template_id in last 24h)
    let mut template_usage_map = std::collections::HashMap::new();
    for job in &jobs_24h {
        let key = (&job.template_id, job.template_version);
        *template_usage_map.entry(key).or_insert(0) += 1;
    }

    let popular_templates: Vec<TemplateUsage> = template_usage_map
        .into_iter()
        .map(|((template_id, version), count)| TemplateUsage {
            template_id: template_id.clone(),
            template_name: template_id.to_string(), // Would get actual name from registry
            version,
            uses_24h: count,
            uses_7d: count, // Simplified
            uses_30d: count, // Simplified
            uses_total: count, // Simplified
            published_at: now, // Would get actual date from registry
            avg_render_time_ms: None,
        })
        .take(5)
        .collect();

    // Get new templates (last 5 published)
    let all_templates = state.registry.list_templates().await?;
    let mut sorted_templates = all_templates;
    sorted_templates.sort_by(|a, b| b.published_at.cmp(&a.published_at));
    
    let new_templates: Vec<TemplateSummary> = sorted_templates
        .into_iter()
        .take(5)
        .map(|vt| TemplateSummary {
            id: vt.template.id,
            name: vt.template.name,
            latest_version: vt.version,
            uses_24h: 0, // Would calculate from jobs
            published_at: vt.published_at,
            author: vt.author,
        })
        .collect();

    let metrics = DashboardMetrics {
        queue_depth,
        p90_latency_ms,
        total_renders_24h,
        success_rate_24h,
        recent_renders,
        popular_templates,
        new_templates,
    };

    Ok(Json(ApiResponse::new(metrics)))
}

/// Get template usage statistics
async fn get_template_usage(
    State(state): State<AppState>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<ApiResponse<Vec<TemplateUsage>>>> {
    debug!("Getting template usage with query: {:?}", query);

    let jobs = state.registry.list_render_jobs().await?;

    // Apply date filtering
    let now = OffsetDateTime::now_utc();
    let filtered_jobs: Vec<_> = jobs
        .iter()
        .filter(|job| {
            if let Some(date_from) = query.date_from {
                if job.created_at < date_from {
                    return false;
                }
            }
            if let Some(date_to) = query.date_to {
                if job.created_at > date_to {
                    return false;
                }
            }
            if let Some(ref template_id) = query.template_id {
                if &job.template_id != template_id {
                    return false;
                }
            }
            true
        })
        .collect();

    // Group by template and calculate usage
    let mut usage_map = std::collections::HashMap::new();
    for job in filtered_jobs {
        let key = (&job.template_id, job.template_version);
        let entry = usage_map.entry(key).or_insert_with(|| {
            (0i64, Vec::new()) // (count, latencies)
        });
        entry.0 += 1;
        if let Some(latency) = job.rendering_latency {
            entry.1.push(latency);
        }
    }

    let mut usage_stats: Vec<TemplateUsage> = usage_map
        .into_iter()
        .map(|((template_id, version), (count, latencies))| {
            let avg_render_time_ms = if !latencies.is_empty() {
                Some(latencies.iter().sum::<i64>() as f64 / latencies.len() as f64)
            } else {
                None
            };

            TemplateUsage {
                template_id: template_id.clone(),
                template_name: template_id.to_string(), // Would get from registry
                version,
                uses_24h: count, // Simplified - would calculate per period
                uses_7d: count,
                uses_30d: count,
                uses_total: count,
                published_at: now, // Would get from registry
                avg_render_time_ms,
            }
        })
        .collect();

    // Sort by usage count
    usage_stats.sort_by(|a, b| b.uses_total.cmp(&a.uses_total));

    // Apply limit
    if let Some(limit) = query.limit {
        usage_stats.truncate(limit as usize);
    }

    Ok(Json(ApiResponse::new(usage_stats)))
}

/// Get analytics for a specific template
async fn get_template_analytics(
    State(state): State<AppState>,
    Path(template_id): Path<TemplateId>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<ApiResponse<TemplateAnalytics>>> {
    debug!("Getting analytics for template: {}", template_id);

    // Get template versions
    let versions = state.registry.list_versions(&template_id).await?;
    let latest_version = versions.iter().max().copied().unwrap_or(1);

    // Get render jobs for this template
    let all_jobs = state.registry.list_render_jobs().await?;
    let template_jobs: Vec<_> = all_jobs
        .iter()
        .filter(|job| job.template_id == template_id)
        .collect();

    // Calculate performance metrics
    let total_renders = template_jobs.len() as i64;
    let successful_renders = template_jobs
        .iter()
        .filter(|job| job.completed_at.is_some() && job.pdf_s3_key.is_some())
        .count() as i64;
    let failed_renders = total_renders - successful_renders;

    let success_rate = if total_renders > 0 {
        successful_renders as f64 / total_renders as f64
    } else {
        1.0
    };

    let latencies: Vec<i64> = template_jobs
        .iter()
        .filter_map(|job| job.rendering_latency)
        .collect();

    let avg_render_time_ms = if !latencies.is_empty() {
        latencies.iter().sum::<i64>() as f64 / latencies.len() as f64
    } else {
        0.0
    };

    let fastest_render_ms = latencies.iter().min().copied();
    let slowest_render_ms = latencies.iter().max().copied();

    // Calculate cache hit rate based on data hash duplicates
    let unique_hashes: std::collections::HashSet<_> = template_jobs
        .iter()
        .map(|job| &job.data_hash)
        .collect();
    let cache_hit_rate = if total_renders > 0 {
        1.0 - (unique_hashes.len() as f64 / total_renders as f64)
    } else {
        0.0
    };

    let performance_metrics = crate::models::TemplatePerformanceMetrics {
        total_renders,
        successful_renders,
        failed_renders,
        success_rate,
        avg_render_time_ms,
        fastest_render_ms,
        slowest_render_ms,
        cache_hit_rate,
    };

    // For usage over time, we'd group by time periods
    // This is a simplified version
    let usage_over_time = vec![]; // Would implement time-series grouping

    let analytics = TemplateAnalytics {
        template_id,
        template_name: template_id.to_string(), // Would get from registry
        total_versions: versions.len() as u64,
        latest_version,
        usage_over_time,
        performance_metrics,
    };

    Ok(Json(ApiResponse::new(analytics)))
}

/// Get performance metrics over time
async fn get_performance_metrics(
    State(state): State<AppState>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<ApiResponse<PerformanceMetrics>>> {
    debug!("Getting performance metrics with query: {:?}", query);

    // This would implement time-series analysis
    // For now, return a simplified response
    
    Err(ApiError::internal("Performance metrics not yet implemented"))
}

/// Get system health status
async fn get_system_health(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SystemHealth>>> {
    debug!("Getting system health");

    let queue_depth = calculate_queue_depth(&state).await?;
    
    // Test database connectivity
    let database_connected = match state.registry.list_templates().await {
        Ok(_) => true,
        Err(_) => false,
    };

    // Test S3 connectivity (simplified)
    let s3_connected = true; // Would implement actual health check

    let queue_health = QueueHealth {
        status: if queue_depth > 100 {
            HealthStatus::Warning
        } else if queue_depth > 500 {
            HealthStatus::Critical
        } else {
            HealthStatus::Healthy
        },
        current_depth: queue_depth,
        max_depth_24h: queue_depth, // Would track over time
        processing_rate: 0.0, // Would calculate
        avg_wait_time_ms: 0.0, // Would calculate
    };

    let storage_health = StorageHealth {
        status: if database_connected && s3_connected {
            HealthStatus::Healthy
        } else {
            HealthStatus::Critical
        },
        database_connected,
        s3_connected,
        database_response_time_ms: None, // Would measure
        s3_response_time_ms: None, // Would measure
    };

    let overall_status = match (&queue_health.status, &storage_health.status) {
        (HealthStatus::Healthy, HealthStatus::Healthy) => HealthStatus::Healthy,
        (HealthStatus::Critical, _) | (_, HealthStatus::Critical) => HealthStatus::Critical,
        _ => HealthStatus::Warning,
    };

    let health = SystemHealth {
        status: overall_status,
        uptime_seconds: 0, // Would track server start time
        queue_health,
        storage_health,
        last_updated: OffsetDateTime::now_utc(),
    };

    Ok(Json(ApiResponse::new(health)))
}

/// Helper function to calculate current queue depth
async fn calculate_queue_depth(state: &AppState) -> Result<i64> {
    let jobs = state.registry.list_render_jobs().await?;
    let pending_jobs = jobs
        .iter()
        .filter(|job| job.completed_at.is_none())
        .count();
    Ok(pending_jobs as i64)
}