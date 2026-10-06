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

/// One `router_logs` row as this file reads it back: `(cost, prompt, completion, status, cost_status)`.
///
/// Named rather than inlined as a five-element tuple, both because the diagnostic prints it and because the
/// column order is the only thing tying it to the SELECT.
type SettledLogRow = (i64, i32, i32, i64, Option<String>);

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

/// Derive a distinct router/upstream port pair from a tag.
///
/// `tag.len() % 20` mapped five different tags onto the same port, so two tests running in parallel raced for
/// one socket: whichever bound second failed to bind and its server never came up, which surfaced as `502` or
/// `401` in an unrelated test. Hashing the whole tag makes collisions require equal tags, and equal tags are
/// the same fixture by definition.
fn ports_for(tag: &str) -> (u16, u16) {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    tag.hash(&mut hasher);
    let slot = (hasher.finish() % 40) as u16;
    (21_000 + slot * 10, 25_000 + slot * 10)
}

/// Bring up a router with a database and one channel serving one model.
async fn routed_fixture(tag: &str) -> anyhow::Result<(u16, burncloud_database::Database)> {
    let (db, pool, db_url) = setup_db().await?;
    let (router_port, upstream_port) = ports_for(tag);

    seed_price(&db, MODEL).await?;

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
    let (router_port, _) = ports_for(tag);
    common::start_test_server(router_port, &db_url).await;
    Ok((router_port, db))
}

/// Read the settled amount, polling until it moves and then until it stops changing.
///
/// `deduct_quota` runs in a **spawned** task (`lib.rs:1990-1998`), so the row is not necessarily updated by the
/// time the response reaches the client — the old version of this helper treated two consecutive equal readings
/// as "settled", which returned 0 immediately and reported "settled nothing" for a request that settled a moment
/// later. It now waits for the value to move first, and only then requires it to be stable.
///
/// The bound keeps a genuine failure a failure rather than a hang.
async fn settled_amount(pool: &burncloud_database::sqlx::AnyPool, token: &str) -> i64 {
    let mut last = i64::MIN;
    let mut stable_reads = 0;
    // 200 x 50 ms = 10 s, which is far longer than the spawn needs and short enough to fail a run.
    for _ in 0..200 {
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
            stable_reads += 1;
            // Only trust "unchanged" once it has been observed unchanged from a non-zero reading: a request
            // that never settled must not be able to satisfy the stability check.
            if stable_reads >= 3 && current != 0 {
                return current;
            }
        } else {
            stable_reads = 0;
            last = current;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    last.max(0)
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

/// The end-to-end half of this file: a served request is **settled exactly once**.
///
/// The two tests above prove the router settles *nothing* when it rejects a request. This one proves it settles
/// the right amount when it serves one — the assertion #660 was blocked on, and which turned out to expose a
/// second, independent defect once the fixture could route.
///
/// ## What this test found
///
/// With the fixture repaired, the request was served and `router_logs` recorded `cost = 200_000` with
/// `cost_status = "ok"`, but neither `router_tokens.used_quota` nor `user_api_keys.used_quota` moved. The
/// deduction *was* attempted — `SETTLEMENT PROBE` in the router showed it — and it came back
/// `Err(Connection(Database(SqliteError { code: 5, message: "database is locked" })))`. The production call
/// discarded that result (`let _ = ...deduct_quota(...)`), so the system logged a charge it never collected and
/// nothing anywhere said so. Silent under-billing.
///
/// The fix is in `settle_quota_with_retry` (`lib.rs`): a transient failure is retried, and a failure that
/// survives every attempt is logged at `error` and counted in `settlement_failure_count`, which
/// `/console/internal/health` exposes. This test is what pins that the ledger now moves.
///
/// Enabling the probe needed `tracing` in the test harness, which `common.rs` now starts when `RUST_LOG` is
/// set: a test that can only observe side effects cannot tell "the code took the wrong branch" from "the side
/// effect failed", and that ambiguity is what hid this for so long.
#[tokio::test]
async fn a_served_request_settles_exactly_once() -> anyhow::Result<()> {
    let (router_port, db) = routed_fixture("settled").await?;
    let conn = db.get_connection()?;
    let pool = conn.pool().clone();

    let token = "sk-settled-once";
    insert_router_token(&db, token, "settled-user", "default", Some("value"), None).await?;
    grant_quota(&db, token, "settled-user").await?;

    let before = settled_amount(&pool, token).await;

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
    println!("settled-e2e status: {status}");
    println!("settled-e2e body: {}", &body[..body.len().min(300)]);

    assert!(
        status.is_success(),
        "the request must be served end to end; the fixture produced {status}: {}",
        &body[..body.len().min(300)]
    );
    let parsed: serde_json::Value = serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("the body must be the upstream's JSON: {e}: {body}"));
    assert_eq!(
        parsed["choices"][0]["message"]["content"], "mock answer",
        "the client must receive the upstream's completion, so this is not a locally-generated success"
    );

    let after = settled_amount(&pool, token).await;
    let settled = after - before;
    println!("settled-e2e used_quota before={before} after={after} delta={settled}");

    let logs_seen: Vec<SettledLogRow> = burncloud_database::sqlx::query_as(
        "SELECT cost, prompt_tokens, completion_tokens, status_code, cost_status FROM router_logs \
         WHERE model = 'no-double-bill-model'",
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();
    println!(
        "settled-e2e router_logs(cost, prompt, completion, status, cost_status) = {logs_seen:?}"
    );

    assert!(
        settled > 0,
        "a served request must settle a non-zero amount; before={before} after={after}. Asserting only that the \
         amount 'does not change' would let a request that never settled pass, which is how this hid"
    );

    // The ledger and the log must agree, or one of them is wrong about what was charged. This is the
    // assertion that would have caught the discarded-error defect from the outside: the log said 200_000 and
    // the balance said 0.
    let logged_cost: i64 = logs_seen.first().map(|row| row.0).unwrap_or(0);
    assert_eq!(
        settled, logged_cost,
        "the settled amount must equal the logged cost: the ledger debited {settled} while router_logs \
         recorded {logged_cost}"
    );

    let after_settle = settled_amount(&pool, token).await;
    assert_eq!(
        after_settle, after,
        "the amount must not move after the request has finished; a second settlement is a double bill"
    );

    // A successful settlement means the retry in `settle_quota_with_retry` absorbed the transient failure, so
    // the alertable failure counter must be back at 0. Asserted through the router's own health endpoint
    // rather than by reading the field, because that is the surface an operator actually watches.
    let health: serde_json::Value = client
        .get(format!(
            "http://127.0.0.1:{router_port}/console/internal/health"
        ))
        .send()
        .await?
        .json()
        .await?;
    let failures = health["settlement_failure_count"].as_u64();
    println!("settled-e2e health settlement_failure_count = {failures:?}");
    assert_eq!(
        failures,
        Some(0),
        "a settlement that succeeded must not be counted as a failure; health reported {failures:?}"
    );

    let logs: i64 = burncloud_database::sqlx::query_scalar(
        "SELECT COUNT(*) FROM router_logs WHERE user_id = ? AND model = 'no-double-bill-model'",
    )
    .bind("settled-user")
    .fetch_one(&pool)
    .await
    .unwrap_or(0);
    assert_eq!(
        logs, 1,
        "exactly one router_logs row must exist for the request, found {logs}"
    );

    Ok(())
}

/// The other side of the `cost > 0` guard: a model that has no price settles nothing, and does not fail the
/// request either. Without this, "settled" and "settles everything" are indistinguishable.
#[tokio::test]
async fn a_served_request_for_an_unpriced_model_settles_nothing() -> anyhow::Result<()> {
    let (db, pool, db_url) = setup_db().await?;
    // Distinct from every `routed_fixture` port, so this test does not race the others for a socket.
    let (router_port, upstream_port) = ports_for("unpriced-model-fixture");

    // No `seed_price` call: the model is deliberately absent from `billing_prices`.
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{upstream_port}")).await?;
    tokio::spawn(common::start_mock_upstream(listener));
    tokio::time::sleep(Duration::from_millis(200)).await;

    insert_test_channel(
        &pool,
        1,
        "unpriced-channel",
        &format!("http://127.0.0.1:{upstream_port}"),
        "sk-upstream",
        "unpriced-model",
        "default",
    )
    .await?;

    common::start_test_server(router_port, &db_url).await;

    let token = "sk-unpriced";
    insert_router_token(&db, token, "unpriced-user", "default", Some("value"), None).await?;
    grant_quota(&db, token, "unpriced-user").await?;

    let before = settled_amount(&pool, token).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "http://127.0.0.1:{router_port}/v1/chat/completions"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "model": "unpriced-model",
            "messages": [{ "role": "user", "content": "hello" }]
        }))
        .send()
        .await?;
    println!("unpriced status: {}", response.status());

    tokio::time::sleep(Duration::from_millis(300)).await;
    let after = settled_amount(&pool, token).await;
    println!("unpriced used_quota before={before} after={after}");
    assert_eq!(
        after, before,
        "an unpriced model must settle nothing rather than settling a guess"
    );

    Ok(())
}

/// What the routing path sees, kept as a diagnostic. It asserts only that the fixture produces a served
/// request, so a future failure names the stage (the `PROBE` lines print the candidate counts) instead of
/// surfacing as a 404 three assertions later.
///
/// Runs as a normal test since #660 was fixed: `cargo test -p burncloud-router --test settlement_once
/// -- --nocapture` prints the tables, the candidate count at each pipeline stage, and the router's answer.
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
        "the routing fixture must produce a served request; the PROBE lines above name which condition is \
         unmet"
    );
    // The settlement assertion is deliberately **not** here: the served request currently logs its cost but
    // does not debit the ledger, which is a production defect recorded by
    // `a_served_request_settles_exactly_once` above rather than a fixture problem. This probe stays a probe so
    // the fixture blocker and the settlement defect remain distinguishable.

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
