//! S3-compatible storage implementation using MinIO client
//!
//! This module provides an S3-compatible storage implementation of the BlobStorage trait.
//! It works with AWS S3, MinIO, and any S3-compatible object storage.

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::StreamExt;
use minio::s3::{
    client::Client,
    creds::StaticProvider,
    http::BaseUrl,
    segmented_bytes::SegmentedBytes,
    types::{S3Api, ToStream},
};
use std::str::FromStr;

use crate::{BlobStorage, storage::blob_storage::StorageError};

/// S3-compatible storage implementation using MinIO client
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
    /// - S3_REGION (optional)
    pub async fn from_env() -> Result<Self, StorageError> {
        let bucket = std::env::var("S3_BUCKET").map_err(|_| {
            StorageError::Backend("S3_BUCKET environment variable not set".to_string())
        })?;

        let access_key = std::env::var("S3_ACCESS_KEY_ID").map_err(|_| {
            StorageError::Backend("S3_ACCESS_KEY_ID environment variable not set".to_string())
        })?;

        let secret_key = std::env::var("S3_SECRET_ACCESS_KEY").map_err(|_| {
            StorageError::Backend("S3_SECRET_ACCESS_KEY environment variable not set".to_string())
        })?;

        let endpoint_url = std::env::var("S3_ENDPOINT_URL").map_err(|_| {
            StorageError::Backend("S3_ENDPOINT_URL environment variable not set".to_string())
        })?;

        // Create base URL for endpoint
        let base_url = BaseUrl::from_str(&endpoint_url)
            .map_err(|e| StorageError::Backend(format!("Invalid S3_ENDPOINT_URL: {}", e)))?;

        // Create credentials provider
        let creds_provider = StaticProvider::new(&access_key, &secret_key, None);

        // Create client
        let client = Client::new(
            base_url,
            Some(Box::new(creds_provider)),
            None, // Default region
            None, // No custom HTTP client
        )
        .map_err(|e| StorageError::Backend(format!("Failed to create S3 client: {}", e)))?;

        Ok(Self::new(client, bucket))
    }

    /// Ensure bucket exists (create if it doesn't)
    pub async fn ensure_bucket(&self) -> Result<(), StorageError> {
        // Check if bucket exists
        match self.client.bucket_exists(&self.bucket).send().await {
            Ok(response) => {
                if response.exists {
                    Ok(()) // Bucket exists
                } else {
                    // Bucket doesn't exist, try to create it
                    match self.client.create_bucket(&self.bucket).send().await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(StorageError::Backend(format!(
                            "Failed to create bucket '{}': {}",
                            self.bucket, e
                        ))),
                    }
                }
            }
            Err(e) => Err(StorageError::Backend(format!(
                "Failed to check bucket '{}': {}",
                self.bucket, e
            ))),
        }
    }

    /// Validate S3 key format
    fn validate_key(&self, key: &str) -> Result<(), StorageError> {
        if key.is_empty() || key.len() > 1024 {
            return Err(StorageError::InvalidKey(
                "Key must be between 1 and 1024 characters".into(),
            ));
        }

        if key.starts_with('/') || key.ends_with('/') {
            return Err(StorageError::InvalidKey(
                "Key cannot start or end with '/'".into(),
            ));
        }

        Ok(())
    }

    /// List files with a given prefix
    pub async fn list_files(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
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
                Ok(response) => {
                    // Collect keys from the response
                    for entry in response.contents {
                        keys.push(entry.name);
                    }
                }
                Err(e) => {
                    return Err(StorageError::Backend(format!(
                        "Failed to list files with prefix '{}': {}",
                        prefix, e
                    )));
                }
            }
        }

        Ok(keys)
    }
}

#[async_trait]
impl BlobStorage for S3Storage {
    async fn put(&self, key: &str, data: Vec<u8>) -> Result<(), StorageError> {
        self.validate_key(key)?;

        let bytes = SegmentedBytes::from(Bytes::from(data));

        self.client
            .put_object(&self.bucket, key, bytes)
            .send()
            .await
            .map_err(|e| StorageError::Backend(format!("Failed to put file '{}': {}", key, e)))?;

        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        self.validate_key(key)?;

        let response = self
            .client
            .get_object(&self.bucket, key)
            .send()
            .await
            .map_err(|e| {
                // Check if it's a not found error
                if e.to_string().contains("NoSuchKey") || e.to_string().contains("404") {
                    StorageError::NotFound(key.to_string())
                } else {
                    StorageError::Backend(format!("Failed to get file '{}': {}", key, e))
                }
            })?;

        let content = response.content.to_segmented_bytes().await.map_err(|e| {
            StorageError::Backend(format!("Failed to read file '{}' content: {}", key, e))
        })?;

        Ok(content.to_bytes().to_vec())
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        self.validate_key(key)?;

        match self.client.stat_object(&self.bucket, key).send().await {
            Ok(_) => Ok(true),
            Err(e) => {
                // Check if it's a not found error
                if e.to_string().contains("NoSuchKey") || e.to_string().contains("404") {
                    Ok(false)
                } else {
                    Err(StorageError::Backend(format!(
                        "Failed to check existence of file '{}': {}",
                        key, e
                    )))
                }
            }
        }
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.validate_key(key)?;

        self.client
            .delete_object(&self.bucket, key)
            .send()
            .await
            .map_err(|e| {
                StorageError::Backend(format!("Failed to delete file '{}': {}", key, e))
            })?;

        Ok(())
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        self.list_files(prefix).await
    }
}

/// Utility functions for generating S3 keys for different types of content
impl S3Storage {
    /// Generate key for content-addressable blob storage
    pub fn blob_key(hash: &str) -> String {
        format!("blobs/sha256/{}", hash)
    }

    /// Generate key for manifest storage
    pub fn manifest_key(hash: &str) -> String {
        format!("manifests/sha256/{}", hash)
    }

    /// Generate key for mutable reference storage
    pub fn ref_key(namespace: &str, tag: &str) -> String {
        format!("refs/{}/{}", namespace, tag)
    }

    /// Generate key prefix for listing templates in a namespace
    pub fn namespace_prefix(namespace: &str) -> String {
        format!("refs/{}/", namespace)
    }

    /// Generate key prefix for listing all references
    pub fn refs_prefix() -> String {
        "refs/".to_string()
    }

    /// Generate key prefix for listing all blobs
    pub fn blobs_prefix() -> String {
        "blobs/".to_string()
    }

    /// Generate key prefix for listing all manifests
    pub fn manifests_prefix() -> String {
        "manifests/".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        assert_eq!(S3Storage::blob_key("abc123"), "blobs/sha256/abc123");

        assert_eq!(S3Storage::manifest_key("def456"), "manifests/sha256/def456");

        assert_eq!(
            S3Storage::ref_key("john/invoice", "latest"),
            "refs/john/invoice/latest"
        );

        assert_eq!(
            S3Storage::ref_key("invoice", "v1.0.0"),
            "refs/invoice/v1.0.0"
        );
    }

    #[test]
    fn test_prefix_generation() {
        assert_eq!(
            S3Storage::namespace_prefix("john/invoice"),
            "refs/john/invoice/"
        );

        assert_eq!(S3Storage::refs_prefix(), "refs/");
        assert_eq!(S3Storage::blobs_prefix(), "blobs/");
        assert_eq!(S3Storage::manifests_prefix(), "manifests/");
    }

    #[test]
    fn test_key_validation() {
        let client = minio::s3::client::Client::new(
            BaseUrl::from_str("http://localhost:9000").unwrap(),
            None,
            None,
            None,
        )
        .unwrap();
        let storage = S3Storage::new(client, "test-bucket");

        // Valid keys
        assert!(storage.validate_key("valid/key.txt").is_ok());
        assert!(storage.validate_key("blobs/sha256/abc123").is_ok());
        assert!(storage.validate_key("refs/john/invoice/latest").is_ok());

        // Invalid keys
        assert!(storage.validate_key("").is_err());
        assert!(storage.validate_key("/starts-with-slash").is_err());
        assert!(storage.validate_key("ends-with-slash/").is_err());
        assert!(storage.validate_key(&"x".repeat(1025)).is_err());
    }

    #[tokio::test]
    async fn test_s3_storage_from_env_missing_vars() {
        // Clear environment variables to test error handling
        unsafe {
            std::env::remove_var("S3_BUCKET");
            std::env::remove_var("S3_ACCESS_KEY_ID");
            std::env::remove_var("S3_SECRET_ACCESS_KEY");
            std::env::remove_var("S3_ENDPOINT_URL");
        }

        let result = S3Storage::from_env().await;
        assert!(result.is_err());

        match result {
            Err(StorageError::Backend(msg)) => {
                assert!(msg.contains("S3_BUCKET"));
            }
            _ => panic!("Expected Backend error for missing S3_BUCKET"),
        }
    }
}
