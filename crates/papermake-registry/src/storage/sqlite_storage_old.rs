//! SQLite metadata storage implementation
//!
//! This module provides a SQLite-based implementation of the MetadataStorage trait.
//! It stores template metadata and render jobs in a local SQLite database file.

use super::MetadataStorage;
use crate::{RegistryError, entities::*, error::Result, template_ref::TemplateRef};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions};
use std::str::FromStr;

/// Helper function to parse version strings to u64 for backward compatibility
/// Supports "v1", "v2", "1", "2", etc. Returns None for non-numeric versions like "draft" or "latest"
fn parse_version_to_u64(version_str: &str) -> Option<u64> {
    let clean_version = if version_str.starts_with('v') {
        &version_str[1..]
    } else {
        version_str
    };

    clean_version.parse::<u64>().ok()
}

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
            CREATE TABLE IF NOT EXISTS templates (
                template_ref TEXT PRIMARY KEY,       -- Full TemplateRef: "org/name:tag[@digest]"
                org TEXT,                            -- Organization (nullable)
                name TEXT NOT NULL,                  -- Template name
                tag TEXT NOT NULL,                   -- Version tag
                digest TEXT,                         -- Content digest (nullable)
                author TEXT NOT NULL,
                published_at TEXT NOT NULL,
                template_data TEXT NOT NULL,         -- JSON of Template struct
                is_draft INTEGER NOT NULL DEFAULT 0, -- SQLite boolean (0/1)
                forked_from TEXT                     -- TemplateRef string if forked
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            RegistryError::Storage(format!("Failed to create templates table: {}", e))
        })?;

        // Create render_jobs table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS render_jobs (
                id TEXT PRIMARY KEY,
                template_ref TEXT NOT NULL,           -- Full TemplateRef string
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

        // Create indexes for performance
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_templates_name_tag ON templates (name, tag);
            CREATE INDEX IF NOT EXISTS idx_templates_digest ON templates (digest) WHERE digest IS NOT NULL;
            CREATE INDEX IF NOT EXISTS idx_templates_org_name ON templates (org, name) WHERE org IS NOT NULL;
            CREATE INDEX IF NOT EXISTS idx_render_jobs_template_hash ON render_jobs (template_ref, data_hash);
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            RegistryError::Storage(format!("Failed to create indexes: {}", e))
        })?;

        Ok(())

    }

}

#[async_trait]
impl MetadataStorage for SqliteStorage {
    async fn save_template(&self, template: &TemplateEntry) -> Result<()> {
        let template_json = serde_json::to_string(&template.template)
            .map_err(|e| RegistryError::Storage(format!("Failed to serialize template: {}", e)))?;

        let published_at = template
            .published_at
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| RegistryError::Storage(format!("Failed to format timestamp: {}", e)))?;

        let forked_from_str = template
            .forked_from
            .as_ref()
            .map(|f| f.to_string());

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO templates
            (template_ref, org, name, tag, digest, author, published_at, template_data, is_draft, forked_from)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(template.template_ref.to_string())
        .bind(&template.template_ref.org)
        .bind(&template.template_ref.name)
        .bind(&template.template_ref.tag)
        .bind(&template.template_ref.digest)
        .bind(&template.author)
        .bind(published_at)
        .bind(template_json)
        .bind(if template.is_draft { 1 } else { 0 })
        .bind(forked_from_str)
        .execute(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to save template: {}", e)))?;

        Ok(())
    }

    async fn get_template_entry(
        &self,
        template_ref: &str,
    ) -> Result<VersionedTemplate> {
        let row = sqlx::query(
            r#"
            SELECT template_id, template_name, display_name, tag, author, created_at, template_data, is_draft, forked_from
            FROM versioned_templates
            WHERE template_id = ? AND tag = ?
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

        let forked_from = if let Some(forked_from_json) =
            row.get::<Option<String>, _>("forked_from")
        {
            serde_json::from_str::<(String, String)>(&forked_from_json)
                .map_err(|e| RegistryError::Storage(format!("Failed to parse forked_from: {}", e)))
                .ok()
        } else {
            None
        };

        Ok(VersionedTemplate {
            template,
            template_name: row.get("template_name"),
            display_name: row.get("display_name"),
            tag: row.get("tag"),
            author: row.get("author"),
            forked_from,
            published_at,
            immutable: true,
            is_draft: row.get::<i32, _>("is_draft") == 1,
            schema: None, // TODO: Store this in database if needed
        })
    }

    async fn list_template_entries(&self, name: &str) -> Result<Vec<TemplateEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT tag FROM versioned_templates
            WHERE template_id = ?
            ORDER BY tag ASC
        "#,
        )
        .bind(id.as_ref())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to list template versions: {}", e)))?;

        let versions: Vec<u64> = rows
            .iter()
            .filter_map(|row| {
                let tag_str: String = row.get("tag");
                parse_version_to_u64(&tag_str)
            })
            .collect();

        Ok(versions)
    }

    async fn delete_template_entry(&self, template_ref: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM versioned_templates
            WHERE template_id = ? AND tag = ?
        "#,
        )
        .bind(id.as_ref())
        .bind(format!("v{}", version))
        .execute(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to delete template: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(RegistryError::TemplateNotFound(id.as_ref().to_string()));
        }

        Ok(())
    }

    async fn search_templates(&self, query: &str) -> Result<Vec<TemplateEntry>> {
        let search_pattern = format!("%{}%", query);

        let rows = sqlx::query(
            r#"
            SELECT template_id, tag FROM versioned_templates
            WHERE template_data LIKE ?
            ORDER BY template_id, tag DESC
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
                let tag_str: String = row.get("tag");
                let version = parse_version_to_u64(&tag_str).unwrap_or(0);
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
            (id, template_name, template_tag, data, data_hash, status, pdf_s3_key, rendering_latency, created_at, completed_at, error_message)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&job.id)
        .bind(&job.template_name)
        .bind(&job.template_tag)
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
            SELECT id, template_name, template_tag, data, data_hash, status, pdf_s3_key, rendering_latency, created_at, completed_at, error_message
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
            template_name: row.get("template_name"),
            template_tag: row.get("template_tag"),
            data,
            data_hash: row.get("data_hash"),
            status,
            pdf_s3_key: row.get("pdf_s3_key"),
            rendering_latency: row
                .get::<Option<i64>, _>("rendering_latency")
                .map(|l| l as u64),
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
            SELECT id, template_name, template_tag, data, data_hash, status, pdf_s3_key, rendering_latency, created_at, completed_at, error_message
            FROM render_jobs
            WHERE template_name = ? AND template_tag = ? AND data_hash = ?
            ORDER BY created_at DESC
            LIMIT 1
        "#)
        .bind(template_id.as_ref())
        .bind(format!("v{}", version))
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
            let data = serde_json::from_str(&data_json).map_err(|e| {
                RegistryError::Storage(format!("Failed to deserialize data: {}", e))
            })?;

            Ok(Some(RenderJob {
                id: row.get("id"),
                template_name: row.get("template_name"),
                template_tag: row.get("template_tag"),
                data,
                data_hash: row.get("data_hash"),
                status,
                pdf_s3_key: row.get("pdf_s3_key"),
                rendering_latency: row
                    .get::<Option<i64>, _>("rendering_latency")
                    .map(|l| l as u64),
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
                SELECT id, template_name, template_tag, data, data_hash, status, pdf_s3_key, rendering_latency, created_at, completed_at, error_message
                FROM render_jobs
                WHERE template_name = ? AND template_tag = ?
                ORDER BY created_at DESC
            "#)
            .bind(template_id.as_ref())
            .bind(format!("v{}", version))
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(r#"
                SELECT id, template_name, template_tag, data, data_hash, status, pdf_s3_key, rendering_latency, created_at, completed_at, error_message
                FROM render_jobs
                WHERE template_name = ?
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
            let data = serde_json::from_str(&data_json).map_err(|e| {
                RegistryError::Storage(format!("Failed to deserialize data: {}", e))
            })?;

            jobs.push(RenderJob {
                id: row.get("id"),
                template_name: row.get("template_name"),
                template_tag: row.get("template_tag"),
                data,
                data_hash: row.get("data_hash"),
                status,
                pdf_s3_key: row.get("pdf_s3_key"),
                rendering_latency: row
                    .get::<Option<i64>, _>("rendering_latency")
                    .map(|l| l as u64),
                created_at,
                completed_at,
                error_message: row.get("error_message"),
            });
        }

        Ok(jobs)
    }

    async fn list_all_templates(&self) -> Result<Vec<VersionedTemplate>> {
        let rows = sqlx::query(
            "SELECT template_id, template_name, display_name, tag, author, created_at, template_data, is_draft, forked_from
             FROM versioned_templates
             WHERE (template_id, tag) IN (
                 SELECT template_id, MAX(tag)
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
            let template: papermake::Template =
                serde_json::from_str(&template_json).map_err(|e| {
                    RegistryError::Storage(format!("Failed to deserialize template: {}", e))
                })?;

            let published_at_str: String = row.get("created_at");
            let published_at = time::OffsetDateTime::parse(
                &published_at_str,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|e| RegistryError::Storage(format!("Failed to parse timestamp: {}", e)))?;

            let forked_from =
                if let Some(forked_from_json) = row.get::<Option<String>, _>("forked_from") {
                    serde_json::from_str::<(String, String)>(&forked_from_json)
                        .map_err(|e| {
                            RegistryError::Storage(format!("Failed to parse forked_from: {}", e))
                        })
                        .ok()
                } else {
                    None
                };

            let versioned_template = VersionedTemplate {
                template,
                template_name: row.get("template_name"),
                display_name: row.get("display_name"),
                tag: row.get("tag"),
                author: row.get("author"),
                forked_from,
                published_at,
                immutable: true,
                is_draft: row.get::<i32, _>("is_draft") == 1,
                schema: None, // TODO: Store this in database if needed
            };

            templates.push(versioned_template);
        }

        Ok(templates)
    }

    async fn list_all_render_jobs(&self) -> Result<Vec<RenderJob>> {
        let rows = sqlx::query(
            "SELECT id, template_name, template_tag, data, data_hash, status,
                    pdf_s3_key, rendering_latency, created_at, completed_at, error_message
             FROM render_jobs
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to list all render jobs: {}", e)))?;

        let mut jobs = Vec::new();
        for row in rows {

            // Parse timestamps
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
            let data = serde_json::from_str(&data_json).map_err(|e| {
                RegistryError::Storage(format!("Failed to deserialize data: {}", e))
            })?;

            let job = RenderJob {
                id: row.get("id"),
                template_name: row.get("template_name"),
                template_tag: row.get("template_tag"),
                data,
                data_hash: row.get("data_hash"),
                status,
                pdf_s3_key: row.get("pdf_s3_key"),
                rendering_latency: row
                    .get::<Option<i64>, _>("rendering_latency")
                    .map(|l| l as u64),
                created_at,
                completed_at,
                error_message: row.get("error_message"),
            };

            jobs.push(job);
        }

        Ok(jobs)
    }

    // === New name:version methods ===

    async fn get_template_entry_by_name_tag(
        &self,
        name: &str,
        tag: &str,
    ) -> Result<TemplateEntry> {
        let row = sqlx::query(
            r#"
            SELECT template_id, template_name, display_name, tag, author, created_at, template_data, is_draft, forked_from
            FROM versioned_templates
            WHERE template_name = ? AND tag = ?
        "#,
        )
        .bind(template_name)
        .bind(tag)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RegistryError::TemplateNotFound(format!("{}:{}", template_name, tag)),
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

        let forked_from = if let Some(forked_from_json) =
            row.get::<Option<String>, _>("forked_from")
        {
            serde_json::from_str::<(String, String)>(&forked_from_json)
                .map_err(|e| RegistryError::Storage(format!("Failed to parse forked_from: {}", e)))
                .ok()
        } else {
            None
        };

        Ok(VersionedTemplate {
            template,
            template_name: row.get("template_name"),
            display_name: row.get("display_name"),
            tag: row.get("tag"),
            author: row.get("author"),
            forked_from,
            published_at,
            immutable: true,
            is_draft: row.get::<i32, _>("is_draft") == 1,
            schema: None,
        })
    }

    async fn list_template_tags(&self, name: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT tag FROM versioned_templates
            WHERE template_name = ? AND is_draft = 0
            ORDER BY tag ASC
        "#,
        )
        .bind(template_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to list template tags: {}", e)))?;

        let tags: Vec<String> = rows.iter().map(|row| row.get::<String, _>("tag")).collect();

        Ok(tags)
    }

    async fn delete_template_entry_by_name_tag(&self, name: &str, tag: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM versioned_templates
            WHERE template_name = ? AND tag = ?
        "#,
        )
        .bind(template_name)
        .bind(tag)
        .execute(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to delete template: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(RegistryError::TemplateNotFound(format!(
                "{}:{}",
                template_name, tag
            )));
        }

        Ok(())
    }

    // === Draft Management ===

    async fn save_draft(&self, template: &VersionedTemplate) -> Result<()> {
        // Ensure this is marked as a draft
        if !template.is_draft {
            return Err(RegistryError::Storage(
                "Template must be marked as draft".to_string(),
            ));
        }

        self.save_versioned_template(template).await
    }

    async fn get_draft(&self, template_name: &str) -> Result<Option<VersionedTemplate>> {
        match self
            .get_versioned_template_by_name(template_name, "draft")
            .await
        {
            Ok(template) => Ok(Some(template)),
            Err(RegistryError::TemplateNotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn delete_draft(&self, template_name: &str) -> Result<()> {
        self.delete_template_tag_by_name(template_name, "draft")
            .await
    }

    async fn has_draft(&self, template_name: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM versioned_templates
            WHERE template_name = ? AND tag = 'draft'
        "#,
        )
        .bind(template_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to check draft: {}", e)))?;

        Ok(count > 0)
    }

    async fn get_next_tag_number(&self, template_name: &str) -> Result<u64> {
        let max_tag: Option<String> = sqlx::query_scalar(
            r#"
            SELECT tag FROM versioned_templates
            WHERE template_name = ? AND is_draft = 0 AND tag LIKE 'v%'
            ORDER BY CAST(SUBSTR(tag, 2) AS INTEGER) DESC
            LIMIT 1
        "#,
        )
        .bind(template_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to get next tag: {}", e)))?;

        if let Some(tag_str) = max_tag {
            if let Some(num_str) = tag_str.strip_prefix('v') {
                if let Ok(num) = num_str.parse::<u64>() {
                    return Ok(num + 1);
                }
            }
        }

        Ok(1) // First tag
    }

    // === New render job methods ===

    async fn find_render_job_by_hash_name(
        &self,
        _template_name: &str,
        _tag: &str,
        _data_hash: &str,
    ) -> Result<Option<RenderJob>> {
        // Note: This will need render jobs table to be updated to store template_name
        // For now, fall back to the existing method
        Ok(None)
    }

    async fn list_render_jobs_by_name(
        &self,
        _template_name: &str,
        _tag: Option<&str>,
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

        let versioned_template = VersionedTemplate::new(
            template,
            "test-template".to_string(),
            "Test Template".to_string(),
            "v1".to_string(),
            "alice".to_string(),
        );

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
        assert_eq!(retrieved.tag, "v1");
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
            "test-template".to_string(),
            "v1".to_string(),
            serde_json::json!({"name": "test"}),
        );

        // Save render job
        storage.save_render_job(&job).await.unwrap();

        // Retrieve render job
        let retrieved = storage.get_render_job(&job.id).await.unwrap();
        assert_eq!(retrieved.id, job.id);
        assert_eq!(retrieved.template_id, job.template_id);
        assert_eq!(retrieved.data_hash, job.data_hash);

        // List render jobs
        let jobs = storage
            .list_render_jobs(&job.template_id, None)
            .await
            .unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, job.id);
    }
}
