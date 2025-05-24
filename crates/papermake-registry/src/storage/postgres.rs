//! PostgreSQL storage implementation for the registry
//!
//! This module provides a PostgreSQL-backed implementation of the RegistryStorage trait.
//! It uses sqlx for async database operations and provides full ACID compliance
//! for all registry operations.

use crate::{entities::*, error::Result};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use time::OffsetDateTime;

/// PostgreSQL storage implementation
pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    /// Create a new PostgreSQL storage instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new PostgreSQL storage instance from a database URL
    pub async fn from_url(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self::new(pool))
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<()> {
        // Create tables if they don't exist
        sqlx::query(&Self::SCHEMA_SQL).execute(&self.pool).await?;
        Ok(())
    }

    /// SQL schema for all tables
    const SCHEMA_SQL: &'static str = r#"
        -- Users table
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL,
            organizations TEXT[] NOT NULL DEFAULT '{}',
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL
        );

        -- Organizations table
        CREATE TABLE IF NOT EXISTS organizations (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            created_at TIMESTAMPTZ NOT NULL
        );

        -- Templates table
        CREATE TABLE IF NOT EXISTS templates (
            id TEXT NOT NULL,
            version BIGINT NOT NULL,
            template_data JSONB NOT NULL,
            scope_type TEXT NOT NULL,
            scope_value TEXT,
            author TEXT NOT NULL,
            forked_from_id TEXT,
            forked_from_version BIGINT,
            published_at TIMESTAMPTZ NOT NULL,
            immutable BOOLEAN NOT NULL,
            marketplace_metadata JSONB,
            PRIMARY KEY (id, version),
            FOREIGN KEY (author) REFERENCES users(id)
        );

        -- Template assets table
        CREATE TABLE IF NOT EXISTS template_assets (
            template_id TEXT NOT NULL,
            path TEXT NOT NULL,
            content BYTEA NOT NULL,
            PRIMARY KEY (template_id, path)
        );

        -- Indexes for performance
        CREATE INDEX IF NOT EXISTS idx_templates_scope ON templates(scope_type, scope_value);
        CREATE INDEX IF NOT EXISTS idx_templates_author ON templates(author);
        CREATE INDEX IF NOT EXISTS idx_templates_published ON templates(published_at);
        CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
    "#;

    /// Helper to serialize template scope to database format
    fn serialize_scope(scope: &TemplateScope) -> (String, Option<String>) {
        match scope {
            TemplateScope::User(user_id) => ("user".to_string(), Some(user_id.as_ref().to_string())),
            TemplateScope::Organization(org_id) => ("organization".to_string(), Some(org_id.as_ref().to_string())),
            TemplateScope::Public => ("public".to_string(), None),
            TemplateScope::Marketplace => ("marketplace".to_string(), None),
        }
    }

    /// Helper to deserialize template scope from database format
    fn deserialize_scope(scope_type: &str, scope_value: Option<&str>) -> TemplateScope {
        match scope_type {
            "user" => TemplateScope::User(UserId::from(scope_value.unwrap_or(""))),
            "organization" => TemplateScope::Organization(OrgId::from(scope_value.unwrap_or(""))),
            "public" => TemplateScope::Public,
            "marketplace" => TemplateScope::Marketplace,
            _ => TemplateScope::Public, // Default fallback
        }
    }
}

#[async_trait]
impl super::RegistryStorage for PostgresStorage {
    // === Template Management ===

    async fn save_versioned_template(&self, template: &VersionedTemplate) -> Result<()> {
        let (scope_type, scope_value) = Self::serialize_scope(&template.scope);
        let template_json = serde_json::to_value(&template.template)?;
        let marketplace_json = template.marketplace_metadata.as_ref()
            .map(|m| serde_json::to_value(m))
            .transpose()?;

        sqlx::query(
            r#"
            INSERT INTO templates (
                id, version, template_data, scope_type, scope_value, author,
                forked_from_id, forked_from_version, published_at, immutable, marketplace_metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id, version) DO UPDATE SET
                template_data = EXCLUDED.template_data,
                scope_type = EXCLUDED.scope_type,
                scope_value = EXCLUDED.scope_value,
                marketplace_metadata = EXCLUDED.marketplace_metadata
            "#
        )
        .bind(template.template.id.as_ref())
        .bind(template.version as i64)
        .bind(&template_json)
        .bind(&scope_type)
        .bind(&scope_value)
        .bind(template.author.as_ref())
        .bind(template.forked_from.as_ref().map(|(id, _)| id.as_ref()))
        .bind(template.forked_from.as_ref().map(|(_, v)| *v as i64))
        .bind(template.published_at)
        .bind(template.immutable)
        .bind(&marketplace_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_versioned_template(
        &self,
        id: &papermake::TemplateId,
        version: u64,
    ) -> Result<VersionedTemplate> {
        let row = sqlx::query(
            r#"
            SELECT id, version, template_data, scope_type, scope_value, author,
                   forked_from_id, forked_from_version, published_at, immutable, marketplace_metadata
            FROM templates
            WHERE id = $1 AND version = $2
            "#
        )
        .bind(id.as_ref())
        .bind(version as i64)
        .fetch_one(&self.pool)
        .await?;

        let template: papermake::Template = serde_json::from_value(row.get("template_data"))?;
        let scope = Self::deserialize_scope(row.get("scope_type"), row.get("scope_value"));
        let marketplace_metadata: Option<MarketplaceMetadata> = row.get::<Option<serde_json::Value>, _>("marketplace_metadata")
            .map(|v| serde_json::from_value(v))
            .transpose()?;

        let forked_from = match (row.get::<Option<String>, _>("forked_from_id"), row.get::<Option<i64>, _>("forked_from_version")) {
            (Some(fork_id), Some(fork_version)) => Some((papermake::TemplateId::from(fork_id), fork_version as u64)),
            _ => None,
        };

        Ok(VersionedTemplate {
            template,
            version: row.get::<i64, _>("version") as u64,
            scope,
            author: UserId::from(row.get::<String, _>("author")),
            forked_from,
            published_at: row.get("published_at"),
            immutable: row.get("immutable"),
            marketplace_metadata,
        })
    }

    async fn list_template_versions(&self, id: &papermake::TemplateId) -> Result<Vec<u64>> {
        let rows = sqlx::query("SELECT version FROM templates WHERE id = $1 ORDER BY version")
            .bind(id.as_ref())
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|row| row.get::<i64, _>("version") as u64).collect())
    }

    async fn delete_template_version(&self, id: &papermake::TemplateId, version: u64) -> Result<()> {
        sqlx::query("DELETE FROM templates WHERE id = $1 AND version = $2")
            .bind(id.as_ref())
            .bind(version as i64)
            .execute(&self.pool)
            .await?;

        // Also delete associated assets
        sqlx::query("DELETE FROM template_assets WHERE template_id = $1")
            .bind(id.as_ref())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn save_template_asset(
        &self,
        template_id: &papermake::TemplateId,
        path: &str,
        content: &[u8],
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO template_assets (template_id, path, content)
            VALUES ($1, $2, $3)
            ON CONFLICT (template_id, path) DO UPDATE SET
                content = EXCLUDED.content
            "#
        )
        .bind(template_id.as_ref())
        .bind(path)
        .bind(content)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_template_asset(
        &self,
        template_id: &papermake::TemplateId,
        path: &str,
    ) -> Result<Vec<u8>> {
        let row = sqlx::query("SELECT content FROM template_assets WHERE template_id = $1 AND path = $2")
            .bind(template_id.as_ref())
            .bind(path)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("content"))
    }

    async fn list_template_assets(&self, template_id: &papermake::TemplateId) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT path FROM template_assets WHERE template_id = $1 ORDER BY path")
            .bind(template_id.as_ref())
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|row| row.get("path")).collect())
    }

    // === User Management ===

    async fn save_user(&self, user: &User) -> Result<()> {
        let orgs: Vec<String> = user.organizations.iter().map(|o| o.as_ref().to_string()).collect();

        sqlx::query(
            r#"
            INSERT INTO users (id, username, email, organizations, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE SET
                username = EXCLUDED.username,
                email = EXCLUDED.email,
                organizations = EXCLUDED.organizations,
                updated_at = EXCLUDED.updated_at
            "#
        )
        .bind(user.id.as_ref())
        .bind(&user.username)
        .bind(&user.email)
        .bind(&orgs)
        .bind(user.created_at)
        .bind(user.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_user(&self, id: &UserId) -> Result<User> {
        let row = sqlx::query(
            "SELECT id, username, email, organizations, created_at, updated_at FROM users WHERE id = $1"
        )
        .bind(id.as_ref())
        .fetch_one(&self.pool)
        .await?;

        let organizations: Vec<String> = row.get("organizations");
        let organizations = organizations.into_iter().map(OrgId::from).collect();

        Ok(User {
            id: UserId::from(row.get::<String, _>("id")),
            username: row.get("username"),
            email: row.get("email"),
            organizations,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    async fn get_user_by_username(&self, username: &str) -> Result<User> {
        let row = sqlx::query(
            "SELECT id, username, email, organizations, created_at, updated_at FROM users WHERE username = $1"
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await?;

        let organizations: Vec<String> = row.get("organizations");
        let organizations = organizations.into_iter().map(OrgId::from).collect();

        Ok(User {
            id: UserId::from(row.get::<String, _>("id")),
            username: row.get("username"),
            email: row.get("email"),
            organizations,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    async fn list_users(&self) -> Result<Vec<User>> {
        let rows = sqlx::query(
            "SELECT id, username, email, organizations, created_at, updated_at FROM users ORDER BY username"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut users = Vec::new();
        for row in rows {
            let organizations: Vec<String> = row.get("organizations");
            let organizations = organizations.into_iter().map(OrgId::from).collect();

            users.push(User {
                id: UserId::from(row.get::<String, _>("id")),
                username: row.get("username"),
                email: row.get("email"),
                organizations,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(users)
    }

    async fn delete_user(&self, id: &UserId) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id.as_ref())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // === Organization Management ===

    async fn save_organization(&self, org: &Organization) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO organizations (id, name, description, created_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description
            "#
        )
        .bind(org.id.as_ref())
        .bind(&org.name)
        .bind(&org.description)
        .bind(org.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_organization(&self, id: &OrgId) -> Result<Organization> {
        let row = sqlx::query("SELECT id, name, description, created_at FROM organizations WHERE id = $1")
            .bind(id.as_ref())
            .fetch_one(&self.pool)
            .await?;

        Ok(Organization {
            id: OrgId::from(row.get::<String, _>("id")),
            name: row.get("name"),
            description: row.get("description"),
            created_at: row.get("created_at"),
        })
    }

    async fn list_organizations(&self) -> Result<Vec<Organization>> {
        let rows = sqlx::query("SELECT id, name, description, created_at FROM organizations ORDER BY name")
            .fetch_all(&self.pool)
            .await?;

        let mut organizations = Vec::new();
        for row in rows {
            organizations.push(Organization {
                id: OrgId::from(row.get::<String, _>("id")),
                name: row.get("name"),
                description: row.get("description"),
                created_at: row.get("created_at"),
            });
        }

        Ok(organizations)
    }

    async fn delete_organization(&self, id: &OrgId) -> Result<()> {
        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(id.as_ref())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // === Discovery and Access Control ===

    async fn list_templates_by_scope(
        &self,
        scope: &TemplateScope,
    ) -> Result<Vec<(papermake::TemplateId, u64)>> {
        let (scope_type, scope_value) = Self::serialize_scope(scope);

        let query = match scope_value {
            Some(_) => "SELECT id, version FROM templates WHERE scope_type = $1 AND scope_value = $2 ORDER BY published_at DESC",
            None => "SELECT id, version FROM templates WHERE scope_type = $1 ORDER BY published_at DESC",
        };

        let rows = match scope_value {
            Some(ref value) => {
                sqlx::query(query)
                    .bind(&scope_type)
                    .bind(value)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                sqlx::query(query)
                    .bind(&scope_type)
                    .fetch_all(&self.pool)
                    .await?
            }
        };

        let mut templates = Vec::new();
        for row in rows {
            templates.push((
                papermake::TemplateId::from(row.get::<String, _>("id")),
                row.get::<i64, _>("version") as u64,
            ));
        }

        Ok(templates)
    }

    async fn search_templates(
        &self,
        query: &str,
        user_id: &UserId,
    ) -> Result<Vec<(papermake::TemplateId, u64)>> {
        // Get user for access control
        let user = self.get_user(user_id).await?;
        let user_orgs: Vec<String> = user.organizations.iter().map(|o| o.as_ref().to_string()).collect();

        // Search in template data (name, description) and marketplace metadata
        let search_query = format!("%{}%", query.to_lowercase());

        let rows = sqlx::query(
            r#"
            SELECT DISTINCT t.id, t.version, t.scope_type, t.scope_value, t.author
            FROM templates t
            WHERE (
                (t.template_data->>'name' ILIKE $1)
                OR (t.template_data->>'description' ILIKE $1)
                OR (t.marketplace_metadata->>'title' ILIKE $1)
                OR (t.marketplace_metadata->>'description' ILIKE $1)
            )
            AND (
                t.scope_type = 'public'
                OR t.scope_type = 'marketplace'
                OR (t.scope_type = 'user' AND t.scope_value = $2)
                OR (t.scope_type = 'organization' AND t.scope_value = ANY($3))
            )
            ORDER BY t.published_at DESC
            "#
        )
        .bind(&search_query)
        .bind(user_id.as_ref())
        .bind(&user_orgs)
        .fetch_all(&self.pool)
        .await?;

        let mut templates = Vec::new();
        for row in rows {
            templates.push((
                papermake::TemplateId::from(row.get::<String, _>("id")),
                row.get::<i64, _>("version") as u64,
            ));
        }

        Ok(templates)
    }

    async fn can_user_access(
        &self,
        template_id: &papermake::TemplateId,
        version: u64,
        user_id: &UserId,
    ) -> Result<bool> {
        // Get template to check scope
        let template = match self.get_versioned_template(template_id, version).await {
            Ok(t) => t,
            Err(_) => return Ok(false), // Template doesn't exist
        };

        // Get user for scope checking
        let user = match self.get_user(user_id).await {
            Ok(u) => u,
            Err(_) => return Ok(false), // User doesn't exist
        };

        Ok(template.can_access(&user))
    }

    // === Marketplace Operations ===

    async fn update_marketplace_metadata(
        &self,
        template_id: &papermake::TemplateId,
        version: u64,
        metadata: &MarketplaceMetadata,
    ) -> Result<()> {
        let metadata_json = serde_json::to_value(metadata)?;

        sqlx::query(
            "UPDATE templates SET marketplace_metadata = $1 WHERE id = $2 AND version = $3"
        )
        .bind(&metadata_json)
        .bind(template_id.as_ref())
        .bind(version as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_marketplace_metadata(
        &self,
        template_id: &papermake::TemplateId,
        version: u64,
    ) -> Result<MarketplaceMetadata> {
        let row = sqlx::query(
            "SELECT marketplace_metadata FROM templates WHERE id = $1 AND version = $2"
        )
        .bind(template_id.as_ref())
        .bind(version as i64)
        .fetch_one(&self.pool)
        .await?;

        let metadata_value: serde_json::Value = row.get("marketplace_metadata");
        let metadata: MarketplaceMetadata = serde_json::from_value(metadata_value)?;

        Ok(metadata)
    }
}

// Implement From trait for sqlx errors
impl From<sqlx::Error> for crate::error::RegistryError {
    fn from(err: sqlx::Error) -> Self {
        crate::error::RegistryError::StorageError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;
    use std::env;

    async fn setup_test_db() -> PgPool {
        let database_url = env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost/papermake_registry_test".to_string());
        
        let pool = PgPool::connect(&database_url).await.expect("Failed to connect to test database");
        
        // Clean up any existing data
        sqlx::query("DROP TABLE IF EXISTS template_assets CASCADE").execute(&pool).await.unwrap();
        sqlx::query("DROP TABLE IF EXISTS templates CASCADE").execute(&pool).await.unwrap();
        sqlx::query("DROP TABLE IF EXISTS organizations CASCADE").execute(&pool).await.unwrap();
        sqlx::query("DROP TABLE IF EXISTS users CASCADE").execute(&pool).await.unwrap();
        
        pool
    }

    #[tokio::test]
    async fn test_postgres_storage_basic_operations() {
        let pool = setup_test_db().await;
        let storage = PostgresStorage::new(pool);
        storage.migrate().await.unwrap();

        // Test user operations
        let user = User::new("testuser", "test@example.com");
        storage.save_user(&user).await.unwrap();
        
        let retrieved_user = storage.get_user(&user.id).await.unwrap();
        assert_eq!(retrieved_user.username, "testuser");
        assert_eq!(retrieved_user.email, "test@example.com");

        // Test organization operations
        let org = Organization::new("Test Org");
        storage.save_organization(&org).await.unwrap();
        
        let retrieved_org = storage.get_organization(&org.id).await.unwrap();
        assert_eq!(retrieved_org.name, "Test Org");

        // Test template operations
        let template = papermake::Template::new("test-template".into())
            .name("Test Template")
            .content("= Test Content")
            .build()
            .unwrap();
        
        let versioned_template = VersionedTemplate::new(
            template,
            1,
            TemplateScope::User(user.id.clone()),
            user.id.clone(),
        );
        
        storage.save_versioned_template(&versioned_template).await.unwrap();
        
        let retrieved_template = storage
            .get_versioned_template(&versioned_template.template.id, 1)
            .await
            .unwrap();
        
        assert_eq!(retrieved_template.version, 1);
        assert_eq!(retrieved_template.template.name, "Test Template");
    }
}