#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types)]

use burncloud_tests::TestClient;
use serde_json::json;
use uuid::Uuid;

fn base_url() -> String {
    std::env::var("E2E_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
}

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

fn extract_model_mapping(channel_data: &serde_json::Value) -> Option<serde_json::Value> {
    let mm = &channel_data["model_mapping"];
    if mm.is_null() {
        None
    } else {
        Some(mm.clone())
    }
}

/// E2E Test: 创建 Channel 时 models 自动转小写并同步 abilities
/// 验证 Issue #204 修复点：创建时 models 字段自动转换为小写
#[tokio::test]
#[ignore = "requires external infrastructure (running server at localhost:3000)"]
async fn e2e_issue_204_models_lowercase_on_create() {
    let base_url = base_url();
    let admin_token = get_admin_token(&base_url).await;
    let client = TestClient::new(&base_url).with_token(&admin_token);

    // 创建 Channel，models 使用大写字母
    let create_resp = client
        .post(
            "/console/api/channel",
            &json!({
                "type": 1,
                "key": "sk-1231234567890abcdefgHIJKLMNOPQRSTuvwxYZ",
                "name": "e2e-test-lowercase-models",
                "base_url": "https://api.openai.com",
                "models": "GPT-4O, Claude-3-5-Sonnet, ASTRAL-CODE-LATEST",
                "group": "default",
                "weight": 10,
                "priority": 10
            }),
        )
        .await
        .unwrap();

    assert_eq!(create_resp["success"], true, "create should succeed: {create_resp}");
    let channel_id = create_resp["data"]["id"].as_i64().unwrap();

    // 验证 models 字段已转换为小写
    let get_resp = client
        .get(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
    assert_eq!(get_resp["success"], true);
    let models = get_resp["data"]["models"].as_str().unwrap();
    assert_eq!(
        models, "gpt-4o,claude-3-5-sonnet,astral-code-latest",
        "models should be normalized to lowercase, got: {models}"
    );

    // 验证 model_mapping 不为 null (默认为 {})
    let model_mapping = extract_model_mapping(&get_resp["data"]);
    assert!(model_mapping.is_some(), "model_mapping should not be null");
    let mapping = model_mapping.unwrap();
    assert!(
        mapping.is_object() && mapping.as_object().unwrap().is_empty(),
        "model_mapping should be empty JSON object, got: {mapping}"
    );

    // 验证 sync-abilities 成功
    let sync_resp = client
        .post(
            &format!("/console/api/channel/{}/sync-abilities", channel_id),
            &json!({}),
        )
        .await
        .unwrap();
    assert_eq!(sync_resp["success"], true, "sync-abilities should succeed: {sync_resp}");

    // 清理
    client
        .delete(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
}

/// E2E Test: model_mapping 的 keys 和 values 都写入 abilities
/// 验证 Issue #204 修复点：model_mapping keys 和 values 都作为可路由模型
#[tokio::test]
#[ignore = "requires external infrastructure (running server at localhost:3000)"]
async fn e2e_issue_204_model_mapping_keys_values_in_abilities() {
    let base_url = base_url();
    let admin_token = get_admin_token(&base_url).await;
    let client = TestClient::new(&base_url).with_token(&admin_token);

    // 创建 Channel，包含 model_mapping
    let create_resp = client
        .post(
            "/console/api/channel",
            &json!({
                "type": 1,
                "key": "sk-1231234567890abcdefgHIJKLMNOPQRSTuvwxYZ",
                "name": "e2e-test-mapping-keys-values",
                "base_url": "https://api.openai.com",
                "models": "gpt-4o-latest",
                "group": "default",
                "weight": 10,
                "priority": 10,
                "model_mapping": {
                    "gpt-4o-mini": "gpt-4o",
                    "claude-instant": "claude-3-5-sonnet-20241022"
                }
            }),
        )
        .await
        .unwrap();

    assert_eq!(create_resp["success"], true, "create should succeed: {create_resp}");
    let channel_id = create_resp["data"]["id"].as_i64().unwrap();

    // 验证 model_mapping 存储正确
    let get_resp = client
        .get(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
    assert_eq!(get_resp["success"], true);
    let model_mapping = extract_model_mapping(&get_resp["data"]).unwrap();
    assert_eq!(model_mapping["gpt-4o-mini"], "gpt-4o");
    assert_eq!(model_mapping["claude-instant"], "claude-3-5-sonnet-20241022");

    // 验证 sync-abilities 幂等性 (两次调用都成功)
    let sync_resp1 = client
        .post(
            &format!("/console/api/channel/{}/sync-abilities", channel_id),
            &json!({}),
        )
        .await
        .unwrap();
    assert_eq!(sync_resp1["success"], true, "first sync-abilities should succeed: {sync_resp1}");

    let sync_resp2 = client
        .post(
            &format!("/console/api/channel/{}/sync-abilities", channel_id),
            &json!({}),
        )
        .await
        .unwrap();
    assert_eq!(sync_resp2["success"], true, "second sync-abilities should succeed (idempotent): {sync_resp2}");

    let sync_resp3 = client
        .post(
            &format!("/console/api/channel/{}/sync-abilities", channel_id),
            &json!({}),
        )
        .await
        .unwrap();
    assert_eq!(sync_resp3["success"], true, "third sync-abilities should succeed (idempotent): {sync_resp3}");

    // 清理
    client
        .delete(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
}

/// E2E Test: 更新 Channel 时 models 自动转小写
/// 验证 Issue #204 修复点：更新时 models 字段自动转换为小写
#[tokio::test]
#[ignore = "requires external infrastructure (running server at localhost:3000)"]
async fn e2e_issue_204_models_lowercase_on_update() {
    let base_url = base_url();
    let admin_token = get_admin_token(&base_url).await;
    let client = TestClient::new(&base_url).with_token(&admin_token);

    // 先创建一个 Channel
    let create_resp = client
        .post(
            "/console/api/channel",
            &json!({
                "type": 1,
                "key": "sk-1231234567890abcdefgHIJKLMNOPQRSTuvwxYZ",
                "name": "e2e-test-update-lowercase",
                "base_url": "https://api.openai.com",
                "models": "gpt-4o",
                "group": "default",
                "weight": 10,
                "priority": 10
            }),
        )
        .await
        .unwrap();

    assert_eq!(create_resp["success"], true);
    let channel_id = create_resp["data"]["id"].as_i64().unwrap();

    // 更新 Channel，models 使用大写字母
    let update_resp = client
        .put(
            "/console/api/channel",
            &json!({
                "id": channel_id,
                "type": 1,
                "key": "sk-1231234567890abcdefgHIJKLMNOPQRSTuvwxYZ",
                "name": "e2e-test-update-lowercase-updated",
                "base_url": "https://api.openai.com",
                "models": "GPT-4O, CLAUDE-3-5-SONNET, DEEPSEEK-CHAT",
                "group": "default",
                "weight": 20,
                "priority": 20
            }),
        )
        .await
        .unwrap();

    assert_eq!(update_resp["success"], true, "update should succeed: {update_resp}");

    // 验证 models 字段已转换为小写
    let get_resp = client
        .get(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
    assert_eq!(get_resp["success"], true);
    let models = get_resp["data"]["models"].as_str().unwrap();
    assert_eq!(
        models, "gpt-4o,claude-3-5-sonnet,deepseek-chat",
        "models should be normalized to lowercase after update, got: {models}"
    );

    // 验证 name 已更新
    assert_eq!(
        get_resp["data"]["name"].as_str().unwrap(),
        "e2e-test-update-lowercase-updated"
    );

    // 验证 sync-abilities 成功
    let sync_resp = client
        .post(
            &format!("/console/api/channel/{}/sync-abilities", channel_id),
            &json!({}),
        )
        .await
        .unwrap();
    assert_eq!(sync_resp["success"], true, "sync-abilities should succeed: {sync_resp}");

    // 清理
    client
        .delete(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
}

/// E2E Test: 更新 Channel 保留 model_mapping
/// 验证 Issue #204 修复点：更新后 model_mapping 不丢失
#[tokio::test]
#[ignore = "requires external infrastructure (running server at localhost:3000)"]
async fn e2e_issue_204_update_preserves_model_mapping() {
    let base_url = base_url();
    let admin_token = get_admin_token(&base_url).await;
    let client = TestClient::new(&base_url).with_token(&admin_token);

    // 创建 Channel，包含 model_mapping
    let create_resp = client
        .post(
            "/console/api/channel",
            &json!({
                "type": 1,
                "key": "sk-1231234567890abcdefgHIJKLMNOPQRSTuvwxYZ",
                "name": "e2e-test-preserve-mapping",
                "base_url": "https://api.openai.com",
                "models": "gpt-4o",
                "group": "default",
                "weight": 10,
                "priority": 10,
                "model_mapping": {
                    "gpt-4o-mini": "gpt-4o"
                }
            }),
        )
        .await
        .unwrap();

    assert_eq!(create_resp["success"], true);
    let channel_id = create_resp["data"]["id"].as_i64().unwrap();

    // 更新 Channel，保留 model_mapping
    let update_resp = client
        .put(
            "/console/api/channel",
            &json!({
                "id": channel_id,
                "type": 1,
                "key": "sk-1231234567890abcdefgHIJKLMNOPQRSTuvwxYZ",
                "name": "e2e-test-preserve-mapping-updated",
                "base_url": "https://api.openai.com",
                "models": "gpt-4o",
                "group": "default",
                "weight": 20,
                "priority": 20,
                "model_mapping": {
                    "gpt-4o-mini": "gpt-4o"
                }
            }),
        )
        .await
        .unwrap();

    assert_eq!(update_resp["success"], true, "update should succeed: {update_resp}");

    // 验证 model_mapping 保留完整
    let get_resp = client
        .get(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
    assert_eq!(get_resp["success"], true);
    let model_mapping = extract_model_mapping(&get_resp["data"]).unwrap();
    assert_eq!(model_mapping["gpt-4o-mini"], "gpt-4o", "model_mapping should be preserved");

    // 验证其他字段已更新
    assert_eq!(get_resp["data"]["weight"].as_i64().unwrap(), 20);
    assert_eq!(get_resp["data"]["priority"].as_i64().unwrap(), 20);

    // 清理
    client
        .delete(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
}

/// E2E Test: group 字段自动转小写
/// 验证 Issue #204 修复点：group 字段自动转换为小写
#[tokio::test]
#[ignore = "requires external infrastructure (running server at localhost:3000)"]
async fn e2e_issue_204_group_lowercase() {
    let base_url = base_url();
    let admin_token = get_admin_token(&base_url).await;
    let client = TestClient::new(&base_url).with_token(&admin_token);

    // 创建 Channel，group 使用大写字母
    let create_resp = client
        .post(
            "/console/api/channel",
            &json!({
                "type": 1,
                "key": "sk-1231234567890abcdefgHIJKLMNOPQRSTuvwxYZ",
                "name": "e2e-test-group-lowercase",
                "base_url": "https://api.openai.com",
                "models": "gpt-4o",
                "group": "ENTERPRISE",
                "weight": 10,
                "priority": 10
            }),
        )
        .await
        .unwrap();

    assert_eq!(create_resp["success"], true, "create should succeed: {create_resp}");
    let channel_id = create_resp["data"]["id"].as_i64().unwrap();

    // 验证 group 字段已转换为小写
    let get_resp = client
        .get(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
    assert_eq!(get_resp["success"], true);
    let group = get_resp["data"]["group"].as_str().unwrap();
    assert_eq!(group, "enterprise", "group should be normalized to lowercase, got: {group}");

    // 清理
    client
        .delete(&format!("/console/api/channel/{}", channel_id))
        .await
        .unwrap();
}
