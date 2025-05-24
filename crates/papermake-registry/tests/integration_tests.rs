//! Integration tests for papermake-registry

use papermake::TemplateBuilder;
use papermake_registry::*;
use tempfile::tempdir;

#[tokio::test]
async fn test_basic_registry_workflow() {
    // Create temporary directory for test
    let temp_dir = tempdir().unwrap();
    let storage = FileSystemStorage::new(temp_dir.path()).await.unwrap();
    let registry = DefaultRegistry::new(storage);

    // Create a test user
    let user = User::new("alice", "alice@example.com");
    registry.save_user(&user).await.unwrap();

    // Create a simple template
    let template = TemplateBuilder::new("test-template".into())
        .name("Test Template")
        .content("#let data = json(sys.inputs.data)\nHello #data.name!")
        .build()
        .unwrap();

    // Publish the template
    let version = registry
        .publish_template(template, &user.id, TemplateScope::User(user.id.clone()))
        .await
        .unwrap();

    assert_eq!(version, 1);

    // Retrieve the template
    let retrieved = registry
        .get_template(&"test-template".into(), Some(version))
        .await
        .unwrap();

    assert_eq!(retrieved.version, 1);
    assert_eq!(retrieved.template.name, "Test Template");
    assert_eq!(retrieved.author, user.id);
    assert!(matches!(retrieved.scope, TemplateScope::User(_)));

    // Test access control
    let can_access = registry
        .can_access(&"test-template".into(), version, &user.id)
        .await
        .unwrap();

    assert!(can_access);

    // Create another user who shouldn't have access
    let other_user = User::new("bob", "bob@example.com");
    registry.save_user(&other_user).await.unwrap();

    let cannot_access = registry
        .can_access(&"test-template".into(), version, &other_user.id)
        .await
        .unwrap();

    assert!(!cannot_access);
}

#[tokio::test]
async fn test_template_versioning() {
    let temp_dir = tempdir().unwrap();
    let storage = FileSystemStorage::new(temp_dir.path()).await.unwrap();
    let registry = DefaultRegistry::new(storage);

    let user = User::new("alice", "alice@example.com");
    registry.save_user(&user).await.unwrap();

    // Publish version 1
    let template_v1 = TemplateBuilder::new("versioned-template".into())
        .name("Versioned Template v1")
        .content("Version 1 content")
        .build()
        .unwrap();

    let version_1 = registry
        .publish_template(template_v1, &user.id, TemplateScope::User(user.id.clone()))
        .await
        .unwrap();

    assert_eq!(version_1, 1);

    // Publish version 2
    let template_v2 = TemplateBuilder::new("versioned-template".into())
        .name("Versioned Template v2")
        .content("Version 2 content")
        .build()
        .unwrap();

    let version_2 = registry
        .publish_template(template_v2, &user.id, TemplateScope::User(user.id.clone()))
        .await
        .unwrap();

    assert_eq!(version_2, 2);

    // Get latest version
    let latest = registry
        .get_latest_version(&"versioned-template".into())
        .await
        .unwrap();

    assert_eq!(latest, 2);

    // List all versions
    let versions = registry
        .list_versions(&"versioned-template".into())
        .await
        .unwrap();

    assert_eq!(versions, vec![1, 2]);

    // Get specific versions
    let v1 = registry
        .get_template(&"versioned-template".into(), Some(1))
        .await
        .unwrap();

    let v2 = registry
        .get_template(&"versioned-template".into(), Some(2))
        .await
        .unwrap();

    assert_eq!(v1.template.content, "Version 1 content");
    assert_eq!(v2.template.content, "Version 2 content");
    assert_eq!(v1.version, 1);
    assert_eq!(v2.version, 2);
}

#[tokio::test]
async fn test_template_forking() {
    let temp_dir = tempdir().unwrap();
    let storage = FileSystemStorage::new(temp_dir.path()).await.unwrap();
    let registry = DefaultRegistry::new(storage);

    // Create original author
    let original_author = User::new("alice", "alice@example.com");
    registry.save_user(&original_author).await.unwrap();

    // Create someone who will fork
    let forker = User::new("bob", "bob@example.com");
    registry.save_user(&forker).await.unwrap();

    // Publish original template as public so bob can access it
    let original_template = TemplateBuilder::new("original-template".into())
        .name("Original Template")
        .content("Original content")
        .build()
        .unwrap();

    let original_version = registry
        .publish_template(original_template, &original_author.id, TemplateScope::Public)
        .await
        .unwrap();

    // Fork the template
    let forked_version = registry
        .fork_template(
            &"original-template".into(),
            original_version,
            &"forked-template".into(),
            &forker.id,
            TemplateScope::User(forker.id.clone()),
        )
        .await
        .unwrap();

    assert_eq!(forked_version, 1); // Forks start at version 1

    // Verify fork attribution
    let forked_template = registry
        .get_template(&"forked-template".into(), Some(forked_version))
        .await
        .unwrap();

    assert_eq!(forked_template.author, forker.id);
    assert_eq!(forked_template.template.content, "Original content");
    assert_eq!(
        forked_template.forked_from,
        Some(("original-template".into(), original_version))
    );

    // Forked template should have its own ID but same content
    assert_eq!(forked_template.template.id, "forked-template".into());
}

#[tokio::test]
async fn test_template_scopes() {
    let temp_dir = tempdir().unwrap();
    let storage = FileSystemStorage::new(temp_dir.path()).await.unwrap();
    let registry = DefaultRegistry::new(storage);

    // Create users and org
    let alice = User::with_id("alice", "alice", "alice@company.com");
    let bob = User::with_id("bob", "bob", "bob@company.com");
    let charlie = User::with_id("charlie", "charlie", "charlie@external.com");

    registry.save_user(&alice).await.unwrap();
    registry.save_user(&bob).await.unwrap();
    registry.save_user(&charlie).await.unwrap();

    // Test user scope access
    let user_scope = TemplateScope::User(alice.id.clone());
    assert!(user_scope.can_access(&alice));
    assert!(!user_scope.can_access(&bob));
    assert!(!user_scope.can_access(&charlie));

    // Test public scope access
    let public_scope = TemplateScope::Public;
    assert!(public_scope.can_access(&alice));
    assert!(public_scope.can_access(&bob));
    assert!(public_scope.can_access(&charlie));

    // Test marketplace scope access
    let marketplace_scope = TemplateScope::Marketplace;
    assert!(marketplace_scope.can_access(&alice));
    assert!(marketplace_scope.can_access(&bob));
    assert!(marketplace_scope.can_access(&charlie));
}

#[tokio::test]
async fn test_user_templates_listing() {
    let temp_dir = tempdir().unwrap();
    let storage = FileSystemStorage::new(temp_dir.path()).await.unwrap();
    let registry = DefaultRegistry::new(storage);

    let user = User::new("alice", "alice@example.com");
    registry.save_user(&user).await.unwrap();

    // Publish multiple templates
    let template1 = TemplateBuilder::new("template-1".into())
        .name("Template 1")
        .content("Content 1")
        .build()
        .unwrap();

    let template2 = TemplateBuilder::new("template-2".into())
        .name("Template 2")
        .content("Content 2")
        .build()
        .unwrap();

    registry
        .publish_template(template1, &user.id, TemplateScope::User(user.id.clone()))
        .await
        .unwrap();

    registry
        .publish_template(template2, &user.id, TemplateScope::User(user.id.clone()))
        .await
        .unwrap();

    // List user templates
    let user_templates = registry.list_user_templates(&user.id).await.unwrap();

    assert_eq!(user_templates.len(), 2);
    
    // Check that both templates are present (order not guaranteed)
    let template_ids: Vec<_> = user_templates.iter().map(|(id, _)| id.as_ref()).collect();
    assert!(template_ids.contains(&"template-1"));
    assert!(template_ids.contains(&"template-2"));
}