//! TiKV metadata storage implementation
//!
//! This module provides a TiKV-backed implementation of the MetadataStorage trait.
//! It uses TiKV's key-value store to persist template metadata and render jobs.

use crate::{entities::*, error::Result, RegistryError};
use super::MetadataStorage;
use async_trait::async_trait;
use papermake::TemplateId;
use serde_json;
use tikv_client::{RawClient, Config};

/// TiKV metadata storage implementation
pub struct TiKVStorage {
    client: RawClient,
}

impl TiKVStorage {
    /// Create a new TiKV storage instance
    pub async fn new(pd_endpoints: Vec<String>) -> Result<Self> {
        let client = RawClient::new(pd_endpoints).await
            .map_err(|e| RegistryError::Storage(format!("Failed to connect to TiKV: {}", e)))?;
        
        Ok(Self { client })
    }
    
    /// Create a new TiKV storage instance with custom configuration
    pub async fn with_config(pd_endpoints: Vec<String>, config: Config) -> Result<Self> {
        let client = RawClient::new_with_config(pd_endpoints, config).await
            .map_err(|e| RegistryError::Storage(format!("Failed to connect to TiKV: {}", e)))?;
        
        Ok(Self { client })
    }
    
    // Key generation helpers
    
    /// Generate key for template metadata: "template:{id}:{version}"
    fn template_key(id: &TemplateId, version: u64) -> String {
        format!("template:{}:{}", id.as_ref(), version)
    }
    
    /// Generate key for template versions list: "template_versions:{id}"
    fn template_versions_key(id: &TemplateId) -> String {
        format!("template_versions:{}", id.as_ref())
    }
    
    /// Generate key for render job: "render_job:{job_id}"
    fn render_job_key(job_id: &str) -> String {
        format!("render_job:{}", job_id)
    }
    
    /// Generate key for render job hash lookup: "render_hash:{template_id}:{version}:{data_hash}"
    fn render_hash_key(template_id: &TemplateId, version: u64, data_hash: &str) -> String {
        format!("render_hash:{}:{}:{}", template_id.as_ref(), version, data_hash)
    }
    
    /// Generate key for template render jobs list: "template_renders:{template_id}:{version}"
    fn template_renders_key(template_id: &TemplateId, version: Option<u64>) -> String {
        match version {
            Some(v) => format!("template_renders:{}:{}", template_id.as_ref(), v),
            None => format!("template_renders:{}", template_id.as_ref()),
        }
    }
    
    /// Generate search index key: "search_index:{search_term}:{template_id}:{version}"
    fn search_index_key(search_term: &str, template_id: &TemplateId, version: u64) -> String {
        format!("search_index:{}:{}:{}", 
            search_term.to_lowercase(), 
            template_id.as_ref(), 
            version
        )
    }
}

#[async_trait]
impl MetadataStorage for TiKVStorage {
    // === Template Management ===
    
    async fn save_versioned_template(&self, template: &VersionedTemplate) -> Result<()> {
        let key = Self::template_key(&template.template.id, template.version);
        let value = serde_json::to_vec(template)
            .map_err(|e| RegistryError::Storage(format!("Failed to serialize template: {}", e)))?;
        
        self.client.put(key, value).await
            .map_err(|e| RegistryError::Storage(format!("Failed to save template: {}", e)))?;
        
        // Update versions list
        let versions_key = Self::template_versions_key(&template.template.id);
        let mut versions = self.list_template_versions(&template.template.id).await?;
        if !versions.contains(&template.version) {
            versions.push(template.version);
            versions.sort_unstable();
            
            let versions_value = serde_json::to_vec(&versions)
                .map_err(|e| RegistryError::Storage(format!("Failed to serialize versions: {}", e)))?;
            
            self.client.put(versions_key, versions_value).await
                .map_err(|e| RegistryError::Storage(format!("Failed to save versions list: {}", e)))?;
        }
        
        // Update search index
        self.update_search_index(template).await?;
        
        Ok(())
    }
    
    async fn get_versioned_template(&self, id: &TemplateId, version: u64) -> Result<VersionedTemplate> {
        let key = Self::template_key(id, version);
        let value = self.client.get(key).await
            .map_err(|e| RegistryError::Storage(format!("Failed to get template: {}", e)))?;
        
        let value = value.ok_or_else(|| RegistryError::VersionNotFound {
            template_id: id.as_ref().to_string(),
            version,
        })?;
        
        let template: VersionedTemplate = serde_json::from_slice(&value)
            .map_err(|e| RegistryError::Storage(format!("Failed to deserialize template: {}", e)))?;
        
        Ok(template)
    }
    
    async fn list_template_versions(&self, id: &TemplateId) -> Result<Vec<u64>> {
        let key = Self::template_versions_key(id);
        let value = self.client.get(key).await
            .map_err(|e| RegistryError::Storage(format!("Failed to get versions: {}", e)))?;
        
        let versions = match value {
            Some(data) => serde_json::from_slice(&data)
                .map_err(|e| RegistryError::Storage(format!("Failed to deserialize versions: {}", e)))?,
            None => Vec::new(),
        };
        
        Ok(versions)
    }
    
    async fn delete_template_version(&self, id: &TemplateId, version: u64) -> Result<()> {
        let key = Self::template_key(id, version);
        self.client.delete(key).await
            .map_err(|e| RegistryError::Storage(format!("Failed to delete template: {}", e)))?;
        
        // Update versions list
        let versions_key = Self::template_versions_key(id);
        let mut versions = self.list_template_versions(id).await?;
        versions.retain(|&v| v != version);
        
        if versions.is_empty() {
            self.client.delete(versions_key).await
                .map_err(|e| RegistryError::Storage(format!("Failed to delete versions list: {}", e)))?;
        } else {
            let versions_value = serde_json::to_vec(&versions)
                .map_err(|e| RegistryError::Storage(format!("Failed to serialize versions: {}", e)))?;
            
            self.client.put(versions_key, versions_value).await
                .map_err(|e| RegistryError::Storage(format!("Failed to update versions list: {}", e)))?;
        }
        
        Ok(())
    }
    
    async fn search_templates(&self, query: &str) -> Result<Vec<(TemplateId, u64)>> {
        // Simple prefix scan implementation
        // In production, you'd want a more sophisticated search index
        let search_prefix = format!("search_index:{}:", query.to_lowercase());
        let search_prefix_bytes = search_prefix.into_bytes();
        let keys = self.client.scan_keys(search_prefix_bytes.clone()..search_prefix_bytes, 1000).await
            .map_err(|e| RegistryError::Storage(format!("Failed to search templates: {}", e)))?;
        
        let mut results = Vec::new();
        for key in keys {
            if let Ok(key_str) = String::from_utf8(key.clone().into()) {
                if let Some(parts) = self.parse_search_key(&key_str) {
                    results.push((TemplateId::from(parts.0), parts.1));
                }
            }
        }
        
        results.sort_by(|a, b| a.0.as_ref().cmp(b.0.as_ref()).then(b.1.cmp(&a.1)));
        results.dedup();
        
        Ok(results)
    }
    
    // === Render Job Management ===
    
    async fn save_render_job(&self, job: &RenderJob) -> Result<()> {
        let key = Self::render_job_key(&job.id);
        let value = serde_json::to_vec(job)
            .map_err(|e| RegistryError::Storage(format!("Failed to serialize render job: {}", e)))?;
        
        self.client.put(key, value).await
            .map_err(|e| RegistryError::Storage(format!("Failed to save render job: {}", e)))?;
        
        // Create hash lookup for caching
        let hash_key = Self::render_hash_key(&job.template_id, job.template_version, &job.data_hash);
        self.client.put(hash_key, job.id.as_bytes().to_vec()).await
            .map_err(|e| RegistryError::Storage(format!("Failed to save render hash: {}", e)))?;
        
        // Add to template renders list
        let renders_key = Self::template_renders_key(&job.template_id, Some(job.template_version));
        let mut job_ids = self.get_template_render_jobs(&job.template_id, Some(job.template_version)).await?;
        if !job_ids.iter().any(|j| j.id == job.id) {
            job_ids.push(job.clone());
            let ids: Vec<String> = job_ids.iter().map(|j| j.id.clone()).collect();
            let value = serde_json::to_vec(&ids)
                .map_err(|e| RegistryError::Storage(format!("Failed to serialize job IDs: {}", e)))?;
            
            self.client.put(renders_key, value).await
                .map_err(|e| RegistryError::Storage(format!("Failed to save render list: {}", e)))?;
        }
        
        Ok(())
    }
    
    async fn get_render_job(&self, job_id: &str) -> Result<RenderJob> {
        let key = Self::render_job_key(job_id);
        let value = self.client.get(key).await
            .map_err(|e| RegistryError::Storage(format!("Failed to get render job: {}", e)))?;
        
        let value = value.ok_or_else(|| RegistryError::Storage(format!("Render job {} not found", job_id)))?;
        
        let job: RenderJob = serde_json::from_slice(&value)
            .map_err(|e| RegistryError::Storage(format!("Failed to deserialize render job: {}", e)))?;
        
        Ok(job)
    }
    
    async fn find_render_job_by_hash(
        &self,
        template_id: &TemplateId,
        version: u64,
        data_hash: &str,
    ) -> Result<Option<RenderJob>> {
        let hash_key = Self::render_hash_key(template_id, version, data_hash);
        let job_id = self.client.get(hash_key).await
            .map_err(|e| RegistryError::Storage(format!("Failed to lookup render hash: {}", e)))?;
        
        match job_id {
            Some(id_bytes) => {
                let job_id = String::from_utf8(id_bytes)
                    .map_err(|e| RegistryError::Storage(format!("Invalid job ID in hash lookup: {}", e)))?;
                let job = self.get_render_job(&job_id).await?;
                Ok(Some(job))
            },
            None => Ok(None),
        }
    }
    
    async fn list_render_jobs(
        &self,
        template_id: &TemplateId,
        version: Option<u64>,
    ) -> Result<Vec<RenderJob>> {
        self.get_template_render_jobs(template_id, version).await
    }
}

impl TiKVStorage {
    /// Update search index for a template
    async fn update_search_index(&self, template: &VersionedTemplate) -> Result<()> {
        // Index template name
        let name_terms: Vec<&str> = template.template.name.split_whitespace().collect();
        for term in name_terms {
            if term.len() >= 2 { // Only index terms with 2+ characters
                let key = Self::search_index_key(term, &template.template.id, template.version);
                self.client.put(key, b"1".to_vec()).await
                    .map_err(|e| RegistryError::Storage(format!("Failed to update search index: {}", e)))?;
            }
        }
        
        // Index template description if available
        if let Some(desc) = &template.template.description {
            let desc_terms: Vec<&str> = desc.split_whitespace().collect();
            for term in desc_terms {
                if term.len() >= 2 {
                    let key = Self::search_index_key(term, &template.template.id, template.version);
                    self.client.put(key, b"1".to_vec()).await
                        .map_err(|e| RegistryError::Storage(format!("Failed to update search index: {}", e)))?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Parse search index key to extract template ID and version
    fn parse_search_key(&self, key: &str) -> Option<(String, u64)> {
        let parts: Vec<&str> = key.split(':').collect();
        if parts.len() >= 4 && parts[0] == "search_index" {
            // Format: "search_index:{term}:{template_id}:{version}"
            let template_id = parts[2].to_string();
            let version = parts[3].parse().ok()?;
            Some((template_id, version))
        } else {
            None
        }
    }
    
    /// Get render jobs for a template
    async fn get_template_render_jobs(
        &self,
        template_id: &TemplateId,
        version: Option<u64>,
    ) -> Result<Vec<RenderJob>> {
        let renders_key = Self::template_renders_key(template_id, version);
        let value = self.client.get(renders_key).await
            .map_err(|e| RegistryError::Storage(format!("Failed to get render jobs: {}", e)))?;
        
        let job_ids: Vec<String> = match value {
            Some(data) => serde_json::from_slice(&data)
                .map_err(|e| RegistryError::Storage(format!("Failed to deserialize job IDs: {}", e)))?,
            None => Vec::new(),
        };
        
        let mut jobs = Vec::new();
        for job_id in job_ids {
            match self.get_render_job(&job_id).await {
                Ok(job) => jobs.push(job),
                Err(_) => continue, // Skip missing jobs
            }
        }
        
        Ok(jobs)
    }
}