//! Background worker for processing render jobs

/*
 * Legacy, needs to be updated
 */

use crate::{AppState, error::Result};
use tokio::time::Instant;
use tracing::{debug, error, info};

/// Helper function to parse version string to u64 for legacy compatibility
fn parse_version_to_u64(version: &str) -> u64 {
    if let Some(v) = version.strip_prefix('v') {
        v.parse().unwrap_or(1)
    } else {
        version.parse().unwrap_or(1)
    }
}

/// Background worker that processes pending render jobs
pub struct RenderWorker {
    state: AppState,
    job_receiver: tokio::sync::mpsc::UnboundedReceiver<_>,
}

impl RenderWorker {
    /// Create a new render worker
    pub fn new(state: AppState, job_receiver: tokio::sync::mpsc::UnboundedReceiver<_>) -> Self {
        Self {
            state,
            job_receiver,
        }
    }

    /// Start the worker loop
    pub async fn start(mut self) {
        info!("Starting event-driven render worker");

        loop {
            // Wait for job from channel
            match self.job_receiver.recv().await {
                Some(job) => {
                    if let Err(e) = self.process_render_job(job).await {
                        error!("Error processing render job: {}", e);
                    }
                }
                None => {
                    info!("Job channel closed, shutting down worker");
                    break;
                }
            }
        }
    }

    /// Process a single render job
    async fn process_render_job(&self, mut job: RenderJob) -> Result<()> {
        let start_time = Instant::now();

        // Mark job as in progress
        job.start();
        self.state.registry.update_render_job(&job).await?;

        // Get the template for this job
        info!(
            "Processing render job {} for template {}",
            job.id, job.template_ref
        );
        let template = match self
            .state
            .registry
            .get_template(&job.template_ref.to_string())
            .await
        {
            Ok(template_entry) => template_entry.template,
            Err(e) => {
                let error_msg = format!("Failed to get template {} {}", job.template_ref, e);
                error!("Render job {} failed: {}", job.id, error_msg);
                job.fail(error_msg);
                // Save failed job status
                if let Err(save_err) = self.state.registry.update_render_job(&job).await {
                    error!(
                        "Failed to save job failure status for {}: {}",
                        job.id, save_err
                    );
                }
                return Ok(());
            }
        };

        // Render the PDF
        debug!("Rendering PDF for job {} with data: {:?}", job.id, job.data);
        match self.render_pdf(&template, &job.data).await {
            Ok(pdf_data) => {
                info!(
                    "Successfully rendered PDF for job {} ({} bytes)",
                    job.id,
                    pdf_data.len()
                );
                // Generate S3 key for the PDF
                let s3_key = format!("renders/{}/{}.pdf", job.template_ref, job.id);

                // Store PDF in file storage
                match self.store_pdf(&s3_key, pdf_data).await {
                    Ok(_) => {
                        let latency_ms = start_time.elapsed().as_millis() as u64;
                        job.complete(s3_key, latency_ms);
                        info!(
                            "Successfully completed render job {} in {}ms",
                            job.id, latency_ms
                        );
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to store PDF: {}", e);
                        error!(
                            "Render job {} failed during S3 upload: {}",
                            job.id, error_msg
                        );
                        job.fail(error_msg);
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("PDF rendering failed: {}", e);
                error!("Render job {} failed: {}", job.id, error_msg);
                job.fail(error_msg);
            }
        }

        // Save final job status
        if let Err(e) = self.state.registry.update_render_job(&job).await {
            error!("Failed to save final job status: {}", e);
        }

        Ok(())
    }

    /// Render PDF using papermake
    async fn render_pdf(
        &self,
        template: &papermake::Template,
        data: &serde_json::Value,
    ) -> Result<Vec<u8>> {
        // Validate data against template schema
        template.validate_data(data)?;

        // Render the PDF
        let render_result = template.render(data)?;

        // Extract PDF bytes
        match render_result.pdf {
            Some(pdf_bytes) => Ok(pdf_bytes),
            None => {
                let error_details = render_result
                    .errors
                    .into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");

                Err(crate::error::ApiError::RenderFailed(format!(
                    "PDF generation failed: {}",
                    error_details
                ))
                .into())
            }
        }
    }

    /// Store PDF in file storage
    async fn store_pdf(&self, s3_key: &str, pdf_data: Vec<u8>) -> Result<()> {
        todo!("Implement with new registry")
    }
}

/// Spawn the render worker in the background
pub fn spawn_render_worker(state: AppState, job_receiver: tokio::sync::mpsc::UnboundedReceiver<_>) {
    tokio::spawn(async move {
        let worker = RenderWorker::new(state, job_receiver);
        worker.start().await;
    });
}
