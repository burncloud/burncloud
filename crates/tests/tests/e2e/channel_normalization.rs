#![allow(clippy::unwrap_used)]

use burncloud_tests::TestClient;
use serde_json::json;
use uuid::Uuid;

fn base_url() -> String {
    std::env::var("E2E_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".into())
}

fn unique_username(prefix: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{prefix}_{ts}")
}

async fn get_admin_token() -> String {
    let client = reqwest::Client::new();
    let username = unique_username("e2e_admin");
    let url = format!("{}/api/auth/register", base_url());
    let body = json!({
        "username": username,
        "password": "QaTest169!",
        "email": format!("{}@e2e.test", username)
    });
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .expect("register request");
    let data: serde_json::Value = resp.json().await.expect("register response json");
    data["data"]["token"].as_str().unwrap_or("").to_string()
}

/// 测试：创建 Channel 时 models 和 group 字段自动规范化
///
/// 场景：用户创建 Channel，models 和 group 字段包含大写字母、多余空格
/// 预期：存储后 models 字段被转换为小写、去除空格，group 同理
/// 覆盖：Issue #204 修复点 — normalize_models_or_group 函数在创建时生效
#[tokio::test]
#[ignore = "requires external server"]
async fn test_models_group_normalized_on_create() {
    let admin_token = get_admin_token().await;
    let client = TestClient::new(&base_url()).with_token(&admin_token);

    let channel_name = format!("Norm Test {}", Uuid::new_v4());

    let body = json!({
        "type": 1,
        "key": "sk-1231234567890abcdefgHIJKLMNOPQRSTuvwxYZ",
        "name": channel_name,
        "base_url": "https://api.openai.com",
        "models": "GPT-4, GPT-3.5-TURBO,  claude-3-opus  ",
        "group": "DEFAULT, Premium,  enterprise",
        "weight": 1,
        "priority": 0
    });

    let res = client
        .post("/console/api/channel", &body)
        .await
        .expect("Create failed");
    assert_eq!(res["success"], true);
    let id = res["data"]["id"].as_i64().expect("No ID returned");

    let get_res = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");
    assert_eq!(get_res["success"], true);

    let models = get_res["data"]["models"].as_str().expect("models missing");
    assert_eq!(models, "gpt-4,gpt-3.5-turbo,claude-3-opus");

    let group = get_res["data"]["group"].as_str().expect("group missing");
    assert_eq!(group, "default,premium,enterprise");

    let _ = client.delete(&format!("/console/api/channel/{}", id)).await;
}

/// 测试：更新 Channel 时 models 和 group 字段重新规范化
///
/// 场景：用户更新已存在的 Channel，修改 models 和 group 字段
/// 预期：更新后 models 和 group 字段被重新规范化，旧值被覆盖
/// 覆盖：Issue #204 修复点 — normalize_models_or_group 函数在更新时生效
#[tokio::test]
#[ignore = "requires external server"]
async fn test_models_group_normalized_on_update() {
    let admin_token = get_admin_token().await;
    let client = TestClient::new(&base_url()).with_token(&admin_token);

    let channel_name = format!("Update Norm Test {}", Uuid::new_v4());

    let create_body = json!({
        "type": 1,
        "key": "sk-1231234567890abcdefgHIJKLMNOPQRSTuvwxYZ",
        "name": channel_name,
        "base_url": "https://api.openai.com",
        "models": "gpt-4",
        "group": "default",
        "weight": 1,
        "priority": 0
    });

    let res = client
        .post("/console/api/channel", &create_body)
        .await
        .expect("Create failed");
    assert_eq!(res["success"], true);
    let id = res["data"]["id"].as_i64().expect("No ID returned");

    let update_body = json!({
        "id": id,
        "type": 1,
        "key": "sk-1231234567890abcdefgHIJKLMNOPQRSTuvwxYZ",
        "name": channel_name,
        "base_url": "https://api.openai.com",
        "models": "  DALL-E-3, WHISPER-1  ,,gpt-4o",
        "group": "VIP,  STANDARD",
        "weight": 2,
        "priority": 10
    });

    let update_res = client
        .put("/console/api/channel", &update_body)
        .await
        .expect("Update failed");
    assert_eq!(update_res["success"], true);

    let get_res = client
        .get(&format!("/console/api/channel/{}", id))
        .await
        .expect("Get failed");
    assert_eq!(get_res["success"], true);

    let models = get_res["data"]["models"].as_str().expect("models missing");
    assert_eq!(models, "dall-e-3,whisper-1,gpt-4o");

    let group = get_res["data"]["group"].as_str().expect("group missing");
    assert_eq!(group, "vip,standard");

    let _ = client.delete(&format!("/console/api/channel/{}", id)).await;
}