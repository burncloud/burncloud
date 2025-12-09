use burncloud_tests::TestClient;
use serde_json::json;
use tokio::time::{sleep, Duration};

use crate::common as common_mod;

#[tokio::test]
async fn test_billing_accuracy() {
    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);

    if let Some((upstream_key, upstream_url)) = common_mod::get_openai_config() {
        let channel_body = json!({
            "type": 1,
            "key": upstream_key,
            "name": "Billing Test Channel",
            "base_url": upstream_url,
            "models": "gpt-3.5-turbo-billing",
            "group": "vip",
            "weight": 10,
            "priority": 100
        });
        let _ = admin_client
            .post("/console/api/channel", &channel_body)
            .await;
    } else {
        println!("SKIPPING: No Upstream Config");
        return;
    }

    let user_id = "1"; // Root user ID
    let initial_usage = admin_client
        .get(&format!("/console/api/usage/{}", user_id))
        .await
        .expect("Failed to get usage");
    let initial_tokens = initial_usage["total_tokens"].as_i64().unwrap_or(0);
    println!("Initial Tokens: {}", initial_tokens);

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());
    let body = json!({
        "model": "gpt-3.5-turbo-billing",
        "messages": [{"role": "user", "content": "Testing billing system..."}]
    });

    let res = user_client.post("/v1/chat/completions", &body).await;
    if let Err(e) = res {
        panic!("Chat request failed: {}", e);
    }

    println!("Waiting for async log write...");
    sleep(Duration::from_secs(2)).await;

    let logs = admin_client
        .get("/console/api/logs")
        .await
        .expect("Failed to get logs");
    let log_list = logs["data"].as_array().expect("Logs data is not an array");

    let my_log = log_list.iter().find(|l| l["request_id"].is_string());
    assert!(my_log.is_some(), "No logs found");

    let final_usage = admin_client
        .get(&format!("/console/api/usage/{}", user_id))
        .await
        .expect("Failed to get final usage");
    let final_tokens = final_usage["total_tokens"].as_i64().unwrap_or(0);
    println!("Final Tokens: {}", final_tokens);

    assert!(
        final_tokens > initial_tokens,
        "User quota did not increase! Billing system failed."
    );
}
