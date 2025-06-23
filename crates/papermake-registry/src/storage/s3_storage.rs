//! S3 file storage implementation
//!
//! This module provides an S3-compatible storage implementation of the FileStorage trait.
//! It works with AWS S3, MinIO, JuiceFS, and any S3-compatible object storage.

use crate::{error::Result, RegistryError};
use super::FileStorage;
use async_trait::async_trait;
use aws_sdk_s3::{Client, Config};
use aws_sdk_s3::primitives::ByteStream;
//use aws_sdk_s3::operation::get_object::GetObjectError;
//use aws_sdk_s3::operation::put_object::PutObjectError;
//use aws_sdk_s3::operation::delete_object::DeleteObjectError;
//use aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Error;

/// S3-compatible file storage implementation
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
    /// - AWS_REGION or AWS_DEFAULT_REGION
    /// - AWS_ACCESS_KEY_ID 
    /// - AWS_SECRET_ACCESS_KEY
    /// - AWS_ENDPOINT_URL (optional, for S3-compatible services)
    /// - S3_BUCKET
    pub async fn from_env() -> Result<Self> {
        let bucket = std::env::var("S3_BUCKET")
            .map_err(|_| RegistryError::Storage("S3_BUCKET environment variable not set".to_string()))?;
        
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let s3_config = aws_sdk_s3::config::Builder::from(&config);
        
        // Check for custom endpoint (for MinIO, JuiceFS, etc.)
        let s3_config = if let Ok(endpoint) = std::env::var("AWS_ENDPOINT_URL") {
            s3_config.endpoint_url(&endpoint)
        } else {
            s3_config
        };
        
        let client = Client::from_conf(s3_config.build());
        
        Ok(Self::new(client, bucket))
    }
    
    /// Create S3 storage with custom configuration
    pub fn with_config(config: Config, bucket: impl Into<String>) -> Self {
        let client = Client::from_conf(config);
        Self::new(client, bucket)
    }
    
    /// Ensure bucket exists (create if it doesn't)
    pub async fn ensure_bucket(&self) -> Result<()> {
        // Try to head the bucket first
        match self.client.head_bucket().bucket(&self.bucket).send().await {
            Ok(_) => return Ok(()), // Bucket exists
            Err(_) => {
                // Bucket doesn't exist or we don't have access, try to create it
                match self.client.create_bucket().bucket(&self.bucket).send().await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(RegistryError::Storage(format!("Failed to create bucket '{}': {}", self.bucket, e))),
                }
            }
        }
    }
}

#[async_trait]
impl FileStorage for S3Storage {
    async fn put_file(&self, key: &str, content: &[u8]) -> Result<()> {
        let body = ByteStream::from(content.to_vec());
        
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .send()
            .await
            .map_err(|e| RegistryError::Storage(format!("Failed to put file '{}': {}", key, e)))?;
        
        Ok(())
    }
    
    async fn get_file(&self, key: &str) -> Result<Vec<u8>> {
        let response = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| RegistryError::Storage(format!("Failed to get file '{}': {}", key, e)))?;
        
        let body = response.body.collect().await
            .map_err(|e| RegistryError::Storage(format!("Failed to read file '{}' body: {}", key, e)))?;
        
        Ok(body.into_bytes().to_vec())
    }
    
    async fn file_exists(&self, key: &str) -> Result<bool> {
        match self.client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false), // Simplified error handling
        }
    }
    
    async fn delete_file(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| RegistryError::Storage(format!("Failed to delete file '{}': {}", key, e)))?;
        
        Ok(())
    }
    
    async fn list_files(&self, prefix: &str) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        let mut continuation_token: Option<String> = None;
        
        loop {
            let mut request = self.client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(prefix);
            
            if let Some(token) = continuation_token {
                request = request.continuation_token(token);
            }
            
            let response = request.send().await
                .map_err(|e| RegistryError::Storage(format!("Failed to list files with prefix '{}': {}", prefix, e)))?;
            
            if let Some(contents) = response.contents {
                for object in contents {
                    if let Some(key) = object.key {
                        keys.push(key);
                    }
                }
            }
            
            // Check if there are more objects to fetch
            if response.is_truncated.unwrap_or(false) {
                continuation_token = response.next_continuation_token;
            } else {
                break;
            }
        }
        
        Ok(keys)
    }
}

impl S3Storage {
    /// Generate S3 key for template source file
    pub fn template_source_key(template_id: &str, version: u64) -> String {
        format!("templates/{}/versions/{}/source.typ", template_id, version)
    }
    
    /// Generate S3 key for template asset
    pub fn template_asset_key(template_id: &str, asset_path: &str) -> String {
        format!("templates/{}/assets/{}", template_id, asset_path)
    }
    
    /// Generate S3 key for rendered PDF
    pub fn render_pdf_key(job_id: &str) -> String {
        format!("renders/{}.pdf", job_id)
    }
    
    /// Generate S3 key prefix for template files
    pub fn template_prefix(template_id: &str) -> String {
        format!("templates/{}/", template_id)
    }
    
    /// Generate S3 key prefix for all renders
    pub fn renders_prefix() -> String {
        "renders/".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_key_generation() {
        assert_eq!(
            S3Storage::template_source_key("my-template", 1),
            "templates/my-template/versions/1/source.typ"
        );
        
        assert_eq!(
            S3Storage::template_asset_key("my-template", "fonts/Arial.ttf"),
            "templates/my-template/assets/fonts/Arial.ttf"
        );
        
        assert_eq!(
            S3Storage::render_pdf_key("job-123"),
            "renders/job-123.pdf"
        );
        
        assert_eq!(
            S3Storage::template_prefix("my-template"),
            "templates/my-template/"
        );
    }
}