use burncloud_tests::TestClient;
use serde_json::json;
use dotenvy::dotenv;
use std::env;
use uuid::Uuid;

use crate::common as common_mod;

#[tokio::test]
async fn test_e2e_real_upstream() {
    let base_url = common_mod::spawn_app().await;
    let (upstream_key, upstream_url) = match common_mod::get_openai_config() {
        Some(c) => c,
        None => {
            println!("SKIPPING: Real upstream not configured in .env");
            return;
        }
    };
    
    let admin_client = TestClient::new(&base_url);
    let channel_name = format!("Real E2E {}", Uuid::new_v4());
    
    let body = json!({
        "type": 1,
        "key": upstream_key,
        "name": channel_name,
        "base_url": upstream_url,
        "models": "gpt-3.5-turbo",
        "group": "vip",
        "weight": 10,
        "priority": 100
    });
    
    let res = admin_client.post("/console/api/channel", &body).await.expect("Create channel failed");
    assert_eq!(res["success"], true);
    
    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());
    let chat_body = json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": "Hello"}]
    });
    
    println!("Sending chat request to {}", base_url);
    let chat_res = user_client.post("/v1/chat/completions", &chat_body).await.expect("Chat failed");
    
    println!("Chat Response: {:?}", chat_res);
    let choices = chat_res.get("choices").and_then(|c| c.as_array());
    if let Some(c) = choices {
        if !c.is_empty() {
            println!("Success: Got response from upstream.");
        } else {
            panic!("Invalid response structure: {:?}", chat_res);
        }
    } else {
        panic!("Invalid response structure: {:?}", chat_res);
    }
}

#[tokio::test]
async fn test_gemini_adaptor() {
    dotenv().ok();
    let gemini_key = match env::var("TEST_GEMINI_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            println!("SKIPPING: TEST_GEMINI_KEY not set");
            return;
        }
    };
    
    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let channel_name = format!("Gemini Test {}", Uuid::new_v4());
    
    let body = json!({
        "type": 24, // Gemini
        "key": gemini_key,
        "name": channel_name,
        "base_url": "https://generativelanguage.googleapis.com",
        "models": "gemini-pro",
        "group": "vip",
        "weight": 10,
        "priority": 100
    });
    
    let res = admin_client.post("/console/api/channel", &body).await.expect("Create channel failed");
    assert_eq!(res["success"], true);
    
    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());
    let chat_body = json!({
        "model": "gemini-pro",
        "messages": [{"role": "user", "content": "Say 'Hello Gemini'"}]
    });
    
    println!("Sending Gemini request...");
    let chat_res = user_client.post("/v1/chat/completions", &chat_body).await.expect("Chat failed");
    
    let content = chat_res["choices"][0]["message"]["content"].as_str();
    assert!(content.is_some(), "Gemini response conversion failed: {:?}", chat_res);
    println!("Gemini Response: {}", content.unwrap());
}
