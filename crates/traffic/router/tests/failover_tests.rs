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

#[tokio::test]
async fn test_failover() -> anyhow::Result<()> {
    let (_db, pool, _db_url) = setup_db().await?;

    // Start Mock Upstream for Alive Node
    let mock_port = 3023;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", mock_port))
        .await
        .unwrap_or_else(|e| panic!("Failed to bind mock port {mock_port}: {e}"));
    tokio::spawn(async move {
        start_mock_upstream(listener).await;
    });

    // Both candidates belong to the same model in current Channel truth.
    let model = "failover-test-model";
    common::insert_test_channel(
        &pool,
        75621,
        "Dead Node",
        "http://127.0.0.1:1/anything",
        "k1",
        model,
        "default",
    )
    .await?;
    common::insert_test_channel(
        &pool,
        75622,
        "Alive Node",
        &format!("http://127.0.0.1:{mock_port}/anything"),
        "k2",
        model,
        "default",
    )
    .await?;
    sqlx::query("INSERT OR IGNORE INTO billing_prices (model, currency, input_price, output_price, region) VALUES (?, 'USD', 1, 1, '')")
        .bind(model).execute(&pool).await?;

    // 4. Start Server
    let port = 3015;
    start_test_server(port, &_db_url).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/v1/chat/completions", port);

    // 5. Send Requests
    // We expect some requests to hit Dead Node first (Round Robin), fail, and then hit Alive Node.
    // Some will hit Alive Node directly.
    // All requests should eventually succeed (200 OK).

    for i in 1..=4 {
        println!("Request {}", i);
        let resp = client
            .post(&url)
            .header("Authorization", "Bearer sk-burncloud-demo")
            // Need valid JSON body for ProxyLogic!
            .json(&serde_json::json!({"model": model, "messages": [{"role": "user", "content": "failover"}]}))
            .send()
            .await?;

        assert_eq!(resp.status(), 200, "Request {} failed to failover", i);
    }

    Ok(())
}
