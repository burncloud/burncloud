#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::panic_in_result_fn,
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

use common::{insert_router_token, insert_test_channel, setup_db};
use std::time::Duration;

/// Print what the routing path will actually see, so a failure names the missing row instead of reporting
/// "no candidates".
///
/// The routing lookup in `model_router.rs:98-145` needs a `channel_abilities` row matching
/// `(group, model, enabled = 1)`; only then does it resolve the channel through
/// `ChannelProviderModel::list_by_ids`. When any of those is absent the router answers 404
/// `no_available_channel`, which does not say which condition failed. This dumps the tables so the
/// condition is visible in the test output.
async fn describe_routing_inputs(
    pool: &burncloud_database::sqlx::AnyPool,
    group: &str,
    model: &str,
) {
    let abilities: Vec<(String, String, i32, i64, i64)> = burncloud_database::sqlx::query_as(
        "SELECT `group`, model, channel_id, enabled, priority FROM channel_abilities",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    println!("channel_abilities rows: {abilities:?}");

    let matching: i64 = burncloud_database::sqlx::query_scalar(
        "SELECT COUNT(*) FROM channel_abilities WHERE `group` = ? AND model = ? AND enabled = 1",
    )
    .bind(group)
    .bind(model)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    println!("rows matching (group={group}, model={model}, enabled=1): {matching}");

    let providers: Vec<(i32, String, i64, String)> = burncloud_database::sqlx::query_as(
        "SELECT id, name, status, models FROM channel_providers",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    println!("channel_providers rows: {providers:?}");

    // The columns the router's channel lookup reads. A fixture created before a migration added a column
    // would be missing them, which is the hypothesis this prints an answer to.
    let cols: Vec<String> = burncloud_database::sqlx::query("PRAGMA table_info(channel_providers)")
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| {
            use burncloud_database::sqlx::Row;
            r.get::<String, _>("name")
        })
        .collect();
    println!("channel_providers columns: {cols:?}");
}

/// Give a credential enough quota that the router will serve it.
///
/// **The served path reads `user_api_keys.remain_quota`, not `router_tokens.quota_limit`.** From
/// `lib.rs:1432-1448`: when `validate_token_and_get_info` succeeds, the router takes the limit from
/// `info.remain_quota`, which is that column. The `router_tokens` row is only read in the `Ok(None)`
/// fallback branch, which a credential present in `user_api_keys` never reaches. Without this the router
/// answers 402 before reaching the upstream.
///
/// Both rows are set: the first because it is the one that is read, the second to keep the fallback usable.
async fn grant_quota(
    db: &burncloud_database::Database,
    token: &str,
    user_id: &str,
) -> anyhow::Result<()> {
    let pool = db.get_connection()?.pool();

    let updated = burncloud_database::sqlx::query(
        "UPDATE user_api_keys SET remain_quota = 1000000000, used_quota = 0 WHERE key = ?",
    )
    .bind(token)
    .execute(pool)
    .await?
    .rows_affected();
    assert_eq!(
        updated, 1,
        "the credential must already exist in user_api_keys for its quota to be set; the fixture inserts \
         it first, so a count of 0 means the fixture changed"
    );

    burncloud_database::sqlx::query(
        "INSERT OR REPLACE INTO router_tokens \
         (token, user_id, status, quota_limit, used_quota, expired_time, accessed_time) \
         VALUES (?, ?, 'active', 1000000000, 0, -1, 0)",
    )
    .bind(token)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// The model this file routes. Named once so a failure message and the seeded price cannot drift apart.
const MODEL: &str = "no-double-bill-model";

/// Seed a price through the same model the production code reads.
///
/// A raw `INSERT INTO billing_prices` is **not** what the router reads: with one written by hand the router
/// answered `400 model_not_found`, "Model 'no-double-bill-model' is not supported or has no price
/// configured". `BillingPriceModel::upsert` is what `e2e_billing_tests.rs` uses and what the price lookup
/// sees, so this goes through it too.
///
/// `PriceInput` has no `Default`, so every field is named. The unset ones are `None` deliberately: this file
/// is about settlement, not about which optional prices exist.
async fn seed_price(db: &burncloud_database::Database, model: &str) -> anyhow::Result<()> {
    use burncloud_commerce_contracts::price_u64::dollars_to_nano;
    use burncloud_database_billing::{BillingPriceModel, PriceInput};

    let input = PriceInput {
        model: model.to_string(),
        // A deliberately large price: one request must settle a visible, non-zero amount, which is what
        // makes "settled once" distinguishable from "never settled".
        input_price: dollars_to_nano(1.0),
        output_price: dollars_to_nano(1.0),
        currency: "USD".to_string(),
        cache_read_input_price: None,
        cache_creation_input_price: None,
        batch_input_price: None,
        batch_output_price: None,
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        audio_output_price: None,
        reasoning_price: None,
        embedding_price: None,
        image_price: None,
        video_price: None,
        music_price: None,
        source: None,
        region: None,
        context_window: None,
        max_output_tokens: None,
        supports_vision: None,
        supports_function_calling: None,
        voices_pricing: None,
        video_pricing: None,
        asr_pricing: None,
        realtime_pricing: None,
        model_type: None,
    };
    BillingPriceModel::upsert(db, &input).await?;

    // Assert the price is readable through the same lookup the router uses, so a seeding mistake fails here
    // rather than as a confusing 400 later.
    let stored = BillingPriceModel::get(db, model, "USD", None).await?;
    assert!(
        stored.is_some(),
        "the price for {model} is not readable after seeding, so the router would answer model_not_found"
    );

    Ok(())
}

/// Bring up a router with a database and one channel serving one model.
async fn routed_fixture(tag: &str) -> anyhow::Result<(u16, burncloud_database::Database)> {
    let (db, pool, db_url) = setup_db().await?;
    let router_port = 21_000 + (tag.len() as u16 % 20) * 100;

    seed_price(&db, MODEL).await?;

    let upstream_port = 21_500 + (tag.len() as u16 % 20) * 100;
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{upstream_port}")).await?;
    tokio::spawn(common::start_mock_upstream(listener));
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

    describe_routing_inputs(&pool, "default", "no-double-bill-model").await;

    common::start_test_server(router_port, &db_url).await;
    Ok((router_port, db))
}

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

/// Diagnostic probe for the routing fixture blocker (#660). Not an assertion about behaviour yet: it prints
/// what the routing path sees and what the router answers, so the failing condition is named rather than
/// inferred from a 404.
///
/// **Ignored, not deleted.** It fails today, on purpose: the fixture does not produce a served request. It is
/// kept so the blocker is reproducible with one command rather than a paragraph --
/// `cargo test -p burncloud-router --test settlement_once -- --ignored --nocapture` prints the tables, the
/// candidate count at each pipeline stage, and the router's answer. Remove the `ignore` when #660 is fixed;
/// the test then asserts the served request and becomes the end-to-end half of this file.
#[ignore = "blocked on #660: the routing fixture does not produce a served request"]
#[tokio::test]
async fn probe_what_the_routing_path_sees() -> anyhow::Result<()> {
    let (router_port, db) = routed_fixture("probe").await?;
    let conn = db.get_connection()?;
    let pool = conn.pool().clone();

    let token = "sk-probe-channel";
    insert_router_token(&db, token, "probe-user", "default", Some("value"), None).await?;
    grant_quota(&db, token, "probe-user").await?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "http://127.0.0.1:{router_port}/v1/chat/completions"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "model": "no-double-bill-model",
            "messages": [{ "role": "user", "content": "hello" }]
        }))
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    println!("PROBE status: {status}");
    println!("PROBE body: {}", &body[..body.len().min(300)]);

    let settled = settled_amount(&pool, token).await;
    println!("PROBE settled: {settled}");
    assert!(
        status.is_success(),
        "the routing fixture still does not produce a served request; the PROBE lines above name which \
         condition is unmet"
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
