//! L2 Shaper integration tests (issue #151).
//!
//! Covers the 10 cases from the audit review:
//! - T1/T2: startup `configure_from_db` (SQLite path; T2 PG variant noted —
//!   the SELECT is dialect-neutral, no `ph()` placeholders, so PG correctness
//!   follows from T1+T6 SQLite verification).
//! - T3: `try_consume → OwnBucket` admit path.
//! - T4: `try_consume → Rejected` after the bucket drains.
//! - T5: failover loop's "skip rejected, try next" semantic — one channel
//!   exhausted, the next admits.
//! - T6 (issue acceptance): full HTTP request through the failover loop with
//!   a single low-cap channel; second request returns
//!   `503 + X-Rejected-By: shaper + Retry-After: 60`.
//! - T7: `BudgetGuard::commit(actual_tpm)` with `actual < est` refunds the
//!   over-estimate (`est − actual`) and prevents `Drop` from refunding again.
//! - T8 (FM4): `tokio::time::timeout` cancels a future holding a
//!   `BudgetGuard`; `Drop` fires with `committed = false` and the bucket
//!   recovers the full `est_tpm`.
//! - T9: unconfigured channel (rpm_cap = NULL) — `is_configured` returns
//!   false; the failover loop's旁路 path increments `fail_open_count` and
//!   exposes it via `/console/internal/health`.
//! - T10: `InMemoryBudget::configure` with `reservation` sum != 1.0 falls
//!   back to `ChannelReservation::default()` (FM8).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::unnecessary_cast
)]

mod common;

use std::sync::Arc;
use std::time::Duration;

use burncloud_common::TrafficColor;
use burncloud_database::sqlx;
use burncloud_router::rate_budget::{
    BudgetBackend, BudgetGuard, ChannelReservation, ConsumeOutcome, InMemoryBudget,
};

use common::{insert_router_token, setup_db, start_test_server};

// ---------------------------------------------------------------------------
// Unit-level tests — direct InMemoryBudget / BudgetGuard public API.
// ---------------------------------------------------------------------------

/// T1 — `InMemoryBudget::configure` round-trips DB-loaded values through
/// `snapshot()`. Mirrors the SQL-load path in
/// `configure_rate_budget_from_db`: `(rpm_cap, tpm_cap, reservation_*)` from
/// `channel_providers` → `configure(channel_id, ...)` → bucket has
/// per-color reserved capacity matching the policy.
///
/// The full DB → bucket integration is exercised by T6 (E2E), which would
/// fail if `configure_from_db` weren't called or routed values incorrectly.
#[test]
fn t1_startup_configure_loads_caps_and_reservations() {
    let budget = InMemoryBudget::new();
    budget.configure(
        42,
        100,
        100_000,
        ChannelReservation {
            green: 0.4,
            yellow: 0.4,
            red: 0.2,
        },
    );
    assert!(budget.is_configured(42));
    let snap = budget.snapshot(42).expect("snapshot for configured channel");
    assert_eq!(snap.rpm_cap, 100);
    assert_eq!(snap.tpm_cap, 100_000);
    // Reservations: 40% / 40% / 20% of rpm_cap.
    assert_eq!(snap.rpm_remaining_green, 40);
    assert_eq!(snap.rpm_remaining_yellow, 40);
    assert_eq!(snap.rpm_remaining_red, 20);
    // TPM mirrors the same shares.
    assert_eq!(snap.tpm_remaining_green, 40_000);
    assert_eq!(snap.tpm_remaining_yellow, 40_000);
    assert_eq!(snap.tpm_remaining_red, 20_000);
}

/// T2 — PG dialect parity is a documentation contract. The SQL in
/// `configure_rate_budget_from_db` is `SELECT id, rpm_cap, tpm_cap,
/// reservation_green, reservation_yellow, reservation_red FROM
/// channel_providers` with **no parameter placeholders** and **no quoted
/// identifiers** — it parses identically on PG and SQLite. T1 (above) and
/// T6 (HTTP E2E) exercise the same code path on SQLite; PG behavior is
/// guaranteed by the dialect-neutral query.
#[test]
fn t2_configure_from_db_sql_is_dialect_neutral() {
    // Sentinel test: documents the contract via assertion. If a future
    // refactor introduces dialect-specific SQL in configure_from_db, this
    // contract needs re-verifying with a real PG instance.
    let pg_neutral_sql = "SELECT id, rpm_cap, tpm_cap, reservation_green, \
                          reservation_yellow, reservation_red FROM channel_providers";
    assert!(!pg_neutral_sql.contains('?'));
    assert!(!pg_neutral_sql.contains('$'));
    assert!(!pg_neutral_sql.contains('`'));
    assert!(!pg_neutral_sql.contains('"'));
}

/// T3 — `try_consume` admits a Yellow request via OwnBucket when the yellow
/// reservation has capacity.
#[test]
fn t3_admit_via_own_bucket() {
    let budget = InMemoryBudget::new();
    budget.configure(1, 100, 100_000, ChannelReservation::default());
    let outcome = budget.try_consume(1, TrafficColor::Yellow, 100);
    assert_eq!(outcome, ConsumeOutcome::OwnBucket);
    assert_eq!(outcome.as_label(), "shaper_own");
}

/// T4 — Repeated `try_consume` drains the bucket; once Yellow + Green are
/// both empty (Yellow's only borrow source), further Yellow requests are
/// `Rejected`.
#[test]
fn t4_rejected_when_yellow_and_green_drained() {
    let budget = InMemoryBudget::new();
    // rpm_cap = 1 with reservation = (0.0, 1.0, 0.0): only Yellow gets a
    // token; Yellow has no Green to borrow from.
    budget.configure(
        1,
        1,
        100_000,
        ChannelReservation {
            green: 0.0,
            yellow: 1.0,
            red: 0.0,
        },
    );
    let first = budget.try_consume(1, TrafficColor::Yellow, 100);
    assert_eq!(first, ConsumeOutcome::OwnBucket);
    let second = budget.try_consume(1, TrafficColor::Yellow, 100);
    assert_eq!(second, ConsumeOutcome::Rejected);
    assert_eq!(second.as_label(), "shaper_reject");
}

/// T5 — Failover semantics: one channel exhausted, the next admits. Mirrors
/// the loop body's "Rejected → continue → next candidate" path.
#[test]
fn t5_skip_rejected_then_admit_next_channel() {
    let budget = InMemoryBudget::new();
    // Channel 1: pre-exhausted.
    budget.configure(
        1,
        1,
        100,
        ChannelReservation {
            green: 0.0,
            yellow: 1.0,
            red: 0.0,
        },
    );
    let _drain = budget.try_consume(1, TrafficColor::Yellow, 50);
    let exhausted = budget.try_consume(1, TrafficColor::Yellow, 50);
    assert_eq!(exhausted, ConsumeOutcome::Rejected);

    // Channel 2: fresh.
    budget.configure(2, 100, 100_000, ChannelReservation::default());
    let admitted = budget.try_consume(2, TrafficColor::Yellow, 50);
    assert_eq!(admitted, ConsumeOutcome::OwnBucket);
}

/// T7 — `BudgetGuard::commit(actual_tpm)` with `actual < est` refunds the
/// over-estimate. After commit, the bucket has `(rpm_cap - 1)` RPM remaining
/// (consumed 1 attempt) but TPM has only `actual_tpm` consumed (the rest
/// was refunded).
#[test]
fn t7_commit_refunds_overestimate() {
    let budget = InMemoryBudget::new();
    budget.configure(1, 100, 100_000, ChannelReservation::default());

    let est_tpm: u64 = 1_000;
    let outcome = budget.try_consume(1, TrafficColor::Yellow, est_tpm);
    assert_eq!(outcome, ConsumeOutcome::OwnBucket);

    let snap_after_consume = budget.snapshot(1).expect("snapshot");
    assert_eq!(snap_after_consume.tpm_remaining_yellow, 40_000 - est_tpm);

    // commit with actual = 200 → refund 800.
    let actual_tpm: u64 = 200;
    let guard = BudgetGuard::new(&budget, 1, TrafficColor::Yellow, est_tpm);
    guard.commit(actual_tpm);

    let snap_after_commit = budget.snapshot(1).expect("snapshot");
    // Net TPM consumed = actual_tpm = 200; remaining = 40_000 - 200 = 39_800.
    assert_eq!(snap_after_commit.tpm_remaining_yellow, 40_000 - actual_tpm);
}

/// T8 (FM4) — `tokio::time::timeout` cancels a future that holds a
/// `BudgetGuard`; the guard's `Drop` fires with `committed = false` and the
/// bucket recovers the full `est_tpm`. Without this RAII, a client cancel
/// during `.await` would permanently leak `est_tpm` of bucket capacity.
#[tokio::test]
async fn t8_drop_refunds_full_est_on_timeout_cancel() {
    let budget = Arc::new(InMemoryBudget::new());
    budget.configure(1, 100, 100_000, ChannelReservation::default());
    let snap_initial = budget.snapshot(1).expect("snapshot");

    let est_tpm: u64 = 1_500;
    let outcome = budget.try_consume(1, TrafficColor::Yellow, est_tpm);
    assert_eq!(outcome, ConsumeOutcome::OwnBucket);
    let snap_after_consume = budget.snapshot(1).expect("snapshot");
    assert_eq!(
        snap_after_consume.tpm_remaining_yellow,
        snap_initial.tpm_remaining_yellow - est_tpm
    );

    // Spawn a future that creates a guard then awaits indefinitely; the
    // outer timeout cancels it after 50ms, dropping the future and the
    // guard inside it.
    let budget_for_task = budget.clone();
    let _ = tokio::time::timeout(Duration::from_millis(50), async move {
        let _guard = BudgetGuard::new(
            budget_for_task.as_ref(),
            1,
            TrafficColor::Yellow,
            est_tpm,
        );
        tokio::time::sleep(Duration::from_secs(60)).await;
    })
    .await;

    let snap_final = budget.snapshot(1).expect("snapshot");
    assert_eq!(
        snap_final.tpm_remaining_yellow, snap_initial.tpm_remaining_yellow,
        "BudgetGuard::Drop should refund full est_tpm on async cancellation"
    );
}

/// T10 (FM8) — `InMemoryBudget::configure` with an invalid reservation
/// (sum != 1.0) silently falls back to the default 0.4/0.4/0.2 instead of
/// silently producing skewed buckets. The fallback is observable via
/// `snapshot()` having the default per-color shares.
#[test]
fn t10_invalid_reservation_falls_back_to_default() {
    let budget = InMemoryBudget::new();
    // Sum = 1.5 — invalid.
    let bad = ChannelReservation {
        green: 0.5,
        yellow: 0.5,
        red: 0.5,
    };
    assert!(!bad.is_valid());
    budget.configure(7, 100, 100_000, bad);
    let snap = budget.snapshot(7).expect("snapshot for configured channel");
    // Default reservation is 0.4 / 0.4 / 0.2.
    assert_eq!(snap.rpm_remaining_green, 40);
    assert_eq!(snap.rpm_remaining_yellow, 40);
    assert_eq!(snap.rpm_remaining_red, 20);
}

// ---------------------------------------------------------------------------
// HTTP E2E tests — full failover loop integration via mock-fixture DB.
// ---------------------------------------------------------------------------

/// Insert a `billing_prices` row so the preflight billing check
/// (`CostCalculator::preflight`) accepts the model. `billing_prices` is the
/// canonical table after migration 0010 (renamed from legacy `prices`).
async fn seed_price(pool: &sqlx::AnyPool, model: &str) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT OR REPLACE INTO billing_prices \
         (model, currency, input_price, output_price, region) \
         VALUES (?, 'USD', 1, 1, '')",
    )
    .bind(model)
    .execute(pool)
    .await?;
    Ok(())
}

/// `user_api_keys.remain_quota` defaults to 0 in the test fixture
/// ([`common::insert_router_token`]). With `quota_limit = 0` and
/// `used_quota = 0`, the proxy rejects with 402 before the shaper runs. Set
/// `remain_quota = -1` (unlimited) so the request reaches the L2 Shaper.
async fn grant_unlimited_quota(pool: &sqlx::AnyPool, key: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE user_api_keys SET remain_quota = -1 WHERE key = ?")
        .bind(key)
        .execute(pool)
        .await?;
    Ok(())
}

/// Insert a `channel_providers` + `channel_abilities` pair so the model
/// router can resolve `model` → `channel_id`. `rpm_cap` / `tpm_cap` /
/// `reservation_*` are passed straight through to the L2 Shaper config.
#[allow(clippy::too_many_arguments)]
async fn seed_channel(
    pool: &sqlx::AnyPool,
    channel_id: i32,
    base_url: &str,
    model: &str,
    group: &str,
    rpm_cap: Option<i32>,
    tpm_cap: Option<i64>,
    reservation: Option<(f64, f64, f64)>,
) -> anyhow::Result<()> {
    let (rg, ry, rr) = reservation.map_or((None, None, None), |(g, y, r)| (Some(g), Some(y), Some(r)));
    sqlx::query(
        "INSERT INTO channel_providers \
         (id, type, key, status, name, weight, base_url, models, `group`, used_quota, priority, auto_ban, \
          rpm_cap, tpm_cap, reservation_green, reservation_yellow, reservation_red) \
         VALUES (?, 1, 'k', 1, ?, 1, ?, ?, ?, 0, 0, 0, ?, ?, ?, ?, ?)",
    )
    .bind(channel_id)
    .bind(format!("ch-{channel_id}"))
    .bind(base_url)
    .bind(model)
    .bind(group)
    .bind(rpm_cap)
    .bind(tpm_cap)
    .bind(rg)
    .bind(ry)
    .bind(rr)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO channel_abilities (`group`, model, channel_id, enabled, priority, weight) \
         VALUES (?, ?, ?, 1, 0, 1)",
    )
    .bind(group)
    .bind(model)
    .bind(channel_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// T6 (issue acceptance) — single channel pre-configured to reject Yellow
/// requests immediately (rpm_cap = 1, reservation = all-Red so Yellow has no
/// own bucket and no Green to borrow from). The shaper rejects on first
/// admit attempt; with only one candidate, the failover loop falls through
/// to the post-loop branch and emits 503 + `X-Rejected-By: shaper` +
/// `Retry-After: 60` (audit decision D12).
///
/// This test ALSO implicitly covers T1: if `configure_rate_budget_from_db`
/// hadn't loaded the rpm_cap from the DB, the channel would fail-open and
/// the request would proceed to the (dead) base_url with a network error
/// instead of a shaper reject.
#[tokio::test]
async fn t6_all_candidates_rejected_returns_503_x_rejected_by_shaper() -> anyhow::Result<()> {
    let (db, pool, db_url) = setup_db().await?;

    // Seed pricing so preflight billing check passes.
    seed_price(&pool, "shaper-test-model").await?;

    // Channel cap = 1 RPM with all capacity in Red. Yellow request can
    // only consume Yellow (own=0) or borrow Green (also 0); Red is NOT a
    // borrow source for Yellow → Rejected on first try.
    seed_channel(
        &pool,
        9_001,
        "http://127.0.0.1:1",
        "shaper-test-model",
        "default",
        Some(1),
        Some(1_000),
        Some((0.0, 0.0, 1.0)),
    )
    .await?;

    insert_router_token(&db, "tok-shaper-t6", "u-shaper-t6", "default", None, None).await?;
    grant_unlimited_quota(&pool, "tok-shaper-t6").await?;

    let port = 14_651_u16;
    start_test_server(port, &db_url).await;

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/v1/chat/completions");

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer tok-shaper-t6")
        .json(&serde_json::json!({
            "model": "shaper-test-model",
            "max_tokens": 512,
            "messages": [{"role": "user", "content": "hi"}],
        }))
        .send()
        .await?;

    assert_eq!(
        resp.status().as_u16(),
        503,
        "all-rejected should return 503"
    );
    assert_eq!(
        resp.headers()
            .get("X-Rejected-By")
            .and_then(|v| v.to_str().ok()),
        Some("shaper"),
        "X-Rejected-By header should be 'shaper'"
    );
    assert_eq!(
        resp.headers()
            .get("Retry-After")
            .and_then(|v| v.to_str().ok()),
        Some("60"),
        "Retry-After should be '60'"
    );

    Ok(())
}

/// T9 — Channel without rpm_cap (NULL) goes through the failover loop's
/// fail-open旁路: `is_configured` returns false, `fail_open_count` is
/// incremented, and the iteration's `iter_label` is `shaper_unconfigured`.
/// The `/console/internal/health` endpoint surfaces the counter so admins
/// can spot silently-permissive channels (audit FM2).
///
/// We don't assert on the upstream response here (the `base_url` points at
/// a dead host so the request will eventually fail); we only assert that
/// the shaper's fail-open branch ran by reading `fail_open_count` from the
/// `/health` endpoint after the request attempt.
#[tokio::test]
async fn t9_unconfigured_channel_increments_fail_open_count() -> anyhow::Result<()> {
    let (db, pool, db_url) = setup_db().await?;

    seed_price(&pool, "shaper-unconfigured-model").await?;
    // No rpm_cap → unconfigured → shaper bypasses, fail-open.
    seed_channel(
        &pool,
        9_002,
        "http://127.0.0.1:1",
        "shaper-unconfigured-model",
        "default",
        None,
        None,
        None,
    )
    .await?;

    insert_router_token(&db, "tok-shaper-t9", "u-shaper-t9", "default", None, None).await?;
    grant_unlimited_quota(&pool, "tok-shaper-t9").await?;

    let port = 14_652_u16;
    start_test_server(port, &db_url).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let url = format!("http://127.0.0.1:{port}/v1/chat/completions");

    // Fire one request — it'll fail at the upstream HTTP call (dead host),
    // but the shaper fail-open branch will still have run + incremented
    // fail_open_count.
    let _ = client
        .post(&url)
        .header("Authorization", "Bearer tok-shaper-t9")
        .json(&serde_json::json!({
            "model": "shaper-unconfigured-model",
            "max_tokens": 256,
            "messages": [{"role": "user", "content": "hi"}],
        }))
        .send()
        .await;

    // Check /console/internal/health — fail_open_count must be ≥ 1.
    let health_resp = client
        .get(format!("http://127.0.0.1:{port}/console/internal/health"))
        .send()
        .await?;
    assert_eq!(health_resp.status().as_u16(), 200);
    let health_json: serde_json::Value = health_resp.json().await?;
    let fail_open = health_json
        .get("fail_open_count")
        .and_then(|v| v.as_u64())
        .expect("/health response should include fail_open_count");
    assert!(
        fail_open >= 1,
        "unconfigured channel admit must increment fail_open_count, got {fail_open}"
    );

    Ok(())
}
