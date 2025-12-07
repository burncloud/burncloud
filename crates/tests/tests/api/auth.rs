use burncloud_tests::TestClient;
use serde_json::json;

use crate::common;

#[tokio::test]
async fn test_auth_invalid_token() {
    let base_url = common::spawn_app().await;
    let client = TestClient::new(&base_url).with_token("invalid-sk-123");
    let body = json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": "hi"}]
    });
    
    // Expect 401
    let res = client.post_expect_error("/v1/chat/completions", &body, 401).await;
    if let Err(e) = res {
        panic!("Invalid token test failed: {}", e);
    }
}

#[tokio::test]
async fn test_auth_no_token() {
    let base_url = common::spawn_app().await;
    let client = TestClient::new(&base_url); // No token
    let body = json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": "hi"}]
    });
    
    // Expect 401
    let res = client.post_expect_error("/v1/chat/completions", &body, 401).await;
    if let Err(ref e) = res {
        panic!("No token test failed: {}", e);
    }
}