//! SQLite metadata storage implementation
//!
//! This module provides a SQLite-based implementation of the MetadataStorage trait.
//! It stores template metadata and render jobs in a local SQLite database file.

use super::MetadataStorage;
use crate::{RegistryError, entities::*, error::Result};
use async_trait::async_trait;
use papermake::TemplateId;
use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions};
use std::str::FromStr;

/// SQLite-based metadata storage implementation
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    /// Create a new SQLite storage instance with the given database path
    pub async fn new(database_path: &str) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(database_path)
            .map_err(|e| RegistryError::Storage(format!("Invalid database path: {}", e)))?
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(|e| RegistryError::Storage(format!("Failed to connect to SQLite: {}", e)))?;

        let storage = Self { pool };
        storage.init_schema().await?;
        Ok(storage)
    }

    /// Create SQLite storage from environment variable
    ///
    /// Expects DATABASE_URL environment variable with SQLite connection string
    /// Example: sqlite:./data/papermake.db
    pub async fn from_env() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:./data/papermake.db".to_string());

        Self::new(&database_url).await
    }

    /// Initialize database schema
    async fn init_schema(&self) -> Result<()> {
        // Create versioned_templates table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS versioned_templates (
                template_id TEXT NOT NULL,           -- UUID for backward compatibility
                template_name TEXT NOT NULL,         -- Machine-readable name (e.g. "invoice-template")
                display_name TEXT NOT NULL,          -- Human-readable name (e.g. "Monthly Invoice Template")
                version TEXT NOT NULL,               -- Version string (e.g. "v1", "v2", "latest", "draft")
                author TEXT NOT NULL,
                created_at TEXT NOT NULL,
                template_data TEXT NOT NULL,         -- JSON
                is_draft INTEGER NOT NULL DEFAULT 0, -- SQLite boolean (0/1)
                PRIMARY KEY (template_name, version),
                UNIQUE (template_id)                 -- Keep id unique for backward compatibility
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            RegistryError::Storage(format!("Failed to create versioned_templates table: {}", e))
        })?;

        // Create render_jobs table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS render_jobs (
                id TEXT PRIMARY KEY,
                template_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                data TEXT NOT NULL,
                data_hash TEXT NOT NULL,
                status TEXT NOT NULL,
                pdf_s3_key TEXT,
                rendering_latency INTEGER,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                error_message TEXT
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            RegistryError::Storage(format!("Failed to create render_jobs table: {}", e))
        })?;

        // Add missing columns if they don't exist (migration)
        let _ = sqlx::query("ALTER TABLE render_jobs ADD COLUMN data TEXT")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("ALTER TABLE render_jobs ADD COLUMN pdf_s3_key TEXT")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("ALTER TABLE render_jobs ADD COLUMN rendering_latency INTEGER")
            .execute(&self.pool)
            .await;

        // Create indexes for efficient queries
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_templates_id ON versioned_templates(template_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to create template id index: {}", e)))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_templates_name ON versioned_templates(template_name)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to create template name index: {}", e)))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_templates_draft ON versioned_templates(template_name, is_draft)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to create template index: {}", e)))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_render_jobs_template ON render_jobs(template_id, version)")
            .execute(&self.pool)
            .await
            .map_err(|e| RegistryError::Storage(format!("Failed to create render jobs template index: {}", e)))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_render_jobs_hash ON render_jobs(template_id, version, data_hash)")
            .execute(&self.pool)
            .await
            .map_err(|e| RegistryError::Storage(format!("Failed to create render jobs hash index: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl MetadataStorage for SqliteStorage {
    async fn save_versioned_template(&self, template: &VersionedTemplate) -> Result<()> {
        let template_json = serde_json::to_string(&template.template)
            .map_err(|e| RegistryError::Storage(format!("Failed to serialize template: {}", e)))?;

        let published_at = template
            .published_at
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| RegistryError::Storage(format!("Failed to format timestamp: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO versioned_templates
            (template_id, template_name, display_name, version, author, created_at, template_data, is_draft)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(template.template.id.as_ref())
        .bind(&template.template_name)
        .bind(&template.display_name)
        .bind(&template.version)
        .bind(&template.author)
        .bind(published_at)
        .bind(template_json)
        .bind(if template.is_draft { 1 } else { 0 })
        .execute(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to save template: {}", e)))?;

        Ok(())
    }

    async fn get_versioned_template(
        &self,
        id: &TemplateId,
        version: u64,
    ) -> Result<VersionedTemplate> {
        let row = sqlx::query(
            r#"
            SELECT template_id, template_name, display_name, version, author, created_at, template_data, is_draft
            FROM versioned_templates
            WHERE template_id = ? AND version = ?
        "#,
        )
        .bind(id.as_ref())
        .bind(format!("v{}", version))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RegistryError::TemplateNotFound(id.as_ref().to_string()),
            _ => RegistryError::Storage(format!("Failed to get template: {}", e)),
        })?;

        let template_json: String = row.get("template_data");
        let template = serde_json::from_str(&template_json).map_err(|e| {
            RegistryError::Storage(format!("Failed to deserialize template: {}", e))
        })?;

        let published_at_str: String = row.get("created_at");
        let published_at = time::OffsetDateTime::parse(
            &published_at_str,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| RegistryError::Storage(format!("Failed to parse timestamp: {}", e)))?;

        Ok(VersionedTemplate {
            template,
            template_name: row.get("template_name"),
            display_name: row.get("display_name"),
            version: row.get("version"),
            author: row.get("author"),
            forked_from: None, // TODO: Store this in database if needed
            published_at,
            immutable: true,
            is_draft: row.get::<i32, _>("is_draft") == 1,
            schema: None, // TODO: Store this in database if needed
        })
    }

    async fn list_template_versions(&self, id: &TemplateId) -> Result<Vec<u64>> {
        let rows = sqlx::query(
            r#"
            SELECT version FROM versioned_templates
            WHERE template_id = ?
            ORDER BY version ASC
        "#,
        )
        .bind(id.as_ref())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to list template versions: {}", e)))?;

        let versions: Vec<u64> = rows
            .iter()
            .map(|row| row.get::<i64, _>("version") as u64)
            .collect();

        Ok(versions)
    }

    async fn delete_template_version(&self, id: &TemplateId, version: u64) -> Result<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM versioned_templates
            WHERE template_id = ? AND version = ?
        "#,
        )
        .bind(id.as_ref())
        .bind(version as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to delete template: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(RegistryError::TemplateNotFound(id.as_ref().to_string()));
        }

        Ok(())
    }

    async fn search_templates(&self, query: &str) -> Result<Vec<(TemplateId, u64)>> {
        let search_pattern = format!("%{}%", query);

        let rows = sqlx::query(
            r#"
            SELECT template_id, version FROM versioned_templates
            WHERE template_data LIKE ?
            ORDER BY template_id, version DESC
        "#,
        )
        .bind(search_pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to search templates: {}", e)))?;

        let results: Vec<(TemplateId, u64)> = rows
            .iter()
            .map(|row| {
                let template_id = TemplateId::from(row.get::<String, _>("template_id"));
                let version = row.get::<i64, _>("version") as u64;
                (template_id, version)
            })
            .collect();

        Ok(results)
    }

    async fn save_render_job(&self, job: &RenderJob) -> Result<()> {
        let created_at = job
            .created_at
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| RegistryError::Storage(format!("Failed to format timestamp: {}", e)))?;

        let completed_at = job
            .completed_at
            .map(|dt| dt.format(&time::format_description::well_known::Rfc3339))
            .transpose()
            .map_err(|e| {
                RegistryError::Storage(format!("Failed to format completed timestamp: {}", e))
            })?;

        let status_str = format!("{:?}", job.status);
        let data_json = serde_json::to_string(&job.data)
            .map_err(|e| RegistryError::Storage(format!("Failed to serialize data: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO render_jobs
            (id, template_id, version, data, data_hash, status, pdf_s3_key, rendering_latency, created_at, completed_at, error_message)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&job.id)
        .bind(job.template_id.as_ref())
        .bind(job.template_version.strip_prefix('v').unwrap_or(&job.template_version).parse::<i64>().unwrap_or(1))
        .bind(data_json)
        .bind(&job.data_hash)
        .bind(status_str)
        .bind(&job.pdf_s3_key)
        .bind(job.rendering_latency.map(|l| l as i64))
        .bind(created_at)
        .bind(completed_at)
        .bind(&job.error_message)
        .execute(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to save render job: {}", e)))?;

        Ok(())
    }

    async fn get_render_job(&self, job_id: &str) -> Result<RenderJob> {
        let row = sqlx::query(r#"
            SELECT id, template_id, version, data, data_hash, status, pdf_s3_key, rendering_latency, created_at, completed_at, error_message
            FROM render_jobs
            WHERE id = ?
        "#)
        .bind(job_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RegistryError::Storage(format!("Render job {} not found", job_id)),
            _ => RegistryError::Storage(format!("Failed to get render job: {}", e)),
        })?;

        let created_at_str: String = row.get("created_at");
        let created_at = time::OffsetDateTime::parse(
            &created_at_str,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| RegistryError::Storage(format!("Failed to parse created timestamp: {}", e)))?;

        let completed_at = if let Some(completed_at_str) =
            row.get::<Option<String>, _>("completed_at")
        {
            Some(
                time::OffsetDateTime::parse(
                    &completed_at_str,
                    &time::format_description::well_known::Rfc3339,
                )
                .map_err(|e| {
                    RegistryError::Storage(format!("Failed to parse completed timestamp: {}", e))
                })?,
            )
        } else {
            None
        };

        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "Pending" => RenderStatus::Pending,
            "InProgress" => RenderStatus::InProgress,
            "Completed" => RenderStatus::Completed,
            "Failed" => RenderStatus::Failed,
            _ => RenderStatus::Pending, // Default fallback
        };

        let data_json: String = row.get("data");
        let data = serde_json::from_str(&data_json)
            .map_err(|e| RegistryError::Storage(format!("Failed to deserialize data: {}", e)))?;

        Ok(RenderJob {
            id: row.get("id"),
            template_id: TemplateId::from(row.get::<String, _>("template_id")),
            template_name: row.get::<String, _>("template_id"), // TODO: Add template_name column to render_jobs
            template_version: format!("v{}", row.get::<i64, _>("version")),
            data,
            data_hash: row.get("data_hash"),
            status,
            pdf_s3_key: row.get("pdf_s3_key"),
            rendering_latency: row.get::<Option<i64>, _>("rendering_latency").map(|l| l as u64),
            created_at,
            completed_at,
            error_message: row.get("error_message"),
        })
    }

    async fn find_render_job_by_hash(
        &self,
        template_id: &TemplateId,
        version: u64,
        data_hash: &str,
    ) -> Result<Option<RenderJob>> {
        let row = sqlx::query(r#"
            SELECT id, template_id, version, data, data_hash, status, pdf_s3_key, rendering_latency, created_at, completed_at, error_message
            FROM render_jobs
            WHERE template_id = ? AND version = ? AND data_hash = ?
            ORDER BY created_at DESC
            LIMIT 1
        "#)
        .bind(template_id.as_ref())
        .bind(version as i64)
        .bind(data_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to find render job by hash: {}", e)))?;

        if let Some(row) = row {
            let created_at_str: String = row.get("created_at");
            let created_at = time::OffsetDateTime::parse(
                &created_at_str,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|e| {
                RegistryError::Storage(format!("Failed to parse created timestamp: {}", e))
            })?;

            let completed_at =
                if let Some(completed_at_str) = row.get::<Option<String>, _>("completed_at") {
                    Some(
                        time::OffsetDateTime::parse(
                            &completed_at_str,
                            &time::format_description::well_known::Rfc3339,
                        )
                        .map_err(|e| {
                            RegistryError::Storage(format!(
                                "Failed to parse completed timestamp: {}",
                                e
                            ))
                        })?,
                    )
                } else {
                    None
                };

            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "Pending" => RenderStatus::Pending,
                "InProgress" => RenderStatus::InProgress,
                "Completed" => RenderStatus::Completed,
                "Failed" => RenderStatus::Failed,
                _ => RenderStatus::Pending,
            };

            let data_json: String = row.get("data");
            let data = serde_json::from_str(&data_json)
                .map_err(|e| RegistryError::Storage(format!("Failed to deserialize data: {}", e)))?;

            Ok(Some(RenderJob {
                id: row.get("id"),
                template_id: TemplateId::from(row.get::<String, _>("template_id")),
                template_name: row.get::<String, _>("template_id"), // TODO: Add template_name column to render_jobs
                template_version: format!("v{}", row.get::<i64, _>("version")),
                data,
                data_hash: row.get("data_hash"),
                status,
                pdf_s3_key: row.get("pdf_s3_key"),
                rendering_latency: row.get::<Option<i64>, _>("rendering_latency").map(|l| l as u64),
                created_at,
                completed_at,
                error_message: row.get("error_message"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_render_jobs(
        &self,
        template_id: &TemplateId,
        version: Option<u64>,
    ) -> Result<Vec<RenderJob>> {
        let rows = if let Some(version) = version {
            sqlx::query(r#"
                SELECT id, template_id, version, data, data_hash, status, pdf_s3_key, rendering_latency, created_at, completed_at, error_message
                FROM render_jobs
                WHERE template_id = ? AND version = ?
                ORDER BY created_at DESC
            "#)
            .bind(template_id.as_ref())
            .bind(version as i64)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(r#"
                SELECT id, template_id, version, data, data_hash, status, pdf_s3_key, rendering_latency, created_at, completed_at, error_message
                FROM render_jobs
                WHERE template_id = ?
                ORDER BY created_at DESC
            "#)
            .bind(template_id.as_ref())
            .fetch_all(&self.pool)
            .await
        };

        let rows =
            rows.map_err(|e| RegistryError::Storage(format!("Failed to list render jobs: {}", e)))?;

        let mut jobs = Vec::new();
        for row in rows {
            let created_at_str: String = row.get("created_at");
            let created_at = time::OffsetDateTime::parse(
                &created_at_str,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|e| {
                RegistryError::Storage(format!("Failed to parse created timestamp: {}", e))
            })?;

            let completed_at =
                if let Some(completed_at_str) = row.get::<Option<String>, _>("completed_at") {
                    Some(
                        time::OffsetDateTime::parse(
                            &completed_at_str,
                            &time::format_description::well_known::Rfc3339,
                        )
                        .map_err(|e| {
                            RegistryError::Storage(format!(
                                "Failed to parse completed timestamp: {}",
                                e
                            ))
                        })?,
                    )
                } else {
                    None
                };

            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "Pending" => RenderStatus::Pending,
                "InProgress" => RenderStatus::InProgress,
                "Completed" => RenderStatus::Completed,
                "Failed" => RenderStatus::Failed,
                _ => RenderStatus::Pending,
            };

            let data_json: String = row.get("data");
            let data = serde_json::from_str(&data_json)
                .map_err(|e| RegistryError::Storage(format!("Failed to deserialize data: {}", e)))?;

            jobs.push(RenderJob {
                id: row.get("id"),
                template_id: TemplateId::from(row.get::<String, _>("template_id")),
                template_name: row.get::<String, _>("template_id"), // TODO: Add template_name column to render_jobs
                template_version: format!("v{}", row.get::<i64, _>("version")),
                data,
                data_hash: row.get("data_hash"),
                status,
                pdf_s3_key: row.get("pdf_s3_key"),
                rendering_latency: row.get::<Option<i64>, _>("rendering_latency").map(|l| l as u64),
                created_at,
                completed_at,
                error_message: row.get("error_message"),
            });
        }

        Ok(jobs)
    }

    async fn list_all_templates(&self) -> Result<Vec<VersionedTemplate>> {
        let rows = sqlx::query(
            "SELECT template_id, version, author, created_at, template_data
             FROM versioned_templates 
             WHERE (template_id, version) IN (
                 SELECT template_id, MAX(version) 
                 FROM versioned_templates 
                 GROUP BY template_id
             )
             ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to list all templates: {}", e)))?;

        let mut templates = Vec::new();
        for row in rows {
            let template_json: String = row.get("template_data");
            let template: papermake::Template = serde_json::from_str(&template_json).map_err(|e| {
                RegistryError::Storage(format!("Failed to deserialize template: {}", e))
            })?;

            let published_at_str: String = row.get("created_at");
            let published_at = time::OffsetDateTime::parse(
                &published_at_str,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|e| RegistryError::Storage(format!("Failed to parse timestamp: {}", e)))?;

            let display_name = template.name.clone();
            let versioned_template = VersionedTemplate {
                template,
                template_name: row.get::<String, _>("template_id"), // TODO: Use actual template_name column
                display_name,
                version: format!("v{}", row.get::<i64, _>("version")),
                author: row.get("author"),
                forked_from: None, // TODO: Store this in database if needed
                published_at,
                immutable: true,
                is_draft: false,
                schema: None, // TODO: Store this in database if needed
            };

            templates.push(versioned_template);
        }

        Ok(templates)
    }

    async fn list_all_render_jobs(&self) -> Result<Vec<RenderJob>> {
        let rows = sqlx::query(
            "SELECT id, template_id, version, data, data_hash, status, 
                    pdf_s3_key, rendering_latency, created_at, completed_at, error_message
             FROM render_jobs 
             ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to list all render jobs: {}", e)))?;

        let mut jobs = Vec::new();
        for row in rows {
            let template_id_str = row.get::<String, _>("template_id");
            let template_id = TemplateId::from(template_id_str.clone());
            
            // Parse timestamps
            let created_at_str: String = row.get("created_at");
            let created_at = time::OffsetDateTime::parse(
                &created_at_str,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|e| RegistryError::Storage(format!("Failed to parse created timestamp: {}", e)))?;

            let completed_at = if let Some(completed_at_str) = row.get::<Option<String>, _>("completed_at") {
                Some(
                    time::OffsetDateTime::parse(
                        &completed_at_str,
                        &time::format_description::well_known::Rfc3339,
                    )
                    .map_err(|e| RegistryError::Storage(format!("Failed to parse completed timestamp: {}", e)))?
                )
            } else {
                None
            };

            // Parse status
            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "Pending" => RenderStatus::Pending,
                "InProgress" => RenderStatus::InProgress,
                "Completed" => RenderStatus::Completed,
                "Failed" => RenderStatus::Failed,
                _ => RenderStatus::Pending, // Default fallback
            };

            let data_json: String = row.get("data");
            let data = serde_json::from_str(&data_json)
                .map_err(|e| RegistryError::Storage(format!("Failed to deserialize data: {}", e)))?;

            let job = RenderJob {
                id: row.get("id"),
                template_id,
                template_name: template_id_str, // TODO: Add template_name column to render_jobs
                template_version: format!("v{}", row.get::<i64, _>("version")),
                data,
                data_hash: row.get("data_hash"),
                status,
                pdf_s3_key: row.get("pdf_s3_key"),
                rendering_latency: row.get::<Option<i64>, _>("rendering_latency").map(|l| l as u64),
                created_at,
                completed_at,
                error_message: row.get("error_message"),
            };

            jobs.push(job);
        }

        Ok(jobs)
    }

    // === New name:version methods ===

    async fn get_versioned_template_by_name(
        &self,
        template_name: &str,
        version: &str,
    ) -> Result<VersionedTemplate> {
        let row = sqlx::query(
            r#"
            SELECT template_id, template_name, display_name, version, author, created_at, template_data, is_draft
            FROM versioned_templates
            WHERE template_name = ? AND version = ?
        "#,
        )
        .bind(template_name)
        .bind(version)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RegistryError::TemplateNotFound(format!("{}:{}", template_name, version)),
            _ => RegistryError::Storage(format!("Failed to get template: {}", e)),
        })?;

        let template_json: String = row.get("template_data");
        let template = serde_json::from_str(&template_json).map_err(|e| {
            RegistryError::Storage(format!("Failed to deserialize template: {}", e))
        })?;

        let published_at_str: String = row.get("created_at");
        let published_at = time::OffsetDateTime::parse(
            &published_at_str,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| RegistryError::Storage(format!("Failed to parse timestamp: {}", e)))?;

        Ok(VersionedTemplate {
            template,
            template_name: row.get("template_name"),
            display_name: row.get("display_name"),
            version: row.get("version"),
            author: row.get("author"),
            forked_from: None,
            published_at,
            immutable: true,
            is_draft: row.get::<i32, _>("is_draft") == 1,
            schema: None,
        })
    }

    async fn list_template_versions_by_name(&self, template_name: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT version FROM versioned_templates
            WHERE template_name = ? AND is_draft = 0
            ORDER BY version ASC
        "#,
        )
        .bind(template_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to list template versions: {}", e)))?;

        let versions: Vec<String> = rows
            .iter()
            .map(|row| row.get::<String, _>("version"))
            .collect();

        Ok(versions)
    }

    async fn delete_template_version_by_name(&self, template_name: &str, version: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM versioned_templates
            WHERE template_name = ? AND version = ?
        "#,
        )
        .bind(template_name)
        .bind(version)
        .execute(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to delete template: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(RegistryError::TemplateNotFound(format!("{}:{}", template_name, version)));
        }

        Ok(())
    }

    // === Draft Management ===

    async fn save_draft(&self, template: &VersionedTemplate) -> Result<()> {
        // Ensure this is marked as a draft
        if !template.is_draft {
            return Err(RegistryError::Storage("Template must be marked as draft".to_string()));
        }

        self.save_versioned_template(template).await
    }

    async fn get_draft(&self, template_name: &str) -> Result<Option<VersionedTemplate>> {
        match self.get_versioned_template_by_name(template_name, "draft").await {
            Ok(template) => Ok(Some(template)),
            Err(RegistryError::TemplateNotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn delete_draft(&self, template_name: &str) -> Result<()> {
        self.delete_template_version_by_name(template_name, "draft").await
    }

    async fn has_draft(&self, template_name: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM versioned_templates
            WHERE template_name = ? AND version = 'draft'
        "#,
        )
        .bind(template_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to check draft: {}", e)))?;

        Ok(count > 0)
    }

    async fn get_next_version_number(&self, template_name: &str) -> Result<u64> {
        let max_version: Option<String> = sqlx::query_scalar(
            r#"
            SELECT version FROM versioned_templates
            WHERE template_name = ? AND is_draft = 0 AND version LIKE 'v%'
            ORDER BY CAST(SUBSTR(version, 2) AS INTEGER) DESC
            LIMIT 1
        "#,
        )
        .bind(template_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to get next version: {}", e)))?;

        if let Some(version_str) = max_version {
            if let Some(num_str) = version_str.strip_prefix('v') {
                if let Ok(num) = num_str.parse::<u64>() {
                    return Ok(num + 1);
                }
            }
        }

        Ok(1) // First version
    }

    // === New render job methods ===

    async fn find_render_job_by_hash_name(
        &self,
        _template_name: &str,
        _version: &str,
        _data_hash: &str,
    ) -> Result<Option<RenderJob>> {
        // Note: This will need render jobs table to be updated to store template_name
        // For now, fall back to the existing method
        Ok(None)
    }

    async fn list_render_jobs_by_name(
        &self,
        _template_name: &str,
        _version: Option<&str>,
    ) -> Result<Vec<RenderJob>> {
        // Note: This will need render jobs table to be updated to store template_name
        // For now, return empty list
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use papermake::{TemplateBuilder, TemplateId};
    use tempfile::tempdir;

    async fn create_test_storage() -> SqliteStorage {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());
        SqliteStorage::new(&db_url).await.unwrap()
    }

    #[tokio::test]
    async fn test_template_crud() {
        let storage = create_test_storage().await;

        // Create a test template
        let template = TemplateBuilder::new(TemplateId::from("test-template"))
            .name("Test Template")
            .content("Hello #data.name!")
            .build()
            .unwrap();

        let versioned_template = VersionedTemplate::new(template, 1, "alice".to_string());

        // Save template
        storage
            .save_versioned_template(&versioned_template)
            .await
            .unwrap();

        // Retrieve template
        let retrieved = storage
            .get_versioned_template(&TemplateId::from("test-template"), 1)
            .await
            .unwrap();
        assert_eq!(retrieved.template.name, "Test Template");
        assert_eq!(retrieved.version, 1);
        assert_eq!(retrieved.author, "alice");

        // List versions
        let versions = storage
            .list_template_versions(&TemplateId::from("test-template"))
            .await
            .unwrap();
        assert_eq!(versions, vec![1]);

        // Delete template
        storage
            .delete_template_version(&TemplateId::from("test-template"), 1)
            .await
            .unwrap();

        // Verify deletion
        let result = storage
            .get_versioned_template(&TemplateId::from("test-template"), 1)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_render_job_crud() {
        let storage = create_test_storage().await;

        let job = RenderJob::new(
            TemplateId::from("test-template"),
            1,
            serde_json::json!({"name": "test"}),
        );

        // Save render job
        storage.save_render_job(&job).await.unwrap();

        // Retrieve render job
        let retrieved = storage.get_render_job(&job.id).await.unwrap();
        assert_eq!(retrieved.id, job.id);
        assert_eq!(retrieved.template_id, job.template_id);
        assert_eq!(retrieved.data_hash, job.data_hash);

        // Find by hash
        let found = storage
            .find_render_job_by_hash(&job.template_id, job.template_version, &job.data_hash)
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, job.id);

        // List render jobs
        let jobs = storage
            .list_render_jobs(&job.template_id, None)
            .await
            .unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, job.id);
    }
}
