// serde_json::Value is required for dynamic JSON parsing in balancer tests
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::unnecessary_cast,
    clippy::let_and_return,
    clippy::redundant_pattern_matching,
    clippy::panic_in_result_fn,
    reason = "integration tests: a failed assertion is the intended failure signal, and clippy.toml's allow-panic-in-tests does not recognise #[tokio::test]"
)]

mod common;

use burncloud_database::sqlx;
use common::{setup_db, start_mock_upstream, start_test_server};
use reqwest::Client;
use serde_json::Value;

#[tokio::test]
async fn test_round_robin_balancer() -> anyhow::Result<()> {
    let (_db, pool, db_url) = setup_db().await?;

    // Start Mock Upstream
    let mock_port = 3022;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", mock_port))
        .await
        .unwrap_or_else(|e| panic!("Failed to bind mock port {mock_port}: {e}"));
    tokio::spawn(async move {
        start_mock_upstream(listener).await;
    });

    // Model-based routing now reads Supply-owned channel data.
    let model = "round-robin-test-model";
    common::insert_test_channel(
        &pool,
        75611,
        "Upstream 1",
        &format!("http://127.0.0.1:{mock_port}/anything/u1"),
        "key1",
        model,
        "default",
    )
    .await?;
    common::insert_test_channel(
        &pool,
        75612,
        "Upstream 2",
        &format!("http://127.0.0.1:{mock_port}/anything/u2"),
        "key2",
        model,
        "default",
    )
    .await?;
    sqlx::query("INSERT OR IGNORE INTO billing_prices (model, currency, input_price, output_price, region) VALUES (?, 'USD', 1, 1, '')")
        .bind(model).execute(&pool).await?;

    // 4. Start Server
    let port = 3014;
    start_test_server(port, &db_url).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/v1/chat/completions", port);

    // 5. Send Requests
    let mut hits_u1 = 0;
    let mut hits_u2 = 0;

    for i in 0..4 {
        // Must send JSON body because ProxyLogic expects it, or at least handles it nicely
        // But ProxyLogic only fails if body is invalid JSON *AND* it needs to parse it?
        // Actually, previous debugging showed it returned 502 with "Invalid JSON body".
        // So we MUST send valid JSON.
        let resp = client
            .post(&url)
            .header("Authorization", "Bearer sk-burncloud-demo")
            .json(&serde_json::json!({"model": model, "messages": [{"role": "user", "content": "data"}]}))
            .send()
            .await?;

        assert_eq!(resp.status(), 200);
        let json: Value = resp.json().await?;
        let target_url = json["url"]
            .as_str()
            .unwrap_or_else(|| panic!("Expected url in response"));

        println!("Request {} hit: {}", i, target_url);

        if target_url.contains("/u1") {
            hits_u1 += 1;
        } else if target_url.contains("/u2") {
            hits_u2 += 1;
        }
    }

    // 6. Verify Distribution
    // Round Robin should be exactly equal for 4 requests with 2 members
    assert_eq!(hits_u1, 2, "Should hit Upstream 1 twice");
    assert_eq!(hits_u2, 2, "Should hit Upstream 2 twice");

    Ok(())
}
