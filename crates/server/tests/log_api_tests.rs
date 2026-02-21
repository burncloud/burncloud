use burncloud_database::create_default_database;
use burncloud_database_router::{DbRouterLog, RouterDatabase};
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_log_api_endpoints() -> anyhow::Result<()> {
    // 1. Setup DB and Insert Dummy Data
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?; // Ensure tables exist

    let log_entry = DbRouterLog {
        request_id: uuid::Uuid::new_v4().to_string(),
        user_id: Some("test-api-user".to_string()),
        path: "/v1/test/log".to_string(),
        upstream_id: Some("test-upstream".to_string()),
        status_code: 200,
        latency_ms: 150,
        prompt_tokens: 10,
        completion_tokens: 20,
        cost: 0.001,
    };

    RouterDatabase::insert_log(&db, &log_entry).await?;

    // 2. Start Server
    let port = 4002;
    tokio::spawn(async move {
        if let Err(_e) = burncloud_server::start_server(port, false).await {
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
