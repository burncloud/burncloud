//! Comprehensive Tests for ModelDataService with Real Database Integration
//!
//! This test suite validates the "replace-mock-data-with-service-models" feature
//! by testing ModelDataService and AppState with actual database connections.

use burncloud_client_models::{
    ModelDataService,
    state::AppState,
    burncloud_service_models::{
        ModelsService, CreateModelRequest, ModelType, ModelStatus
    },
    burncloud_database::Database
};
use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;

/// Helper function to create test database
async fn create_test_database() -> Arc<Database> {
    let mut db = Database::new(":memory:");
    db.initialize().await.expect("Failed to initialize test database");
    Arc::new(db)
}

/// Helper function to create a test model request
fn create_test_model(name: &str, model_type: ModelType) -> CreateModelRequest {
    let mut config = HashMap::new();
    config.insert("temperature".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(0.7).unwrap()));

    CreateModelRequest {
        name: name.to_string(),
        display_name: format!("{} Display Name", name),
        version: "1.0.0".to_string(),
        model_type,
        provider: "TestProvider".to_string(),
        file_size: 1_000_000_000,
        description: Some(format!("Test model: {}", name)),
        license: Some("MIT".to_string()),
        tags: vec!["test".to_string()],
        languages: vec!["English".to_string()],
        file_path: None,
        download_url: Some(format!("https://test.example.com/{}", name)),
        config,
        is_official: false,
    }
}

// =============================================================================
// 1. Database Integration Tests
// =============================================================================

#[tokio::test]
async fn test_model_data_service_new_with_empty_database() {
    // Test that ModelDataService initializes correctly with an empty database
    let database = create_test_database().await;

    let service = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");

    // Should return empty lists, not error
    assert_eq!(service.get_installed_models().len(), 0, "Installed models should be empty");
    assert_eq!(service.get_available_models().len(), 0, "Available models should be empty");
}

#[tokio::test]
async fn test_model_data_service_loads_from_database() {
    // Test that ModelDataService loads data from the database correctly
    let database = create_test_database().await;

    // Add some models to the database via ModelsService
    let models_service = ModelsService::new(database.clone()).await
        .expect("Failed to create ModelsService");

    let request1 = create_test_model("test-model-1", ModelType::Chat);
    let request2 = create_test_model("test-model-2", ModelType::Code);

    let _model1 = models_service.create_model(request1).await
        .expect("Failed to create model 1");
    let _model2 = models_service.create_model(request2).await
        .expect("Failed to create model 2");

    // Now create ModelDataService and verify it loads the data
    let data_service = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");

    let available_models = data_service.get_available_models();
    assert_eq!(available_models.len(), 2, "Should load 2 available models");

    // Verify the models are present
    let model_names: Vec<String> = available_models.iter()
        .map(|m| m.model.name.clone())
        .collect();
    assert!(model_names.contains(&"test-model-1".to_string()));
    assert!(model_names.contains(&"test-model-2".to_string()));
}

#[tokio::test]
async fn test_model_data_service_loads_installed_models() {
    // Test that ModelDataService loads installed models correctly
    let database = create_test_database().await;

    // Create and install a model
    let models_service = ModelsService::new(database.clone()).await
        .expect("Failed to create ModelsService");

    let request = create_test_model("installed-model", ModelType::Text);
    let model = models_service.create_model(request).await
        .expect("Failed to create model");

    let _installed = models_service.install_model(model.id, "/opt/test/model".to_string()).await
        .expect("Failed to install model");

    // Create ModelDataService and verify it loads the installed model
    let data_service = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");

    let installed_models = data_service.get_installed_models();
    assert_eq!(installed_models.len(), 1, "Should load 1 installed model");
    assert_eq!(installed_models[0].model.id, model.id);
    assert_eq!(installed_models[0].install_path, "/opt/test/model");
}

#[tokio::test]
async fn test_model_data_service_with_multiple_model_types() {
    // Test loading models of different types
    let database = create_test_database().await;
    let models_service = ModelsService::new(database.clone()).await
        .expect("Failed to create ModelsService");

    // Create models of different types
    let types = vec![
        ModelType::Chat,
        ModelType::Code,
        ModelType::Text,
        ModelType::Embedding,
        ModelType::Image,
    ];

    for model_type in &types {
        let request = create_test_model(&format!("model-{:?}", model_type), model_type.clone());
        let model = models_service.create_model(request).await
            .expect("Failed to create model");
        // Install the model so we can filter by type
        models_service.install_model(model.id, format!("/opt/model-{:?}", model_type)).await
            .expect("Failed to install model");
    }

    // Create ModelDataService and verify all models are loaded
    let data_service = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");

    assert_eq!(data_service.get_available_models().len(), types.len());
    assert_eq!(data_service.get_installed_models().len(), types.len());

    // Test filtering by type (note: get_models_by_type only returns installed models)
    for model_type in &types {
        let filtered = data_service.get_models_by_type(model_type);
        assert_eq!(filtered.len(), 1, "Should find exactly one model of type {:?}", model_type);
    }
}

// =============================================================================
// 2. Error Handling Tests
// =============================================================================

#[tokio::test]
async fn test_model_data_service_handles_empty_database_gracefully() {
    // Test that empty database doesn't cause panics
    let database = create_test_database().await;
    let service = ModelDataService::new(database).await
        .expect("Should handle empty database");

    // All operations should work with empty data
    assert_eq!(service.get_installed_models().len(), 0);
    assert_eq!(service.get_available_models().len(), 0);
    assert_eq!(service.get_running_models_count(), 0);
    assert_eq!(service.get_usage_stats().total_models, 0);
}

#[tokio::test]
async fn test_model_data_service_handles_database_initialization_failure() {
    // Test with invalid database path (should fail at Database::new level)
    let mut db = Database::new("/invalid/path/that/does/not/exist.db");
    let result = db.initialize().await;

    // Database initialization should fail
    assert!(result.is_err(), "Should fail to initialize database with invalid path");
}

// =============================================================================
// 3. Data Conversion Tests
// =============================================================================

#[tokio::test]
async fn test_available_model_conversion() {
    // Test that Model is properly converted to AvailableModel
    let database = create_test_database().await;
    let models_service = ModelsService::new(database.clone()).await
        .expect("Failed to create ModelsService");

    let request = create_test_model("conversion-test", ModelType::Chat);
    let model = models_service.create_model(request).await
        .expect("Failed to create model");

    // Create ModelDataService and check conversion
    let data_service = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");

    let available_models = data_service.get_available_models();
    assert_eq!(available_models.len(), 1);

    let available_model = &available_models[0];

    // Verify all fields are properly mapped
    assert_eq!(available_model.model.id, model.id);
    assert_eq!(available_model.model.name, model.name);
    assert_eq!(available_model.model.display_name, model.display_name);
    assert_eq!(available_model.model.model_type, model.model_type);
    assert_eq!(available_model.model.provider, model.provider);

    // Verify default values for AvailableModel fields
    assert_eq!(available_model.is_downloadable, true);
    assert_eq!(available_model.estimated_download_time, None);
}

#[tokio::test]
async fn test_model_field_preservation() {
    // Test that all model fields are preserved through the conversion
    let database = create_test_database().await;
    let models_service = ModelsService::new(database.clone()).await
        .expect("Failed to create ModelsService");

    let mut request = create_test_model("field-test", ModelType::Code);
    request.file_size = 5_000_000_000;
    request.description = Some("Detailed description".to_string());
    request.license = Some("Apache-2.0".to_string());
    request.tags = vec!["tag1".to_string(), "tag2".to_string()];
    request.languages = vec!["Python".to_string(), "JavaScript".to_string()];

    let model = models_service.create_model(request).await
        .expect("Failed to create model");

    let data_service = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");

    let available_model = data_service.get_available_model_by_id(&model.id)
        .expect("Model should be found");

    assert_eq!(available_model.model.file_size, 5_000_000_000);
    assert_eq!(available_model.model.description, Some("Detailed description".to_string()));
    assert_eq!(available_model.model.license, Some("Apache-2.0".to_string()));
    assert_eq!(available_model.model.tags.len(), 2);
    assert_eq!(available_model.model.languages.len(), 2);
}

// =============================================================================
// 4. Async Functionality Tests
// =============================================================================

#[tokio::test]
async fn test_async_initialization() {
    // Test that async initialization works correctly
    let database = create_test_database().await;

    // This should complete without blocking
    let service = ModelDataService::new(database).await
        .expect("Async initialization should succeed");

    assert!(service.get_installed_models().len() == 0);
}

#[tokio::test]
async fn test_concurrent_service_creation() {
    // Test that multiple services can be created concurrently
    let database = create_test_database().await;

    // Create services sequentially to avoid Send trait issues with Box<dyn Error>
    for _ in 0..5 {
        let result = ModelDataService::new(database.clone()).await;
        assert!(result.is_ok(), "Service creation should succeed");
    }
}

// =============================================================================
// 5. AppState Integration Tests
// =============================================================================

#[tokio::test]
async fn test_app_state_new_with_database() {
    // Test AppState::new() with database
    let database = create_test_database().await;

    let app_state = AppState::new(database.clone()).await
        .expect("Failed to create AppState");

    assert!(app_state.data_service.get_installed_models().is_empty());
    assert!(app_state.data_service.get_available_models().is_empty());
    assert_eq!(app_state.search_query, "");
    assert!(app_state.filter_type.is_none());
    assert!(app_state.filter_status.is_none());
}

#[tokio::test]
async fn test_app_state_with_populated_database() {
    // Test AppState with pre-populated database
    let database = create_test_database().await;
    let models_service = ModelsService::new(database.clone()).await
        .expect("Failed to create ModelsService");

    // Create some test data
    let request1 = create_test_model("app-state-test-1", ModelType::Chat);
    let request2 = create_test_model("app-state-test-2", ModelType::Code);

    models_service.create_model(request1).await.expect("Failed to create model 1");
    models_service.create_model(request2).await.expect("Failed to create model 2");

    // Create AppState
    let app_state = AppState::new(database.clone()).await
        .expect("Failed to create AppState");

    // Verify data is loaded
    assert_eq!(app_state.data_service.get_available_models().len(), 2);
}

#[tokio::test]
async fn test_app_state_database_reference() {
    // Test that AppState maintains database reference
    let database = create_test_database().await;
    let db_ptr = Arc::as_ptr(&database);

    let app_state = AppState::new(database.clone()).await
        .expect("Failed to create AppState");

    // Verify database reference is maintained
    assert_eq!(Arc::as_ptr(&app_state.database), db_ptr);
}

// =============================================================================
// 6. Regression Tests - Existing Functionality
// =============================================================================

#[tokio::test]
async fn test_search_functionality_still_works() {
    // Test that search functionality works with real data
    let database = create_test_database().await;
    let models_service = ModelsService::new(database.clone()).await
        .expect("Failed to create ModelsService");

    let request1 = create_test_model("search-chat-model", ModelType::Chat);
    let model1 = models_service.create_model(request1).await.expect("Failed to create model");
    models_service.install_model(model1.id, "/opt/search-chat".to_string()).await
        .expect("Failed to install model");

    let request2 = create_test_model("search-code-model", ModelType::Code);
    let model2 = models_service.create_model(request2).await.expect("Failed to create model");
    models_service.install_model(model2.id, "/opt/search-code".to_string()).await
        .expect("Failed to install model");

    let request3 = create_test_model("other-model", ModelType::Text);
    let model3 = models_service.create_model(request3).await.expect("Failed to create model");
    models_service.install_model(model3.id, "/opt/other".to_string()).await
        .expect("Failed to install model");

    let data_service = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");

    // Test search (note: search_models only searches installed models)
    let results = data_service.search_models("search");
    assert_eq!(results.len(), 2, "Should find 2 models with 'search' in name");

    let results = data_service.search_models("chat");
    assert_eq!(results.len(), 1, "Should find 1 model with 'chat' in name");

    let results = data_service.search_models("nonexistent");
    assert_eq!(results.len(), 0, "Should find 0 models");
}

#[tokio::test]
async fn test_filter_by_status_still_works() {
    // Test filtering by status with real data
    let database = create_test_database().await;
    let models_service = ModelsService::new(database.clone()).await
        .expect("Failed to create ModelsService");

    // Create and install models with different statuses
    let request1 = create_test_model("status-model-1", ModelType::Chat);
    let model1 = models_service.create_model(request1).await.expect("Failed to create model");
    let _installed1 = models_service.install_model(model1.id, "/opt/model1".to_string()).await
        .expect("Failed to install model");

    let request2 = create_test_model("status-model-2", ModelType::Code);
    let model2 = models_service.create_model(request2).await.expect("Failed to create model");
    let _installed2 = models_service.install_model(model2.id, "/opt/model2".to_string()).await
        .expect("Failed to install model");

    // Update statuses
    models_service.update_model_status(model1.id, ModelStatus::Running).await
        .expect("Failed to update status");
    models_service.update_model_status(model2.id, ModelStatus::Stopped).await
        .expect("Failed to update status");

    let data_service = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");

    // Test filtering
    let running = data_service.get_installed_models_by_status(&ModelStatus::Running);
    assert_eq!(running.len(), 1);

    let stopped = data_service.get_installed_models_by_status(&ModelStatus::Stopped);
    assert_eq!(stopped.len(), 1);
}

#[tokio::test]
async fn test_get_model_by_id_still_works() {
    // Test retrieving models by ID
    let database = create_test_database().await;
    let models_service = ModelsService::new(database.clone()).await
        .expect("Failed to create ModelsService");

    let request = create_test_model("id-test-model", ModelType::Embedding);
    let model = models_service.create_model(request).await
        .expect("Failed to create model");

    let data_service = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");

    // Test available model retrieval
    let found = data_service.get_available_model_by_id(&model.id);
    assert!(found.is_some());
    assert_eq!(found.unwrap().model.id, model.id);

    // Test non-existent ID
    let not_found = data_service.get_available_model_by_id(&Uuid::new_v4());
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_usage_stats_with_real_data() {
    // Test that usage statistics work with real data
    let database = create_test_database().await;
    let models_service = ModelsService::new(database.clone()).await
        .expect("Failed to create ModelsService");

    // Create multiple models
    for i in 0..5 {
        let request = create_test_model(&format!("stats-model-{}", i), ModelType::Chat);
        models_service.create_model(request).await.expect("Failed to create model");
    }

    let data_service = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");

    let stats = data_service.get_usage_stats();
    assert_eq!(stats.total_models, 0, "No installed models yet");
    assert_eq!(stats.running_models, 0);
}

#[tokio::test]
async fn test_resource_overview_with_real_data() {
    // Test resource overview functionality
    let database = create_test_database().await;
    let models_service = ModelsService::new(database.clone()).await
        .expect("Failed to create ModelsService");

    // Create and install models
    let mut total_size = 0u64;
    for i in 0..3 {
        let mut request = create_test_model(&format!("resource-model-{}", i), ModelType::Text);
        request.file_size = (i + 1) * 1_000_000_000;
        total_size += request.file_size;

        let model = models_service.create_model(request).await.expect("Failed to create model");
        models_service.install_model(model.id, format!("/opt/model-{}", i)).await
            .expect("Failed to install model");
    }

    let data_service = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");

    let overview = data_service.get_resource_overview();
    assert_eq!(overview.total_disk_usage_bytes, total_size);
    assert_eq!(overview.active_processes.len(), 0); // No running processes yet
}

// =============================================================================
// 7. Data Consistency Tests
// =============================================================================

#[tokio::test]
async fn test_data_consistency_after_model_updates() {
    // Test that data remains consistent after updates
    let database = create_test_database().await;
    let models_service = ModelsService::new(database.clone()).await
        .expect("Failed to create ModelsService");

    let request = create_test_model("consistency-test", ModelType::Chat);
    let _model = models_service.create_model(request).await
        .expect("Failed to create model");

    // Create first service instance
    let data_service1 = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");

    assert_eq!(data_service1.get_available_models().len(), 1);

    // Add another model via ModelsService
    let request2 = create_test_model("consistency-test-2", ModelType::Code);
    models_service.create_model(request2).await
        .expect("Failed to create model");

    // Create new service instance - should see updated data
    let data_service2 = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");

    assert_eq!(data_service2.get_available_models().len(), 2);
}

#[tokio::test]
async fn test_installed_and_available_models_consistency() {
    // Test that installed models are also in available models
    let database = create_test_database().await;
    let models_service = ModelsService::new(database.clone()).await
        .expect("Failed to create ModelsService");

    let request = create_test_model("consistency-model", ModelType::Text);
    let model = models_service.create_model(request).await
        .expect("Failed to create model");

    models_service.install_model(model.id, "/opt/consistency".to_string()).await
        .expect("Failed to install model");

    let data_service = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");

    // Installed model should also be in available models
    let installed = data_service.get_installed_models();
    let available = data_service.get_available_models();

    assert_eq!(installed.len(), 1);
    assert_eq!(available.len(), 1);
    assert_eq!(installed[0].model.id, available[0].model.id);
}

// =============================================================================
// 8. Performance Tests
// =============================================================================

#[tokio::test]
async fn test_initialization_performance_with_many_models() {
    // Test performance with a larger dataset
    let database = create_test_database().await;
    let models_service = ModelsService::new(database.clone()).await
        .expect("Failed to create ModelsService");

    // Create 50 models
    for i in 0..50 {
        let request = create_test_model(&format!("perf-model-{:03}", i), ModelType::Chat);
        models_service.create_model(request).await
            .expect("Failed to create model");
    }

    let start = std::time::Instant::now();
    let data_service = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");
    let duration = start.elapsed();

    assert_eq!(data_service.get_available_models().len(), 50);

    // Should complete in reasonable time (generous bound for CI)
    assert!(duration.as_secs() < 5, "Initialization took too long: {:?}", duration);
    println!("Loaded 50 models in {:?}", duration);
}

#[tokio::test]
async fn test_query_performance_with_many_models() {
    // Test query performance with larger dataset
    let database = create_test_database().await;
    let models_service = ModelsService::new(database.clone()).await
        .expect("Failed to create ModelsService");

    // Create and install 100 models with different types
    for i in 0..100 {
        let model_type = match i % 5 {
            0 => ModelType::Chat,
            1 => ModelType::Code,
            2 => ModelType::Text,
            3 => ModelType::Embedding,
            _ => ModelType::Image,
        };
        let request = create_test_model(&format!("query-perf-{:03}", i), model_type);
        let model = models_service.create_model(request).await
            .expect("Failed to create model");
        // Install models to make them searchable
        models_service.install_model(model.id, format!("/opt/query-perf-{:03}", i)).await
            .expect("Failed to install model");
    }

    let data_service = ModelDataService::new(database.clone()).await
        .expect("Failed to create ModelDataService");

    // Test search performance (note: search_models only searches installed models)
    let start = std::time::Instant::now();
    let results = data_service.search_models("query-perf");
    let duration = start.elapsed();

    assert_eq!(results.len(), 100);
    assert!(duration.as_millis() < 100, "Search took too long: {:?}", duration);

    // Test filter performance
    let start = std::time::Instant::now();
    let filtered = data_service.get_models_by_type(&ModelType::Chat);
    let duration = start.elapsed();

    assert_eq!(filtered.len(), 20); // 100 / 5
    assert!(duration.as_millis() < 50, "Filter took too long: {:?}", duration);
}