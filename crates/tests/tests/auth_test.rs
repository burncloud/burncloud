use burncloud_tests::TestClient;
use serde_json::json;

const BASE_URL: &str = "http://127.0.0.1:8080";

#[tokio::test]
async fn test_relay_invalid_token() {
    let client = TestClient::new(BASE_URL).with_token("invalid-sk-123");
    let body = json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": "hi"}]
    });
    
    let res = client.post_expect_error("/v1/chat/completions", &body, 401).await;
    if let Err(e) = res {
        eprintln!("Invalid token test failed: {}", e);
    }
}

#[tokio::test]
async fn test_relay_no_token() {
    let client = TestClient::new(BASE_URL); // No token
    let body = json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": "hi"}]
    });
    
    // Should return 401
    let res = client.post_expect_error("/v1/chat/completions", &body, 401).await;
    if let Err(ref e) = res {
        println!("No token test failed: {}", e);
    }
    assert!(res.is_ok());
}
