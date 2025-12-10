use burncloud_tests::TestClient;
use dotenvy::dotenv;
use serde_json::json;
use std::env;
use uuid::Uuid;

use crate::common as common_mod;

#[tokio::test]
async fn test_claude_adaptor_e2e() {
    dotenv().ok();
    // 1. Check Preconditions
    let claude_key = match env::var("TEST_ANTHROPIC_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            println!("SKIPPING: TEST_ANTHROPIC_KEY not set");
            return;
        }
    };

    // 2. Spawn App (Self-Bootstrapping)
    let base_url = common_mod::spawn_app().await;
    
    // 3. Setup: Create Channel via Admin API
    let admin_client = TestClient::new(&base_url);
    let channel_name = format!("Claude Test {}", Uuid::new_v4());

    // ChannelType::Anthropic = 14
    // Using a specific model name that we will request later
    let model_name = "claude-3-opus-20240229"; 
    
    let body = json!({
        "type": 14,
        "key": claude_key,
        "name": channel_name,
        "base_url": "https://api.anthropic.com",
        "models": model_name,
        "group": "vip", // Ensure demo token has access to this group (or default)
                        // Demo token usually has 'default' or 'vip'? 
                        // In `router/src/lib.rs`, fallback is "default".
                        // Let's use "default" to be safe unless we know demo user group.
                        // `RouterDatabase::init` inserts demo-user. `UserDatabase::init` creates it.
                        // Default group for new users is usually "default".
        "group": "default", 
        "weight": 10,
        "priority": 100
    });

    let res = admin_client
        .post("/console/api/channel", &body)
        .await
        .expect("Create channel failed");
    
    assert_eq!(res["success"], true, "Failed to create Claude channel: {:?}", res);

    // 4. Execution: Send Chat Request via Relay
    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());
    
    let chat_body = json!({
        "model": model_name,
        "messages": [
            {"role": "user", "content": "Return the string 'ADAPTOR_OK'"}
        ],
        "max_tokens": 50
    });

    println!("Sending Claude request to {}...", base_url);
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    println!("Claude Response: {:?}", chat_res);
    
    // 5. Validation
    let choices = chat_res.get("choices").and_then(|c| c.as_array());
    assert!(choices.is_some(), "Response missing 'choices' array: {:?}", chat_res);
    
    let content = choices.unwrap()[0]["message"]["content"].as_str();
    assert!(content.is_some(), "Response missing message content");
    
    let text = content.unwrap();
    println!("Response Text: {}", text);
    
    // We check if the model followed instruction (it usually does)
    // But mostly we check structure validity (which proves Adaptor worked to convert response)
    // If Adaptor failed to convert, we would likely see empty content or original Claude JSON (which has "content" array at top level).
    // The presence of "choices" -> "message" -> "content" proves `ClaudeAdaptor::convert_response` ran.
}
