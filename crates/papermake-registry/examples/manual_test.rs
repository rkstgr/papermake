//! Manual testing example for papermake-registry with SQLite + S3 storage
//!
//! This example demonstrates the full registry workflow with real storage backends.
//! 
//! Prerequisites:
//! 1. Run `docker-compose up -d` to start MinIO (SQLite is file-based, no container needed)
//! 2. Copy `.env.example` to `.env` and adjust if needed
//! 3. Run with: `cargo run --example manual_test --features sqlite,s3`

use papermake::{TemplateBuilder, TemplateId, TypstFileSystem};
use papermake_registry::{
    DefaultRegistry, SqliteStorage, S3Storage, RegistryFileSystem,
    VersionedTemplate, RenderJob, TemplateRegistry
};
use std::sync::Arc;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file if present
    dotenv::dotenv().ok();
    
    println!("🚀 Starting papermake-registry manual test");
    println!("📋 This will test the complete SQLite + S3 storage workflow\n");

    // Initialize storage backends
    println!("🔌 Connecting to storage backends...");
    
    let sqlite_storage = Arc::new(
        SqliteStorage::from_env().await
            .expect("Failed to connect to SQLite database")
    );
    println!("✅ Connected to SQLite database");

    let s3_storage = Arc::new(
        S3Storage::from_env().await
            .expect("Failed to connect to S3 - ensure MinIO is running")
    );
    
    // Ensure bucket exists (commented out since bucket is created by docker-compose)
    // s3_storage.ensure_bucket().await
    //     .expect("Failed to ensure S3 bucket exists");
    println!("✅ Connected to S3/MinIO");

    // Create registry
    let registry = DefaultRegistry::new(sqlite_storage.clone(), s3_storage.clone());
    println!("🏗️ Created registry with SQLite + S3 backends\n");

    // Test 1: Publish a simple template
    println!("📝 Test 1: Publishing a simple template");
    
    let template_id = TemplateId::from("invoice-template");
    let template = TemplateBuilder::new(template_id.clone())
        .name("Invoice Template")
        .content(r#"
= Invoice

*Customer:* #data.customer_name
*Date:* #data.date
*Amount:* $#data.amount

Thank you for your business!
        "#.trim())
        .build()?;

    let versioned_template = registry.publish_template(
        template,
        1,
        "alice".to_string(),
    ).await?;

    println!("✅ Published template {} v{}", versioned_template.template.id.as_ref(), versioned_template.version);

    // Store template source in S3
    let registry_fs = RegistryFileSystem::new(s3_storage.clone(), template_id.clone());
    registry_fs.store_template_source(
        versioned_template.version,
        versioned_template.template.content.as_str(),
    ).await?;
    println!("✅ Stored template source in S3");

    // Test 2: Retrieve the template
    println!("\n📖 Test 2: Retrieving template");
    
    let retrieved = registry.get_template(&template_id, 1).await?;
    println!("✅ Retrieved template: {}", retrieved.template.name);
    println!("   Version: {}", retrieved.version);
    println!("   Author: {}", retrieved.author);

    // Test 3: List template versions
    println!("\n📋 Test 3: Listing template versions");
    
    let versions = registry.list_versions(&template_id).await?;
    println!("✅ Template versions: {:?}", versions);

    // Test 4: Create a render job (with caching test)
    println!("\n🎨 Test 4: Creating render jobs and testing caching");
    
    let render_data = json!({
        "customer_name": "ACME Corporation",
        "date": "2024-01-15",
        "amount": "1,250.00"
    });

    // First render
    let render_job1 = registry.render_template(
        &template_id,
        1,
        &render_data,
    ).await?;
    println!("✅ Created render job: {}", render_job1.id);
    println!("   Status: {:?}", render_job1.status);
    println!("   Data hash: {}", render_job1.data_hash);

    // Second render with same data (should find cached)
    let cached_job = registry.find_cached_render(
        &template_id,
        1,
        &render_data,
    ).await?;

    match cached_job {
        Some(job) => println!("✅ Found cached render job: {}", job.id),
        None => println!("⚠️  No cached render found (this is unexpected)"),
    }

    // Test 5: Search templates
    println!("\n🔍 Test 5: Searching templates");
    
    let search_results = registry.search_templates("invoice").await?;
    println!("✅ Search results for 'invoice': {} templates found", search_results.len());
    for (id, version) in search_results {
        println!("   - {} v{}", id.as_ref(), version);
    }

    // Test 6: Publish a new version
    println!("\n📝 Test 6: Publishing new version");
    
    let template_v2 = TemplateBuilder::new(template_id.clone())
        .name("Invoice Template v2")
        .content(r#"
= 🧾 INVOICE

**Customer:** #data.customer_name  
**Date:** #data.date  
**Amount:** $#data.amount  

Items:
#for item in data.items [
  - #item.description: $#item.price
]

*Thank you for choosing our services!*
        "#.trim())
        .build()?;

    let versioned_template_v2 = registry.publish_template(
        template_v2,
        2,
        "alice".to_string(),
    ).await?;

    println!("✅ Published template v{}", versioned_template_v2.version);

    // Store v2 source
    registry_fs.store_template_source(
        versioned_template_v2.version,
        versioned_template_v2.template.content.as_str(),
    ).await?;

    // Test 7: Store and retrieve template assets
    println!("\n📎 Test 7: Working with template assets");
    
    // Store a mock font file
    let font_data = b"Mock font file content for Arial.ttf";
    registry_fs.store_template_asset("fonts/Arial.ttf", font_data).await?;
    println!("✅ Stored template asset: fonts/Arial.ttf");

    // Store a mock image
    let image_data = b"Mock image data for logo.png";
    registry_fs.store_template_asset("images/logo.png", image_data).await?;
    println!("✅ Stored template asset: images/logo.png");

    // List assets
    let assets = registry_fs.list_template_assets().await?;
    println!("✅ Template assets: {:?}", assets);

    // Test TypstFileSystem interface
    let retrieved_font = registry_fs.get_file("fonts/Arial.ttf").await?;
    println!("✅ Retrieved font via TypstFileSystem: {} bytes", retrieved_font.len());

    // Final summary
    println!("\n🎉 Manual test completed successfully!");
    println!("📊 Summary:");
    println!("   - Templates published: 2 versions");
    println!("   - Render jobs created: 1");
    println!("   - Assets stored: 2");
    println!("   - All storage backends working correctly");
    
    println!("\n💡 You can now:");
    println!("   - Check MinIO console at http://localhost:9001 (minioadmin/minioadmin)");
    println!("   - Inspect stored files in the 'papermake-dev' bucket");
    println!("   - Check SQLite database at ./data/papermake.db");
    println!("   - Run the test multiple times to verify persistence");

    Ok(())
}