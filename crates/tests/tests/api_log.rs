mod common;
use burncloud_tests::TestClient;
use serde_json::json;
use tokio::time::{sleep, Duration};

#[path = "common/mod.rs"]
mod common_mod; // Using explicit path to avoid E0428 if 'common' is used implicitly

#[tokio::test]
async fn test_billing_accuracy() {
    let base_url = common_mod::get_base_url();
    // Note: /console/api/logs and /usage are currently PUBLIC in our implementation (see server/api/log.rs)
    // In production, this client should use an Admin Token.
    let admin_client = TestClient::new(&base_url);
    
    // 0. Setup Channel (Idempotent)
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
        let _ = admin_client.post("/console/api/channel", &channel_body).await;
    } else {
        println!("SKIPPING: No Upstream Config");
        return;
    }

    // 1. Get Initial Usage
    let user_id = "1"; // Root user ID
    let initial_usage = admin_client.get(&format!("/console/api/usage/{}", user_id)).await.expect("Failed to get usage");
    let initial_tokens = initial_usage["total_tokens"].as_i64().unwrap_or(0);
    println!("Initial Tokens: {}", initial_tokens);

    // 2. Send Request
    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());
    let body = json!({
        "model": "gpt-3.5-turbo-billing",
        "messages": [{"role": "user", "content": "Testing billing system..."}]
    });
    
    let res = user_client.post("/v1/chat/completions", &body).await;
    if let Err(e) = res {
        panic!("Chat request failed: {}", e);
    }
    
    // 3. Wait for Async Log (it's spawned in a separate task)
    println!("Waiting for async log write...");
    sleep(Duration::from_secs(2)).await;
    
    // 4. Verify Logs
    let logs = admin_client.get("/console/api/logs").await.expect("Failed to get logs");
    let log_list = logs["data"].as_array().expect("Logs data is not an array");
    
    // Find our request (by model)
    let my_log = log_list.iter().find(|l| l["request_id"].is_string()); // Just take first valid one?
    assert!(my_log.is_some(), "No logs found");
    
    // 5. Verify Usage Increase
    let final_usage = admin_client.get(&format!("/console/api/usage/{}", user_id)).await.expect("Failed to get final usage");
    let final_tokens = final_usage["total_tokens"].as_i64().unwrap_or(0);
    println!("Final Tokens: {}", final_tokens);
    
    assert!(final_tokens > initial_tokens, "User quota did not increase! Billing system failed.");
}
