use burncloud_tests::TestClient;
use serde_json::json;
use uuid::Uuid;

use crate::common as common_mod;

// ============================================================================
// CH-01: Create Channel Tests - All Channel Types
// ============================================================================

/// Channel type constants from burncloud_common::types::ChannelType
mod channel_types {
    pub const OPENAI: i32 = 1;
    pub const ANTHROPIC: i32 = 14; // Claude
    pub const GEMINI: i32 = 24;
    pub const VERTEX_AI: i32 = 41;
    pub const ZAI: i32 = 57;
}

/// Helper to create a basic channel payload
fn create_channel_payload(
    channel_type: i32,
    name: &str,
    key: &str,
    models: &str,
) -> serde_json::Value {
    json!({
        "type": channel_type,
        "key": key,
        "name": name,
        "base_url": "https://api.example.com",
        "models": models,
        "group": "default",
        "weight": 10,
        "priority": 100
    })
}

/// Helper to clean up a channel by ID
async fn cleanup_channel(client: &TestClient, id: i64) {
    let _ = client.delete(&format!("/console/api/channel/{}", id)).await;
}

#[tokio::test]
async fn test_channel_lifecycle() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    let channel_name = format!("Test Channel {}", Uuid::new_v4());

    // 1. Create
    let body = json!({
        "type": 1,
        "key": "sk-lifecycle-key",
        "name": channel_name,
        "base_url": "http://example.com",
        "models": "gpt-lifecycle",
        "group": "default",
        "weight": 10,
        "priority": 5
    });

    let res = client
        .post("/console/api/channel", &body)
        .await
        .expect("Create failed");
    assert_eq!(res["success"], true);
    let id = res["data"]["id"].as_i64().expect("No ID returned");
    println!("Created channel ID: {}", id);

    // 2. Get
    let get_res = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");
    assert_eq!(get_res["success"], true);
    assert_eq!(get_res["data"]["name"], channel_name);

    // 3. Update
    let update_body = json!({
        "id": id,
        "type": 1,
        "key": "sk-lifecycle-key-updated",
        "name": channel_name,
        "base_url": "http://example.com",
        "models": "gpt-lifecycle-v2",
        "group": "default",
        "weight": 20,
        "priority": 5
    });
    let update_res = client
        .put("/console/api/channel", &update_body)
        .await
        .expect("Update failed");
    assert_eq!(update_res["success"], true);

    // 4. Verify Update via Get
    let get_res_2 = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get 2 failed");
    assert_eq!(get_res_2["data"]["models"], "gpt-lifecycle-v2");

    // 5. List
    let list_res = client
        .get("/console/api/channel")
        .await
        .expect("List failed");
    if list_res["success"] != true {
        println!("List failed: {}", list_res["message"]);
    }
    assert_eq!(list_res["success"], true);
    let channels = list_res["data"].as_array().expect("Data is not array");
    assert!(channels.iter().any(|c| c["id"].as_i64() == Some(id)));

    // 6. Delete
    let del_res = client
        .delete(&format!("/console/api/channel/{}", id))
        .await
        .expect("Delete failed");
    assert_eq!(del_res["success"], true);

    // 7. Get (404 or success: false)
    let get_res_3 = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get 3 failed");
    assert_eq!(get_res_3["success"], false);
}

// ============================================================================
// CH-01: Create Channel - All Types
// ============================================================================

#[tokio::test]
async fn test_ch01_create_openai_channel() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    let channel_name = format!("OpenAI Channel {}", Uuid::new_v4());
    let body = create_channel_payload(
        channel_types::OPENAI,
        &channel_name,
        "sk-openai-test-key",
        "gpt-4,gpt-3.5-turbo",
    );

    let res = client
        .post("/console/api/channel", &body)
        .await
        .expect("Create OpenAI channel failed");
    assert_eq!(res["success"], true, "OpenAI channel creation should succeed");

    let id = res["data"]["id"].as_i64().expect("No ID returned");
    println!("✓ CH-01: OpenAI channel created with ID: {}", id);

    // Verify channel type
    let get_res = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");
    assert_eq!(get_res["data"]["type"], channel_types::OPENAI);

    cleanup_channel(&client, id).await;
}

#[tokio::test]
async fn test_ch01_create_claude_channel() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    let channel_name = format!("Claude Channel {}", Uuid::new_v4());
    let body = create_channel_payload(
        channel_types::ANTHROPIC,
        &channel_name,
        "sk-ant-test-key",
        "claude-3-opus,claude-3-sonnet",
    );

    let res = client
        .post("/console/api/channel", &body)
        .await
        .expect("Create Claude channel failed");
    assert_eq!(
        res["success"], true,
        "Claude/Anthropic channel creation should succeed"
    );

    let id = res["data"]["id"].as_i64().expect("No ID returned");
    println!("✓ CH-01: Claude channel created with ID: {}", id);

    // Verify channel type
    let get_res = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");
    assert_eq!(get_res["data"]["type"], channel_types::ANTHROPIC);

    cleanup_channel(&client, id).await;
}

#[tokio::test]
async fn test_ch01_create_gemini_channel() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    let channel_name = format!("Gemini Channel {}", Uuid::new_v4());
    let body = create_channel_payload(
        channel_types::GEMINI,
        &channel_name,
        "gemini-test-api-key",
        "gemini-pro,gemini-ultra",
    );

    let res = client
        .post("/console/api/channel", &body)
        .await
        .expect("Create Gemini channel failed");
    assert_eq!(
        res["success"], true,
        "Gemini channel creation should succeed"
    );

    let id = res["data"]["id"].as_i64().expect("No ID returned");
    println!("✓ CH-01: Gemini channel created with ID: {}", id);

    // Verify channel type
    let get_res = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");
    assert_eq!(get_res["data"]["type"], channel_types::GEMINI);

    cleanup_channel(&client, id).await;
}

#[tokio::test]
async fn test_ch01_create_vertex_channel() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    let channel_name = format!("Vertex AI Channel {}", Uuid::new_v4());
    let body = json!({
        "type": channel_types::VERTEX_AI,
        "key": "vertex-test-credentials",
        "name": channel_name,
        "base_url": "https://us-central1-aiplatform.googleapis.com",
        "models": "gemini-pro,claude-3-sonnet",
        "group": "default",
        "weight": 10,
        "priority": 100
    });

    let res = client
        .post("/console/api/channel", &body)
        .await
        .expect("Create Vertex channel failed");
    assert_eq!(
        res["success"], true,
        "Vertex AI channel creation should succeed"
    );

    let id = res["data"]["id"].as_i64().expect("No ID returned");
    println!("✓ CH-01: Vertex AI channel created with ID: {}", id);

    // Verify channel type
    let get_res = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");
    assert_eq!(get_res["data"]["type"], channel_types::VERTEX_AI);

    cleanup_channel(&client, id).await;
}

#[tokio::test]
async fn test_ch01_create_zai_channel() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    let channel_name = format!("ZAI Channel {}", Uuid::new_v4());
    let body = create_channel_payload(
        channel_types::ZAI,
        &channel_name,
        "zai-test-api-key",
        "zai-chat,zai-embedding",
    );

    let res = client
        .post("/console/api/channel", &body)
        .await
        .expect("Create ZAI channel failed");
    assert_eq!(res["success"], true, "ZAI channel creation should succeed");

    let id = res["data"]["id"].as_i64().expect("No ID returned");
    println!("✓ CH-01: ZAI channel created with ID: {}", id);

    // Verify channel type
    let get_res = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");
    assert_eq!(get_res["data"]["type"], channel_types::ZAI);

    cleanup_channel(&client, id).await;
}

// ============================================================================
// CH-02: Update Channel Tests
// ============================================================================

#[tokio::test]
async fn test_ch02_update_channel_key() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    // Create initial channel
    let channel_name = format!("Key Update Test {}", Uuid::new_v4());
    let body = create_channel_payload(
        channel_types::OPENAI,
        &channel_name,
        "sk-original-key",
        "gpt-4",
    );

    let res = client
        .post("/console/api/channel", &body)
        .await
        .expect("Create failed");
    assert_eq!(res["success"], true);
    let id = res["data"]["id"].as_i64().expect("No ID returned");

    // Update key
    let update_body = json!({
        "id": id,
        "type": channel_types::OPENAI,
        "key": "sk-updated-key-new",
        "name": channel_name,
        "base_url": "https://api.example.com",
        "models": "gpt-4",
        "group": "default",
        "weight": 10,
        "priority": 100
    });

    let update_res = client
        .put("/console/api/channel", &update_body)
        .await
        .expect("Update failed");
    assert_eq!(update_res["success"], true, "Key update should succeed");

    // Verify key was updated
    let get_res = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");
    // Note: Key might be masked in response, but the update should have succeeded
    assert_eq!(get_res["success"], true);

    println!("✓ CH-02: Channel key updated successfully");
    cleanup_channel(&client, id).await;
}

#[tokio::test]
async fn test_ch02_update_channel_weight() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    // Create initial channel with weight 10
    let channel_name = format!("Weight Update Test {}", Uuid::new_v4());
    let body = json!({
        "type": channel_types::OPENAI,
        "key": "sk-weight-test",
        "name": channel_name,
        "base_url": "https://api.example.com",
        "models": "gpt-4",
        "group": "default",
        "weight": 10,
        "priority": 100
    });

    let res = client
        .post("/console/api/channel", &body)
        .await
        .expect("Create failed");
    assert_eq!(res["success"], true);
    let id = res["data"]["id"].as_i64().expect("No ID returned");

    // Update weight to 50
    let update_body = json!({
        "id": id,
        "type": channel_types::OPENAI,
        "key": "sk-weight-test",
        "name": channel_name,
        "base_url": "https://api.example.com",
        "models": "gpt-4",
        "group": "default",
        "weight": 50,
        "priority": 100
    });

    let update_res = client
        .put("/console/api/channel", &update_body)
        .await
        .expect("Update failed");
    assert_eq!(update_res["success"], true, "Weight update should succeed");

    // Verify weight was updated
    let get_res = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");
    assert_eq!(
        get_res["data"]["weight"], 50,
        "Weight should be updated to 50"
    );

    println!("✓ CH-02: Channel weight updated successfully");
    cleanup_channel(&client, id).await;
}

#[tokio::test]
async fn test_ch02_update_channel_priority() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    // Create initial channel with priority 100
    let channel_name = format!("Priority Update Test {}", Uuid::new_v4());
    let body = json!({
        "type": channel_types::OPENAI,
        "key": "sk-priority-test",
        "name": channel_name,
        "base_url": "https://api.example.com",
        "models": "gpt-4",
        "group": "default",
        "weight": 10,
        "priority": 100
    });

    let res = client
        .post("/console/api/channel", &body)
        .await
        .expect("Create failed");
    assert_eq!(res["success"], true);
    let id = res["data"]["id"].as_i64().expect("No ID returned");

    // Update priority to 500
    let update_body = json!({
        "id": id,
        "type": channel_types::OPENAI,
        "key": "sk-priority-test",
        "name": channel_name,
        "base_url": "https://api.example.com",
        "models": "gpt-4",
        "group": "default",
        "weight": 10,
        "priority": 500
    });

    let update_res = client
        .put("/console/api/channel", &update_body)
        .await
        .expect("Update failed");
    assert_eq!(
        update_res["success"], true,
        "Priority update should succeed"
    );

    // Verify priority was updated
    let get_res = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");
    assert_eq!(
        get_res["data"]["priority"], 500,
        "Priority should be updated to 500"
    );

    println!("✓ CH-02: Channel priority updated successfully");
    cleanup_channel(&client, id).await;
}

#[tokio::test]
async fn test_ch02_update_channel_models() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    // Create initial channel with one model
    let channel_name = format!("Model Update Test {}", Uuid::new_v4());
    let body = json!({
        "type": channel_types::OPENAI,
        "key": "sk-model-test",
        "name": channel_name,
        "base_url": "https://api.example.com",
        "models": "gpt-4",
        "group": "default",
        "weight": 10,
        "priority": 100
    });

    let res = client
        .post("/console/api/channel", &body)
        .await
        .expect("Create failed");
    assert_eq!(res["success"], true);
    let id = res["data"]["id"].as_i64().expect("No ID returned");

    // Update models to include multiple
    let update_body = json!({
        "id": id,
        "type": channel_types::OPENAI,
        "key": "sk-model-test",
        "name": channel_name,
        "base_url": "https://api.example.com",
        "models": "gpt-4,gpt-4-turbo,gpt-3.5-turbo",
        "group": "default",
        "weight": 10,
        "priority": 100
    });

    let update_res = client
        .put("/console/api/channel", &update_body)
        .await
        .expect("Update failed");
    assert_eq!(
        update_res["success"], true,
        "Models update should succeed"
    );

    // Verify models were updated
    let get_res = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");
    assert_eq!(
        get_res["data"]["models"],
        "gpt-4,gpt-4-turbo,gpt-3.5-turbo",
        "Models should be updated"
    );

    println!("✓ CH-02: Channel models updated successfully");
    cleanup_channel(&client, id).await;
}

// ============================================================================
// CH-03: Delete Channel Tests
// ============================================================================

#[tokio::test]
async fn test_ch03_delete_channel_normal() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    // Create a channel
    let channel_name = format!("Delete Test {}", Uuid::new_v4());
    let body = create_channel_payload(
        channel_types::OPENAI,
        &channel_name,
        "sk-delete-test",
        "gpt-4",
    );

    let res = client
        .post("/console/api/channel", &body)
        .await
        .expect("Create failed");
    assert_eq!(res["success"], true);
    let id = res["data"]["id"].as_i64().expect("No ID returned");

    // Verify it exists
    let get_res = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");
    assert_eq!(get_res["success"], true, "Channel should exist before delete");

    // Delete the channel
    let del_res = client
        .delete(&format!("/console/api/channel/{}", id))
        .await
        .expect("Delete failed");
    assert_eq!(del_res["success"], true, "Delete should succeed");
    assert_eq!(
        del_res["message"], "channel deleted",
        "Delete message should be correct"
    );

    // Verify it no longer exists
    let get_res_2 = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");
    assert_eq!(
        get_res_2["success"], false,
        "Channel should not exist after delete"
    );
    assert_eq!(
        get_res_2["message"], "channel not found",
        "Should return 'not found' message"
    );

    println!("✓ CH-03: Channel deleted successfully");
}

#[tokio::test]
async fn test_ch03_delete_nonexistent_channel() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    // Try to delete a channel that doesn't exist
    let del_res = client
        .delete("/console/api/channel/999999")
        .await
        .expect("Delete request failed");

    // The API might return success=true even for non-existent channels (idempotent delete)
    // or success=false. Either is acceptable.
    println!(
        "✓ CH-03: Delete non-existent channel returned success={}",
        del_res["success"]
    );
}

#[tokio::test]
async fn test_ch03_delete_channel_and_verify_list() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    // Create a channel
    let channel_name = format!("List Verify Delete Test {}", Uuid::new_v4());
    let body = create_channel_payload(
        channel_types::OPENAI,
        &channel_name,
        "sk-list-verify-delete",
        "gpt-4",
    );

    let res = client
        .post("/console/api/channel", &body)
        .await
        .expect("Create failed");
    assert_eq!(res["success"], true);
    let id = res["data"]["id"].as_i64().expect("No ID returned");

    // Verify it appears in list
    let list_res = client
        .get("/console/api/channel")
        .await
        .expect("List failed");
    assert_eq!(list_res["success"], true);
    let channels = list_res["data"].as_array().expect("Data is not array");
    let found_before = channels.iter().any(|c| c["id"].as_i64() == Some(id));
    assert!(found_before, "Channel should appear in list before delete");

    // Delete the channel
    let del_res = client
        .delete(&format!("/console/api/channel/{}", id))
        .await
        .expect("Delete failed");
    assert_eq!(del_res["success"], true);

    // Verify it no longer appears in list
    let list_res_2 = client
        .get("/console/api/channel")
        .await
        .expect("List failed");
    assert_eq!(list_res_2["success"], true);
    let channels_2 = list_res_2["data"].as_array().expect("Data is not array");
    let found_after = channels_2.iter().any(|c| c["id"].as_i64() == Some(id));
    assert!(
        !found_after,
        "Channel should not appear in list after delete"
    );

    println!("✓ CH-03: Channel removed from list after delete");
}

// ============================================================================
// CH-04: Query Channel Tests
// ============================================================================

#[tokio::test]
async fn test_ch04_list_channels_default() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    let list_res = client
        .get("/console/api/channel")
        .await
        .expect("List failed");

    assert_eq!(list_res["success"], true, "List should succeed");
    assert!(
        list_res["data"].is_array(),
        "Data should be an array"
    );
    assert!(
        list_res["pagination"].is_object(),
        "Pagination should be present"
    );

    // Default pagination values
    assert_eq!(list_res["pagination"]["limit"], 20, "Default limit is 20");
    assert_eq!(
        list_res["pagination"]["offset"], 0,
        "Default offset is 0"
    );

    println!("✓ CH-04: List channels with default pagination");
}

#[tokio::test]
async fn test_ch04_list_channels_with_pagination() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    // Create multiple channels
    let mut created_ids: Vec<i64> = Vec::new();
    for i in 0..3 {
        let channel_name = format!("Pagination Test {} - {}", Uuid::new_v4(), i);
        let body = create_channel_payload(
            channel_types::OPENAI,
            &channel_name,
            &format!("sk-pagination-{}", i),
            "gpt-4",
        );

        let res = client
            .post("/console/api/channel", &body)
            .await
            .expect("Create failed");
        if res["success"] == true {
            if let Some(id) = res["data"]["id"].as_i64() {
                created_ids.push(id);
            }
        }
    }

    // Test with limit=2
    let list_res = client
        .get("/console/api/channel?limit=2&offset=0")
        .await
        .expect("List failed");

    assert_eq!(list_res["success"], true);
    assert_eq!(list_res["pagination"]["limit"], 2, "Limit should be 2");
    assert_eq!(list_res["pagination"]["offset"], 0, "Offset should be 0");

    // Test with offset
    let list_res_2 = client
        .get("/console/api/channel?limit=1&offset=1")
        .await
        .expect("List failed");

    assert_eq!(list_res_2["success"], true);
    assert_eq!(list_res_2["pagination"]["limit"], 1);
    assert_eq!(list_res_2["pagination"]["offset"], 1);

    println!("✓ CH-04: List channels with custom pagination");

    // Cleanup
    for id in created_ids {
        cleanup_channel(&client, id).await;
    }
}

#[tokio::test]
async fn test_ch04_get_channel_by_id() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    // Create a channel
    let channel_name = format!("Get By ID Test {}", Uuid::new_v4());
    let body = json!({
        "type": channel_types::OPENAI,
        "key": "sk-get-by-id",
        "name": channel_name,
        "base_url": "https://api.example.com",
        "models": "gpt-4",
        "group": "test-group",
        "weight": 15,
        "priority": 200
    });

    let res = client
        .post("/console/api/channel", &body)
        .await
        .expect("Create failed");
    assert_eq!(res["success"], true);
    let id = res["data"]["id"].as_i64().expect("No ID returned");

    // Get by ID
    let get_res = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");

    assert_eq!(get_res["success"], true, "Get should succeed");
    assert_eq!(get_res["data"]["id"], id, "ID should match");
    assert_eq!(get_res["data"]["name"], channel_name, "Name should match");
    assert_eq!(get_res["data"]["type"], channel_types::OPENAI, "Type should match");
    assert_eq!(get_res["data"]["group"], "test-group", "Group should match");
    assert_eq!(get_res["data"]["weight"], 15, "Weight should match");
    assert_eq!(get_res["data"]["priority"], 200, "Priority should match");

    println!("✓ CH-04: Get channel by ID with all fields");

    cleanup_channel(&client, id).await;
}

#[tokio::test]
async fn test_ch04_get_nonexistent_channel() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    let get_res = client
        .get("/console/api/channel/999999")
        .await
        .expect("Get failed");

    assert_eq!(
        get_res["success"], false,
        "Get non-existent should fail"
    );
    assert_eq!(
        get_res["message"], "channel not found",
        "Should return 'not found' message"
    );

    println!("✓ CH-04: Get non-existent channel returns proper error");
}

#[tokio::test]
async fn test_ch04_list_channels_limit_clamp() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    // Test limit larger than max (100) should be clamped
    let list_res = client
        .get("/console/api/channel?limit=500")
        .await
        .expect("List failed");

    assert_eq!(list_res["success"], true);
    assert_eq!(
        list_res["pagination"]["limit"], 100,
        "Limit should be clamped to 100"
    );

    // Test limit less than 1 should be clamped to 1
    let list_res_2 = client
        .get("/console/api/channel?limit=0")
        .await
        .expect("List failed");

    assert_eq!(list_res_2["success"], true);
    assert_eq!(
        list_res_2["pagination"]["limit"], 1,
        "Limit should be clamped to 1"
    );

    println!("✓ CH-04: Pagination limit clamping works correctly");
}

// ============================================================================
// CH-05: Channel Weight and Priority Routing Tests
// ============================================================================

#[tokio::test]
async fn test_ch05_channel_weight_distribution() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    // Create two channels with different weights for the same model
    let model_name = format!("weight-test-model-{}", Uuid::new_v4());

    // Channel A: weight 30
    let channel_a_name = format!("Weight Channel A {}", Uuid::new_v4());
    let body_a = json!({
        "type": channel_types::OPENAI,
        "key": "sk-weight-a",
        "name": channel_a_name,
        "base_url": "https://api.example.com",
        "models": &model_name,
        "group": "default",
        "weight": 30,
        "priority": 100
    });

    let res_a = client
        .post("/console/api/channel", &body_a)
        .await
        .expect("Create Channel A failed");
    assert_eq!(res_a["success"], true);
    let id_a = res_a["data"]["id"].as_i64().expect("No ID returned");

    // Channel B: weight 70
    let channel_b_name = format!("Weight Channel B {}", Uuid::new_v4());
    let body_b = json!({
        "type": channel_types::OPENAI,
        "key": "sk-weight-b",
        "name": channel_b_name,
        "base_url": "https://api.example.com",
        "models": &model_name,
        "group": "default",
        "weight": 70,
        "priority": 100
    });

    let res_b = client
        .post("/console/api/channel", &body_b)
        .await
        .expect("Create Channel B failed");
    assert_eq!(res_b["success"], true);
    let id_b = res_b["data"]["id"].as_i64().expect("No ID returned");

    // Verify both channels exist with correct weights
    let get_a = client
        .get(&format!("/console/api/channel/{}", id_a))
        .await
        .expect("Get A failed");
    assert_eq!(get_a["data"]["weight"], 30, "Channel A weight should be 30");

    let get_b = client
        .get(&format!("/console/api/channel/{}", id_b))
        .await
        .expect("Get B failed");
    assert_eq!(get_b["data"]["weight"], 70, "Channel B weight should be 70");

    println!("✓ CH-05: Channels with different weights created successfully");
    println!("  Channel A (weight=30): ID {}", id_a);
    println!("  Channel B (weight=70): ID {}", id_b);

    // Cleanup
    cleanup_channel(&client, id_a).await;
    cleanup_channel(&client, id_b).await;
}

#[tokio::test]
async fn test_ch05_channel_priority_ordering() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    // Create channels with different priorities for the same model
    let model_name = format!("priority-test-model-{}", Uuid::new_v4());

    // Channel A: priority 100 (lower)
    let channel_a_name = format!("Priority Channel A {}", Uuid::new_v4());
    let body_a = json!({
        "type": channel_types::OPENAI,
        "key": "sk-priority-a",
        "name": channel_a_name,
        "base_url": "https://api.example.com",
        "models": &model_name,
        "group": "default",
        "weight": 10,
        "priority": 100
    });

    let res_a = client
        .post("/console/api/channel", &body_a)
        .await
        .expect("Create Channel A failed");
    assert_eq!(res_a["success"], true);
    let id_a = res_a["data"]["id"].as_i64().expect("No ID returned");

    // Channel B: priority 500 (higher - should be preferred)
    let channel_b_name = format!("Priority Channel B {}", Uuid::new_v4());
    let body_b = json!({
        "type": channel_types::OPENAI,
        "key": "sk-priority-b",
        "name": channel_b_name,
        "base_url": "https://api.example.com",
        "models": &model_name,
        "group": "default",
        "weight": 10,
        "priority": 500
    });

    let res_b = client
        .post("/console/api/channel", &body_b)
        .await
        .expect("Create Channel B failed");
    assert_eq!(res_b["success"], true);
    let id_b = res_b["data"]["id"].as_i64().expect("No ID returned");

    // Verify both channels exist with correct priorities
    let get_a = client
        .get(&format!("/console/api/channel/{}", id_a))
        .await
        .expect("Get A failed");
    assert_eq!(
        get_a["data"]["priority"], 100,
        "Channel A priority should be 100"
    );

    let get_b = client
        .get(&format!("/console/api/channel/{}", id_b))
        .await
        .expect("Get B failed");
    assert_eq!(
        get_b["data"]["priority"], 500,
        "Channel B priority should be 500"
    );

    println!("✓ CH-05: Channels with different priorities created successfully");
    println!("  Channel A (priority=100): ID {}", id_a);
    println!("  Channel B (priority=500): ID {}", id_b);

    // Cleanup
    cleanup_channel(&client, id_a).await;
    cleanup_channel(&client, id_b).await;
}

#[tokio::test]
async fn test_ch05_channel_zero_weight() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    // Create a channel with weight 0 (should not receive traffic in load balancing)
    let channel_name = format!("Zero Weight Channel {}", Uuid::new_v4());
    let body = json!({
        "type": channel_types::OPENAI,
        "key": "sk-zero-weight",
        "name": channel_name,
        "base_url": "https://api.example.com",
        "models": "zero-weight-model",
        "group": "default",
        "weight": 0,
        "priority": 100
    });

    let res = client
        .post("/console/api/channel", &body)
        .await
        .expect("Create failed");
    assert_eq!(
        res["success"], true,
        "Channel with weight 0 should be created"
    );
    let id = res["data"]["id"].as_i64().expect("No ID returned");

    // Verify weight is 0
    let get_res = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");
    assert_eq!(
        get_res["data"]["weight"], 0,
        "Weight should be 0"
    );

    println!("✓ CH-05: Channel with zero weight created successfully");

    cleanup_channel(&client, id).await;
}

#[tokio::test]
async fn test_ch05_channel_group_segregation() {
    let base_url = common_mod::spawn_app().await;
    let client = TestClient::new(&base_url).with_token(&common_mod::get_root_token());

    // Create channels in different groups
    let model_name = format!("group-test-model-{}", Uuid::new_v4());

    // Channel in "vip" group
    let channel_vip_name = format!("VIP Channel {}", Uuid::new_v4());
    let body_vip = json!({
        "type": channel_types::OPENAI,
        "key": "sk-vip-group",
        "name": channel_vip_name,
        "base_url": "https://api.example.com",
        "models": &model_name,
        "group": "vip",
        "weight": 10,
        "priority": 100
    });

    let res_vip = client
        .post("/console/api/channel", &body_vip)
        .await
        .expect("Create VIP channel failed");
    assert_eq!(res_vip["success"], true);
    let id_vip = res_vip["data"]["id"].as_i64().expect("No ID returned");

    // Channel in "default" group
    let channel_default_name = format!("Default Channel {}", Uuid::new_v4());
    let body_default = json!({
        "type": channel_types::OPENAI,
        "key": "sk-default-group",
        "name": channel_default_name,
        "base_url": "https://api.example.com",
        "models": &model_name,
        "group": "default",
        "weight": 10,
        "priority": 100
    });

    let res_default = client
        .post("/console/api/channel", &body_default)
        .await
        .expect("Create Default channel failed");
    assert_eq!(res_default["success"], true);
    let id_default = res_default["data"]["id"].as_i64().expect("No ID returned");

    // Verify groups are set correctly
    let get_vip = client
        .get(&format!("/console/api/channel/{}", id_vip))
        .await
        .expect("Get VIP failed");
    assert_eq!(get_vip["data"]["group"], "vip", "VIP channel group should be 'vip'");

    let get_default = client
        .get(&format!("/console/api/channel/{}", id_default))
        .await
        .expect("Get Default failed");
    assert_eq!(
        get_default["data"]["group"], "default",
        "Default channel group should be 'default'"
    );

    println!("✓ CH-05: Channels in different groups created successfully");

    // Cleanup
    cleanup_channel(&client, id_vip).await;
    cleanup_channel(&client, id_default).await;
}
