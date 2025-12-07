use burncloud_tests::TestClient;
use serde_json::json;
use uuid::Uuid;

#[path = "common/mod.rs"]
mod common;

#[tokio::test]
async fn test_e2e_real_upstream() {
    // 1. Get Config
    let base_url = common::get_base_url();
    let (upstream_key, upstream_url) = match common::get_openai_config() {
        Some(c) => c,
        None => {
            println!("SKIPPING: Real upstream not configured in .env");
            return;
        }
    };
    
    // 2. Create Channel
    let admin_client = TestClient::new(&base_url); // TODO: auth
    let channel_name = format!("Real E2E {}", Uuid::new_v4());
    
    let body = json!({
        "type": 1,
        "key": upstream_key,
        "name": channel_name,
        "base_url": upstream_url,
        "models": "gpt-3.5-turbo",
        "group": "vip", // Root user group
        "weight": 10,
        "priority": 100
    });
    
    println!("Creating Channel: {}", channel_name);
    let res = admin_client.post("/console/api/channel", &body).await.expect("Create channel failed");
    assert_eq!(res["success"], true);
    
    // 3. Chat Completion
    let user_client = TestClient::new(&base_url).with_token(&common::get_demo_token());
    let chat_body = json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": "Hello"}]
    });
    
    println!("Sending Chat Request...");
    let chat_res = user_client.post("/v1/chat/completions", &chat_body).await.expect("Chat failed");
    
    // 4. Verify
    println!("Response: {:?}", chat_res);
    let content = chat_res["choices"][0]["message"]["content"].as_str();
    assert!(content.is_some());
    println!("Content: {}", content.unwrap());
}
