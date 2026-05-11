#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types)]

use burncloud_tests::TestClient;
use serde_json::json;
use uuid::Uuid;

use crate::common as common_mod;

fn unique_username(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().as_simple())
}

async fn get_admin_token(base_url: &str) -> String {
    let client = reqwest::Client::new();
    let username = unique_username("e2e_204_admin");
    let register_url = format!("{base_url}/api/auth/register");
    let register_body = json!({
        "username": username,
        "password": "QaTest169!",
        "email": "qa204@e2e.test"
    });
    let resp: serde_json::Value = client
        .post(&register_url)
        .json(&register_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resp["success"], true, "register should succeed: {resp}");
    resp["data"]["token"].as_str().unwrap().to_string()
}

/// Helper: extract model_mapping from a channel API response.
/// model_mapping is serialized as a native JSON object (not a string) by model_mapping_serde.
/// Returns None if null, Some(Value) otherwise.
fn extract_model_mapping(channel_data: &serde_json::Value) -> Option<serde_json::Value> {
    let mm = &channel_data["model_mapping"];
    if mm.is_null() {
        None
    } else {
        Some(mm.clone())
    }
}

/// Test that creating a channel with models and model_mapping
/// causes sync_abilities to write all models, mapping keys, and mapping values
/// to channel_abilities (lowercased). Verified via the sync-abilities endpoint
/// and the get_channel API (which returns the stored models/model_mapping).
#[tokio::test]
async fn test_sync_abilities_writes_models_keys_and_values() {
    let base_url = common_mod::spawn_app().await;
    let admin_token = get_admin_token(&base_url).await;
    let client = TestClient::new(&base_url).with_token(&admin_token);

    // Create a channel with models and model_mapping
    let create_resp = client
        .post(
            "/console/api/channel",
            &json!({
                "type": 1,
                "key": "sk-test-abilities",
                "name": "abilities-test-channel",
                "weight": 10,
                "base_url": "https://api.openai.com",
                "models": "gpt-4o-latest,astral-3-5-sonnet-20241022",
                "group": "default",
                "priority": 10,
                "model_mapping": "{\"gpt-4o-mini\": \"gpt-4o\"}"
            }),
        )
        .await
        .unwrap();

    let channel_id = create_resp["data"]["id"].as_i64().unwrap();

    // Verify the channel was created with correct models (lowercased)
    let get_resp = client
        .get(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
    assert_eq!(get_resp["success"], true);
    let models = get_resp["data"]["models"].as_str().unwrap();
    assert!(
        models.contains("gpt-4o-latest"),
        "models should contain 'gpt-4o-latest', got: {models}"
    );
    assert!(
        models.contains("astral-3-5-sonnet-20241022"),
        "models should contain 'astral-3-5-sonnet-20241022', got: {models}"
    );

    // Verify model_mapping is stored (not NULL) — serialized as native JSON object
    let model_mapping = extract_model_mapping(&get_resp["data"]);
    assert!(
        model_mapping.is_some(),
        "model_mapping should not be null after create"
    );
    let mapping = model_mapping.unwrap();
    assert_eq!(mapping["gpt-4o-mini"], "gpt-4o");

    // Call sync-abilities endpoint — should succeed (no UNIQUE constraint error)
    let sync_resp = client
        .post(
            &format!("/console/api/channel/{}/sync-abilities", channel_id),
            &json!({}),
        )
        .await
        .unwrap();
    assert_eq!(
        sync_resp["success"], true,
        "sync-abilities endpoint should return success: {sync_resp}"
    );

    // Call sync-abilities again — idempotent, should still succeed
    let sync_resp2 = client
        .post(
            &format!("/console/api/channel/{}/sync-abilities", channel_id),
            &json!({}),
        )
        .await
        .unwrap();
    assert_eq!(
        sync_resp2["success"], true,
        "sync-abilities should be idempotent: {sync_resp2}"
    );

    client
        .delete(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
}

/// Test that model_mapping is never NULL in the channel_providers table.
/// The fix ensures model_mapping defaults to "{}" when not provided.
#[tokio::test]
async fn test_model_mapping_defaults_to_empty_json() {
    let base_url = common_mod::spawn_app().await;
    let admin_token = get_admin_token(&base_url).await;
    let client = TestClient::new(&base_url).with_token(&admin_token);

    // Create a channel WITHOUT model_mapping field
    let create_resp = client
        .post(
            "/console/api/channel",
            &json!({
                "type": 1,
                "key": "sk-test-no-mapping",
                "name": "no-mapping-channel",
                "weight": 10,
                "base_url": "https://api.openai.com",
                "models": "gpt-4o",
                "group": "default",
                "priority": 10
            }),
        )
        .await
        .unwrap();

    let channel_id = create_resp["data"]["id"].as_i64().unwrap();

    // Verify via get_channel that model_mapping is not null (defaults to "{}")
    let get_resp = client
        .get(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
    assert_eq!(get_resp["success"], true);
    let model_mapping = extract_model_mapping(&get_resp["data"]);
    assert!(
        model_mapping.is_some(),
        "model_mapping should default to non-null in DB when not provided"
    );
    // Empty JSON object "{}" is serialized as a native empty object
    let mapping = model_mapping.unwrap();
    assert!(
        mapping.is_object() && mapping.as_object().unwrap().is_empty(),
        "model_mapping should default to empty JSON object, got: {mapping}"
    );

    client
        .delete(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
}

/// Test that models field is normalized to lowercase when creating a channel.
#[tokio::test]
async fn test_models_normalized_to_lowercase() {
    let base_url = common_mod::spawn_app().await;
    let admin_token = get_admin_token(&base_url).await;
    let client = TestClient::new(&base_url).with_token(&admin_token);

    // Create a channel with mixed-case model names
    let create_resp = client
        .post(
            "/console/api/channel",
            &json!({
                "type": 1,
                "key": "sk-test-lowercase",
                "name": "lowercase-test-channel",
                "weight": 10,
                "base_url": "https://api.openai.com",
                "models": "GPT-4O, Claude-3-5-Sonnet",
                "group": "default",
                "priority": 10
            }),
        )
        .await
        .unwrap();

    let channel_id = create_resp["data"]["id"].as_i64().unwrap();

    // Verify via get_channel that models are lowercase
    let get_resp = client
        .get(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
    assert_eq!(get_resp["success"], true);
    let models = get_resp["data"]["models"].as_str().unwrap();
    assert_eq!(
        models, "gpt-4o,claude-3-5-sonnet",
        "models field should be normalized to lowercase in DB"
    );

    // sync-abilities should succeed (verifies channel_abilities has lowercase entries)
    let sync_resp = client
        .post(
            &format!("/console/api/channel/{}/sync-abilities", channel_id),
            &json!({}),
        )
        .await
        .unwrap();
    assert_eq!(
        sync_resp["success"], true,
        "sync-abilities should succeed for lowercase models"
    );

    client
        .delete(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
}

/// Test that the sync-abilities endpoint correctly re-syncs abilities
/// after a channel update.
#[tokio::test]
async fn test_sync_abilities_endpoint_after_update() {
    let base_url = common_mod::spawn_app().await;
    let admin_token = get_admin_token(&base_url).await;
    let client = TestClient::new(&base_url).with_token(&admin_token);

    // Create a channel with a simple model
    let create_resp = client
        .post(
            "/console/api/channel",
            &json!({
                "type": 1,
                "key": "sk-test-sync-endpoint",
                "name": "sync-endpoint-channel",
                "weight": 10,
                "base_url": "https://api.openai.com",
                "models": "gpt-4o",
                "group": "default",
                "priority": 10
            }),
        )
        .await
        .unwrap();

    let channel_id = create_resp["data"]["id"].as_i64().unwrap();

    // Update the channel to add a model — PUT is on /console/api/channel with id in body
    let _update_resp = client
        .put(
            "/console/api/channel",
            &json!({
                "id": channel_id,
                "type": 1,
                "key": "sk-test-sync-endpoint",
                "name": "sync-endpoint-channel",
                "weight": 10,
                "base_url": "https://api.openai.com",
                "models": "gpt-4o,astral-code-latest",
                "group": "default",
                "priority": 10
            }),
        )
        .await
        .unwrap();

    // Call sync-abilities endpoint
    let sync_resp = client
        .post(
            &format!("/console/api/channel/{}/sync-abilities", channel_id),
            &json!({}),
        )
        .await
        .unwrap();

    assert_eq!(
        sync_resp["success"], true,
        "sync-abilities endpoint should return success"
    );

    // Verify the updated channel has the new model
    let get_resp = client
        .get(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
    assert_eq!(get_resp["success"], true);
    let models = get_resp["data"]["models"].as_str().unwrap();
    assert!(
        models.contains("astral-code-latest"),
        "updated channel should contain 'astral-code-latest', got: {models}"
    );

    client
        .delete(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
}

/// Test that model_mapping values are also written to channel_abilities.
/// Verified by checking the stored model_mapping via get_channel API.
#[tokio::test]
async fn test_model_mapping_values_in_abilities() {
    let base_url = common_mod::spawn_app().await;
    let admin_token = get_admin_token(&base_url).await;
    let client = TestClient::new(&base_url).with_token(&admin_token);

    // Create a channel where model_mapping values have suffixes
    let create_resp = client
        .post(
            "/console/api/channel",
            &json!({
                "type": 1,
                "key": "sk-test-mapping-values",
                "name": "mapping-values-channel",
                "weight": 10,
                "base_url": "https://api.openai.com",
                "models": "gpt-4o",
                "group": "default",
                "priority": 10,
                "model_mapping": "{\"gpt-4o-mini\": \"gpt-4o-latest\", \"claude-instant\": \"claude-3-5-sonnet-20241022\"}"
            }),
        )
        .await
        .unwrap();

    let channel_id = create_resp["data"]["id"].as_i64().unwrap();

    // Verify model_mapping is stored correctly — serialized as native JSON object
    let get_resp = client
        .get(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
    assert_eq!(get_resp["success"], true);
    let model_mapping = extract_model_mapping(&get_resp["data"]).unwrap();
    assert_eq!(model_mapping["gpt-4o-mini"], "gpt-4o-latest");
    assert_eq!(model_mapping["claude-instant"], "claude-3-5-sonnet-20241022");

    // sync-abilities should succeed (verifies abilities rows were written)
    let sync_resp = client
        .post(
            &format!("/console/api/channel/{}/sync-abilities", channel_id),
            &json!({}),
        )
        .await
        .unwrap();
    assert_eq!(
        sync_resp["success"], true,
        "sync-abilities should succeed: {sync_resp}"
    );

    client
        .delete(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
}

/// Test that updating a channel preserves model_mapping (never becomes NULL).
#[tokio::test]
async fn test_update_preserves_model_mapping() {
    let base_url = common_mod::spawn_app().await;
    let admin_token = get_admin_token(&base_url).await;
    let client = TestClient::new(&base_url).with_token(&admin_token);

    // Create a channel with model_mapping
    let create_resp = client
        .post(
            "/console/api/channel",
            &json!({
                "type": 1,
                "key": "sk-test-update-mapping",
                "name": "update-mapping-channel",
                "weight": 10,
                "base_url": "https://api.openai.com",
                "models": "gpt-4o",
                "group": "default",
                "priority": 10,
                "model_mapping": "{\"gpt-4o-mini\": \"gpt-4o\"}"
            }),
        )
        .await
        .unwrap();

    let channel_id = create_resp["data"]["id"].as_i64().unwrap();

    // Update the channel — PUT is on /console/api/channel with id in body
    let _update_resp = client
        .put(
            "/console/api/channel",
            &json!({
                "id": channel_id,
                "type": 1,
                "key": "sk-test-update-mapping",
                "name": "update-mapping-channel-updated",
                "weight": 20,
                "base_url": "https://api.openai.com",
                "models": "gpt-4o",
                "group": "default",
                "priority": 10,
                "model_mapping": "{\"gpt-4o-mini\": \"gpt-4o\"}"
            }),
        )
        .await
        .unwrap();

    // Verify via get_channel that model_mapping is still present
    let get_resp = client
        .get(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
    assert_eq!(get_resp["success"], true);
    let model_mapping = extract_model_mapping(&get_resp["data"]);
    assert!(
        model_mapping.is_some(),
        "model_mapping should remain non-NULL after update when explicitly provided"
    );
    let mapping = model_mapping.unwrap();
    assert_eq!(mapping["gpt-4o-mini"], "gpt-4o");

    client
        .delete(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
}

/// Test that models field is normalized to lowercase when updating a channel.
/// This verifies the fix for subtask 1: update() now normalizes models.
#[tokio::test]
async fn test_models_normalized_to_lowercase_on_update() {
    let base_url = common_mod::spawn_app().await;
    let admin_token = get_admin_token(&base_url).await;
    let client = TestClient::new(&base_url).with_token(&admin_token);

    // Create a channel with lowercase models
    let create_resp = client
        .post(
            "/console/api/channel",
            &json!({
                "type": 1,
                "key": "sk-test-update-models-lowercase",
                "name": "update-models-lowercase-channel",
                "weight": 10,
                "base_url": "https://api.openai.com",
                "models": "gpt-4o",
                "group": "default",
                "priority": 10
            }),
        )
        .await
        .unwrap();

    let channel_id = create_resp["data"]["id"].as_i64().unwrap();

    // Update the channel with mixed-case model names (e.g., "GPT-4O, Claude-3-5-Sonnet")
    let _update_resp = client
        .put(
            "/console/api/channel",
            &json!({
                "id": channel_id,
                "type": 1,
                "key": "sk-test-update-models-lowercase",
                "name": "update-models-lowercase-channel",
                "weight": 10,
                "base_url": "https://api.openai.com",
                "models": "GPT-4O, Claude-3-5-Sonnet",
                "group": "default",
                "priority": 10
            }),
        )
        .await
        .unwrap();

    // Verify via get_channel that models are normalized to lowercase
    let get_resp = client
        .get(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
    assert_eq!(get_resp["success"], true);
    let models = get_resp["data"]["models"].as_str().unwrap();
    assert_eq!(
        models, "gpt-4o,claude-3-5-sonnet",
        "models field should be normalized to lowercase after update, got: {models}"
    );

    // sync-abilities endpoint should work correctly after update
    let sync_resp = client
        .post(
            &format!("/console/api/channel/{}/sync-abilities", channel_id),
            &json!({}),
        )
        .await
        .unwrap();
    assert_eq!(
        sync_resp["success"], true,
        "sync-abilities endpoint should work correctly after update: {sync_resp}"
    );

    client
        .delete(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
}
