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
                template_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                author TEXT NOT NULL,
                created_at TEXT NOT NULL,
                template_data TEXT NOT NULL, -- JSON
                PRIMARY KEY (template_id, version)
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
                data_hash TEXT NOT NULL,
                status TEXT NOT NULL,
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

        // Create indexes for efficient queries
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_templates_id ON versioned_templates(template_id)",
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
            (template_id, version, author, created_at, template_data)
            VALUES (?, ?, ?, ?, ?)
        "#,
        )
        .bind(template.template.id.as_ref())
        .bind(template.version as i64)
        .bind(&template.author)
        .bind(published_at)
        .bind(template_json)
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
            SELECT template_id, version, author, created_at, template_data
            FROM versioned_templates
            WHERE template_id = ? AND version = ?
        "#,
        )
        .bind(id.as_ref())
        .bind(version as i64)
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
            version: row.get::<i64, _>("version") as u64,
            author: row.get("author"),
            forked_from: None, // TODO: Store this in database if needed
            published_at,
            immutable: true,
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

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO render_jobs
            (id, template_id, version, data_hash, status, created_at, completed_at, error_message)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&job.id)
        .bind(job.template_id.as_ref())
        .bind(job.template_version as i64)
        .bind(&job.data_hash)
        .bind(status_str)
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
            SELECT id, template_id, version, data_hash, status, created_at, completed_at, error_message
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

        Ok(RenderJob {
            id: row.get("id"),
            template_id: TemplateId::from(row.get::<String, _>("template_id")),
            template_version: row.get::<i64, _>("version") as u64,
            data: serde_json::Value::Null, // TODO: Store actual data if needed
            data_hash: row.get("data_hash"),
            status,
            pdf_s3_key: None, // TODO: Store this in database
            rendering_latency: None, // TODO: Store this in database
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
            SELECT id, template_id, version, data_hash, status, created_at, completed_at, error_message
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

            Ok(Some(RenderJob {
                id: row.get("id"),
                template_id: TemplateId::from(row.get::<String, _>("template_id")),
                template_version: row.get::<i64, _>("version") as u64,
                data: serde_json::Value::Null, // TODO: Store actual data if needed
                data_hash: row.get("data_hash"),
                status,
                pdf_s3_key: None, // TODO: Store this in database
                rendering_latency: None, // TODO: Store this in database
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
                SELECT id, template_id, version, data_hash, status, created_at, completed_at, error_message
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
                SELECT id, template_id, version, data_hash, status, created_at, completed_at, error_message
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

            jobs.push(RenderJob {
                id: row.get("id"),
                template_id: TemplateId::from(row.get::<String, _>("template_id")),
                template_version: row.get::<i64, _>("version") as u64,
                data: serde_json::Value::Null, // TODO: Store actual data if needed
                data_hash: row.get("data_hash"),
                status,
                pdf_s3_key: None, // TODO: Store this in database
                rendering_latency: None, // TODO: Store this in database
                created_at,
                completed_at,
                error_message: row.get("error_message"),
            });
        }

        Ok(jobs)
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
