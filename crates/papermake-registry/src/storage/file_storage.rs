use crate::RegistryError;

use super::*;
use serde_json;
use std::path::{Path, PathBuf};
use tokio::fs;

/// File-based implementation of RegistryStorage
///
/// Directory structure:
/// ```text
/// base_path/
/// ├── templates/
/// │   └── template_id/
/// │       ├── versions/
/// │       │   ├── 1.json
/// │       │   ├── 2.json
/// │       │   └── ...
/// │       └── assets/
/// │           ├── fonts/
/// │           └── images/
/// ├── users/
/// │   └── user_id.json
/// └── organizations/
///     └── org_id.json
/// ```
pub struct FileSystemStorage {
    base_path: PathBuf,
}

impl FileSystemStorage {
    /// Create a new filesystem storage
    pub async fn new(base_path: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();

        // Create required directories
        fs::create_dir_all(base_path.join("templates")).await?;
        fs::create_dir_all(base_path.join("users")).await?;
        fs::create_dir_all(base_path.join("organizations")).await?;

        Ok(Self { base_path })
    }

    /// Get path to a template's base directory
    fn template_dir(&self, id: &papermake::TemplateId) -> PathBuf {
        self.base_path.join("templates").join(&id.0)
    }

    /// Get path to a template's versions directory
    fn template_versions_dir(&self, id: &papermake::TemplateId) -> PathBuf {
        self.template_dir(id).join("versions")
    }

    /// Get path to a specific version file
    fn version_file(&self, id: &papermake::TemplateId, version: u64) -> PathBuf {
        self.template_versions_dir(id)
            .join(format!("{}.json", version))
    }

    /// Get path to a template's assets directory
    fn template_assets_dir(&self, id: &papermake::TemplateId) -> PathBuf {
        self.template_dir(id).join("assets")
    }

    /// Get path to a user file
    fn user_file(&self, id: &UserId) -> PathBuf {
        self.base_path.join("users").join(format!("{}.json", id.0))
    }

    /// Get path to an organization file
    fn org_file(&self, id: &OrgId) -> PathBuf {
        self.base_path
            .join("organizations")
            .join(format!("{}.json", id.0))
    }
}

#[async_trait]
impl RegistryStorage for FileSystemStorage {
    async fn save_versioned_template(&self, template: &VersionedTemplate) -> Result<()> {
        // Create template directories
        let versions_dir = self.template_versions_dir(&template.template.id);
        fs::create_dir_all(&versions_dir).await?;

        // Save version metadata
        let version_json = serde_json::to_string_pretty(template)?;
        fs::write(
            self.version_file(&template.template.id, template.version),
            version_json,
        )
        .await?;

        Ok(())
    }

    async fn get_versioned_template(
        &self,
        id: &papermake::TemplateId,
        version: u64,
    ) -> Result<VersionedTemplate> {
        let version_file = self.version_file(id, version);
        if !version_file.exists() {
            return Err(RegistryError::VersionNotFound {
                template_id: id.as_ref().to_string(),
                version,
            });
        }

        let content = fs::read_to_string(&version_file).await?;
        let template: VersionedTemplate = serde_json::from_str(&content)?;
        Ok(template)
    }

    async fn list_template_versions(&self, id: &papermake::TemplateId) -> Result<Vec<u64>> {
        let versions_dir = self.template_versions_dir(id);
        if !versions_dir.exists() {
            return Ok(Vec::new());
        }

        let mut versions = Vec::new();
        let mut entries = fs::read_dir(&versions_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                if let Some(version_str) = name.strip_suffix(".json") {
                    if let Ok(version) = version_str.parse::<u64>() {
                        versions.push(version);
                    }
                }
            }
        }

        versions.sort_unstable();
        Ok(versions)
    }

    async fn delete_template_version(
        &self,
        id: &papermake::TemplateId,
        version: u64,
    ) -> Result<()> {
        let version_file = self.version_file(id, version);
        if version_file.exists() {
            fs::remove_file(&version_file).await?;
        }
        Ok(())
    }

    async fn save_template_asset(
        &self,
        template_id: &papermake::TemplateId,
        path: &str,
        content: &[u8],
    ) -> Result<()> {
        let asset_path = self.template_assets_dir(template_id).join(path);

        // Ensure parent directory exists
        if let Some(parent) = asset_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&asset_path, content).await?;
        Ok(())
    }

    async fn get_template_asset(
        &self,
        template_id: &papermake::TemplateId,
        path: &str,
    ) -> Result<Vec<u8>> {
        let asset_path = self.template_assets_dir(template_id).join(path);
        fs::read(&asset_path).await.map_err(|e| {
            RegistryError::Storage(format!(
                "Failed to read asset {}: {}",
                asset_path.display(),
                e
            ))
        })
    }

    async fn list_template_assets(
        &self,
        template_id: &papermake::TemplateId,
    ) -> Result<Vec<String>> {
        let assets_dir = self.template_assets_dir(template_id);
        if !assets_dir.exists() {
            return Ok(Vec::new());
        }

        let mut assets = Vec::new();
        self.list_files_recursive(&assets_dir, &assets_dir, &mut assets)
            .await?;
        Ok(assets)
    }

    async fn save_user(&self, user: &User) -> Result<()> {
        let user_json = serde_json::to_string_pretty(user)?;
        fs::write(self.user_file(&user.id), user_json).await?;
        Ok(())
    }

    async fn get_user(&self, id: &UserId) -> Result<User> {
        let user_file = self.user_file(id);
        if !user_file.exists() {
            return Err(RegistryError::UserNotFound(id.as_ref().to_string()));
        }

        let content = fs::read_to_string(&user_file).await?;
        let user: User = serde_json::from_str(&content)?;
        Ok(user)
    }

    async fn get_user_by_username(&self, username: &str) -> Result<User> {
        // Simple implementation: scan all user files
        // In production, you'd want an index
        let users_dir = self.base_path.join("users");
        if !users_dir.exists() {
            return Err(RegistryError::UserNotFound(username.to_string()));
        }

        let mut entries = fs::read_dir(&users_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_file() {
                let content = fs::read_to_string(entry.path()).await?;
                if let Ok(user) = serde_json::from_str::<User>(&content) {
                    if user.username == username {
                        return Ok(user);
                    }
                }
            }
        }

        Err(RegistryError::UserNotFound(username.to_string()))
    }

    async fn list_users(&self) -> Result<Vec<User>> {
        let users_dir = self.base_path.join("users");
        if !users_dir.exists() {
            return Ok(Vec::new());
        }

        let mut users = Vec::new();
        let mut entries = fs::read_dir(&users_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_file() {
                let content = fs::read_to_string(entry.path()).await?;
                if let Ok(user) = serde_json::from_str::<User>(&content) {
                    users.push(user);
                }
            }
        }

        Ok(users)
    }

    async fn delete_user(&self, id: &UserId) -> Result<()> {
        let user_file = self.user_file(id);
        if user_file.exists() {
            fs::remove_file(&user_file).await?;
        }
        Ok(())
    }

    async fn save_organization(&self, org: &Organization) -> Result<()> {
        let org_json = serde_json::to_string_pretty(org)?;
        fs::write(self.org_file(&org.id), org_json).await?;
        Ok(())
    }

    async fn get_organization(&self, id: &OrgId) -> Result<Organization> {
        let org_file = self.org_file(id);
        if !org_file.exists() {
            return Err(RegistryError::OrganizationNotFound(id.as_ref().to_string()));
        }

        let content = fs::read_to_string(&org_file).await?;
        let org: Organization = serde_json::from_str(&content)?;
        Ok(org)
    }

    async fn list_organizations(&self) -> Result<Vec<Organization>> {
        let orgs_dir = self.base_path.join("organizations");
        if !orgs_dir.exists() {
            return Ok(Vec::new());
        }

        let mut organizations = Vec::new();
        let mut entries = fs::read_dir(&orgs_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_file() {
                let content = fs::read_to_string(entry.path()).await?;
                if let Ok(org) = serde_json::from_str::<Organization>(&content) {
                    organizations.push(org);
                }
            }
        }

        Ok(organizations)
    }

    async fn delete_organization(&self, id: &OrgId) -> Result<()> {
        let org_file = self.org_file(id);
        if org_file.exists() {
            fs::remove_file(&org_file).await?;
        }
        Ok(())
    }

    async fn list_templates_by_scope(
        &self,
        scope: &TemplateScope,
    ) -> Result<Vec<(papermake::TemplateId, u64)>> {
        let templates_dir = self.base_path.join("templates");
        if !templates_dir.exists() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        let mut entries = fs::read_dir(&templates_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let template_id =
                    papermake::TemplateId(entry.file_name().to_string_lossy().to_string());
                let versions = self.list_template_versions(&template_id).await?;

                for version in versions {
                    if let Ok(versioned_template) =
                        self.get_versioned_template(&template_id, version).await
                    {
                        if &versioned_template.scope == scope {
                            results.push((template_id.clone(), version));
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    async fn search_templates(
        &self,
        query: &str,
        _user_id: &UserId,
    ) -> Result<Vec<(papermake::TemplateId, u64)>> {
        // Simple implementation: search template names and descriptions
        let templates_dir = self.base_path.join("templates");
        if !templates_dir.exists() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        let mut entries = fs::read_dir(&templates_dir).await?;
        let query_lower = query.to_lowercase();

        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let template_id =
                    papermake::TemplateId(entry.file_name().to_string_lossy().to_string());
                let versions = self.list_template_versions(&template_id).await?;

                for version in versions {
                    if let Ok(versioned_template) =
                        self.get_versioned_template(&template_id, version).await
                    {
                        let name_matches = versioned_template
                            .template
                            .name
                            .to_lowercase()
                            .contains(&query_lower);
                        let desc_matches = versioned_template
                            .template
                            .description
                            .as_ref()
                            .map(|d| d.to_lowercase().contains(&query_lower))
                            .unwrap_or(false);

                        if name_matches || desc_matches {
                            results.push((template_id.clone(), version));
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    async fn can_user_access(
        &self,
        template_id: &papermake::TemplateId,
        version: u64,
        user_id: &UserId,
    ) -> Result<bool> {
        let template = self.get_versioned_template(template_id, version).await?;
        let user = self.get_user(user_id).await?;
        Ok(template.can_access(&user))
    }

    async fn update_marketplace_metadata(
        &self,
        template_id: &papermake::TemplateId,
        version: u64,
        metadata: &MarketplaceMetadata,
    ) -> Result<()> {
        let mut template = self.get_versioned_template(template_id, version).await?;
        template.marketplace_metadata = Some(metadata.clone());
        self.save_versioned_template(&template).await
    }

    async fn get_marketplace_metadata(
        &self,
        template_id: &papermake::TemplateId,
        version: u64,
    ) -> Result<MarketplaceMetadata> {
        let template = self.get_versioned_template(template_id, version).await?;
        template.marketplace_metadata.ok_or_else(|| {
            RegistryError::Storage(format!(
                "No marketplace metadata for template {} version {}",
                template_id.as_ref(),
                version
            ))
        })
    }
}

impl FileSystemStorage {
    /// Helper function to recursively list files in a directory
    async fn list_files_recursive(
        &self,
        dir: &Path,
        base: &Path,
        files: &mut Vec<String>,
    ) -> Result<()> {
        let mut entries = fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() {
                // Use Box::pin for recursive call
                Box::pin(self.list_files_recursive(&path, base, files)).await?;
            } else {
                if let Ok(rel_path) = path.strip_prefix(base) {
                    if let Some(path_str) = rel_path.to_str() {
                        files.push(path_str.to_string());
                    }
                }
            }
        }

        Ok(())
    }
}
