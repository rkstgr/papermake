//! Blob storage abstraction for the registry
//!
//! This module provides the core storage trait and in-memory implementation
//! for testing and development.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Key not found: {0}")]
    NotFound(String),

    #[error("Access denied: {0}")]
    AccessDenied(String),

    #[error("Storage backend error: {0}")]
    Backend(String),

    #[error("Invalid key format: {0}")]
    InvalidKey(String),
}

/// Abstraction for blob storage backends
#[async_trait]
pub trait BlobStorage: Send + Sync {
    /// Store data at the given key
    async fn put(&self, key: &str, data: Vec<u8>) -> Result<(), StorageError>;

    /// Retrieve data by key
    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError>;

    /// Check if key exists
    async fn exists(&self, key: &str) -> Result<bool, StorageError>;

    /// Delete data by key
    async fn delete(&self, key: &str) -> Result<(), StorageError>;

    /// List all keys with the given prefix
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, StorageError>;
}

/// In-memory storage implementation for testing
#[derive(Debug, Default)]
pub struct MemoryStorage {
    data: Mutex<HashMap<String, Vec<u8>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }

    /// Get all stored keys (useful for testing)
    pub fn keys(&self) -> Vec<String> {
        self.data.lock().unwrap().keys().cloned().collect()
    }

    /// Clear all data (useful for testing)
    pub fn clear(&self) {
        self.data.lock().unwrap().clear();
    }

    /// Get number of stored items
    pub fn len(&self) -> usize {
        self.data.lock().unwrap().len()
    }

    /// Check if storage is empty
    pub fn is_empty(&self) -> bool {
        self.data.lock().unwrap().is_empty()
    }
}

#[async_trait]
impl BlobStorage for MemoryStorage {
    async fn put(&self, key: &str, data: Vec<u8>) -> Result<(), StorageError> {
        let mut storage = self
            .data
            .lock()
            .map_err(|_| StorageError::Backend("Lock poisoned".into()))?;

        storage.insert(key.to_string(), data);
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let storage = self
            .data
            .lock()
            .map_err(|_| StorageError::Backend("Lock poisoned".into()))?;

        storage
            .get(key)
            .cloned()
            .ok_or_else(|| StorageError::NotFound(key.to_string()))
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        let storage = self
            .data
            .lock()
            .map_err(|_| StorageError::Backend("Lock poisoned".into()))?;

        Ok(storage.contains_key(key))
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let mut storage = self
            .data
            .lock()
            .map_err(|_| StorageError::Backend("Lock poisoned".into()))?;

        storage.remove(key);
        Ok(())
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        let storage = self
            .data
            .lock()
            .map_err(|_| StorageError::Backend("Lock poisoned".into()))?;

        let mut keys: Vec<String> = storage
            .keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect();

        keys.sort();
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_storage_basic_operations() {
        let storage = MemoryStorage::new();
        let key = "test/file.txt";
        let data = b"Hello, World!".to_vec();

        // Test put and get
        storage.put(key, data.clone()).await.unwrap();
        let retrieved = storage.get(key).await.unwrap();
        assert_eq!(data, retrieved);

        // Test exists
        assert!(storage.exists(key).await.unwrap());
        assert!(!storage.exists("nonexistent").await.unwrap());

        // Test delete
        storage.delete(key).await.unwrap();
        assert!(!storage.exists(key).await.unwrap());
        assert!(storage.get(key).await.is_err());
    }

    #[tokio::test]
    async fn test_memory_storage_not_found() {
        let storage = MemoryStorage::new();
        let result = storage.get("nonexistent").await;

        match result {
            Err(StorageError::NotFound(key)) => assert_eq!(key, "nonexistent"),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_memory_storage_utilities() {
        let storage = MemoryStorage::new();

        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());

        storage.put("key1", b"data1".to_vec()).await.unwrap();
        storage.put("key2", b"data2".to_vec()).await.unwrap();

        assert_eq!(storage.len(), 2);
        assert!(!storage.is_empty());

        let keys = storage.keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));

        storage.clear();
        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
    }

    #[tokio::test]
    async fn test_memory_storage_list_keys() {
        let storage = MemoryStorage::new();

        // Add some test keys
        storage.put("refs/john/invoice/latest", b"hash1".to_vec()).await.unwrap();
        storage.put("refs/john/invoice/v1.0.0", b"hash2".to_vec()).await.unwrap();
        storage.put("refs/alice/letter/latest", b"hash3".to_vec()).await.unwrap();
        storage.put("blobs/sha256/abc123", b"data1".to_vec()).await.unwrap();
        storage.put("manifests/sha256/def456", b"data2".to_vec()).await.unwrap();

        // Test listing with "refs/" prefix
        let ref_keys = storage.list_keys("refs/").await.unwrap();
        assert_eq!(ref_keys.len(), 3);
        assert!(ref_keys.contains(&"refs/alice/letter/latest".to_string()));
        assert!(ref_keys.contains(&"refs/john/invoice/latest".to_string()));
        assert!(ref_keys.contains(&"refs/john/invoice/v1.0.0".to_string()));

        // Test listing with more specific prefix
        let john_keys = storage.list_keys("refs/john/").await.unwrap();
        assert_eq!(john_keys.len(), 2);
        assert!(john_keys.contains(&"refs/john/invoice/latest".to_string()));
        assert!(john_keys.contains(&"refs/john/invoice/v1.0.0".to_string()));

        // Test listing with prefix that doesn't match anything
        let empty_keys = storage.list_keys("nonexistent/").await.unwrap();
        assert!(empty_keys.is_empty());

        // Test listing all keys with empty prefix
        let all_keys = storage.list_keys("").await.unwrap();
        assert_eq!(all_keys.len(), 5);

        // Results should be sorted
        let sorted_keys = storage.list_keys("refs/").await.unwrap();
        let mut expected = vec![
            "refs/alice/letter/latest".to_string(),
            "refs/john/invoice/latest".to_string(),
            "refs/john/invoice/v1.0.0".to_string(),
        ];
        expected.sort();
        assert_eq!(sorted_keys, expected);
    }
}
