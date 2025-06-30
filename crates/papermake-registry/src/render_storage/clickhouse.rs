//! ClickHouse implementation of RenderStorage trait

use async_trait::async_trait;
use clickhouse::{Client, Row};
use serde::{Deserialize, Serialize};
use std::env;
use time::{Duration, OffsetDateTime};

use super::{
    DurationPoint, RenderRecord, RenderStorage, RenderStorageError, TemplateStats, VolumePoint,
};

/// ClickHouse storage implementation for render records
#[derive(Clone)]
pub struct ClickHouseStorage {
    client: Client,
}

impl std::fmt::Debug for ClickHouseStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClickHouseStorage")
            .field("client", &"<ClickHouse Client>")
            .finish()
    }
}

/// ClickHouse row structure for render records
#[derive(Debug, Row, Serialize, Deserialize)]
struct ClickHouseRenderRecord {
    render_id: String,
    timestamp: u64, // Unix timestamp in milliseconds
    template_ref: String,
    template_name: String,
    template_tag: String,
    manifest_hash: String,
    data_hash: String,
    pdf_hash: String,
    success: u8, // 0 or 1
    duration_ms: u32,
    pdf_size_bytes: u32,
    error: String,
}

impl From<RenderRecord> for ClickHouseRenderRecord {
    fn from(record: RenderRecord) -> Self {
        Self {
            render_id: record.render_id,
            timestamp: record.timestamp.unix_timestamp_nanos() as u64 / 1_000_000, // Convert to milliseconds
            template_ref: record.template_ref,
            template_name: record.template_name,
            template_tag: record.template_tag,
            manifest_hash: record.manifest_hash,
            data_hash: record.data_hash,
            pdf_hash: record.pdf_hash,
            success: if record.success { 1 } else { 0 },
            duration_ms: record.duration_ms,
            pdf_size_bytes: record.pdf_size_bytes,
            error: record.error.unwrap_or_default(),
        }
    }
}

impl TryFrom<ClickHouseRenderRecord> for RenderRecord {
    type Error = RenderStorageError;

    fn try_from(ch_record: ClickHouseRenderRecord) -> Result<Self, Self::Error> {
        let timestamp = OffsetDateTime::from_unix_timestamp_nanos((ch_record.timestamp * 1_000_000) as i128)
            .map_err(|e| RenderStorageError::Query(format!("Invalid timestamp: {}", e)))?;

        Ok(Self {
            render_id: ch_record.render_id,
            timestamp,
            template_ref: ch_record.template_ref,
            template_name: ch_record.template_name,
            template_tag: ch_record.template_tag,
            manifest_hash: ch_record.manifest_hash,
            data_hash: ch_record.data_hash,
            pdf_hash: ch_record.pdf_hash,
            success: ch_record.success == 1,
            duration_ms: ch_record.duration_ms,
            pdf_size_bytes: ch_record.pdf_size_bytes,
            error: if ch_record.error.is_empty() {
                None
            } else {
                Some(ch_record.error)
            },
        })
    }
}

impl ClickHouseStorage {
    /// Create a new ClickHouse storage instance from environment variables
    pub fn from_env() -> Result<Self, RenderStorageError> {
        let url = env::var("CLICKHOUSE_URL")
            .unwrap_or_else(|_| "http://localhost:8123".to_string());
        
        let user = env::var("CLICKHOUSE_USER")
            .unwrap_or_else(|_| "default".to_string());
        
        let password = env::var("CLICKHOUSE_PASSWORD")
            .unwrap_or_else(|_| "".to_string());
        
        let database = env::var("CLICKHOUSE_DATABASE")
            .unwrap_or_else(|_| "papermake".to_string());

        let mut client = Client::default()
            .with_url(url)
            .with_user(user)
            .with_database(database);

        if !password.is_empty() {
            client = client.with_password(password);
        }

        Ok(Self { client })
    }

    /// Create a new ClickHouse storage instance with explicit configuration
    pub fn new(
        url: impl Into<String>,
        user: impl Into<String>,
        password: impl Into<String>,
        database: impl Into<String>,
    ) -> Self {
        let client = Client::default()
            .with_url(url.into())
            .with_user(user.into())
            .with_password(password.into())
            .with_database(database.into());

        Self { client }
    }

    /// Initialize the database schema
    pub async fn init_schema(&self) -> Result<(), RenderStorageError> {
        let create_table_sql = r#"
            CREATE TABLE IF NOT EXISTS renders (
                render_id String,
                timestamp UInt64,
                template_ref String,
                template_name String,
                template_tag String,
                manifest_hash String,
                data_hash String,
                pdf_hash String,
                success UInt8,
                duration_ms UInt32,
                pdf_size_bytes UInt32,
                error String
            ) ENGINE = MergeTree()
            PARTITION BY toYYYYMM(toDateTime(timestamp / 1000))
            ORDER BY (timestamp, template_name)
        "#;

        self.client
            .query(create_table_sql)
            .execute()
            .await
            .map_err(|e| RenderStorageError::Query(format!("Failed to create table: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl RenderStorage for ClickHouseStorage {
    async fn store_render(&self, record: RenderRecord) -> Result<(), RenderStorageError> {
        let ch_record = ClickHouseRenderRecord::from(record);
        
        let mut insert = self.client.insert("renders")?;
        insert.write(&ch_record).await?;
        insert.end().await?;
        
        Ok(())
    }

    async fn get_render(&self, render_id: &str) -> Result<Option<RenderRecord>, RenderStorageError> {
        let query = "SELECT * FROM renders WHERE render_id = ? LIMIT 1";
        
        let mut cursor = self.client
            .query(query)
            .bind(render_id)
            .fetch::<ClickHouseRenderRecord>()?;

        if let Some(ch_record) = cursor.next().await? {
            Ok(Some(ch_record.try_into()?))
        } else {
            Ok(None)
        }
    }

    async fn list_recent_renders(&self, limit: u32) -> Result<Vec<RenderRecord>, RenderStorageError> {
        let query = "SELECT * FROM renders ORDER BY timestamp DESC LIMIT ?";
        
        let mut cursor = self.client
            .query(query)
            .bind(limit)
            .fetch::<ClickHouseRenderRecord>()?;

        let mut records = Vec::new();
        while let Some(ch_record) = cursor.next().await? {
            records.push(ch_record.try_into()?);
        }

        Ok(records)
    }

    async fn list_template_renders(
        &self,
        template_name: &str,
        limit: u32,
    ) -> Result<Vec<RenderRecord>, RenderStorageError> {
        let query = "SELECT * FROM renders WHERE template_name = ? ORDER BY timestamp DESC LIMIT ?";
        
        let mut cursor = self.client
            .query(query)
            .bind(template_name)
            .bind(limit)
            .fetch::<ClickHouseRenderRecord>()?;

        let mut records = Vec::new();
        while let Some(ch_record) = cursor.next().await? {
            records.push(ch_record.try_into()?);
        }

        Ok(records)
    }

    async fn render_volume_over_time(&self, days: u32) -> Result<Vec<VolumePoint>, RenderStorageError> {
        let cutoff_timestamp = (OffsetDateTime::now_utc() - Duration::days(days as i64))
            .unix_timestamp_nanos() as u64 / 1_000_000;

        let query = r#"
            SELECT 
                toDate(toDateTime(timestamp / 1000)) as date,
                count() as renders
            FROM renders 
            WHERE timestamp >= ?
            GROUP BY date
            ORDER BY date
        "#;

        #[derive(Row, Deserialize)]
        struct VolumeRow {
            date: u16, // ClickHouse date as days since 1900-01-01
            renders: u64,
        }

        let mut cursor = self.client
            .query(query)
            .bind(cutoff_timestamp)
            .fetch::<VolumeRow>()?;

        let mut points = Vec::new();
        while let Some(row) = cursor.next().await? {
            // Convert ClickHouse date (days since 1900-01-01) to time::Date
            let days_since_1900 = row.date as i32;
            let days_since_unix_epoch = days_since_1900 - 25567; // Days from 1900-01-01 to 1970-01-01
            
            if let Ok(date) = time::Date::from_julian_day(days_since_unix_epoch + 2440588) { // Julian day adjustment
                points.push(VolumePoint {
                    date,
                    renders: row.renders,
                });
            }
        }

        Ok(points)
    }

    async fn total_renders_per_template(&self) -> Result<Vec<TemplateStats>, RenderStorageError> {
        let query = r#"
            SELECT 
                template_name,
                count() as total_renders
            FROM renders 
            GROUP BY template_name
            ORDER BY total_renders DESC
        "#;

        #[derive(Row, Deserialize)]
        struct TemplateRow {
            template_name: String,
            total_renders: u64,
        }

        let mut cursor = self.client
            .query(query)
            .fetch::<TemplateRow>()?;

        let mut stats = Vec::new();
        while let Some(row) = cursor.next().await? {
            stats.push(TemplateStats {
                template_name: row.template_name,
                total_renders: row.total_renders,
            });
        }

        Ok(stats)
    }

    async fn average_duration_over_time(
        &self,
        days: u32,
    ) -> Result<Vec<DurationPoint>, RenderStorageError> {
        let cutoff_timestamp = (OffsetDateTime::now_utc() - Duration::days(days as i64))
            .unix_timestamp_nanos() as u64 / 1_000_000;

        let query = r#"
            SELECT 
                toDate(toDateTime(timestamp / 1000)) as date,
                avg(duration_ms) as avg_duration_ms
            FROM renders 
            WHERE timestamp >= ? AND success = 1
            GROUP BY date
            ORDER BY date
        "#;

        #[derive(Row, Deserialize)]
        struct DurationRow {
            date: u16, // ClickHouse date as days since 1900-01-01
            avg_duration_ms: f64,
        }

        let mut cursor = self.client
            .query(query)
            .bind(cutoff_timestamp)
            .fetch::<DurationRow>()?;

        let mut points = Vec::new();
        while let Some(row) = cursor.next().await? {
            // Convert ClickHouse date (days since 1900-01-01) to time::Date
            let days_since_1900 = row.date as i32;
            let days_since_unix_epoch = days_since_1900 - 25567; // Days from 1900-01-01 to 1970-01-01
            
            if let Ok(date) = time::Date::from_julian_day(days_since_unix_epoch + 2440588) { // Julian day adjustment
                points.push(DurationPoint {
                    date,
                    avg_duration_ms: row.avg_duration_ms,
                });
            }
        }

        Ok(points)
    }
}

impl From<clickhouse::error::Error> for RenderStorageError {
    fn from(err: clickhouse::error::Error) -> Self {
        RenderStorageError::Query(err.to_string())
    }
}