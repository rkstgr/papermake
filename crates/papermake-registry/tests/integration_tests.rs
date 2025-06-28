//! Integration tests for papermake-registry

use papermake::TemplateBuilder;
use papermake_registry::storage::{MetadataStorage, sqlite_storage::SqliteStorage};
use papermake_registry::*;
use tempfile::tempdir;

#[tokio::test]
async fn test_basic_sqlite_storage_workflow() {
    // Create temporary directory for test database
    let temp_dir = tempdir().unwrap();
    let db_path = format!("sqlite:{}/test.db", temp_dir.path().display());

    let storage = SqliteStorage::new(&db_path).await.unwrap();

    // Create a simple template
    let template = TemplateBuilder::new()
        .content("Hello #data.name!")
        .build()
        .unwrap();

    let template_ref = TemplateRef::new("test-template").with_tag("v1");

    // Create a versioned template with tag "v1"
    let template_entry = TemplateEntry::new(template, template_ref.clone(), "alice".to_string());

    // Save the template
    storage.save_template(&template_entry).await.unwrap();

    // Retrieve the template by name and tag
    let retrieved = storage.get_template(&template_ref).await.unwrap();

    assert_eq!(retrieved.template_ref.tag, "v1");
    assert_eq!(retrieved.author, "alice");
}

#[tokio::test]
async fn test_render_job_workflow() {
    let temp_dir = tempdir().unwrap();
    let db_path = format!("sqlite:{}/test.db", temp_dir.path().display());

    let storage = SqliteStorage::new(&db_path).await.unwrap();

    // Create a template first
    let template = TemplateBuilder::new()
        .content("Hello #data.name!")
        .build()
        .unwrap();

    // Create a versioned template with tag "v1"
    let template_entry = TemplateEntry::new(
        template,
        TemplateRef::new("test-template").with_tag("v1"),
        "alice".to_string(),
    );

    // Save the template
    storage.save_template(&template_entry).await.unwrap();

    // Create render job data
    let render_data = serde_json::json!({"name": "World"});

    // Create a render job
    let render_job = RenderJob::new(
        TemplateRef::new("test-template").with_tag("v1"),
        render_data.clone(),
    );

    let job_id = render_job.id.clone();

    // Save the render job
    storage.save_render_job(&render_job).await.unwrap();

    // Retrieve the render job
    let retrieved_job = storage.get_render_job(&job_id).await.unwrap();

    assert_eq!(retrieved_job.id, job_id);
    assert_eq!(retrieved_job.data, render_data);
    assert_eq!(retrieved_job.status, RenderStatus::Pending);
}

#[tokio::test]
async fn test_registry_integration() {
    let temp_dir = tempdir().unwrap();
    let db_path = format!("sqlite:{}/test.db", temp_dir.path().display());

    // Create storage backends
    let metadata_storage = std::sync::Arc::new(SqliteStorage::new(&db_path).await.unwrap());
    // For tests, we can use a simple in-memory file storage
    let file_storage = std::sync::Arc::new(InMemoryFileStorage::new());

    let registry = DefaultRegistry::new(metadata_storage, file_storage);
    let content = "Test Content";
    // Create a template
    let template = TemplateBuilder::new().content(content).build().unwrap();

    // Publish the template through the registry
    let published_template = registry
        .publish_template(
            template,
            TemplateRef::new("test-template").with_tag("v1"),
            "alice".to_string(),
        )
        .await
        .unwrap();

    assert_eq!(published_template.template_ref.tag, "v1");
    assert_eq!(published_template.author, "alice");

    // Get the template back through the registry
    let retrieved = registry
        .get_template(&TemplateRef::new("test-template").with_tag("v1"))
        .await
        .unwrap();

    assert_eq!(retrieved.template.content, content);
}

// Simple in-memory file storage for testing
struct InMemoryFileStorage {
    files: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
}

impl InMemoryFileStorage {
    fn new() -> Self {
        Self {
            files: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl papermake_registry::storage::FileStorage for InMemoryFileStorage {
    async fn put_file(&self, key: &str, content: &[u8]) -> papermake_registry::error::Result<()> {
        self.files
            .lock()
            .unwrap()
            .insert(key.to_string(), content.to_vec());
        Ok(())
    }

    async fn get_file(&self, key: &str) -> papermake_registry::error::Result<Vec<u8>> {
        self.files.lock().unwrap().get(key).cloned().ok_or_else(|| {
            papermake_registry::RegistryError::Storage(format!("File not found: {}", key))
        })
    }

    async fn file_exists(&self, key: &str) -> papermake_registry::error::Result<bool> {
        Ok(self.files.lock().unwrap().contains_key(key))
    }

    async fn delete_file(&self, key: &str) -> papermake_registry::error::Result<()> {
        self.files.lock().unwrap().remove(key);
        Ok(())
    }

    async fn list_files(&self, prefix: &str) -> papermake_registry::error::Result<Vec<String>> {
        let files = self.files.lock().unwrap();
        let matching_files: Vec<String> = files
            .keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect();
        Ok(matching_files)
    }
}
