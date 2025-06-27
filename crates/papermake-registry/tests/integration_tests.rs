//! Integration tests for papermake-registry

use papermake::TemplateBuilder;
use papermake_registry::*;
use papermake_registry::storage::{MetadataStorage, sqlite_storage::SqliteStorage};
use tempfile::tempdir;

#[tokio::test]
async fn test_basic_sqlite_storage_workflow() {
    // Create temporary directory for test database
    let temp_dir = tempdir().unwrap();
    let db_path = format!("sqlite:{}/test.db", temp_dir.path().display());
    
    let storage = SqliteStorage::new(&db_path).await.unwrap();

    // Create a simple template
    let template = TemplateBuilder::new("test-template".into())
        .name("Test Template")
        .content("#let data = json(sys.inputs.data)\nHello #data.name!")
        .build()
        .unwrap();

    // Create a versioned template with tag "v1"
    let versioned_template = VersionedTemplate::new(
        template,
        "test-template".to_string(),
        "Test Template Display".to_string(),
        "v1".to_string(),
        "alice".to_string(),
    );

    // Save the template
    storage.save_versioned_template(&versioned_template).await.unwrap();

    // Retrieve the template by name and tag
    let retrieved = storage
        .get_versioned_template_by_name("test-template", "v1")
        .await
        .unwrap();

    assert_eq!(retrieved.tag, "v1");
    assert_eq!(retrieved.template.name, "Test Template");
    assert_eq!(retrieved.author, "alice");
    assert_eq!(retrieved.template_name, "test-template");
    assert_eq!(retrieved.display_name, "Test Template Display");
}

#[tokio::test]
async fn test_template_tagging_and_versioning() {
    let temp_dir = tempdir().unwrap();
    let db_path = format!("sqlite:{}/test.db", temp_dir.path().display());
    
    let storage = SqliteStorage::new(&db_path).await.unwrap();

    let template_name = "versioned-template";

    // Publish tag v1
    let template_v1 = TemplateBuilder::new("versioned-template-v1".into())
        .name("Versioned Template v1")
        .content("Version 1 content")
        .build()
        .unwrap();

    let versioned_template_v1 = VersionedTemplate::new(
        template_v1,
        template_name.to_string(),
        "Versioned Template v1".to_string(),
        "v1".to_string(),
        "alice".to_string(),
    );

    storage.save_versioned_template(&versioned_template_v1).await.unwrap();

    // Publish tag v2
    let template_v2 = TemplateBuilder::new("versioned-template-v2".into())
        .name("Versioned Template v2")
        .content("Version 2 content")
        .build()
        .unwrap();

    let versioned_template_v2 = VersionedTemplate::new(
        template_v2,
        template_name.to_string(),
        "Versioned Template v2".to_string(),
        "v2".to_string(),
        "alice".to_string(),
    );

    storage.save_versioned_template(&versioned_template_v2).await.unwrap();

    // List all tags for the template
    let tags = storage
        .list_template_tags_by_name(template_name)
        .await
        .unwrap();

    assert_eq!(tags, vec!["v1".to_string(), "v2".to_string()]);

    // Get specific tags
    let v1 = storage
        .get_versioned_template_by_name(template_name, "v1")
        .await
        .unwrap();

    let v2 = storage
        .get_versioned_template_by_name(template_name, "v2")
        .await
        .unwrap();

    assert_eq!(v1.template.content, "Version 1 content");
    assert_eq!(v2.template.content, "Version 2 content");
    assert_eq!(v1.tag, "v1");
    assert_eq!(v2.tag, "v2");
}

#[tokio::test]
async fn test_draft_workflow() {
    let temp_dir = tempdir().unwrap();
    let db_path = format!("sqlite:{}/test.db", temp_dir.path().display());
    
    let storage = SqliteStorage::new(&db_path).await.unwrap();

    let template_name = "draft-template";

    // Create a draft template
    let template = TemplateBuilder::new("draft-template-id".into())
        .name("Draft Template")
        .content("Draft content")
        .build()
        .unwrap();

    let draft_template = VersionedTemplate::new_draft(
        template,
        template_name.to_string(),
        "Draft Template Display".to_string(),
        "alice".to_string(),
    );

    // Save the draft
    storage.save_draft(&draft_template).await.unwrap();

    // Check if draft exists
    let has_draft = storage.has_draft(template_name).await.unwrap();
    assert!(has_draft);

    // Get the draft
    let retrieved_draft = storage.get_draft(template_name).await.unwrap();
    assert!(retrieved_draft.is_some());
    
    let draft = retrieved_draft.unwrap();
    assert_eq!(draft.tag, "draft");
    assert_eq!(draft.template.content, "Draft content");
    assert!(draft.is_draft);

    // Get next tag number (should be 1 since no published versions exist)
    let next_tag_num = storage.get_next_tag_number(template_name).await.unwrap();
    assert_eq!(next_tag_num, 1);

    // Publish the draft as v1
    let published_template = draft.publish("v1".to_string());
    storage.save_versioned_template(&published_template).await.unwrap();

    // Verify the draft still exists before deletion
    let draft_still_exists = storage.has_draft(template_name).await.unwrap();
    assert!(draft_still_exists, "Draft should still exist after publishing");

    // Delete the draft
    storage.delete_draft(template_name).await.unwrap();

    // Verify draft is gone
    let has_draft_after = storage.has_draft(template_name).await.unwrap();
    assert!(!has_draft_after);

    // Verify published version exists
    let published = storage
        .get_versioned_template_by_name(template_name, "v1")
        .await
        .unwrap();
    
    assert_eq!(published.tag, "v1");
    assert_eq!(published.template.content, "Draft content");
    assert!(!published.is_draft);
}

#[tokio::test]
async fn test_template_forking() {
    let temp_dir = tempdir().unwrap();
    let db_path = format!("sqlite:{}/test.db", temp_dir.path().display());
    
    let storage = SqliteStorage::new(&db_path).await.unwrap();

    // Create original template
    let original_template = TemplateBuilder::new("original-template-id".into())
        .name("Original Template")
        .content("Original content")
        .build()
        .unwrap();

    let original_versioned = VersionedTemplate::new(
        original_template,
        "original-template".to_string(),
        "Original Template".to_string(),
        "v1".to_string(),
        "alice".to_string(),
    );

    storage.save_versioned_template(&original_versioned).await.unwrap();

    // Fork the template
    let forked_template = TemplateBuilder::new("forked-template-id".into())
        .name("Forked Template")
        .content("Original content") // Same content as original
        .build()
        .unwrap();

    let forked_versioned = VersionedTemplate::forked_from(
        forked_template,
        "forked-template".to_string(),
        "Forked Template".to_string(),
        "v1".to_string(),
        "bob".to_string(),
        ("original-template".to_string(), "v1".to_string()),
    );

    storage.save_versioned_template(&forked_versioned).await.unwrap();

    // Verify fork attribution
    let retrieved_fork = storage
        .get_versioned_template_by_name("forked-template", "v1")
        .await
        .unwrap();

    assert_eq!(retrieved_fork.author, "bob");
    assert_eq!(retrieved_fork.template.content, "Original content");
    assert_eq!(
        retrieved_fork.forked_from,
        Some(("original-template".to_string(), "v1".to_string()))
    );
    assert_eq!(retrieved_fork.template_name, "forked-template");
}

#[tokio::test]
async fn test_render_job_workflow() {
    let temp_dir = tempdir().unwrap();
    let db_path = format!("sqlite:{}/test.db", temp_dir.path().display());
    
    let storage = SqliteStorage::new(&db_path).await.unwrap();

    // Create a template first
    let template = TemplateBuilder::new("render-template".into())
        .name("Render Template")
        .content("Hello #data.name!")
        .build()
        .unwrap();

    let versioned_template = VersionedTemplate::new(
        template.clone(),
        "render-template".to_string(),
        "Render Template".to_string(),
        "v1".to_string(),
        "alice".to_string(),
    );

    storage.save_versioned_template(&versioned_template).await.unwrap();

    // Create render job data
    let render_data = serde_json::json!({"name": "World"});

    // Create a render job
    let render_job = RenderJob::new(
        template.id.clone(),
        "render-template".to_string(),
        "v1".to_string(),
        render_data.clone(),
    );

    let job_id = render_job.id.clone();

    // Save the render job
    storage.save_render_job(&render_job).await.unwrap();

    // Retrieve the render job
    let retrieved_job = storage.get_render_job(&job_id).await.unwrap();

    assert_eq!(retrieved_job.id, job_id);
    assert_eq!(retrieved_job.template_id, template.id);
    assert_eq!(retrieved_job.template_tag, "v1");
    assert_eq!(retrieved_job.template_name, "render-template");
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

    // Create a template
    let template = TemplateBuilder::new("registry-test".into())
        .name("Registry Test Template")
        .content("Registry test content")
        .build()
        .unwrap();

    // Publish the template through the registry
    let published_template = registry
        .publish_template(template, "alice".to_string())
        .await
        .unwrap();

    assert_eq!(published_template.tag, "v1");
    assert_eq!(published_template.author, "alice");

    // Get the template back through the registry
    let retrieved = registry
        .get_template_by_name("registry-test", "v1")
        .await
        .unwrap();

    assert_eq!(retrieved.template.name, "Registry Test Template");
    assert_eq!(retrieved.template.content, "Registry test content");

    // Test draft workflow
    let draft_template = TemplateBuilder::new("registry-test".into())
        .name("Registry Test Template")
        .content("Updated draft content")
        .build()
        .unwrap();

    // Save as draft
    let saved_draft = registry
        .save_draft(
            draft_template,
            "registry-test".to_string(),
            "Registry Test Template".to_string(),
            "alice".to_string(),
        )
        .await
        .unwrap();

    assert_eq!(saved_draft.tag, "draft");
    assert!(saved_draft.is_draft);

    // Publish the draft
    let published_v2 = registry
        .publish_draft("registry-test")
        .await
        .unwrap();

    assert_eq!(published_v2.tag, "v2");
    assert_eq!(published_v2.template.content, "Updated draft content");
    assert!(!published_v2.is_draft);

    // List tags
    let tags = registry
        .list_tags_by_name("registry-test")
        .await
        .unwrap();

    assert_eq!(tags, vec!["v1".to_string(), "v2".to_string()]);
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
        self.files.lock().unwrap().insert(key.to_string(), content.to_vec());
        Ok(())
    }

    async fn get_file(&self, key: &str) -> papermake_registry::error::Result<Vec<u8>> {
        self.files.lock().unwrap()
            .get(key)
            .cloned()
            .ok_or_else(|| papermake_registry::RegistryError::Storage(format!("File not found: {}", key)))
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