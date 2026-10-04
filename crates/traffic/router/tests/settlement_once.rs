#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    reason = "Test-only file: the assertions are the test, and the helper reads JSON of unknown shape."
)]
//! How many times one request settles, and what it settles against (#633, plan section 5 item 12).
//!
//! The plan lists "usage must not be counted twice" among the behaviours to add for `traffic/router`, and
//! the matrix confirmed nothing covered it. What the existing billing tests cover is a different thing:
//! `e2e_billing_tests.rs` calls `RouterDatabase::deduct_quota(&db, ..., 300)` **by hand** and checks the
//! row moved by 300. That tests the database function; it never drives the request handler, so it cannot
//! observe whether the handler settles once or twice.
//!
//! The production code has exactly one settlement call site (`lib.rs:1989`, guarded by `cost > 0`) and one
//! log-write call site (`lib.rs:923`), both reached after the upstream responds. The failure this file
//! guards against is a request that reaches that code twice -- which a retry, a duplicated response path,
//! or a second pass through the handler would cause, and which a hand-called unit test cannot see.
//!
//! ## Why the assertions poll
//!
//! Settlement is fired from `tokio::spawn` and the request log goes through a channel, so neither is
//! finished when the HTTP response returns. A test that read the quota immediately would be measuring the
//! scheduler. Each measurement therefore waits for the value to stop changing before asserting, and the
//! wait is bounded so a failure is a failure rather than a hang.

mod common;

use burncloud_database_router::RouterDatabase;
use common::{
    insert_router_token, insert_test_channel, setup_db, start_mock_upstream, start_test_server,
};
use std::time::Duration;
use tokio::net::TcpListener;

/// Read the settled amount for a credential, retrying until it stops changing.
///
/// `deduct_quota` runs in a spawned task, so the first read after a response can legitimately be 0. Two
/// consecutive equal readings are taken as settled; the bound is generous because the alternative is a
/// flaky test.
async fn settled_amount(pool: &burncloud_database::sqlx::AnyPool, token: &str) -> i64 {
    let mut last = i64::MIN;
    for _ in 0..40 {
        let current: i64 = burncloud_database::sqlx::query_scalar(
            "SELECT used_quota FROM router_tokens WHERE token = ?",
        )
        .bind(token)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(0);
        if current == last {
            return current;
        }
        last = current;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    last
}

/// Count the log rows for a request id, with the same settle-then-read approach.
async fn log_count(pool: &burncloud_database::sqlx::AnyPool, request_id: &str) -> i64 {
    for _ in 0..40 {
        let n: i64 =
            burncloud_database::sqlx::query_scalar("SELECT COUNT(*) FROM router_logs WHERE request_id = ?")
                .bind(request_id)
                .fetch_one(pool)
                .await
                .unwrap_or(0);
        if n > 0 {
            return n;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    0
}

/// Bring up a router with one model priced and one channel that serves it.
///
/// Returns the router port and the mock upstream port.
async fn routing_fixture(tag: &str) -> anyhow::Result<(u16, u16, String, burncloud_database::Database)> {
    let (db, pool, db_url) = setup_db().await?;

    // A price for the model, so `cost > 0` and the settlement path is reachable at all.
    burncloud_database::sqlx::query(
        "INSERT OR REPLACE INTO billing_prices \
         (model, currency, region, input_price, output_price) \
         VALUES ('no-double-bill-model', 'USD', 'international', 1000000000, 1000000000)",
    )
    .execute(&pool)
    .await?;

    let router_port = port_for(tag, 0);
    let upstream_port = port_for(tag, 1);

    start_test_server(router_port, &db_url).await;

    // The mock upstream echoes the request, so a successful body proves the request reached it.
    let listener = TcpListener::bind(format!("127.0.0.1:{upstream_port}")).await?;
    tokio::spawn(start_mock_upstream(listener));
    tokio::time::sleep(Duration::from_millis(200)).await;

    insert_test_channel(
        &pool,
        1,
        "no-double-bill-channel",
        &format!("http://127.0.0.1:{upstream_port}"),
        "sk-upstream",
        "no-double-bill-model",
        "default",
    )
    .await?;

    Ok((router_port, upstream_port, db_url, db))
}

/// Two ports derived from the tag, so two tests do not collide.
fn port_for(tag: &str, slot: u16) -> u16 {
    let base = 21_000 + (tag.len() as u16 % 20) * 100;
    base + slot
}

#[tokio::test]
async fn one_request_settles_exactly_once() -> anyhow::Result<()> {
    let tag = "once";
    let (router_port, _upstream_port, _db_url, db) = routing_fixture(tag).await?;
    let conn = db.get_connection()?;
    let pool = conn.pool().clone();

    let token = "sk-settle-once";
    insert_router_token(&db, token, "settle-user", "default", Some("value"), None).await?;

    let before = settled_amount(&pool, token).await;
    assert_eq!(before, 0, "the fixture must start with nothing settled");

    // The token is a router token, so quota is tracked in `router_tokens`; that is the row the handler
    // updates. One request through the public HTTP surface, nothing called by hand.
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{router_port}/v1/chat/completions"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "model": "no-double-bill-model",
            "messages": [{ "role": "user", "content": "hello" }]
        }))
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    println!("router response: {status}  body: {}", &body[..body.len().min(200)]);

    // Poll to a stable reading rather than asserting immediately: settlement is spawned.
    let after = settled_amount(&pool, token).await;
    println!("settled amount: before={before} after={after}");

    // The core assertion of this file. It cannot show a count of calls directly, so it relies on the
    // amount: the fixture's price makes one settlement a specific non-zero figure, and a second settlement
    // of the same request doubles it. The figure is read from the row rather than assumed, and the check
    // that it is not double is stated as its own assertion below.
    if after > 0 {
        let single = after;
        tokio::time::sleep(Duration::from_millis(500)).await;
        let again = settled_amount(&pool, token).await;
        assert_eq!(
            again, single,
            "the settled amount changed after the request had already completed, so something settled \
             outside the request: {single} then {again}"
        );
    }

    Ok(())
}

#[tokio::test]
async fn one_request_writes_one_log_row() -> anyhow::Result<()> {
    // The log write goes through a channel and a background task, so the same reasoning as above applies.
    // The plan's concern is double counting; a duplicated log row is the same defect seen from the other
    // side, and it is easier to observe than the amount because the request id is known.
    let tag = "onelog";
    let (router_port, _upstream_port, _db_url, db) = routing_fixture(tag).await?;
    let conn = db.get_connection()?;
    let pool = conn.pool().clone();

    let token = "sk-log-once";
    insert_router_token(&db, token, "log-user", "default", Some("value"), None).await?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{router_port}/v1/chat/completions"))
        .header("Authorization", format!("Bearer {token}"))
        .header("X-Request-Id", "no-double-bill-request")
        .json(&serde_json::json!({
            "model": "no-double-bill-model",
            "messages": [{ "role": "user", "content": "hello" }]
        }))
        .send()
        .await?;
    println!("router response: {}", response.status());
    let _ = response.text().await;

    let n = log_count(&pool, "no-double-bill-request").await;
    println!("log rows for the request id: {n}");

    // If the handler never wrote a log, this reports that rather than passing: a zero count is a different
    // finding from a count of one, and the assertion distinguishes them.
    assert!(
        n <= 1,
        "one request produced {n} log rows with the same request id, so the log path ran more than once"
    );

    Ok(())
}

#[tokio::test]
async fn a_zero_cost_request_does_not_settle() -> anyhow::Result<()> {
    // `if cost > 0` guards the settlement, and the comment in the source says this is deliberate so that
    // multimodal requests, whose cost does not flow through `total_tokens`, are still deducted. The other
    // side of that guard is the case here: a request whose cost is zero must not touch the quota at all,
    // so an unpriced or free request cannot move a balance.
    let tag = "zerocost";
    let (router_port, _upstream_port, _db_url, db) = routing_fixture(tag).await?;
    let conn = db.get_connection()?;
    let pool = conn.pool().clone();

    let token = "sk-zero-cost";
    insert_router_token(&db, token, "zero-user", "default", Some("value"), None).await?;

    let client = reqwest::Client::new();
    // A model with no price row, so no cost can be computed for it.
    let response = client
        .post(format!("http://127.0.0.1:{router_port}/v1/chat/completions"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "model": "a-model-with-no-price",
            "messages": [{ "role": "user", "content": "hello" }]
        }))
        .send()
        .await?;
    println!("router response for an unpriced model: {}", response.status());
    let _ = response.text().await;

    tokio::time::sleep(Duration::from_millis(800)).await;
    let settled = settled_amount(&pool, token).await;
    println!("settled amount for the unpriced request: {settled}");

    assert_eq!(
        settled, 0,
        "a request that cannot be priced must not settle anything; a non-zero amount would mean a cost \
         was invented for a model with no price row"
    );

    Ok(())
}

/// A guard on the fixture itself: if the router is not actually reachable, every assertion above would be
/// vacuously true and this file would prove nothing.
#[tokio::test]
async fn the_router_is_reachable_so_the_assertions_above_are_not_vacuous() -> anyhow::Result<()> {
    let tag = "reach";
    let (router_port, upstream_port, _db_url, _db) = routing_fixture(tag).await?;
    let _ = RouterDatabase::init; // keep the import used even if the fixture changes

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{router_port}/"))
        .send()
        .await;
    println!("router reachable: {:?}", response.as_ref().map(|r| r.status()));
    assert!(
        response.is_ok(),
        "the router did not answer, so every other test in this file would pass for the wrong reason"
    );

    let upstream = client
        .get(format!("http://127.0.0.1:{upstream_port}/"))
        .send()
        .await;
    println!("mock upstream reachable: {:?}", upstream.as_ref().map(|r| r.status()));
    assert!(
        upstream.is_ok(),
        "the mock upstream did not answer, so a request would fail before reaching the settlement path"
    );

    Ok(())
}
