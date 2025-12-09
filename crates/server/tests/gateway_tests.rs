use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_unified_gateway_routing() -> anyhow::Result<()> {
    // Start Unified Gateway on port 3000
    let port = 3000;
    tokio::spawn(async move {
        // Set env var just in case, though main.rs parses it or defaults
        // But here we call start_server directly which takes port arg
        if let Err(_e) = burncloud_server::start_server(port).await {
            // Ignore bind errors if already running (it might fail in CI if port is taken)
        }
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{}", port);

    // 1. Test Dashboard HTML (Root /)
    let resp_html = client.get(&base_url).send().await?;
    assert_eq!(resp_html.status(), 200);
    let html_text = resp_html.text().await?;
    assert!(html_text.contains("<!DOCTYPE html>"), "Should return HTML");
    assert!(html_text.contains("BurnCloud"), "Should contain App Title");

    // 2. Test Management API (/console/api/channels)
    let resp_api = client
        .get(format!("{}/console/api/channels", base_url))
        .send()
        .await?;
    assert_eq!(resp_api.status(), 200);
    // Should return JSON array
    let _channels: serde_json::Value = resp_api.json().await?;

    // 3. Test Router Fallback (/v1/chat/completions)
    let resp_llm = client
        .post(format!("{}/v1/chat/completions", base_url))
        .body("{}")
        .send()
        .await?;

    let status = resp_llm.status();
    let body = resp_llm.text().await?;
    println!("LLM Response ({}): {}", status, body);

    // Expect 401 because we didn't provide Bearer token
    assert_eq!(
        status, 401,
        "Router should intercept LLM path and demand Auth. Got: {}",
        body
    );

    // 4. Test Router Fallback with Auth (Invalid Token)
    let resp_llm_auth = client
        .post(format!("{}/v1/chat/completions", base_url))
        .header("Authorization", "Bearer invalid-sk")
        .body("{}")
        .send()
        .await?;

    assert_eq!(resp_llm_auth.status(), 401);

    Ok(())
}
