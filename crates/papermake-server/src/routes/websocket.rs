//! WebSocket routes for real-time updates

use crate::{
    error::{ApiError, Result},
    models::RenderJobUpdate,
    AppState,
};
use papermake_registry::TemplateRegistry;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::time::{Duration, interval};
use tracing::{debug, error, info, warn};

/// WebSocket endpoint for render job status updates
pub async fn render_status_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(render_id): Path<String>,
) -> Response {
    info!("WebSocket connection requested for render job: {}", render_id);
    
    ws.on_upgrade(move |socket| handle_render_status_ws(socket, state, render_id))
}

/// Handle WebSocket connection for render job status
async fn handle_render_status_ws(socket: WebSocket, state: AppState, render_id: String) {
    debug!("WebSocket connected for render job: {}", render_id);

    let (mut sender, mut receiver) = socket.split();
    
    // Check if render job exists first
    match state.registry.get_render_job(&render_id).await {
        Ok(job) => {
            // Send initial status
            let initial_status = create_render_update(&job);
            if let Err(e) = send_update(&mut sender, &initial_status).await {
                error!("Failed to send initial status: {}", e);
                return;
            }

            // If job is already completed, just send the status and close
            if job.completed_at.is_some() {
                info!("Render job {} already completed, closing WebSocket", render_id);
                let _ = sender.close().await;
                return;
            }
        }
        Err(e) => {
            error!("Render job {} not found: {}", render_id, e);
            let error_msg = serde_json::json!({
                "error": "Render job not found",
                "job_id": render_id
            });
            let _ = sender.send(Message::Text(error_msg.to_string().into())).await;
            let _ = sender.close().await;
            return;
        }
    }

    // Set up periodic status checking
    let mut status_interval = interval(Duration::from_secs(2));
    let mut last_status = None;

    loop {
        tokio::select! {
            // Handle incoming messages from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        debug!("Received WebSocket message: {}", text);
                        // Could handle client commands like "ping", "get_status", etc.
                        if text == "ping" {
                            if let Err(e) = sender.send(Message::Text("pong".to_string().into())).await {
                                error!("Failed to send pong: {}", e);
                                break;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket closed by client for render job: {}", render_id);
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error for render job {}: {}", render_id, e);
                        break;
                    }
                    None => {
                        debug!("WebSocket stream ended for render job: {}", render_id);
                        break;
                    }
                    _ => {
                        // Ignore other message types (binary, ping, pong)
                    }
                }
            }
            
            // Periodic status check
            _ = status_interval.tick() => {
                match state.registry.get_render_job(&render_id).await {
                    Ok(job) => {
                        let current_status = create_render_update(&job);
                        
                        // Only send update if status changed
                        if last_status.as_ref() != Some(&current_status) {
                            if let Err(e) = send_update(&mut sender, &current_status).await {
                                error!("Failed to send status update: {}", e);
                                break;
                            }
                            last_status = Some(current_status.clone());
                        }
                        
                        // Close connection if job is completed
                        if job.completed_at.is_some() {
                            info!("Render job {} completed, closing WebSocket", render_id);
                            let _ = sender.close().await;
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to get render job status: {}", e);
                        let error_msg = serde_json::json!({
                            "error": "Failed to get job status",
                            "job_id": render_id
                        });
                        let _ = sender.send(Message::Text(error_msg.to_string().into())).await;
                        break;
                    }
                }
            }
        }
    }

    debug!("WebSocket connection closed for render job: {}", render_id);
}

/// Create a render job update from a render job
fn create_render_update(job: &papermake_registry::entities::RenderJob) -> RenderJobUpdate {
    let status = if job.completed_at.is_some() {
        if job.pdf_s3_key.is_some() {
            crate::models::RenderStatus::Completed
        } else {
            crate::models::RenderStatus::Failed
        }
    } else {
        crate::models::RenderStatus::Processing
    };

    let progress = if job.completed_at.is_some() {
        Some(1.0)
    } else {
        // Could calculate progress based on various factors
        // For now, just use a simple heuristic
        Some(0.5) // Always 50% for processing jobs
    };

    let message = match status {
        crate::models::RenderStatus::Processing => Some("Rendering PDF...".to_string()),
        crate::models::RenderStatus::Completed => Some("PDF rendered successfully".to_string()),
        crate::models::RenderStatus::Failed => Some("Rendering failed".to_string()),
        _ => None,
    };

    RenderJobUpdate {
        job_id: job.id.clone(),
        status,
        progress,
        message,
        completed_at: job.completed_at,
        pdf_url: job.pdf_s3_key.as_ref().map(|_| format!("/api/renders/{}/pdf", job.id)),
    }
}

/// Send a render job update via WebSocket
async fn send_update(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    update: &RenderJobUpdate,
) -> Result<()> {
    let message_text = serde_json::to_string(update)
        .map_err(|e| ApiError::Serialization(e))?;
    
    sender
        .send(Message::Text(message_text.into()))
        .await
        .map_err(|e| ApiError::internal(&format!("WebSocket send failed: {}", e)))?;
    
    Ok(())
}