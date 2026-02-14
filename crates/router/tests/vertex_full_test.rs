mod common;
use burncloud_database::sqlx;
use common::{setup_db, start_test_server};
use reqwest::Client;
use serde_json::json;

#[tokio::test]
async fn test_vertex_full_flow() -> anyhow::Result<()> {
    let (_db, pool) = setup_db().await?;

    // 1. Start Mock Upstream (Vertex API & Auth)
    let mut server = mockito::Server::new_async().await;

    // Mock Auth
    let auth_mock = server
        .mock("POST", "/auth")
        .with_status(200)
        .with_body(
            r#"{"access_token": "mock_vertex_token", "expires_in": 3600, "token_type": "Bearer"}"#,
        )
        .create_async()
        .await;

    // Mock Gemini Stream Response (Vertex Format)
    // Vertex returns a list of objects in a stream.
    // We mock a single response for simplicity, or chunked.
    // Let's return a simple JSON array.
    let vertex_resp = json!([{
        "candidates": [{
            "content": {
                "parts": [{ "text": "Hello from Vertex Mock" }],
                "role": "model"
            },
            "finishReason": "STOP",
            "index": 0
        }]
    }]);

    let api_mock = server.mock("POST", "/v1/projects/mock-project/locations/us-central1/publishers/google/models/gemini-pro:streamGenerateContent")
        .match_header("Authorization", "Bearer mock_vertex_token")
        .with_status(200)
        .with_body(vertex_resp.to_string())
        .expect(2) // Expect 2 calls (Non-Stream + Stream)
        .create_async().await;

    // 2. Configure Upstream
    let id = "vertex-test";
    let name = "Vertex Test";
    let base_url = "https://ignored-but-required.com";

    let private_key = r#"-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQDaJKsOxgH3D2ah
v8vbh9n99AvHPOoIuJur/sV7tHZ9/bzMvnzVsQxxciagrVFve+XaE1mQjzNbRKB3
zsdpW2n1eUEtrO1PrQCA8BuaAnL/le4RryHyiDMy/hhGDVzvF55gSIgHv+aThDz7
/bK+GGJbLsoHOeFH7OV7wBWhNHpd5+I2GmAXEbCozPO9QjXkpsxOCbMu3lLQq99V
/F/HOWUbPMSIrVLKleL/yUhNO+0VUhYpWVAvO8gKP4hAf/qLvKNlV9zclBgANIhb
Fh8sC3OpvNVWiHNBoclVnmRB+OHJC51JBhHeZCJhM5IxDRtcYR4gGxcpNev8YFRF
xb76yA8dAgMBAAECggEAJ6D/MVsf2sPhu3M2I87Jd5TY+ewzQPvWjfel5Sv61bcd
kB1v3LtB/S8FXO23jFb4Afa/b99P713nf/Rg7h8k/+r0AAn4/584ZvQXs5IL1aol
WnGUK3T6RiJ6gullD2tdQnUSv0OprfVZRdcIHHgeEB4PJiJp7nDXHLTfyQ4ZR8sl
GstLN63/ZHNy4CyBdsjvJe0dqtJdXqK/ME6w5MtcHGpur8oSNqLsKKgIyXkcSasL
rhINjqIC1pN096a0nn9j9kYxJHas+JSu2gdhCuJ94t5B84B+Eb/7+MxmMLwygD0m
SbBA0MLfwzwmv6zsgLeXBxeK26AeeUTRjhXNVGSs8QKBgQD7alSesb/2Ecr8+2tm
UtzTY2wMKVcwNuYTLEmksIr43jIV1Gl73rMu3DH5hkXS1rOxBt6839QZDbWcL+7p
ruJpHW8o9/Qj7ELewqg8bKqXVvFTpqNQb13H0tQCrj1gQTopHwzBoThAvpVVxyZ2
s7FndVz+xsx53GXQnfisPb5YsQKBgQDeHwPepm3ABTGd1Qbp50ixEz/UtFL2F1Dy
jy4ylQR8ygqkgeE4NYh5WaubZnIKgn56cN2Rombv3LbqIe/N36Gj282k2rM4h7Km
1U1r1auIMZIon+zt1a2PlgmttUoAX5x2AuHI2DWE7ROmMTImV1SsW61qfg2xl1Nh
n/oyipf4LQKBgEtOKBZ4i1T7M1/fNuYpP7eZeg2SfGkWqIdppo1Ly/SLKVlcjFPr
+qO4lMd2rodeg+gsdJ8CNBdlAdbMjLU2Ct8NT/RngJsZ81Wh3J5sthQqmJJDwXsg
QGjP/2zmH8ArCW6zvDBrR9wsubI9uomnfSXOA5LUnP6LQ3vfNVLyE4ehAoGAe8kXE
/72DNwYIZh1iOb+6MgMe5Ke5UxrLTJEEaZgYNcMBU/oXrXev5oMe8ck6Nx+defu
Ytn5udTsDyEojjgB0dqOCUBkPq3JDxayVdU3CehuRruRg53gYrO/4xG0Eu81t8K1
Z4Oul8yzdZvXEez7YC6bP0zOftkRe8d23LHGLWUCgYAhW6lqcEmOqtw+TtQlVGQR
0K6nDeP5P0EnaG4ZiwVIiMpJhqj5avwlyDBeg9QdM+ubhqXHB5oCcLRLrP9PITf+
q/3tDxsxpwLbEpeg6nqaTxylV1V6Ky5oLq8u9tOsqP6eZ83STlGlPpimKH2FlO21
Apfww82b16AoK7qgtPcI8g==
-----END PRIVATE KEY-----"#;

    let api_key = serde_json::json!({
        "type": "service_account",
        "project_id": "mock-project", // Used by adaptor if not overridden, but we override base URL which includes project if we want
        // Actually our base URL override logic uses this project_id unless overridden in extra
        "private_key_id": "123",
        "private_key": private_key,
        "client_email": "test@mock-project.iam.gserviceaccount.com",
        "client_id": "123"
    })
    .to_string();

    let match_path = "/v1/chat/completions";
    let auth_type = "VertexAi"; // Important: Triggers factory

    // Inject overrides
    let param_override = json!({
        "base_url": server.url(),
        "auth_url": format!("{}/auth", server.url())
    })
    .to_string();

    // Clear existing upstreams (e.g. defaults)
    sqlx::query("DELETE FROM router_upstreams")
        .execute(&pool)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type, param_override, protocol)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(base_url)
    .bind(api_key)
    .bind(match_path)
    .bind(auth_type)
    .bind(param_override)
    .bind("vertex") // Protocol
    .execute(&pool)
    .await?;

    // 3. Start Router
    let port = 3050;
    start_test_server(port).await;

    // 4. Send Request (Non-Stream for simplicity first, or Stream if we want to test that too)
    let client = Client::new();
    let url = format!("http://localhost:{}/v1/chat/completions", port);

    // Non-Stream Request
    let resp = client
        .post(&url)
        .header("Authorization", "Bearer sk-burncloud-demo")
        .json(&json!({
            "model": "gemini-pro", // Mapped via ModelRouter?
            // If ModelRouter fails, it falls back to path routing if we match path?
            // But we set match_path = /v1/chat/completions.
            // Wait, path routing fallback will round-robin.
            // If we rely on ModelRouter, we need to add model mapping to DB.
            // Let's rely on path routing fallback for this test.
            // But ModelRouter logic runs first.
            // Ensure request has model field.
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .send()
        .await?;

    // Debug
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await?;
        println!("Error Resp: {}", text);
        panic!("Request failed with status {}", status);
    }
    assert_eq!(status, 200);

    let json: serde_json::Value = resp.json().await?;
    println!("Response: {}", json);

    // Validate OpenAI Format
    assert_eq!(json["object"], "chat.completion");
    assert_eq!(
        json["choices"][0]["message"]["content"],
        "Hello from Vertex Mock"
    );

    // 5. Send Stream Request
    let stream_resp = client
        .post(&url)
        .header("Authorization", "Bearer sk-burncloud-demo")
        .json(&json!({
            "model": "gemini-pro",
            "messages": [{"role": "user", "content": "stream me"}],
            "stream": true
        }))
        .send()
        .await?;

    let status = stream_resp.status();
    if !status.is_success() {
        let text = stream_resp.text().await?;
        println!("Error Stream Resp: {}", text);
        panic!("Stream Request failed with status {}", status);
    }
    assert_eq!(status, 200);
    // Should be text/event-stream
    let ct = stream_resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.contains("text/event-stream"));

    let bytes = stream_resp.bytes().await?;
    let sse_str = String::from_utf8(bytes.to_vec())?;
    println!("SSE Response: {}", sse_str);

    assert!(sse_str.contains("data:"));
    assert!(sse_str.contains("Hello from Vertex Mock"));
    assert!(sse_str.contains("[DONE]"));

    auth_mock.assert();
    api_mock.assert();

    Ok(())
}
