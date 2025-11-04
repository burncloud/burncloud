//! Diagnostic test to identify the root cause of test failures

use burncloud_client_models::IntegratedModelService;
use burncloud_service_models::{CreateModelRequest, ModelType};
use std::collections::HashMap;

#[tokio::test]
async fn test_basic_service_initialization() {
    println!("Step 1: Creating IntegratedModelService with memory database...");

    let service_result = IntegratedModelService::new(Some(":memory:".to_string())).await;

    match &service_result {
        Ok(_) => println!("✅ Service created successfully"),
        Err(e) => {
            println!("❌ Service creation failed: {:?}", e);
            panic!("Service initialization failed: {:?}", e);
        }
    }

    let service = service_result.unwrap();
    println!("Step 2: Service initialized, attempting to create a model...");

    let request = CreateModelRequest {
        name: "diagnostic-test".to_string(),
        display_name: "Diagnostic Test Model".to_string(),
        version: "1.0.0".to_string(),
        model_type: ModelType::Text,
        provider: "TestProvider".to_string(),
        file_size: 1024,
        description: Some("Test model".to_string()),
        license: None,
        tags: vec![],
        languages: vec![],
        file_path: None,
        download_url: None,
        config: HashMap::new(),
        is_official: false,
    };

    println!("Step 3: Calling create_model...");
    let create_result = service.create_model(request).await;

    match &create_result {
        Ok(model) => {
            println!("✅ Model created successfully!");
            println!("   ID: {}", model.id);
            println!("   Name: {}", model.name);
        }
        Err(e) => {
            println!("❌ Model creation failed: {:?}", e);
            panic!("Model creation failed: {:?}", e);
        }
    }

    let created = create_result.unwrap();

    println!("Step 4: Verifying model can be retrieved...");
    let get_result = service.get_model(created.id).await;

    match &get_result {
        Ok(Some(model)) => {
            println!("✅ Model retrieved successfully: {}", model.name);
        }
        Ok(None) => {
            println!("❌ Model not found in database");
            panic!("Model not found after creation");
        }
        Err(e) => {
            println!("❌ Failed to retrieve model: {:?}", e);
            panic!("Failed to retrieve model: {:?}", e);
        }
    }

    println!("✅ All diagnostic steps passed!");
}

#[tokio::test]
async fn test_list_models_empty() {
    println!("Testing list_models on empty database...");

    let service = IntegratedModelService::new(Some(":memory:".to_string()))
        .await
        .expect("Failed to create service");

    let models = service.list_models(None).await.expect("Failed to list models");

    println!("Empty database returned {} models", models.len());
    assert_eq!(models.len(), 0, "Empty database should return 0 models");

    println!("✅ Empty list test passed!");
}

#[tokio::test]
async fn test_search_models_empty() {
    println!("Testing search_models on empty database...");

    let service = IntegratedModelService::new(Some(":memory:".to_string()))
        .await
        .expect("Failed to create service");

    let results = service.search_models("test", Some(10)).await.expect("Failed to search");

    println!("Search on empty database returned {} results", results.len());
    assert_eq!(results.len(), 0, "Empty database search should return 0 results");

    println!("✅ Empty search test passed!");
}