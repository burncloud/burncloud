#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types)]
use burncloud_database::create_default_database;
use burncloud_database_router::{RouterDatabase, RouterLog};
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_log_api_endpoints() -> anyhow::Result<()> {
    // 1. Setup DB and Insert Dummy Data
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?; // Ensure tables exist

    let log_entry = RouterLog {
        id: 0,
        request_id: uuid::Uuid::new_v4().to_string(),
        user_id: Some("test-api-user".to_string()),
        path: "/v1/test/log".to_string(),
        upstream_id: Some("test-upstream".to_string()),
        status_code: 200,
        latency_ms: 150,
        prompt_tokens: 10,
        completion_tokens: 20,
        cost: 1_000_000, // 0.001 in nanodollars
        model: None,
        cache_read_tokens: 0,
        reasoning_tokens: 0,
        pricing_region: None,
        video_tokens: 0,
        cache_write_tokens: 0,
        audio_input_tokens: 0,
        audio_output_tokens: 0,
        image_tokens: 0,
        embedding_tokens: 0,
        input_cost: 0,
        output_cost: 0,
        cache_read_cost: 0,
        cache_write_cost: 0,
        audio_cost: 0,
        image_cost: 0,
        video_cost: 0,
        reasoning_cost: 0,
        embedding_cost: 0,
        created_at: None,
    };

    RouterDatabase::insert_log(&db, &log_entry).await?;

    // 2. Start Server
    let port = 4002;
    tokio::spawn(async move {
        if let Err(_e) = burncloud_server::start_server("127.0.0.1", port, false).await {
            // Ignore bind errors if already running
        }
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let base_url = format!("http://localhost:{}", port);

    // 3. Test GET /console/api/logs
    let resp = client
        .get(format!("{}/console/api/logs", base_url))
        .send()
        .await?;

    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await?;
    let logs = json["data"].as_array().expect("data should be an array");

    // Verify we can find our log
    let found = logs.iter().any(|l| l["request_id"] == log_entry.request_id);
    assert!(found, "inserted log not found in API response");

    // 4. Test GET /console/api/usage/{user_id}
    let resp_usage = client
        .get(format!("{}/console/api/usage/test-api-user", base_url))
        .send()
        .await?;

    assert_eq!(resp_usage.status(), 200);
    let usage: serde_json::Value = resp_usage.json().await?;

    // Check stats
    let prompt = usage["prompt_tokens"].as_i64().unwrap();
    let completion = usage["completion_tokens"].as_i64().unwrap();
    let total = usage["total_tokens"].as_i64().unwrap();

    // Since this is a shared DB, there might be other logs from other runs.
    // So we assert >= our values.
    assert!(prompt >= 10);
    assert!(completion >= 20);
    assert_eq!(total, prompt + completion);

    Ok(())
}
