#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::panic_in_result_fn,
    reason = "Test-only file: the assertions are the test, and the helper reads JSON of unknown shape."
)]
//! End-to-end settlement contract for #660 and #633 plan section 5 item 12.
//!
//! The test fixture now mirrors the two production contracts that previously made the
//! served path unreachable:
//! - OpenAI-format routes require an OpenAI/Zai channel type.
//! - Billing prices are seeded through `BillingPriceModel`, the same model the router reads.
//!
//! The remaining 502 was a third fixture mismatch: the generic mock upstream returned an
//! echo object with no OpenAI `usage` or `choices`, so response-quality detection classified
//! the HTTP 200 body as malformed and silently failed over. This file therefore uses a
//! dedicated OpenAI-shaped mock response while retaining a `fixture_echo` field so the test
//! can prove that the request actually reached the upstream.
//!
//! The positive contract asserts a non-zero, exact single settlement, stability after the
//! request has finished, and exactly one `router_logs` row for the generated request id.
//! A separate mutation test performs two real quota deductions and proves the exact-once
//! invariant rejects that doubled state.

mod common;

use burncloud_database_router::RouterDatabase;
use common::{insert_router_token, insert_test_channel, setup_db};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

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
const UNPRICED_MODEL: &str = "unpriced-no-settlement-model";
const MOCK_PROMPT_TOKENS: i64 = 1_000;
const MOCK_COMPLETION_TOKENS: i64 = 500;
// $1 / 1M input + $1 / 1M output for 1,000 + 500 tokens.
const EXPECTED_COST_NANODOLLARS: i64 = 1_500_000;

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

/// OpenAI-shaped upstream used only by the settlement contract.
///
/// The generic helper in `common.rs` intentionally returns an arbitrary echo object. That is
/// useful for header/proxy tests, but it is not a valid OpenAI completion response and is therefore
/// rejected by `ResponseQualityDetector`. This mock keeps the echo evidence while also supplying
/// the response shape and usage that the production router expects.
async fn start_openai_mock_upstream(
    listener: tokio::net::TcpListener,
    received: Arc<AtomicUsize>,
    model: String,
) {
    let handler = move |method: axum::http::Method, uri: axum::http::Uri, body: String| {
        let received = Arc::clone(&received);
        let model = model.clone();
        async move {
            received.fetch_add(1, Ordering::SeqCst);
            let request_json = serde_json::from_str::<serde_json::Value>(&body).ok();

            serde_json::json!({
                "id": "chatcmpl-burncloud-settlement-fixture",
                "object": "chat.completion",
                "model": model,
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "fixture-ok"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": MOCK_PROMPT_TOKENS,
                    "completion_tokens": MOCK_COMPLETION_TOKENS,
                    "total_tokens": MOCK_PROMPT_TOKENS + MOCK_COMPLETION_TOKENS
                },
                "fixture_echo": {
                    "method": method.to_string(),
                    "url": uri.to_string(),
                    "json": request_json
                }
            })
            .to_string()
        }
    };

    axum::serve(listener, axum::Router::new().fallback(handler))
        .await
        .unwrap_or_else(|e| panic!("OpenAI mock upstream server error: {e}"));
}

/// Bring up a router with one channel serving `model`.
///
/// `priced=false` deliberately omits the billing price so the other side of the
/// `cost > 0` deduction guard can be asserted.
async fn routed_fixture_for_model(
    tag: &str,
    model: &str,
    priced: bool,
) -> anyhow::Result<(u16, burncloud_database::Database, Arc<AtomicUsize>)> {
    let (db, pool, db_url) = setup_db().await?;
    let router_port = 21_000 + (tag.len() as u16 % 20) * 100;

    if priced {
        seed_price(&db, model).await?;
    }

    let upstream_port = 21_500 + (tag.len() as u16 % 20) * 100;
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{upstream_port}")).await?;
    let received = Arc::new(AtomicUsize::new(0));
    tokio::spawn(start_openai_mock_upstream(
        listener,
        Arc::clone(&received),
        model.to_string(),
    ));
    tokio::time::sleep(Duration::from_millis(200)).await;

    insert_test_channel(
        &pool,
        1,
        "no-double-bill-channel",
        &format!("http://127.0.0.1:{upstream_port}"),
        "sk-upstream",
        model,
        "default",
    )
    .await?;

    describe_routing_inputs(&pool, "default", model).await;

    common::start_test_server(router_port, &db_url).await;
    Ok((router_port, db, received))
}

async fn routed_fixture(
    tag: &str,
) -> anyhow::Result<(u16, burncloud_database::Database, Arc<AtomicUsize>)> {
    routed_fixture_for_model(tag, MODEL, true).await
}

/// Bring up a router with a database, without needing the upstream or channel fixtures.
async fn router_fixture(tag: &str) -> anyhow::Result<(u16, burncloud_database::Database)> {
    let (db, _pool, db_url) = setup_db().await?;
    let router_port = 21_000 + (tag.len() as u16 % 20) * 100;
    common::start_test_server(router_port, &db_url).await;
    Ok((router_port, db))
}

async fn used_quota(pool: &burncloud_database::sqlx::AnyPool, token: &str) -> i64 {
    burncloud_database::sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind(token)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(0)
}

/// Used by the negative tests, where zero is the expected settled amount.
async fn settled_amount(pool: &burncloud_database::sqlx::AnyPool, token: &str) -> i64 {
    used_quota(pool, token).await
}

/// Wait for the async quota deduction to become observable.
///
/// Unlike the old "two equal reads" helper, this never treats two consecutive zeroes as a
/// successful settlement. A served request must first produce a positive amount.
async fn wait_for_positive_settlement(
    pool: &burncloud_database::sqlx::AnyPool,
    token: &str,
) -> i64 {
    let mut last = 0;
    for _ in 0..80 {
        last = used_quota(pool, token).await;
        if last > 0 {
            return last;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    last
}

fn settlement_is_exactly_once(before: i64, after: i64, expected_cost: i64) -> bool {
    after - before == expected_cost
}

type RouterLogProbe = (String, i64, i32, i32, i32);

async fn wait_for_router_log(
    pool: &burncloud_database::sqlx::AnyPool,
    model: &str,
) -> Option<RouterLogProbe> {
    for _ in 0..80 {
        let row: Option<RouterLogProbe> = burncloud_database::sqlx::query_as(
            "SELECT request_id, cost, status_code, prompt_tokens, completion_tokens \
             FROM router_logs WHERE model = ? ORDER BY id DESC LIMIT 1",
        )
        .bind(model)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        if row.is_some() {
            return row;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    None
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

/// A successful request reaches the upstream, settles exactly once, and logs exactly once.
#[tokio::test]
async fn a_successful_request_settles_exactly_once_and_logs_once() -> anyhow::Result<()> {
    let (router_port, db, upstream_received) = routed_fixture("served_once").await?;
    let conn = db.get_connection()?;
    let pool = conn.pool().clone();

    let token = "sk-settlement-served";
    insert_router_token(&db, token, "served-user", "default", Some("value"), None).await?;
    grant_quota(&db, token, "served-user").await?;

    let before = used_quota(&pool, token).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "http://127.0.0.1:{router_port}/v1/chat/completions"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "model": MODEL,
            "messages": [{ "role": "user", "content": "hello" }]
        }))
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    println!(
        "served response status: {status}; body: {}",
        &body[..body.len().min(320)]
    );

    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "the fixture must produce a served request, not another routing/preflight/failover refusal"
    );
    assert!(
        body.contains("\"fixture_echo\"") && body.contains("\"fixture-ok\""),
        "the response must be the OpenAI-shaped mock upstream response; body: {}",
        &body[..body.len().min(320)]
    );
    assert_eq!(
        upstream_received.load(Ordering::SeqCst),
        1,
        "exactly one request must reach the mock upstream"
    );

    // Read the billing log before asserting quota. Besides being part of the contract,
    // this makes a failed settlement self-diagnosing: non-zero log cost + zero quota means
    // the write path failed; zero log cost means usage/cost calculation failed earlier.
    let log_probe = wait_for_router_log(&pool, MODEL).await;
    let legacy_used: i64 = burncloud_database::sqlx::query_scalar(
        "SELECT used_quota FROM user_api_keys WHERE key = ?",
    )
    .bind(token)
    .fetch_one(&pool)
    .await
    .unwrap_or(0);

    let after = wait_for_positive_settlement(&pool, token).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let stable_after = used_quota(&pool, token).await;

    println!(
        "used_quota before={before}, after={after}, stable_after={stable_after}, user_api_keys.used_quota={legacy_used}, expected_once={EXPECTED_COST_NANODOLLARS}, router_log={log_probe:?}"
    );

    assert!(
        after > before,
        "a served request must produce a non-zero settlement; before={before}, after={after}, user_api_keys.used_quota={legacy_used}, router_log={log_probe:?}"
    );
    assert!(
        settlement_is_exactly_once(before, after, EXPECTED_COST_NANODOLLARS),
        "settlement must equal exactly one request cost; before={before}, after={after}, expected_delta={EXPECTED_COST_NANODOLLARS}"
    );
    assert_eq!(
        stable_after, after,
        "used_quota changed after the request had already settled, which is evidence of a delayed duplicate settlement"
    );

    let (request_id, log_cost, log_status, prompt_tokens, completion_tokens) =
        wait_for_router_log(&pool, MODEL)
            .await
            .expect("the served request must produce a router_logs row");

    println!(
        "router_log request_id={request_id}, status={log_status}, cost={log_cost}, prompt_tokens={prompt_tokens}, completion_tokens={completion_tokens}"
    );

    assert_eq!(
        log_status, 200,
        "the billing log must record the served status"
    );
    assert_eq!(
        log_cost, EXPECTED_COST_NANODOLLARS,
        "the logged cost must equal the one-request settlement"
    );
    assert_eq!(prompt_tokens as i64, MOCK_PROMPT_TOKENS);
    assert_eq!(completion_tokens as i64, MOCK_COMPLETION_TOKENS);

    let rows_for_request: i64 = burncloud_database::sqlx::query_scalar(
        "SELECT COUNT(*) FROM router_logs WHERE request_id = ?",
    )
    .bind(&request_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        rows_for_request, 1,
        "exactly one router_logs row must carry request_id={request_id}"
    );

    Ok(())
}

/// The other side of the `cost > 0` guard: an unpriced model is rejected before the
/// upstream and must not move quota.
#[tokio::test]
async fn an_unpriced_model_settles_zero() -> anyhow::Result<()> {
    let (router_port, db, upstream_received) =
        routed_fixture_for_model("unpriced_zero", UNPRICED_MODEL, false).await?;
    let conn = db.get_connection()?;
    let pool = conn.pool().clone();

    let token = "sk-unpriced-zero";
    insert_router_token(
        &db,
        token,
        "unpriced-zero-user",
        "default",
        Some("value"),
        None,
    )
    .await?;
    grant_quota(&db, token, "unpriced-zero-user").await?;

    let before = used_quota(&pool, token).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "http://127.0.0.1:{router_port}/v1/chat/completions"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "model": UNPRICED_MODEL,
            "messages": [{ "role": "user", "content": "hello" }]
        }))
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let after = used_quota(&pool, token).await;

    println!(
        "unpriced response status: {status}; used_quota before={before}, after={after}; body: {}",
        &body[..body.len().min(240)]
    );

    assert_eq!(
        status,
        reqwest::StatusCode::BAD_REQUEST,
        "strict billing must reject an unpriced model before upstream"
    );
    assert!(
        body.contains("model_not_found"),
        "the refusal must be attributable to missing pricing; body: {}",
        &body[..body.len().min(240)]
    );
    assert_eq!(
        upstream_received.load(Ordering::SeqCst),
        0,
        "preflight pricing rejection must happen before contacting upstream"
    );
    assert_eq!(
        after, before,
        "an unpriced request has cost=0 and must not trigger quota deduction"
    );

    Ok(())
}

/// Mutation evidence for the key invariant.
///
/// This deliberately performs two real quota deductions. The first assertion proves the mutation
/// actually changed state; the second proves the exact-once predicate rejects that doubled state.
#[tokio::test]
async fn the_exact_once_contract_rejects_a_double_deduction_mutation() -> anyhow::Result<()> {
    let (db, pool, _db_url) = setup_db().await?;
    let token = "sk-double-settlement-mutation";
    let user_id = "double-settlement-user";

    insert_router_token(&db, token, user_id, "default", Some("value"), None).await?;
    grant_quota(&db, token, user_id).await?;

    let before = used_quota(&pool, token).await;
    assert!(
        RouterDatabase::deduct_quota(&db, user_id, token, EXPECTED_COST_NANODOLLARS).await?,
        "first injected deduction must execute"
    );
    assert!(
        RouterDatabase::deduct_quota(&db, user_id, token, EXPECTED_COST_NANODOLLARS).await?,
        "second injected deduction must execute"
    );
    let after = used_quota(&pool, token).await;

    println!(
        "double-settlement mutation before={before}, after={after}, delta={}",
        after - before
    );

    assert_eq!(
        after - before,
        EXPECTED_COST_NANODOLLARS * 2,
        "the mutation must really double the settled amount before claiming the invariant catches it"
    );
    assert!(
        !settlement_is_exactly_once(before, after, EXPECTED_COST_NANODOLLARS),
        "the exact-once invariant failed to detect a doubled settlement"
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
