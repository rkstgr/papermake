//! Render storage abstraction for tracking template render operations and analytics

pub mod types;

#[cfg(feature = "clickhouse")]
pub mod clickhouse;

#[cfg(test)]
mod tests {
    use super::{MemoryRenderStorage, RenderRecord, RenderStorage};

    #[tokio::test]
    async fn test_memory_render_storage_basic_operations() {
        let storage = MemoryRenderStorage::new();
        
        // Create a test render record
        let record = RenderRecord::success(
            "invoice:latest".to_string(),
            "invoice".to_string(),
            "latest".to_string(),
            "sha256:manifest123".to_string(),
            "sha256:data456".to_string(),
            "sha256:pdf789".to_string(),
            1000,
            1024,
        );
        let render_id = record.render_id.clone();
        
        // Store the record
        storage.store_render(record.clone()).await.unwrap();
        
        // Retrieve the record
        let retrieved = storage.get_render(&render_id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.render_id, render_id);
        assert_eq!(retrieved.template_name, "invoice");
        assert!(retrieved.success);
        
        // List recent renders
        let recent = storage.list_recent_renders(10).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].render_id, render_id);
    }

    #[tokio::test]
    async fn test_memory_render_storage_template_filtering() {
        let storage = MemoryRenderStorage::new();
        
        // Create multiple test records for different templates
        let record1 = RenderRecord::success(
            "invoice:latest".to_string(),
            "invoice".to_string(),
            "latest".to_string(),
            "sha256:manifest123".to_string(),
            "sha256:data456".to_string(),
            "sha256:pdf789".to_string(),
            1000,
            1024,
        );
        
        let record2 = RenderRecord::success(
            "letterhead:v1".to_string(),
            "letterhead".to_string(),
            "v1".to_string(),
            "sha256:manifest456".to_string(),
            "sha256:data789".to_string(),
            "sha256:pdf012".to_string(),
            1500,
            2048,
        );
        
        let record3 = RenderRecord::success(
            "invoice:v2".to_string(),
            "invoice".to_string(),
            "v2".to_string(),
            "sha256:manifest789".to_string(),
            "sha256:data012".to_string(),
            "sha256:pdf345".to_string(),
            800,
            512,
        );
        
        // Store all records
        storage.store_render(record1).await.unwrap();
        storage.store_render(record2).await.unwrap();
        storage.store_render(record3).await.unwrap();
        
        // List template-specific renders
        let invoice_renders = storage.list_template_renders("invoice", 10).await.unwrap();
        assert_eq!(invoice_renders.len(), 2);
        
        let letterhead_renders = storage.list_template_renders("letterhead", 10).await.unwrap();
        assert_eq!(letterhead_renders.len(), 1);
        assert_eq!(letterhead_renders[0].template_name, "letterhead");
    }

    #[tokio::test]
    async fn test_render_record_constructors() {
        let success_record = RenderRecord::success(
            "test:latest".to_string(),
            "test".to_string(),
            "latest".to_string(),
            "sha256:manifest".to_string(),
            "sha256:data".to_string(),
            "sha256:pdf".to_string(),
            1000,
            2048,
        );
        
        assert!(success_record.success);
        assert!(success_record.error.is_none());
        assert_eq!(success_record.duration_ms, 1000);
        assert_eq!(success_record.pdf_size_bytes, 2048);
        assert!(!success_record.render_id.is_empty());
        
        let error_record = RenderRecord::failure(
            "test:latest".to_string(),
            "test".to_string(),
            "latest".to_string(),
            "sha256:manifest".to_string(),
            "sha256:data".to_string(),
            "Compilation failed".to_string(),
            500,
        );
        
        assert!(!error_record.success);
        assert_eq!(error_record.error, Some("Compilation failed".to_string()));
        assert_eq!(error_record.duration_ms, 500);
        assert_eq!(error_record.pdf_size_bytes, 0);
        assert!(error_record.pdf_hash.is_empty());
    }
}

use async_trait::async_trait;
pub use types::*;

/// Trait for storing and querying template render records
#[async_trait]
pub trait RenderStorage: Send + Sync {
    /// Store a render record
    async fn store_render(&self, record: RenderRecord) -> Result<(), RenderStorageError>;
    
    /// Get a specific render record by ID
    async fn get_render(&self, render_id: &str) -> Result<Option<RenderRecord>, RenderStorageError>;
    
    /// List recent render records with optional limit
    async fn list_recent_renders(&self, limit: u32) -> Result<Vec<RenderRecord>, RenderStorageError>;
    
    /// List renders for a specific template with optional limit
    async fn list_template_renders(
        &self,
        template_name: &str,
        limit: u32,
    ) -> Result<Vec<RenderRecord>, RenderStorageError>;
    
    /// Get render volume over time for analytics
    async fn render_volume_over_time(&self, days: u32) -> Result<Vec<VolumePoint>, RenderStorageError>;
    
    /// Get total renders per template for analytics
    async fn total_renders_per_template(&self) -> Result<Vec<TemplateStats>, RenderStorageError>;
    
    /// Get average render duration over time for analytics
    async fn average_duration_over_time(
        &self,
        days: u32,
    ) -> Result<Vec<DurationPoint>, RenderStorageError>;
}

/// In-memory render storage implementation for testing
#[derive(Debug, Default)]
pub struct MemoryRenderStorage {
    records: std::sync::Arc<tokio::sync::RwLock<Vec<RenderRecord>>>,
}

impl MemoryRenderStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl RenderStorage for MemoryRenderStorage {
    async fn store_render(&self, record: RenderRecord) -> Result<(), RenderStorageError> {
        let mut records = self.records.write().await;
        records.push(record);
        Ok(())
    }
    
    async fn get_render(&self, render_id: &str) -> Result<Option<RenderRecord>, RenderStorageError> {
        let records = self.records.read().await;
        Ok(records.iter().find(|r| r.render_id == render_id).cloned())
    }
    
    async fn list_recent_renders(&self, limit: u32) -> Result<Vec<RenderRecord>, RenderStorageError> {
        let records = self.records.read().await;
        let mut sorted_records = records.clone();
        sorted_records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(sorted_records.into_iter().take(limit as usize).collect())
    }
    
    async fn list_template_renders(
        &self,
        template_name: &str,
        limit: u32,
    ) -> Result<Vec<RenderRecord>, RenderStorageError> {
        let records = self.records.read().await;
        let mut filtered_records: Vec<_> = records
            .iter()
            .filter(|r| r.template_name == template_name)
            .cloned()
            .collect();
        filtered_records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(filtered_records.into_iter().take(limit as usize).collect())
    }
    
    async fn render_volume_over_time(&self, days: u32) -> Result<Vec<VolumePoint>, RenderStorageError> {
        use std::collections::HashMap;
        use time::{Duration, OffsetDateTime};
        
        let records = self.records.read().await;
        let cutoff = OffsetDateTime::now_utc() - Duration::days(days as i64);
        
        let mut daily_counts: HashMap<time::Date, u64> = HashMap::new();
        
        for record in records.iter() {
            if record.timestamp >= cutoff {
                let date = record.timestamp.date();
                *daily_counts.entry(date).or_insert(0) += 1;
            }
        }
        
        let mut result: Vec<VolumePoint> = daily_counts
            .into_iter()
            .map(|(date, renders)| VolumePoint { date, renders })
            .collect();
        
        result.sort_by(|a, b| a.date.cmp(&b.date));
        Ok(result)
    }
    
    async fn total_renders_per_template(&self) -> Result<Vec<TemplateStats>, RenderStorageError> {
        use std::collections::HashMap;
        
        let records = self.records.read().await;
        let mut template_counts: HashMap<String, u64> = HashMap::new();
        
        for record in records.iter() {
            *template_counts.entry(record.template_name.clone()).or_insert(0) += 1;
        }
        
        let mut result: Vec<TemplateStats> = template_counts
            .into_iter()
            .map(|(template_name, total_renders)| TemplateStats {
                template_name,
                total_renders,
            })
            .collect();
        
        result.sort_by(|a, b| b.total_renders.cmp(&a.total_renders));
        Ok(result)
    }
    
    async fn average_duration_over_time(
        &self,
        days: u32,
    ) -> Result<Vec<DurationPoint>, RenderStorageError> {
        use std::collections::HashMap;
        use time::{Duration, OffsetDateTime};
        
        let records = self.records.read().await;
        let cutoff = OffsetDateTime::now_utc() - Duration::days(days as i64);
        
        let mut daily_stats: HashMap<time::Date, (u64, u64)> = HashMap::new(); // (total_duration, count)
        
        for record in records.iter() {
            if record.timestamp >= cutoff && record.success {
                let date = record.timestamp.date();
                let (total_duration, count) = daily_stats.entry(date).or_insert((0, 0));
                *total_duration += record.duration_ms as u64;
                *count += 1;
            }
        }
        
        let mut result: Vec<DurationPoint> = daily_stats
            .into_iter()
            .map(|(date, (total_duration, count))| DurationPoint {
                date,
                avg_duration_ms: total_duration as f64 / count as f64,
            })
            .collect();
        
        result.sort_by(|a, b| a.date.cmp(&b.date));
        Ok(result)
    }
}