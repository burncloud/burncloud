#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    reason = "Test-only file: the assertions are the test, and the helper reads JSON of unknown shape."
)]
//! Whether one request settles once (#633, plan section 5 item 12), and the fixture work it needs.
//!
//! ## What this file currently contains, and why it is the honest version
//!
//! The first version drove a request through the router and asserted that the settled amount did not
//! change. **It passed while the request was being refused with 402**, because "the amount did not
//! change" is trivially true when nothing settles. A mutation run -- three mutations, none caught --
//! exposed that, and the assertion was changed to require that a settlement happened *at all*.
//!
//! With the stronger assertion the reason for the refusal became visible: `402 Payment Required`,
//! because the router reads `user_api_keys.remain_quota` while the fixture only set `router_tokens`. Fixing
//! that moved the failure to `404 no_available_channel`, because the channel fixture in `common.rs` is not
//! visible to the routing path.
//!
//! So the end-to-end test cannot pass yet, and rather than commit a test that passes for the wrong reason
//! or a red test with an unexplained cause, this file keeps the part that is sound and records the
//! blocker. The blocker is filed as its own issue with everything found so far.
//!
//! ## What is asserted here
//!
//! A credential the router will not authenticate is rejected with 401 and settles nothing. That is a
//! real assertion about the settlement path -- no settlement can happen for a request that is refused --
//! it runs with the fixture as it is, and it is the half of "settles once" that is observable today.

mod common;

use common::{insert_router_token, setup_db};
use std::time::Duration;

/// Bring up a router with a database, without needing the upstream or channel fixtures.
async fn router_fixture(tag: &str) -> anyhow::Result<(u16, burncloud_database::Database)> {
    let (db, _pool, db_url) = setup_db().await?;
    let router_port = 21_000 + (tag.len() as u16 % 20) * 100;
    common::start_test_server(router_port, &db_url).await;
    Ok((router_port, db))
}

/// Read the settled amount, retrying until it stops changing.
///
/// `deduct_quota` runs in a spawned task, so the first read after a response can legitimately be 0. Two
/// consecutive equal readings are taken as settled; the bound keeps a failure a failure rather than a hang.
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

#[tokio::test]
async fn a_request_that_is_not_authenticated_settles_nothing() -> anyhow::Result<()> {
    // The negative half of "settles once", and the half that is observable with the current fixture. A
    // refused request must not touch a balance: an authentication failure that still settled would charge
    // for work that never happened.
    let (router_port, db) = router_fixture("unauth").await?;
    let conn = db.get_connection()?;
    let pool = conn.pool().clone();

    // A credential that exists and is active, so this is about the request and not about a missing row.
    let token = "sk-settlement-unauth";
    insert_router_token(&db, token, "unauth-user", "default", Some("value"), None).await?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "http://127.0.0.1:{router_port}/v1/chat/completions"
        ))
        // No Authorization header at all, so authentication cannot succeed.
        .json(&serde_json::json!({
            "model": "any-model",
            "messages": [{ "role": "user", "content": "hello" }]
        }))
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    println!("response: {status}  body: {}", &body[..body.len().min(200)]);

    assert_eq!(
        status,
        reqwest::StatusCode::UNAUTHORIZED,
        "a request without a bearer token must be refused with 401, got {status}"
    );

    tokio::time::sleep(Duration::from_millis(500)).await;
    let settled = settled_amount(&pool, token).await;
    assert_eq!(
        settled, 0,
        "a refused request settled {settled}, so a balance moved for a request that was never served"
    );

    Ok(())
}

#[tokio::test]
async fn a_request_with_an_unknown_credential_settles_nothing() -> anyhow::Result<()> {
    // The same property for a credential the router cannot resolve, which takes the other branch of the
    // authentication path.
    let (router_port, db) = router_fixture("unknown").await?;
    let conn = db.get_connection()?;
    let pool = conn.pool().clone();

    let token = "sk-never-issued";
    // Deliberately not inserted: the credential does not exist.

    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "http://127.0.0.1:{router_port}/v1/chat/completions"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "model": "any-model",
            "messages": [{ "role": "user", "content": "hello" }]
        }))
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    println!("response: {status}  body: {}", &body[..body.len().min(200)]);

    assert!(
        status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN,
        "an unknown credential must be refused with 401 or 403, got {status}"
    );

    // Nothing to read a quota from, so the assertion is that no row appeared for it.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let rows: i64 = burncloud_database::sqlx::query_scalar(
        "SELECT COUNT(*) FROM router_tokens WHERE token = ?",
    )
    .bind(token)
    .fetch_one(&pool)
    .await
    .unwrap_or(0);
    assert_eq!(
        rows, 0,
        "a settlement created a router_tokens row for a credential that was never issued"
    );

    Ok(())
}

/// The fixture must answer, or the assertions above would pass because nothing was reachable.
#[tokio::test]
async fn the_router_is_reachable_so_the_assertions_above_are_not_vacuous() -> anyhow::Result<()> {
    let (router_port, _db) = router_fixture("reach").await?;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{router_port}/"))
        .send()
        .await;
    println!(
        "router reachable: {:?}",
        response.as_ref().map(|r| r.status())
    );
    assert!(
        response.is_ok(),
        "the router did not answer, so every other test in this file would pass for the wrong reason"
    );

    Ok(())
}
