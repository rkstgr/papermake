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
}
