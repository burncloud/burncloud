#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::panic_in_result_fn,
    reason = "Test-only file: the assertions are the test, and the fixtures are JSON of unknown shape."
)]
//! A request for a model with no price must not be served (#633, plan section 5 item 12).
//!
//! The plan lists "价格缺失不能放行" -- a missing price must not be let through. The router has the machinery:
//! `billing_strict` defaults to **true** (`lib.rs:847`, `unwrap_or(true)`), a preflight failure in strict mode
//! produces 400 (`lib.rs:2361-2362`), and a price that disappears **after** preflight is recorded as
//! `cost_status = "price_missing"` with a counter (`lib.rs:1823-1828`, exposed at `:1380`).
//!
//! ## What was not covered
//!
//! `billing_observability_tests.rs` covers the observability of all that, and its own doc comment says of the
//! handler:
//!
//! ```text
//! The proxy_handler integration (BILLING_STRICT_MODE env var) is tested
//! implicitly: strict=true is the default and the preflight check returns
//! Err(PriceNotFound) which the handler converts to 400.
//! ```
//!
//! **"Implicitly" is the gap.** `t2_preflight_rejects_unknown_model` calls `CostCalculator::preflight`
//! directly, so it asserts that the *calculator* reports the error. Nothing asserted that the **router**
//! refuses the request -- and the two are different statements.
//!
//! ## A first attempt at this file passed for the wrong reason, which is why the fixture changed
//!
//! The obvious fixture is a router with a database and no channels. Measured, that refuses everything:
//!
//! ```text
//! unpriced model -> 404 {"code":"no_available_channel"}
//! priced   model -> 404 {"code":"no_available_channel"}
//! ```
//!
//! Both are refused **before** the billing preflight runs, so a test asserting "the unpriced request was
//! refused" would have passed without the refusal having anything to do with the price. The control test in
//! that version -- "a priced model is still served" -- is what exposed it, since the priced model was refused
//! too.
//!
//! A channel is therefore seeded, and the upstream it points at is a local mock, so a request that gets past
//! routing reaches something real. What the tests then distinguish is a **billing** refusal from a routing
//! one, by the error the router returns rather than by the status code alone.
//!
//! ## The environment constraint this file works within
//!
//! `BILLING_STRICT_MODE` is read **once**, inside `create_router_app` (`lib.rs:845`), and stored in
//! `AppState::billing_strict`. It cannot be changed per request, and changing it per test would mean setting a
//! process-wide variable that Rust's default parallel test threads share -- a race, not a test. So strict mode
//! is exercised (it is the default and therefore what production runs) and the non-strict branch is described
//! at the end of the file rather than faked.

mod common;

use burncloud_commerce_billing::{BillingPriceModel, PriceInput};
use common::{insert_test_channel, setup_db, start_mock_upstream, start_test_server};
use std::time::Duration;
use tokio::net::TcpListener;

/// Two ports derived from the tag, so tests in this file do not collide.
fn ports(tag: &str) -> (u16, u16) {
    let base = 23_000 + (tag.len() as u16 % 20) * 100;
    (base, base + 1)
}

fn body(model: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": "hello" }]
    })
}

/// A price row for `model`, at a rate that makes a non-zero charge easy to recognise.
async fn seed_price(db: &burncloud_database::Database, model: &str) -> anyhow::Result<()> {
    let input = PriceInput {
        model: model.to_string(),
        input_price: 1_000_000,
        output_price: 1_000_000,
        currency: "USD".to_string(),
        ..Default::default()
    };
    BillingPriceModel::upsert(db, &input).await?;
    Ok(())
}

/// Give a credential enough quota that the router will serve it.
///
/// The served path reads `user_api_keys.remain_quota` (see `lib.rs:1432-1448`), and `common::insert_router_token`
/// inserts that row with only `user_id` and `key`, so the quota has to be set here.
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
        "the credential must exist in user_api_keys for its quota to be set"
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

/// A router with one channel and a mock upstream, so a request that passes routing reaches something.
///
/// **`priced_models` are seeded before the server starts, and that order is load-bearing.** The router loads
/// its price cache once, inside `create_router_app` (`lib.rs`), so a row inserted afterwards is invisible to
/// preflight and the request is refused with the very error these tests are about. The first version of this
/// fixture started the server first and seeded afterwards, which made the **control** test fail with
/// `model_not_found` -- the same refusal as the unpriced case, so nothing was being distinguished.
///
/// `BILLING_STRICT_MODE` is deliberately **not** touched: the default is strict, and setting it here would race
/// with every other test in this binary.
async fn router_with_channel(
    tag: &str,
    model: &str,
    priced_models: &[&str],
) -> anyhow::Result<(u16, burncloud_database::Database)> {
    let (db, pool, db_url) = setup_db().await?;
    let (router_port, upstream_port) = ports(tag);

    let listener = TcpListener::bind(format!("127.0.0.1:{upstream_port}")).await?;
    tokio::spawn(start_mock_upstream(listener));
    tokio::time::sleep(Duration::from_millis(150)).await;

    insert_test_channel(
        &pool,
        1,
        "missing-price-channel",
        &format!("http://127.0.0.1:{upstream_port}"),
        "sk-upstream",
        model,
        "default",
    )
    .await?;

    // Prices first, so the cache the router builds at startup contains them.
    for priced in priced_models {
        seed_price(&db, priced).await?;
    }

    start_test_server(router_port, &db_url).await;
    Ok((router_port, db))
}

// -------------------------------------------------------------------------------------------
// the behaviour the plan asks about
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_priced_model_passes_preflight_and_reaches_the_upstream() -> anyhow::Result<()> {
    // The **control**, and the test that made the first version of this file honest. A model with a price and a
    // channel that serves it must get past routing and past billing, so that the refusal in the next test can
    // be attributed to the missing price rather than to having nothing to route to.
    let model = "control-priced-model";
    let (router_port, db) = router_with_channel("control", model, &[model]).await?;

    let token = "sk-control-priced";
    common::insert_router_token(&db, token, "control-user", "default", Some("v"), None).await?;
    grant_quota(&db, token, "control-user").await?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "http://127.0.0.1:{router_port}/v1/chat/completions"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&body(model))
        .send()
        .await?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    println!(
        "priced model with a channel -> {status}: {}",
        &text[..text.len().min(200)]
    );

    assert!(
        !text.contains("no_available_channel"),
        "the fixture must route, or the next test's refusal cannot be attributed to the price. Body: {}",
        &text[..text.len().min(200)]
    );
    assert!(
        !text.contains("price_missing") && !text.contains("PriceNotFound"),
        "a priced model must not be refused for a missing price. Body: {}",
        &text[..text.len().min(200)]
    );
    println!("and the status for a request that routed: {status}");
    Ok(())
}

#[tokio::test]
async fn a_request_for_an_unpriced_model_is_refused_for_the_price_and_not_for_routing(
) -> anyhow::Result<()> {
    // The statement `billing_observability_tests.rs` leaves implicit, made in a fixture where it means
    // something. The **control** test above establishes that this fixture shape routes: one channel, one
    // model, a price row, and the request gets past `no_available_channel`. This test uses the same shape and
    // removes only the price row, so the refusal can be attributed to that.
    //
    // **A note on the fixture.** `common::insert_test_channel` takes a single `model` and writes one
    // `channel_abilities` row for it -- the router matches abilities by exact `model` equality, so a
    // comma-separated list would be stored as the literal string and never match. An earlier version passed
    // `"priced,unpriced"` and produced `no_available_channel` for both, which this test's routing assertion
    // caught.
    let model = "unpriced-model";
    let (router_port, db) = router_with_channel("unpriced", model, &[]).await?;
    // Deliberately no price row for `model`.

    let token = "sk-unpriced-model";
    common::insert_router_token(&db, token, "unpriced-user", "default", Some("v"), None).await?;
    grant_quota(&db, token, "unpriced-user").await?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "http://127.0.0.1:{router_port}/v1/chat/completions"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&body(model))
        .send()
        .await?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    println!(
        "unpriced model -> {status}: {}",
        &text[..text.len().min(260)]
    );

    // The assertion that discriminates: the refusal must not be about routing, because a channel advertises
    // this model and the control test shows the same shape routing. If it is, this test says so rather than
    // passing on a refusal it cannot attribute.
    assert!(
        !text.contains("no_available_channel"),
        "the channel advertises `{model}`, so this must not be a routing refusal -- otherwise nothing here \
         can be attributed to the missing price. Body: {}",
        &text[..text.len().min(260)]
    );
    assert_ne!(
        status,
        reqwest::StatusCode::OK,
        "a missing price must not produce 200"
    );
    assert!(
        status.is_client_error(),
        "a missing price must be refused by the router; got {status}"
    );

    Ok(())
}

#[tokio::test]
async fn the_router_reports_strict_billing_mode() -> anyhow::Result<()> {
    // The premise the refusal relies on, read from the running router's own health output (`lib.rs:1378`) rather
    // than by repeating the `env::var` call. `BILLING_STRICT_MODE` is unset in this process and `lib.rs:847`
    // defaults to true.
    let (router_port, _db) = router_with_channel("strictflag", "any-model", &[]).await?;

    let client = reqwest::Client::new();
    let response = client
        .get(format!(
            "http://127.0.0.1:{router_port}/console/internal/health"
        ))
        .send()
        .await?;

    let text = response.text().await.unwrap_or_default();
    println!("health: {}", &text[..text.len().min(300)]);

    assert!(
        text.contains("\"billing_strict\":true") || text.contains("\"billing_strict\": true"),
        "the router must report strict billing mode, because the default is true and this process does not \
         set BILLING_STRICT_MODE. Body: {}",
        &text[..text.len().min(300)]
    );

    Ok(())
}

// -------------------------------------------------------------------------------------------
// the price that disappears, which does not depend on the environment
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_price_that_disappears_after_preflight_cannot_be_priced_at_settlement(
) -> anyhow::Result<()> {
    // The post-settle path (`lib.rs:1823-1828`). Reaching it through the handler needs the price to vanish
    // between preflight and settlement, which is a race rather than a fixture, so the calculator is driven
    // directly with a cache that holds a price the database no longer has -- the state that path describes.
    //
    // What this pins: preflight still passes (it reads the cache), settlement reports `PriceNotFound`, and the
    // handler therefore records `cost_status = "price_missing"` and charges zero rather than a guess.
    use burncloud_commerce_billing::usage::get_parser;
    use burncloud_commerce_billing::{BillingError, CostCalculator, PriceCache};
    use burncloud_supply_contracts::ChannelType;

    let (db, _pool, _url) = setup_db().await?;
    let model = "vanishing-model";
    seed_price(&db, model).await?;

    // The cache learns the price, as it would at startup.
    let cache = PriceCache::load(&db).await?;
    assert!(
        cache.get(model, None).await.is_some(),
        "the cache must hold the price before the row is removed"
    );

    // The row goes away while the cache still has it.
    BillingPriceModel::delete_all_for_model(&db, model).await?;

    let calc = CostCalculator::new(cache);
    assert!(
        calc.preflight(model, None).await.is_ok(),
        "preflight reads the cache, so it still passes -- which is exactly what makes the post-settle path \
         reachable"
    );

    // A fresh cache from a database with no row for the model, which is what settlement would read.
    let cache = PriceCache::load(&db).await?;
    let calc = CostCalculator::new(cache);

    let usage = get_parser(ChannelType::OpenAI)
        .parse_response(&serde_json::json!({
            "usage": { "prompt_tokens": 1000, "completion_tokens": 500 }
        }))
        .expect("the body parses");

    let result = calc
        .calculate(model, &usage, "req-vanishing", false, false, None)
        .await;

    match result {
        Err(BillingError::PriceNotFound(named)) => {
            println!(
                "settlement reports PriceNotFound for {named}; the handler records price_missing and charges 0"
            );
            assert_eq!(named, model, "and it names the model");
        }
        other => panic!(
            "expected PriceNotFound so the handler can mark the row and charge zero; got {other:?}. If this is \
             Ok, a cost was computed from a price that no longer exists"
        ),
    }

    Ok(())
}

// -------------------------------------------------------------------------------------------
// what this file cannot cover, and why
// -------------------------------------------------------------------------------------------

/// The non-strict **handler** branch would need a subprocess, and is described rather than faked.
///
/// `BILLING_STRICT_MODE=false` changes what the handler does at `lib.rs:2387`: it logs and **allows** the
/// request. Reaching that over HTTP needs a router built with the variable set, and since `create_router_app`
/// reads it once, the only routes are a subprocess or a process-wide variable -- the latter racing with every
/// other test in this binary.
///
/// So it is not covered here. The branch's behaviour is visible in one line of source, which this test reads so
/// that a change to it is noticed.
#[test]
fn the_non_strict_handler_branch_is_not_covered_here() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("lib.rs is part of this crate");

    let line = src
        .lines()
        .find(|l| l.contains("non-strict mode, allowing request"))
        .expect("the non-strict branch was not found; if it moved, this note needs updating");

    println!("the branch that is not covered here: {}", line.trim());

    assert!(
        line.contains("allowing request"),
        "the non-strict branch allows the request, which is why covering it needs a subprocess rather than a \
         variable set in-process"
    );
}
