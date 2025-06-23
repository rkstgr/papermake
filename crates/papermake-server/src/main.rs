//! Papermake HTTP API Server
//!
//! Provides REST API endpoints for template management, PDF rendering,
//! and analytics for the Papermake PDF generation system.

use axum::{
    extract::DefaultBodyLimit,
    response::Json,
    routing::get,
    Router,
};
use papermake_registry::{
    storage::{sqlite_storage::SqliteStorage, s3_storage::S3Storage},
    DefaultRegistry,
};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};

mod config;
mod error;
mod models;
mod routes;
mod services;
mod worker;

use config::ServerConfig;
use error::Result;

/// Main application state
#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<DefaultRegistry>,
    pub config: ServerConfig,
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
    info!("Starting Papermake Server on {}:{}", config.host, config.port);

    // Initialize storage backends
    let sqlite_storage = Arc::new(SqliteStorage::from_env().await?);
    let s3_storage = Arc::new(S3Storage::from_env().await?);

    // Ensure S3 bucket exists
    if let Err(e) = s3_storage.ensure_bucket().await {
        warn!("Failed to ensure S3 bucket exists: {}", e);
    }

    // Create registry
    let registry = Arc::new(DefaultRegistry::new(sqlite_storage, s3_storage));

    // Create application state
    let state = AppState { registry, config: config.clone() };

    // Start background render worker
    worker::spawn_render_worker(state.clone());
    info!("🔧 Background render worker started");

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
        // Static file serving (for frontend assets)
        .nest("/static", static_routes())
        // Middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
                .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // 50MB for large PDFs
        )
        .with_state(state)
}

/// API routes
fn api_routes() -> Router<AppState> {
    Router::new()
        .nest("/templates", routes::templates::router())
        .nest("/renders", routes::renders::router())
        .nest("/analytics", routes::analytics::router())
        // WebSocket endpoint
        .route("/ws/renders/:render_id", get(routes::websocket::render_status_ws))
}

/// Static file routes
fn static_routes() -> Router<AppState> {
    Router::new()
        // Serve static files from a directory
        .fallback_service(tower_http::services::ServeDir::new("static"))
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