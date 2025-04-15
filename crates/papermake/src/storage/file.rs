use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;
use serde::{Serialize, Deserialize};
use crate::error::{PapermakeError, Result};
use crate::template::{Template, TemplateId};
use crate::schema::Schema;

use super::Storage;

/// Metadata stored alongside the template
#[derive(Serialize, Deserialize)]
struct TemplateMetadata {
    id: TemplateId,
    name: String,
    schema: Schema,
    description: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: time::OffsetDateTime,
}

/// File-based storage implementation
pub struct FileStorage {
    base_path: PathBuf,
}

impl FileStorage {
    /// Create a new file storage with the given base path
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// Get the path to a template directory
    fn template_dir(&self, id: &TemplateId) -> PathBuf {
        self.base_path.join("templates").join(&id.0)
    }

    /// Get the path to a template's main file
    fn template_file(&self, id: &TemplateId) -> PathBuf {
        self.template_dir(id).join("main.typ")
    }

    /// Get the path to a template's metadata file
    fn metadata_file(&self, id: &TemplateId) -> PathBuf {
        self.template_dir(id).join("metadata.json")
    }
    
    /// Helper function to recursively list files in a directory
    async fn list_files_recursive(&self, dir: &Path, base: &Path) -> Result<Vec<String>> {
        let mut files = Vec::new();
        let mut entries = fs::read_dir(dir).await
            .map_err(|e| PapermakeError::Storage(format!("Failed to read directory: {}", e)))?;
            
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| PapermakeError::Storage(format!("Failed to read directory entry: {}", e)))? {
                
            let path = entry.path();
            
            if path.is_dir() {
                // Use Box::pin for recursive call
                let subdir_files = Box::pin(self.list_files_recursive(&path, base)).await?;
                files.extend(subdir_files);
            } else {
                if let Ok(rel_path) = path.strip_prefix(base) {
                    if let Some(path_str) = rel_path.to_str() {
                        if path_str != "metadata.json" && path_str != "main.typ" {
                            files.push(path_str.to_string());
                        }
                    }
                }
            }
        }
        
        Ok(files)
    }
}

#[async_trait]
impl Storage for FileStorage {
    /// Save a template to storage
    async fn save_template(&self, template: &Template) -> Result<()> {
        // Create template directory
        let template_dir = self.template_dir(&template.id);
        fs::create_dir_all(&template_dir).await
            .map_err(|e| PapermakeError::Storage(format!("Failed to create template directory: {}", e)))?;

        // Write template content
        fs::write(self.template_file(&template.id), &template.content).await
            .map_err(|e| PapermakeError::Storage(format!("Failed to write template file: {}", e)))?;

        // Write metadata
        let metadata = TemplateMetadata {
            id: template.id.clone(),
            name: template.name.clone(),
            schema: template.schema.clone(),
            description: template.description.clone(),
            created_at: template.created_at,
            updated_at: template.updated_at,
        };

        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| PapermakeError::Storage(format!("Failed to serialize metadata: {}", e)))?;

        fs::write(self.metadata_file(&template.id), metadata_json).await
            .map_err(|e| PapermakeError::Storage(format!("Failed to write metadata file: {}", e)))?;

        Ok(())
    }

    /// Get a template from storage by ID
    async fn get_template(&self, id: &TemplateId) -> Result<Template> {
        // Read template content
        let content = fs::read_to_string(self.template_file(id)).await
            .map_err(|e| PapermakeError::Storage(format!("Failed to read template file: {}", e)))?;

        // Read metadata
        let metadata_json = fs::read_to_string(self.metadata_file(id)).await
            .map_err(|e| PapermakeError::Storage(format!("Failed to read metadata file: {}", e)))?;

        let metadata: TemplateMetadata = serde_json::from_str(&metadata_json)
            .map_err(|e| PapermakeError::Storage(format!("Failed to parse metadata: {}", e)))?;

        // Construct template
        Ok(Template {
            id: metadata.id,
            name: metadata.name,
            content,
            schema: metadata.schema,
            description: metadata.description,
            created_at: metadata.created_at,
            updated_at: metadata.updated_at,
        })
    }

    /// List all templates in storage
    async fn list_templates(&self) -> Result<Vec<Template>> {
        let templates_dir = self.base_path.join("templates");

        // Create directory if it doesn't exist
        if !templates_dir.exists() {
            fs::create_dir_all(&templates_dir).await
                .map_err(|e| PapermakeError::Storage(format!("Failed to create templates directory: {}", e)))?;
            return Ok(Vec::new());
        }

        let mut templates = Vec::new();
        let mut entries = fs::read_dir(templates_dir).await
            .map_err(|e| PapermakeError::Storage(format!("Failed to read templates directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| PapermakeError::Storage(format!("Failed to read directory entry: {}", e)))? {
                
            let path = entry.path();
            if path.is_dir() {
                let id = path.file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| PapermakeError::Storage("Invalid template directory name".to_string()))?;
                
                match self.get_template(&TemplateId(id.to_string())).await {
                    Ok(template) => templates.push(template),
                    Err(e) => eprintln!("Error loading template {}: {}", id, e),
                }
            }
        }

        Ok(templates)
    }

    /// Delete a template from storage
    async fn delete_template(&self, id: &TemplateId) -> Result<()> {
        let template_dir = self.template_dir(id);
        
        if template_dir.exists() {
            fs::remove_dir_all(&template_dir).await
                .map_err(|e| PapermakeError::Storage(format!("Failed to delete template directory: {}", e)))?;
        }

        Ok(())
    }

    /// Save an additional file associated with a template
    async fn save_template_file(&self, template_id: &TemplateId, path: &str, content: &[u8]) -> Result<()> {
        let file_path = self.template_dir(template_id).join(path);
        
        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| PapermakeError::Storage(format!("Failed to create directory: {}", e)))?;
        }
        
        // Write file
        fs::write(&file_path, content).await
            .map_err(|e| PapermakeError::Storage(format!("Failed to write file: {}", e)))?;
            
        Ok(())
    }

    /// Get a file associated with a template
    async fn get_template_file(&self, template_id: &TemplateId, path: &str) -> Result<Vec<u8>> {
        let file_path = self.template_dir(template_id).join(path);
        
        fs::read(&file_path).await
            .map_err(|e| PapermakeError::Storage(format!("Failed to read file {}: {}", file_path.display(), e)))
    }

    /// List all files associated with a template
    async fn list_template_files(&self, template_id: &TemplateId) -> Result<Vec<String>> {
        let template_dir = self.template_dir(template_id);
        let mut files = Vec::new();
        
        // Use tokio's async directory reading
        let mut entries = fs::read_dir(&template_dir).await
            .map_err(|e| PapermakeError::Storage(format!("Failed to read template directory: {}", e)))?;
            
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| PapermakeError::Storage(format!("Failed to read directory entry: {}", e)))? {
                
            let path = entry.path();
            
            if path.is_dir() {
                // Recursively list files in subdirectories
                let mut subdir_files = self.list_files_recursive(&path, &template_dir).await?;
                files.append(&mut subdir_files);
            } else {
                // Skip metadata and main template file
                if let Ok(rel_path) = path.strip_prefix(&template_dir) {
                    if let Some(path_str) = rel_path.to_str() {
                        if path_str != "metadata.json" && path_str != "main.typ" {
                            files.push(path_str.to_string());
                        }
                    }
                }
            }
        }
        
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Schema, SchemaField, FieldType};
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_file_storage() {
        // Create a temporary directory
        let temp_dir = tempdir().unwrap();
        let storage = FileStorage::new(temp_dir.path());
        
        // Create a simple schema
        let mut schema = Schema::new();
        schema.add_field(SchemaField {
            key: "name".to_string(),
            label: Some("Name".to_string()),
            field_type: FieldType::String,
            required: true,
            description: None,
            default: None,
        });
        
        // Create a template
        let template = Template::new(
            "test-template",
            "Test Template",
            "#let data = json.decode(sys.inputs.data)\nHello #data.name!",
            schema,
        ).with_description("A test template");
        
        // Save template
        storage.save_template(&template).await.unwrap();
        
        // Get template
        let retrieved = storage.get_template(&template.id).await.unwrap();
        assert_eq!(retrieved.id.0, "test-template");
        assert_eq!(retrieved.name, "Test Template");
        assert_eq!(retrieved.content, "#let data = json.decode(sys.inputs.data)\nHello #data.name!");
        assert_eq!(retrieved.description, Some("A test template".to_string()));
        
        // List templates
        let templates = storage.list_templates().await.unwrap();
        assert_eq!(templates.len(), 1);
        
        // Save additional file
        let style_content = "#let title(text) = [*#text*]".as_bytes();
        storage.save_template_file(&template.id, "style.typ", style_content).await.unwrap();
        
        // Get file
        let retrieved_content = storage.get_template_file(&template.id, "style.typ").await.unwrap();
        assert_eq!(retrieved_content, style_content);
        
        // List files
        let files = storage.list_template_files(&template.id).await.unwrap();
        assert_eq!(files, vec!["style.typ"]);
        
        // Delete template
        storage.delete_template(&template.id).await.unwrap();
        
        // Verify deletion
        let templates = storage.list_templates().await.unwrap();
        assert_eq!(templates.len(), 0);
    }
}