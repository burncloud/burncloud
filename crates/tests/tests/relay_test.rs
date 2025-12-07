use burncloud_tests::TestClient;
use serde_json::json;

const BASE_URL: &str = "http://127.0.0.1:8080";
// This token should be pre-seeded by Database::init
const DEMO_TOKEN: &str = "sk-burncloud-demo";

mod common;

use burncloud_tests::TestClient;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const BASE_URL: &str = "http://127.0.0.1:8080";
const DEMO_TOKEN: &str = "sk-burncloud-demo";

#[tokio::test]
async fn test_v1_chat_completions_mock() {
    // 1. Start Mock Upstream
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl-mock",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-3.5-turbo",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "Mock Response from Wiremock" },
                "finish_reason": "stop"
            }]
        })))
        .mount(&mock_server)
        .await;

    println!("Mock Server running at {}", mock_server.uri());

    // 2. Seed DB with this Mock Server URL
    common::seed_demo_data(&mock_server.uri()).await;

    // 3. Send Request to BurnCloud
    let client = TestClient::new(BASE_URL).with_token(DEMO_TOKEN);
    let body = json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": "Hello"}]
    });
    
    println!("Sending request to {}", BASE_URL);
    let res = client.post("/v1/chat/completions", &body).await;
    
    match res {
        Ok(val) => {
            println!("Success: {:?}", val);
            // Verify content
            let content = val["choices"][0]["message"]["content"].as_str().unwrap();
            assert_eq!(content, "Mock Response from Wiremock");
        },
        Err(e) => panic!("Request failed: {}", e),
    }
}

#[tokio::test]
async fn test_v1_models_list() {
// ... (keep existing)
    let client = TestClient::new(BASE_URL).with_token(DEMO_TOKEN);
    let res = client.get("/v1/models").await;
    assert!(res.is_ok(), "Failed to list models");
}
