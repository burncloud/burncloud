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
use std::hash::{Hash, Hasher};

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

    // The retired router_groups round_robin strategy is not used by Channel
    // routing. Current L3 session affinity uses deterministic HRW, so choose
    // distinct conversation IDs that select each provider twice. This keeps
    // the original four HTTP 200 and 2/2 distribution assertions meaningful
    // without changing the production scheduler or forcing legacy fixtures.
    let mut sessions = Vec::new();
    let mut counts = [0_u32; 2];
    for i in 0..1_024 {
        let session = format!("distribution-session-{i}");
        let hash = |channel: i32| {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            session.hash(&mut h);
            channel.hash(&mut h);
            h.finish()
        };
        let h1 = hash(75611);
        let h2 = hash(75612);
        // Prefer distinct, strong affinity keys to avoid marginal ties.
        let (winner, high, low) = if h1 > h2 { (0, h1, h2) } else { (1, h2, h1) };
        if counts[winner] < 2 && high.saturating_sub(low) > u64::MAX / 3 {
            sessions.push(session);
            counts[winner] += 1;
        }
        if counts == [2, 2] {
            break;
        }
    }
    assert_eq!(counts, [2, 2], "test needs two strong HRW keys per channel");

    // 5. Send Requests
    let mut hits_u1 = 0;
    let mut hits_u2 = 0;
    let mut selected_urls = Vec::new();

    for (i, session) in sessions.iter().enumerate() {
        let resp = client
            .post(&url)
            .header("Authorization", "Bearer sk-burncloud-demo")
            .json(&serde_json::json!({"model": model, "conversation_id": session, "messages": [{"role": "user", "content": "data"}]}))
            .send()
            .await?;

        assert_eq!(resp.status(), 200);
        let json: Value = resp.json().await?;
        let target_url = json["url"]
            .as_str()
            .unwrap_or_else(|| panic!("Expected url in response"));

        println!("Request {} hit: {}", i, target_url);
        selected_urls.push(target_url.to_owned());

        if target_url.contains("/u1") {
            hits_u1 += 1;
        } else if target_url.contains("/u2") {
            hits_u2 += 1;
        }
    }

    // 6. Verify Distribution
    // The original HTTP distribution assertion remains strict (two each).
    assert_eq!(hits_u1, 2, "Should hit Upstream 1 twice");
    assert_eq!(hits_u2, 2, "Should hit Upstream 2 twice");

    // A repeated conversation must retain the same upstream. The legacy
    // test name is kept for #756, but current Channel routing uses HRW
    // affinity, not the retired router_groups round_robin strategy.
    let sticky_response = client
        .post(&url)
        .header("Authorization", "Bearer sk-burncloud-demo")
        .json(&serde_json::json!({
            "model": model,
            "conversation_id": sessions[0],
            "messages": [{"role": "user", "content": "sticky follow-up"}]
        }))
        .send()
        .await?;
    assert_eq!(sticky_response.status(), 200);
    let sticky_json: Value = sticky_response.json().await?;
    assert_eq!(
        sticky_json["url"].as_str(),
        selected_urls.first().map(String::as_str),
        "A repeated conversation must remain on the same upstream"
    );

    Ok(())
}
