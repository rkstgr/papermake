//! Papermake HTTP API Server
//!
//! Provides REST API endpoints for template management, PDF rendering,
//! and analytics for the Papermake PDF generation system.

use axum::{Router, extract::DefaultBodyLimit, response::Json, routing::get};
use papermake_registry::{ClickHouseStorage, Registry, S3Storage};
use serde_json::{Value, json};
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};

mod config;
mod error;
mod models;
mod routes;

use config::ServerConfig;
use error::Result;

use crate::models::RenderJob;

/// Main application state
#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<Registry<S3Storage, ClickHouseStorage>>,
    pub config: ServerConfig,
    pub job_sender: tokio::sync::mpsc::UnboundedSender<RenderJob>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "papermake_server=debug,tower_http=debug".to_string()),
        )
        .init();

    // Load configuration
    let config = ServerConfig::from_env()?;
    info!(
        "Starting Papermake Server on {}:{}",
        config.host, config.port
    );

    let s3_storage = S3Storage::from_env().unwrap(); // TODO: improve error handling

    // Ensure S3 bucket exists
    if let Err(e) = s3_storage.ensure_bucket().await {
        error!("Failed to ensure S3 bucket exists: {}", e);
    }

    let clickhouse = ClickHouseStorage::from_env().unwrap();
    if let Err(e) = clickhouse.init_schema().await {
        error!("Failed to initialize ClickHouse schema: {}", e);
    }

    // Create registry
    let registry = Arc::new(Registry::new(s3_storage, clickhouse));

    // Create job channel for event-driven processing
    let (job_sender, _job_receiver) = tokio::sync::mpsc::unbounded_channel();

    // Create application state
    let state = AppState {
        registry,
        config: config.clone(),
        job_sender,
    };

    // Start background render worker
    // worker::spawn_render_worker(state.clone(), job_receiver); TODO: enable
    // info!("🔧 Background render worker started");

    // Build router
    let app = create_router(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("🚀 Server listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

/// Create the main application router
fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))
        // API routes
        .nest("/api", api_routes())
        // Middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
                .layer(DefaultBodyLimit::max(50 * 1024 * 1024)), // 50MB for large PDFs
        )
        .with_state(state)
}

/// API routes
fn api_routes() -> Router<AppState> {
    Router::new()
        .nest("/templates", routes::templates::router())
        .nest("/render", routes::render::router())
        .nest("/renders", routes::renders::router())
        .nest("/analytics", routes::analytics::router())
}

/// Health check endpoint
async fn health_check() -> Result<Json<Value>> {
    Ok(Json(json!({
        "status": "healthy",
        "service": "papermake-server",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": time::OffsetDateTime::now_utc()
    })))
}
