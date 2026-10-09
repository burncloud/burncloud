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
use common::{setup_db, start_mock_upstream, start_test_server_on};
use reqwest::Client;
use std::{
    hash::{Hash, Hasher},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

#[tokio::test]
async fn test_failover() -> anyhow::Result<()> {
    let (_db, pool, db_url) = setup_db().await?;

    // A reachable failed upstream proves that the Router actually attempted
    // it. A dead hostname alone allows the test to pass without exercising
    // failover whenever the scheduler selects the healthy channel first.
    let failed_hits = Arc::new(AtomicUsize::new(0));
    let failed_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let failed_port = failed_listener.local_addr()?.port();
    let failed_hits_for_handler = Arc::clone(&failed_hits);
    tokio::spawn(async move {
        let router = axum::Router::new().fallback(move || {
            let hits = Arc::clone(&failed_hits_for_handler);
            async move {
                hits.fetch_add(1, Ordering::SeqCst);
                (
                    axum::http::StatusCode::SERVICE_UNAVAILABLE,
                    "simulated upstream outage",
                )
            }
        });
        axum::serve(failed_listener, router)
            .await
            .unwrap_or_else(|e| panic!("Failed upstream mock error: {e}"));
    });

    let healthy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let healthy_port = healthy_listener.local_addr()?.port();
    tokio::spawn(async move {
        start_mock_upstream(healthy_listener).await;
    });

    let model = "failover-test-model";
    common::insert_test_channel(
        &pool,
        75621,
        "Failed Node",
        &format!("http://127.0.0.1:{failed_port}/anything/failed"),
        "k1",
        model,
        "default",
    )
    .await?;
    common::insert_test_channel(
        &pool,
        75622,
        "Healthy Node",
        &format!("http://127.0.0.1:{healthy_port}/anything/healthy"),
        "k2",
        model,
        "default",
    )
    .await?;
    sqlx::query(
        "INSERT OR IGNORE INTO billing_prices (model, currency, input_price, output_price, region) VALUES (?, 'USD', 1, 1, '')",
    )
    .bind(model)
    .execute(&pool)
    .await?;

    // Select a conversation that HRW prefers to route to the failing channel.
    // This guarantees that the first successful HTTP response proves an
    // upstream error was retried through the second candidate.
    let session = (0..1_024)
        .map(|i| format!("failover-session-{i}"))
        .find(|key| {
            let score = |channel_id: i32| {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                key.hash(&mut hasher);
                channel_id.hash(&mut hasher);
                hasher.finish()
            };
            score(75621) > score(75622) && score(75621).saturating_sub(score(75622)) > u64::MAX / 3
        })
        .ok_or_else(|| anyhow::anyhow!("No session preferring the failed channel"))?;

    let server_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = server_listener.local_addr()?.port();
    start_test_server_on(server_listener, &db_url).await;

    let client = Client::new();
    let url = format!("http://localhost:{port}/v1/chat/completions");

    for request_number in 1..=4 {
        let resp = client
            .post(&url)
            .header("Authorization", "Bearer sk-burncloud-demo")
            .json(&serde_json::json!({
                "model": model,
                "conversation_id": session,
                "messages": [{"role": "user", "content": "failover"}]
            }))
            .send()
            .await?;

        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;
        assert_eq!(status, 200, "Request {request_number} failed: {body}");
        assert!(
            body["url"]
                .as_str()
                .is_some_and(|url| url.contains("/healthy")),
            "Request {request_number} was not answered by the healthy channel: {body}"
        );

        if request_number == 1 {
            assert!(
                failed_hits.load(Ordering::SeqCst) > 0,
                "First request reached the healthy channel without trying the failed upstream"
            );
        }
    }

    assert!(
        failed_hits.load(Ordering::SeqCst) > 0,
        "The failed mock was never called; the test did not exercise failover"
    );

    Ok(())
}
