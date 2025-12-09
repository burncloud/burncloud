use axum::{body::Body, extract::State, response::Response, routing::post, Router};
use reqwest::Client;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

// Mock Upstream State to count requests
struct MockUpstreamState {
    count_a: Arc<Mutex<i32>>,
    count_b: Arc<Mutex<i32>>,
}

#[tokio::test]
async fn test_group_routing_logic() -> anyhow::Result<()> {
    // 1. Start Mock Upstreams
    let count_a = Arc::new(Mutex::new(0));
    let count_b = Arc::new(Mutex::new(0));
    let state = Arc::new(MockUpstreamState {
        count_a: count_a.clone(),
        count_b: count_b.clone(),
    });

    let upstream_port = 5000;
    let upstream_app = Router::new()
        .route(
            "/upstream-a/v1/group-chat/chat/completions",
            post(|State(s): State<Arc<MockUpstreamState>>| async move {
                let mut c = s.count_a.lock().unwrap();
                *c += 1;
                Response::new(Body::from(r#"{"id":"mock-a"}"#))
            }),
        )
        .route(
            "/upstream-b/v1/group-chat/chat/completions",
            post(|State(s): State<Arc<MockUpstreamState>>| async move {
                let mut c = s.count_b.lock().unwrap();
                *c += 1;
                Response::new(Body::from(r#"{"id":"mock-b"}"#))
            }),
        )
        .with_state(state);

    tokio::spawn(async move {
        let addr = SocketAddr::from(([127, 0, 0, 1], upstream_port));
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, upstream_app).await.unwrap();
    });

    // 2. Start BurnCloud Gateway
    let gateway_port = 3005;
    tokio::spawn(async move {
        if let Err(_e) = burncloud_server::start_server(gateway_port).await {
            // ignore
        }
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let api_base = format!("http://127.0.0.1:{}/console/api", gateway_port);

    // 3. Configure Channels
    // Channel A -> Mock A
    let chan_a = serde_json::json!({
        "id": "chan-a",
        "name": "Mock A",
        "base_url": format!("http://127.0.0.1:{}/upstream-a", upstream_port),
        "api_key": "sk-a",
        "match_path": "/v1/mock", // Won't match directly, used by group
        "auth_type": "Bearer",
        "priority": 0
    });
    client
        .post(format!("{}/channels", api_base))
        .json(&chan_a)
        .send()
        .await?
        .error_for_status()?;

    // Channel B -> Mock B
    let chan_b = serde_json::json!({
        "id": "chan-b",
        "name": "Mock B",
        "base_url": format!("http://127.0.0.1:{}/upstream-b", upstream_port),
        "api_key": "sk-b",
        "match_path": "/v1/mock",
        "auth_type": "Bearer",
        "priority": 0
    });
    client
        .post(format!("{}/channels", api_base))
        .json(&chan_b)
        .send()
        .await?
        .error_for_status()?;

    // 4. Configure Group
    let group_id = "group-round-robin";
    let group = serde_json::json!({
        "id": group_id,
        "name": "Round Robin Group",
        "strategy": "round_robin",
        "match_path": "/v1/group-chat" // This is the path we will hit
    });
    client
        .post(format!("{}/groups", api_base))
        .json(&group)
        .send()
        .await?
        .error_for_status()?;

    // Add Members
    let members = serde_json::json!([
        { "upstream_id": "chan-a", "weight": 1 },
        { "upstream_id": "chan-b", "weight": 1 }
    ]);
    client
        .put(format!("{}/groups/{}/members", api_base, group_id))
        .json(&members)
        .send()
        .await?
        .error_for_status()?;

    // Force Config Reload (Internal API)
    client
        .post(format!(
            "http://127.0.0.1:{}/console/internal/reload",
            gateway_port
        ))
        .send()
        .await?
        .error_for_status()?;
    sleep(Duration::from_millis(500)).await;

    // 5. Create a User Token to allow access
    let token_req = serde_json::json!({ "user_id": "test-user" });
    let token_resp = client
        .post(format!("{}/tokens", api_base))
        .json(&token_req)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    let user_token = token_resp["token"].as_str().unwrap();

    // 6. Send Requests to Group Path
    let gateway_url = format!(
        "http://127.0.0.1:{}/v1/group-chat/chat/completions",
        gateway_port
    );

    for i in 0..4 {
        println!("Sending request {}", i + 1);
        let resp = client
            .post(&gateway_url)
            .header("Authorization", format!("Bearer {}", user_token))
            .body("{}")
            .send()
            .await?;

        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = resp.json().await?;
        println!("Response: {:?}", body);
    }

    // 7. Verify Distribution
    // With Round Robin and 2 members, 4 requests should be 2 A, 2 B.
    // Or at least both should be > 0.

    let a = *count_a.lock().unwrap();
    let b = *count_b.lock().unwrap();
    println!("Final Counts: A={}, B={}", a, b);

    assert!(a > 0, "Upstream A should receive requests");
    assert!(b > 0, "Upstream B should receive requests");
    assert_eq!(a + b, 4, "Total requests handled should be 4");

    Ok(())
}
