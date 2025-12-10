use crate::common as common_mod;
use axum::{body::Body, extract::Request, routing::post, Json, Router};
use burncloud_tests::TestClient;
use serde_json::json;
use tokio::net::TcpListener;
use uuid::Uuid;

async fn spawn_mock_server() -> String {
    let listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let app = Router::new().route("/v1/chat/completions", post(mock_chat_handler));

    tokio::spawn(async move {
        println!("MOCK: Server starting on port {}", port);
        if let Err(e) = axum::serve(listener, app).await {
            println!("MOCK: Server failed: {}", e);
        }
    });

    let url = format!("http://127.0.0.1:{}", port);

    // Wait for server to be ready
    let client = reqwest::Client::new();
    for i in 0..20 {
        // Send a dummy request to check if port is listening
        // We expect 405 Method Not Allowed (GET) or 400 Bad Request (POST empty) or 200
        // Just checking connection.
        match client
            .post(format!("{}/v1/chat/completions", url))
            .body("{}")
            .send()
            .await
        {
            Ok(_) => {
                println!("MOCK: Server ready at {}", url);
                break;
            }
            Err(e) => {
                if i == 19 {
                    println!("MOCK: Server failed to start: {}", e);
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }

    url
}

async fn mock_chat_handler(req: Request<Body>) -> Json<serde_json::Value> {
    println!("MOCK: Received request");
    let headers = req.headers().clone();
    println!("MOCK: Headers: {:?}", headers);

    let body_bytes = axum::body::to_bytes(req.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap_or(json!({}));
    println!("MOCK: Body: {:?}", body_json);

    let mock_id = headers
        .get("x-mock-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("UNKNOWN");
    println!("MOCK: Found ID: {}", mock_id);

    // Echo back the mock-id and the request body
    Json(json!({
        "id": "mock-response",
        "object": "chat.completion",
        "created": 1234567890,
        "model": body_json.get("model").unwrap_or(&json!("unknown")),
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": format!("MOCK_ID: {}", mock_id)
                },
                "finish_reason": "stop"
            }
        ],
        "echoed_body": body_json
    }))
}

#[tokio::test]
async fn test_ability_routing_and_passthrough() {
    // 1. Setup
    let app_url = common_mod::spawn_app().await;
    let mock_url = spawn_mock_server().await;
    let admin_client = TestClient::new(&app_url);
    let user_client = TestClient::new(&app_url).with_token(&common_mod::get_demo_token());

    let unique_model = format!("mock-model-{}", Uuid::new_v4());

    // 2. Create Channels
    // Channel A: Priority 200 (Higher than default 100)
    // We use type=1 (OpenAI) so that we can test "openai" protocol passthrough behavior too (if we wanted)
    // Here we mainly test ability routing.
    let chan_a = json!({
        "type": 1, // OpenAI
        "key": "sk-mock-a",
        "name": format!("Mock A {}", Uuid::new_v4()),
        "base_url": mock_url,
        "models": unique_model,
        "group": "default",
        "priority": 200,
        "weight": 1,
        "header_override": json!({"x-mock-id": "A"}).to_string()
    });
    let res_a = admin_client
        .post("/console/api/channel", &chan_a)
        .await
        .expect("Failed to create Channel A");
    assert_eq!(res_a["success"], true);

    // Channel B: Priority 100 (Lower)
    let chan_b = json!({
        "type": 1,
        "key": "sk-mock-b",
        "name": format!("Mock B {}", Uuid::new_v4()),
        "base_url": mock_url,
        "models": unique_model,
        "group": "default",
        "priority": 100,
        "weight": 1,
        "header_override": json!({"x-mock-id": "B"}).to_string()
    });
    let res_b = admin_client
        .post("/console/api/channel", &chan_b)
        .await
        .expect("Failed to create Channel B");
    assert_eq!(res_b["success"], true);

    // 3. Test Priority Routing
    // Should hit Channel A because Priority 200 > 100
    let req_body = json!({
        "model": unique_model,
        "messages": [{"role": "user", "content": "hi"}],
        "extra_field": "passthrough_check" // Test Passthrough
    });

    println!("Sending request to: {}", app_url);
    let resp = user_client
        .post("/v1/chat/completions", &req_body)
        .await
        .expect("Request failed");

    // Verify Routing
    let content = resp["choices"][0]["message"]["content"]
        .as_str()
        .expect("No content");
    println!("Response Content: {}", content);
    assert!(
        content.contains("MOCK_ID: A"),
        "Should be routed to Channel A (Priority 200), got: {}",
        content
    );

    // Verify Passthrough
    // The mock server echoes the body in "echoed_body"
    let echoed = &resp["echoed_body"];
    assert_eq!(
        echoed["extra_field"], "passthrough_check",
        "Generic passthrough failed"
    );

    println!("Ability Routing & Passthrough Test Passed!");
}
