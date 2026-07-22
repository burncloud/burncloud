#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::unnecessary_cast,
    clippy::let_and_return,
    clippy::redundant_pattern_matching
)]

mod common;

use common::{insert_test_channel, setup_db, start_mock_upstream, start_test_server};
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

    // 1. Create Upstreams
    let dead_id = 30_231;
    let dead_url = "http://127.0.0.1:9";

    let alive_id = 30_232;
    let alive_url = format!("http://127.0.0.1:{}/anything", mock_port);

    let model = "failover-test-model";
    insert_test_channel(&pool, dead_id, 1, "Dead Node", dead_url, "k1", model, "default").await?;
    insert_test_channel(&pool, alive_id, 1, "Alive Node", &alive_url, "k2", model, "default").await?;

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
            .get(&url)
            .header("Authorization", "Bearer sk-burncloud-demo")
            // Need valid JSON body for ProxyLogic!
            .json(&serde_json::json!({"model": model, "messages": [{"role": "user", "content": "failover"}]}))
            .send()
            .await?;

        assert_eq!(resp.status(), 200, "Request {} failed to failover", i);
    }

    Ok(())
}
