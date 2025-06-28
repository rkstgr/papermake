//! SQLite metadata storage implementation - New TemplateRef-first design

use super::MetadataStorage;
use crate::{RegistryError, entities::*, error::Result, template_ref::TemplateRef};
use async_trait::async_trait;
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
    pub async fn from_env() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL").map_err(|_| {
            RegistryError::Storage("DATABASE_URL environment variable not set".to_string())
        })?;
        Self::new(&database_url).await
    }

    /// Initialize database schema
    async fn init_schema(&self) -> Result<()> {
        // Create templates table with TemplateRef-first design
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
                forked_from TEXT                     -- TemplateRef string if forked
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to create templates table: {}", e)))?;

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

        let forked_from_str = template.forked_from.as_ref().map(|f| f.to_string());

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO templates
            (template_ref, org, name, tag, digest, author, published_at, template_data, forked_from)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .bind(forked_from_str)
        .execute(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to save template: {}", e)))?;

        Ok(())
    }

    async fn get_template(&self, template_ref: &str) -> Result<TemplateEntry> {
        let row = sqlx::query(
            r#"
            SELECT template_ref, org, name, tag, digest, author, published_at, template_data, is_draft, forked_from
            FROM templates
            WHERE template_ref = ?
        "#,
        )
        .bind(template_ref)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RegistryError::TemplateNotFound(template_ref.to_string()),
            _ => RegistryError::Storage(format!("Failed to get template: {}", e)),
        })?;

        let template_json: String = row.get("template_data");
        let template = serde_json::from_str(&template_json).map_err(|e| {
            RegistryError::Storage(format!("Failed to deserialize template: {}", e))
        })?;

        let published_at_str: String = row.get("published_at");
        let published_at = time::OffsetDateTime::parse(
            &published_at_str,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| RegistryError::Storage(format!("Failed to parse timestamp: {}", e)))?;

        let template_ref_parsed: TemplateRef = template_ref.parse().map_err(|_| {
            RegistryError::Storage(format!("Invalid template reference: {}", template_ref))
        })?;

        let forked_from = row
            .get::<Option<String>, _>("forked_from")
            .and_then(|s| s.parse().ok());

        Ok(TemplateEntry {
            template,
            template_ref: template_ref_parsed,
            author: row.get("author"),
            forked_from,
            published_at,
        })
    }

    async fn delete_template(&self, template_ref: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM templates
            WHERE template_ref = ?
        "#,
        )
        .bind(template_ref)
        .execute(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to delete template: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(RegistryError::TemplateNotFound(template_ref.to_string()));
        }

        Ok(())
    }

    async fn list_template_tags(&self, name: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT tag FROM templates
            WHERE name = ?
            ORDER BY tag ASC
        "#,
        )
        .bind(name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to list template tags: {}", e)))?;

        let tags: Vec<String> = rows.iter().map(|row| row.get("tag")).collect();

        Ok(tags)
    }

    async fn search_templates(&self, query: &str) -> Result<Vec<TemplateEntry>> {
        let search_pattern = format!("%{}%", query);
        let rows = sqlx::query(
            r#"
            SELECT template_ref, org, name, tag, digest, author, published_at, template_data, forked_from
            FROM templates
            WHERE name LIKE ? OR template_ref LIKE ?
            ORDER BY name, tag
        "#,
        )
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to search templates: {}", e)))?;

        let mut templates = Vec::new();
        for row in rows {
            let template_json: String = row.get("template_data");
            let template = serde_json::from_str(&template_json).map_err(|e| {
                RegistryError::Storage(format!("Failed to deserialize template: {}", e))
            })?;

            let published_at_str: String = row.get("published_at");
            let published_at = time::OffsetDateTime::parse(
                &published_at_str,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|e| RegistryError::Storage(format!("Failed to parse timestamp: {}", e)))?;

            let template_ref_str: String = row.get("template_ref");
            let template_ref: TemplateRef = template_ref_str.parse().map_err(|_| {
                RegistryError::Storage(format!("Invalid template reference: {}", template_ref_str))
            })?;

            let forked_from = row
                .get::<Option<String>, _>("forked_from")
                .and_then(|s| s.parse().ok());

            templates.push(TemplateEntry {
                template,
                template_ref,
                author: row.get("author"),
                forked_from,
                published_at,
            });
        }

        Ok(templates)
    }

    async fn get_next_version_number(&self, name: &str) -> Result<u64> {
        let row = sqlx::query(
            r#"
            SELECT tag FROM templates
            WHERE name = ? AND tag LIKE 'v%'
            ORDER BY CAST(SUBSTR(tag, 2) AS INTEGER) DESC
            LIMIT 1
        "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to get next version: {}", e)))?;

        if let Some(row) = row {
            let tag: String = row.get("tag");
            if let Some(version_str) = tag.strip_prefix('v') {
                if let Ok(version) = version_str.parse::<u64>() {
                    return Ok(version + 1);
                }
            }
        }

        Ok(1) // Start with version 1
    }

    async fn save_render_job(&self, job: &RenderJob) -> Result<()> {
        let data_json = serde_json::to_string(&job.data).map_err(|e| {
            RegistryError::Storage(format!("Failed to serialize render job data: {}", e))
        })?;

        let status_str = match &job.status {
            RenderStatus::Pending => "pending",
            RenderStatus::InProgress => "in_progress",
            RenderStatus::Completed => "completed",
            RenderStatus::Failed => "failed",
        };

        let created_at = job
            .created_at
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| RegistryError::Storage(format!("Failed to format timestamp: {}", e)))?;

        let completed_at = job
            .completed_at
            .map(|dt| dt.format(&time::format_description::well_known::Rfc3339))
            .transpose()
            .map_err(|e| RegistryError::Storage(format!("Failed to format timestamp: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO render_jobs
            (id, template_ref, data, data_hash, status, pdf_s3_key, rendering_latency, created_at, completed_at, error_message)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&job.id)
        .bind(job.template_ref.to_string())
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
        let row = sqlx::query(
            r#"
            SELECT id, template_ref, data, data_hash, status, pdf_s3_key, rendering_latency, created_at, completed_at, error_message
            FROM render_jobs
            WHERE id = ?
        "#,
        )
        .bind(job_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RegistryError::RenderJobNotFound(job_id.to_string()),
            _ => RegistryError::Storage(format!("Failed to get render job: {}", e)),
        })?;

        let data_json: String = row.get("data");
        let data = serde_json::from_str(&data_json).map_err(|e| {
            RegistryError::Storage(format!("Failed to deserialize render job data: {}", e))
        })?;

        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "pending" => RenderStatus::Pending,
            "in_progress" => RenderStatus::InProgress,
            "completed" => RenderStatus::Completed,
            "failed" => RenderStatus::Failed,
            _ => {
                return Err(RegistryError::Storage(format!(
                    "Invalid render status: {}",
                    status_str
                )));
            }
        };

        let created_at_str: String = row.get("created_at");
        let created_at = time::OffsetDateTime::parse(
            &created_at_str,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| RegistryError::Storage(format!("Failed to parse timestamp: {}", e)))?;

        let completed_at = row
            .get::<Option<String>, _>("completed_at")
            .as_ref()
            .map(|s| time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339))
            .transpose()
            .map_err(|e| RegistryError::Storage(format!("Failed to parse timestamp: {}", e)))?;

        let template_ref_str: String = row.get("template_ref");
        let template_ref: TemplateRef = template_ref_str.parse().map_err(|_| {
            RegistryError::Storage(format!("Invalid template reference: {}", template_ref_str))
        })?;

        Ok(RenderJob {
            id: row.get("id"),
            template_ref,
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

    async fn find_cached_render(
        &self,
        template_ref: &str,
        data_hash: &str,
    ) -> Result<Option<RenderJob>> {
        let row = sqlx::query(
            r#"
            SELECT id, template_ref, data, data_hash, status, pdf_s3_key, rendering_latency, created_at, completed_at, error_message
            FROM render_jobs
            WHERE template_ref = ? AND data_hash = ? AND status = 'completed'
            ORDER BY created_at DESC
            LIMIT 1
        "#,
        )
        .bind(template_ref)
        .bind(data_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to find cached render: {}", e)))?;

        if let Some(row) = row {
            // Reuse the get_render_job logic
            let job_id: String = row.get("id");
            Ok(Some(self.get_render_job(&job_id).await?))
        } else {
            Ok(None)
        }
    }

    async fn list_render_jobs(&self, template_ref: &str) -> Result<Vec<RenderJob>> {
        let rows = sqlx::query(
            r#"
            SELECT id FROM render_jobs
            WHERE template_ref = ?
            ORDER BY created_at DESC
        "#,
        )
        .bind(template_ref)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to list render jobs: {}", e)))?;

        let mut jobs = Vec::new();
        for row in rows {
            let job_id: String = row.get("id");
            jobs.push(self.get_render_job(&job_id).await?);
        }

        Ok(jobs)
    }

    async fn list_all_templates(&self) -> Result<Vec<TemplateEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT template_ref FROM templates
            ORDER BY name, tag
        "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to list all templates: {}", e)))?;

        let mut templates = Vec::new();
        for row in rows {
            let template_ref: String = row.get("template_ref");
            templates.push(self.get_template(&template_ref).await?);
        }

        Ok(templates)
    }

    async fn list_all_render_jobs(&self) -> Result<Vec<RenderJob>> {
        let rows = sqlx::query(
            r#"
            SELECT id FROM render_jobs
            ORDER BY created_at DESC
        "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RegistryError::Storage(format!("Failed to list all render jobs: {}", e)))?;

        let mut jobs = Vec::new();
        for row in rows {
            let job_id: String = row.get("id");
            jobs.push(self.get_render_job(&job_id).await?);
        }

        Ok(jobs)
    }
}
