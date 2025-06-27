//! S3 file storage implementation
//!
//! This module provides an S3-compatible storage implementation of the FileStorage trait.
//! It works with AWS S3, MinIO, JuiceFS, and any S3-compatible object storage.

use super::FileStorage;
use crate::{RegistryError, error::Result, template_ref::TemplateRef};
use async_trait::async_trait;
use futures_util::StreamExt;
use minio::s3::{
    client::Client,
    creds::StaticProvider,
    http::BaseUrl,
    segmented_bytes::SegmentedBytes,
    types::{S3Api, ToStream},
};
use std::str::FromStr;

/// S3-compatible file storage implementation using MinIO client
pub struct S3Storage {
    client: Client,
    bucket: String,
}

impl S3Storage {
    /// Create a new S3 storage instance
    pub fn new(client: Client, bucket: impl Into<String>) -> Self {
        Self {
            client,
            bucket: bucket.into(),
        }
    }

    /// Create S3 storage from environment variables
    ///
    /// Expects:
    /// - S3_ACCESS_KEY_ID
    /// - S3_SECRET_ACCESS_KEY
    /// - S3_ENDPOINT_URL (for S3-compatible services like MinIO)
    /// - S3_BUCKET
    /// - S3_REGION (optional, defaults to us-east-1)
    pub async fn from_env() -> Result<Self> {
        let bucket = std::env::var("S3_BUCKET").map_err(|_| {
            RegistryError::Storage("S3_BUCKET environment variable not set".to_string())
        })?;

        let access_key = std::env::var("S3_ACCESS_KEY_ID").map_err(|_| {
            RegistryError::Storage("S3_ACCESS_KEY_ID environment variable not set".to_string())
        })?;

        let secret_key = std::env::var("S3_SECRET_ACCESS_KEY").map_err(|_| {
            RegistryError::Storage("S3_SECRET_ACCESS_KEY environment variable not set".to_string())
        })?;

        let endpoint_url = std::env::var("S3_ENDPOINT_URL").map_err(|_| {
            RegistryError::Storage("S3_ENDPOINT_URL environment variable not set".to_string())
        })?;

        // Create base URL for endpoint
        let base_url = BaseUrl::from_str(&endpoint_url)
            .map_err(|e| RegistryError::Storage(format!("Invalid S3_ENDPOINT_URL: {}", e)))?;

        // Create credentials provider
        let creds_provider = StaticProvider::new(&access_key, &secret_key, None);

        // Create client
        let client = Client::new(
            base_url,
            Some(Box::new(creds_provider)),
            None, // Default region
            None, // No custom HTTP client
        )
        .map_err(|e| RegistryError::Storage(format!("Failed to create S3 client: {}", e)))?;

        Ok(Self::new(client, bucket))
    }

    /// Ensure bucket exists (create if it doesn't)
    pub async fn ensure_bucket(&self) -> Result<()> {
        // Check if bucket exists
        match self.client.bucket_exists(&self.bucket).send().await {
            Ok(response) => {
                if response.exists {
                    Ok(()) // Bucket exists
                } else {
                    // Bucket doesn't exist, try to create it
                    match self.client.create_bucket(&self.bucket).send().await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(RegistryError::Storage(format!(
                            "Failed to create bucket '{}': {}",
                            self.bucket, e
                        ))),
                    }
                }
            }
            Err(e) => Err(RegistryError::Storage(format!(
                "Failed to check bucket '{}': {}",
                self.bucket, e
            ))),
        }
    }
}

#[async_trait]
impl FileStorage for S3Storage {
    async fn put_file(&self, key: &str, content: &[u8]) -> Result<()> {
        let bytes = bytes::Bytes::from(content.to_vec());
        let segmented_bytes = SegmentedBytes::from(bytes);

        self.client
            .put_object(&self.bucket, key, segmented_bytes)
            .send()
            .await
            .map_err(|e| RegistryError::Storage(format!("Failed to put file '{}': {}", key, e)))?;

        Ok(())
    }

    async fn get_file(&self, key: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get_object(&self.bucket, key)
            .send()
            .await
            .map_err(|e| RegistryError::Storage(format!("Failed to get file '{}': {}", key, e)))?;

        let content = response.content.to_segmented_bytes().await.map_err(|e| {
            RegistryError::Storage(format!("Failed to read file '{}' content: {}", key, e))
        })?;

        Ok(content.to_bytes().to_vec())
    }

    async fn file_exists(&self, key: &str) -> Result<bool> {
        match self.client.stat_object(&self.bucket, key).send().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false), // Simplified error handling
        }
    }

    async fn delete_file(&self, key: &str) -> Result<()> {
        self.client
            .delete_object(&self.bucket, key)
            .send()
            .await
            .map_err(|e| {
                RegistryError::Storage(format!("Failed to delete file '{}': {}", key, e))
            })?;

        Ok(())
    }

    async fn list_files(&self, prefix: &str) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        let mut stream = self
            .client
            .list_objects(&self.bucket)
            .prefix(Some(prefix.to_string()))
            .recursive(true)
            .to_stream()
            .await;

        while let Some(result) = stream.next().await {
            match result {
                Ok(item) => {
                    keys.push(item.name);
                }
                Err(e) => {
                    return Err(RegistryError::Storage(format!(
                        "Failed to list files with prefix '{}': {}",
                        prefix, e
                    )));
                }
            }
        }

        Ok(keys)
    }
}

impl S3Storage {
    /// Generate S3 key for template source file using Docker-style TemplateRef
    pub fn template_source_key(template_ref: &TemplateRef) -> String {
        match &template_ref.org {
            Some(org) => format!("templates/{}/{}/{}/source.typ", org, template_ref.name, template_ref.tag),
            None => format!("templates/{}/{}/source.typ", template_ref.name, template_ref.tag),
        }
    }

    /// Generate S3 key for template asset using Docker-style TemplateRef
    /// Assets are stored without version to enable reuse across template versions
    pub fn template_asset_key(template_ref: &TemplateRef, asset_path: &str) -> String {
        match &template_ref.org {
            Some(org) => format!("templates/{}/{}/assets/{}", org, template_ref.name, asset_path),
            None => format!("templates/{}/assets/{}", template_ref.name, asset_path),
        }
    }

    /// Generate S3 key for rendered PDF
    pub fn render_pdf_key(job_id: &str) -> String {
        format!("renders/{}.pdf", job_id)
    }

    /// Generate S3 key prefix for template files using Docker-style TemplateRef
    pub fn template_prefix(template_ref: &TemplateRef) -> String {
        match &template_ref.org {
            Some(org) => format!("templates/{}/{}/", org, template_ref.name),
            None => format!("templates/{}/", template_ref.name),
        }
    }

    /// Generate S3 key prefix for all renders
    pub fn renders_prefix() -> String {
        "renders/".to_string()
    }

    /// Generate S3 key prefix for organization templates
    pub fn org_templates_prefix(org: &str) -> String {
        format!("templates/{}/", org)
    }

    /// Generate S3 key prefix for all templates (no org)
    pub fn all_templates_prefix() -> String {
        "templates/".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation_without_org() {
        let template_ref = TemplateRef::new("my-template").with_tag("v1");

        assert_eq!(
            S3Storage::template_source_key(&template_ref),
            "templates/my-template/v1/source.typ"
        );

        assert_eq!(
            S3Storage::template_asset_key(&template_ref, "fonts/Arial.ttf"),
            "templates/my-template/assets/fonts/Arial.ttf"
        );

        assert_eq!(
            S3Storage::template_prefix(&template_ref),
            "templates/my-template/"
        );
    }

    #[test]
    fn test_key_generation_with_org() {
        let template_ref = TemplateRef::with_org("mycompany", "invoice").with_tag("v2");

        assert_eq!(
            S3Storage::template_source_key(&template_ref),
            "templates/mycompany/invoice/v2/source.typ"
        );

        assert_eq!(
            S3Storage::template_asset_key(&template_ref, "fonts/Arial.ttf"),
            "templates/mycompany/invoice/assets/fonts/Arial.ttf"
        );

        assert_eq!(
            S3Storage::template_prefix(&template_ref),
            "templates/mycompany/invoice/"
        );
    }

    #[test]
    fn test_render_and_prefix_keys() {
        assert_eq!(S3Storage::render_pdf_key("job-123"), "renders/job-123.pdf");
        assert_eq!(S3Storage::renders_prefix(), "renders/");
        assert_eq!(S3Storage::org_templates_prefix("mycompany"), "templates/mycompany/");
        assert_eq!(S3Storage::all_templates_prefix(), "templates/");
    }

    #[test]
    fn test_latest_tag() {
        let template_ref = TemplateRef::new("my-template"); // defaults to "latest"

        assert_eq!(
            S3Storage::template_source_key(&template_ref),
            "templates/my-template/latest/source.typ"
        );
    }

    #[test]
    fn test_draft_tag() {
        let template_ref = TemplateRef::new("my-template").with_tag("draft");

        assert_eq!(
            S3Storage::template_source_key(&template_ref),
            "templates/my-template/draft/source.typ"
        );
    }
}
