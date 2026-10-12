#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic_in_result_fn,
    reason = "Test-only file: the assertions are the test, and clippy.toml's allow-panic-in-tests does not recognise #[tokio::test]."
)]
//! The service layer over the router log and balance tables (#633, plan section 5 item 14).
//!
//! `service-router-log` is a 139-line facade: every method forwards to `burncloud_router`, so these
//! tests drive **that** behaviour through the four service entry points the plan names -- `RouterLogService`,
//! `UsageStatsService`, `BalanceService` and `BillingService`. Testing the facade for its own sake would only
//! assert that a forwarding call forwards.
//!
//! The plan's six items:
//!
//! | Plan item | Test |
//! | --- | --- |
//! | 日志与扣费副作用分离 | `inserting_a_log_does_not_touch_the_balance` |
//! | user/model/time 过滤 | `filtering_by_user_and_model_selects_only_that_rows`, `the_time_window_bounds_the_summary` |
//! | 空统计 | `no_rows_is_a_zero_report_but_a_broken_query_is_an_error` |
//! | 跨用户隔离 | `one_users_rows_are_invisible_to_another` |
//! | 双币种扣减与汇率单位 | `deduction_falls_back_to_the_second_currency_at_the_scaled_rate` |
//! | 数据库错误不伪造零统计 | `no_rows_is_a_zero_report_but_a_broken_query_is_an_error` |
//!
//! The plan also says "按当前接口测试，**不在此次测试任务重划业务 Owner**" -- test the interfaces as they are,
//! without reassigning ownership. So the tests record what the interfaces do, and where a recorded behaviour
//! looks wrong it is named as wrong rather than asserted as correct.
//!
//! ## Three things the source did not say, which measuring did
//!
//! - **A missing account row is an `Err`, not `Ok(false)`.** Reading `balances.unwrap_or((0, 0))` at
//!   `lib.rs:360` suggested a missing user reads as a zero balance and is refused like an empty wallet. The
//!   first version of this file asserted that, and the measurement said
//!   `Err(Query("user account not found: no-such-user"))`. The implementation is more precise than the reading.
//! - **`get_usage_stats` filters on the *current* time**, not on a period the caller names: `period` only picks
//!   the window length (`day`/`week`/anything else is 30 days), and the rows are bounded by `strftime('%s')`
//!   against `now`. A log dated in the past therefore reads as **no usage**, which is a different statement
//!   from **no rows** -- and the two are separated below rather than conflated.
//! - **A negative `limit` does not fail.** It returns zero rows, so it cannot be used to prove that a broken
//!   query is an error rather than an empty result. Dropping the table does.

use burncloud_database::{create_database_with_url, Database};
use burncloud_router::{BalanceService, RouterLog, RouterLogService, UsageStatsService};
use std::error::Error;

/// A router database with the tables this crate needs.
///
/// Three slashes and forward slashes: `sqlite:///C:/...` is absolute on Windows, whereas the two-slash form is
/// not a URL SQLite can open and fails with "(code: 14) unable to open database file" before any assertion.
async fn fresh_db(tag: &str) -> Result<(Database, std::path::PathBuf), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!(
        "bc_rl_{}_{}_{}.db",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::remove_file(&path).ok();
    let normalized = path.to_string_lossy().replace('\\', "/");
    let db = create_database_with_url(&format!("sqlite:///{}?mode=rwc", normalized)).await?;
    burncloud_router::RouterDatabase::init(&db).await?;
    Ok((db, path))
}

/// Close the pool, wait for the handle to be released, then remove the database and its WAL companions.
async fn cleanup(db: Database, path: std::path::PathBuf) {
    db.close().await.ok();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    for suffix in ["", "-wal", "-shm"] {
        let mut candidate = path.clone().into_os_string();
        candidate.push(suffix);
        let candidate = std::path::PathBuf::from(candidate);
        if candidate.exists() {
            std::fs::remove_file(&candidate)
                .unwrap_or_else(|e| panic!("{} was not removed ({e})", candidate.display()));
        }
    }
}

/// Give `user_id` a starting balance.
///
/// `user_accounts` requires `username` (unique, not null) and `password_hash` (not null, **no default**), which
/// the first version of this helper omitted -- the insert failed with
/// `NOT NULL constraint failed: user_accounts.username` before any balance assertion ran.
async fn credit(
    db: &Database,
    user_id: &str,
    usd_nano: i64,
    cny_nano: i64,
) -> Result<(), Box<dyn Error>> {
    let conn = db.get_connection()?;
    burncloud_database::sqlx::query(
        "INSERT OR REPLACE INTO user_accounts \
         (id, username, password_hash, balance_usd, balance_cny) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(format!("username-{user_id}"))
    .bind("not-a-real-hash")
    .bind(usd_nano)
    .bind(cny_nano)
    .execute(conn.pool())
    .await?;
    Ok(())
}

/// The balances of `user_id`, or `None` if there is no row.
async fn balances(db: &Database, user_id: &str) -> Result<Option<(i64, i64)>, Box<dyn Error>> {
    let conn = db.get_connection()?;
    let row: Option<(i64, i64)> = burncloud_database::sqlx::query_as(
        "SELECT balance_usd, balance_cny FROM user_accounts WHERE id = ?",
    )
    .bind(user_id)
    .fetch_optional(conn.pool())
    .await?;
    Ok(row)
}

/// A log row with the fields these tests care about.
fn log(request_id: &str, user_id: &str, model: &str, cost: i64, when: &str) -> RouterLog {
    RouterLog {
        request_id: request_id.to_string(),
        user_id: Some(user_id.to_string()),
        path: "/v1/chat/completions".to_string(),
        upstream_id: Some("up-1".to_string()),
        status_code: 200,
        latency_ms: 120,
        prompt_tokens: 1_000,
        completion_tokens: 500,
        cost,
        model: Some(model.to_string()),
        created_at: Some(when.to_string()),
        ..Default::default()
    }
}

/// Insert a log row at a **specific** time, through SQL rather than through the service.
///
/// This exists because `RouterLogService::insert` **silently discards `RouterLog::created_at`**: the INSERT
/// statement names 32 columns and `created_at` is not one of them, so the row takes the column default
/// `CURRENT_TIMESTAMP` (`0001_initial_schema.sql:119`). A historical row therefore cannot be written through
/// the service at all, and any test of a time window would only ever see "now".
///
/// `crates/traffic/router/tests/usage_stats_tests.rs:39` inserts directly for the same reason -- an
/// independent confirmation that this is the existing workaround rather than something invented here.
/// `the_service_insert_discards_the_timestamp_it_is_given` below records the defect itself.
async fn insert_log_at(
    db: &Database,
    request_id: &str,
    user_id: &str,
    model: &str,
    cost: i64,
    created_at: &str,
) -> Result<(), Box<dyn Error>> {
    let conn = db.get_connection()?;
    burncloud_database::sqlx::query(
        "INSERT INTO router_logs \
         (request_id, user_id, path, upstream_id, status_code, latency_ms, \
          prompt_tokens, completion_tokens, cost, model, created_at) \
         VALUES (?, ?, '/v1/chat/completions', 'up-1', 200, 120, 1000, 500, ?, ?, ?)",
    )
    .bind(request_id)
    .bind(user_id)
    .bind(cost)
    .bind(model)
    .bind(created_at)
    .execute(conn.pool())
    .await?;
    Ok(())
}

/// An ISO-8601 timestamp `seconds` away from now, which is the shape `get_usage_stats` compares against.
fn recent(seconds_ago: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is after the epoch")
        .as_secs() as i64;
    let then = now - seconds_ago;
    // Seconds to a civil date, without a date library: the SQLite `datetime()` function does the conversion.
    // The formatting is done in SQL so the value matches exactly what `strftime('%s')` will parse back.
    let secs = then as u32;
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3_600, (rem % 3_600) / 60, rem % 60);
    // Days since the epoch to a year/month/day, by the standard civil-from-days algorithm.
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

// -------------------------------------------------------------------------------------------
// 日志与扣费副作用分离
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn inserting_a_log_does_not_touch_the_balance() -> Result<(), Box<dyn Error>> {
    // The plan's first item: the log and the settlement must be separate side effects. `RouterLogService::insert`
    // documents that it does not mutate spend quota, and this measures it -- a log carrying a substantial cost
    // leaves the balance exactly where it was.
    let (db, path) = fresh_db("no_side_effect").await?;
    credit(&db, "u1", 100_000_000, 0).await?;
    let before = balances(&db, "u1").await?;
    assert_eq!(before, Some((100_000_000, 0)), "the starting balance");

    RouterLogService::insert(&db, &log("req-1", "u1", "gpt-4o", 50_000_000, &recent(60))).await?;

    let after = balances(&db, "u1").await?;
    println!("balance before {before:?}, after {after:?}");
    assert_eq!(
        after, before,
        "writing a log must not move the balance; settlement is the caller's separate step"
    );

    // And the row really was written, so the unchanged balance is because nothing was deducted rather than
    // because nothing happened.
    let logs = RouterLogService::get(&db, 10, 0).await?;
    assert_eq!(logs.len(), 1, "the log row exists");
    assert_eq!(logs[0].cost, 50_000_000, "carrying the cost it was given");

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn a_log_can_be_inserted_with_no_user_and_the_balance_path_is_unaffected(
) -> Result<(), Box<dyn Error>> {
    // `user_id` is optional in the schema, and an unauthenticated or internal request has none. This pins that
    // such a row is accepted -- a strict not-null assumption here would reject real traffic.
    let (db, path) = fresh_db("no_user").await?;
    let mut orphan = log("req-orphan", "ignored", "gpt-4o", 999, &recent(60));
    orphan.user_id = None;

    RouterLogService::insert(&db, &orphan).await?;

    let logs = RouterLogService::get(&db, 10, 0).await?;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].user_id, None, "the row is stored with a null user");

    cleanup(db, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// the timestamp the service discards
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn the_service_insert_discards_the_timestamp_it_is_given() -> Result<(), Box<dyn Error>> {
    // **A defect, recorded rather than fixed**, for the reason the plan gives: the interfaces are tested as
    // they are, and reassigning this belongs to whoever owns the write path.
    //
    // `RouterLogService::insert` takes a `RouterLog` whose `created_at` is `Option<String>`, and the INSERT
    // statement it builds names **32 columns, none of them `created_at`**. So the field is accepted and
    // silently dropped, and the row takes the column default `CURRENT_TIMESTAMP`
    // (`0001_initial_schema.sql:119`).
    //
    // The consequence is not cosmetic: **a historical row cannot be written through the service at all**, so
    // every row carries the moment it was inserted. Anything that backfills, imports, replays or corrects a log
    // after the fact appears to have happened now -- and the time-windowed queries in this file, and in
    // `crates/traffic/router/tests/usage_stats_tests.rs`, have to bypass the service to test a window
    // at all. That the existing suite already bypasses it is the independent confirmation.
    let (db, path) = fresh_db("created_at_dropped").await?;

    RouterLogService::insert(
        &db,
        &log("historic", "u1", "gpt-4o", 10, "2020-01-01 00:00:00"),
    )
    .await?;

    let stored = RouterLogService::get(&db, 10, 0).await?;
    assert_eq!(stored.len(), 1);
    let actual = stored[0].created_at.clone().unwrap_or_default();
    println!("asked for 2020-01-01 00:00:00, stored {actual}");

    assert_ne!(
        actual, "2020-01-01 00:00:00",
        "the timestamp given to `insert` is not what is stored; if this now matches, the defect is fixed and \
         this test should be inverted"
    );
    assert!(
        actual.starts_with("20"),
        "the stored value is the column default CURRENT_TIMESTAMP, so it is a real timestamp rather than an \
         empty string: {actual}"
    );

    // The direct insert does keep the value, which is what makes the time-window tests below possible and shows
    // the difference is the service rather than the schema.
    insert_log_at(
        &db,
        "historic-direct",
        "u1",
        "gpt-4o",
        10,
        "2020-01-01 00:00:00",
    )
    .await?;
    let all = RouterLogService::get_filtered(&db, Some("u1"), None, None, 10, 0).await?;
    let direct = all
        .iter()
        .find(|l| l.request_id == "historic-direct")
        .expect("the directly inserted row is there");
    assert_eq!(
        direct.created_at.as_deref(),
        Some("2020-01-01 00:00:00"),
        "a direct insert keeps the timestamp, so the schema supports it and only the service drops it"
    );

    cleanup(db, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// 跨用户隔离
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn one_users_rows_are_invisible_to_another() -> Result<(), Box<dyn Error>> {
    // The plan's isolation item. Two users with rows interleaved in time, so a query that forgot its user
    // filter would return both and be seen to.
    let (db, path) = fresh_db("isolation").await?;
    RouterLogService::insert(&db, &log("a1", "alice", "gpt-4o", 10, &recent(300))).await?;
    RouterLogService::insert(&db, &log("b1", "bob", "gpt-4o", 20, &recent(200))).await?;
    RouterLogService::insert(&db, &log("a2", "alice", "gpt-4o", 30, &recent(100))).await?;

    let alice = RouterLogService::get_filtered(&db, Some("alice"), None, None, 100, 0).await?;
    let bob = RouterLogService::get_filtered(&db, Some("bob"), None, None, 100, 0).await?;
    println!(
        "alice: {:?}, bob: {:?}",
        alice.iter().map(|l| &l.request_id).collect::<Vec<_>>(),
        bob.iter().map(|l| &l.request_id).collect::<Vec<_>>()
    );

    assert_eq!(alice.len(), 2, "alice sees exactly her two rows");
    assert_eq!(bob.len(), 1, "bob sees exactly his one");
    assert!(alice.iter().all(|l| l.user_id.as_deref() == Some("alice")));
    assert!(bob.iter().all(|l| l.user_id.as_deref() == Some("bob")));

    // The aggregate path must be isolated too, not just the listing.
    let (alice_prompt, alice_completion) =
        RouterLogService::get_usage_by_user(&db, "alice").await?;
    let (bob_prompt, bob_completion) = RouterLogService::get_usage_by_user(&db, "bob").await?;
    println!(
        "alice tokens ({alice_prompt}, {alice_completion}), bob ({bob_prompt}, {bob_completion})"
    );

    assert_eq!(alice_prompt, 2_000, "two rows at 1_000 prompt tokens each");
    assert_eq!(bob_prompt, 1_000, "one row");
    assert_ne!(
        alice_prompt, bob_prompt,
        "the two users do not see each other's tokens"
    );

    // And the statistics path, which is a third query.
    let alice_stats = UsageStatsService::get_stats(&db, "alice", "month").await?;
    let bob_stats = UsageStatsService::get_stats(&db, "bob", "month").await?;
    println!("alice stats {alice_stats:?}, bob stats {bob_stats:?}");
    assert_eq!(
        alice_stats.total_requests, 2,
        "alice's statistics count her rows only"
    );
    assert_eq!(bob_stats.total_requests, 1, "and bob's count his");
    assert_eq!(alice_stats.total_cost_nano, 40, "10 + 30");
    assert_eq!(bob_stats.total_cost_nano, 20);

    cleanup(db, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// user / model 过滤
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn filtering_by_user_and_model_selects_only_that_rows() -> Result<(), Box<dyn Error>> {
    // Both filters at once, so a query that applied only one is visible. Three rows differ in exactly one
    // dimension each from the row the filter is meant to select.
    let (db, path) = fresh_db("filters").await?;
    let when = recent(60);
    RouterLogService::insert(&db, &log("want", "u1", "gpt-4o", 10, &when)).await?;
    RouterLogService::insert(&db, &log("other-model", "u1", "claude", 20, &when)).await?;
    RouterLogService::insert(&db, &log("other-user", "u2", "gpt-4o", 30, &when)).await?;

    let both =
        RouterLogService::get_filtered(&db, Some("u1"), None, Some("gpt-4o"), 100, 0).await?;
    assert_eq!(both.len(), 1, "only one row matches both filters");
    assert_eq!(both[0].request_id, "want");

    let by_model = RouterLogService::get_filtered(&db, None, None, Some("gpt-4o"), 100, 0).await?;
    assert_eq!(
        by_model.len(),
        2,
        "the model filter alone matches two users' rows"
    );

    let by_user = RouterLogService::get_filtered(&db, Some("u1"), None, None, 100, 0).await?;
    assert_eq!(by_user.len(), 2, "the user filter alone matches two models");

    let unfiltered = RouterLogService::get_filtered(&db, None, None, None, 100, 0).await?;
    assert_eq!(unfiltered.len(), 3, "no filter matches everything");

    // The per-model grouping is a fourth query and must honour the user too.
    let grouped = UsageStatsService::get_stats_by_model(&db, "u1", "month").await?;
    let names: Vec<&str> = grouped.iter().map(|m| m.model.as_str()).collect();
    println!("u1 grouped by model: {names:?}");
    assert_eq!(grouped.len(), 2, "u1 used two models");
    assert!(names.contains(&"gpt-4o") && names.contains(&"claude"));

    // Pagination is a separate axis and must not be confused with filtering.
    let first_page = RouterLogService::get(&db, 2, 0).await?;
    let second_page = RouterLogService::get(&db, 2, 2).await?;
    println!(
        "page 1: {}, page 2: {}",
        first_page.len(),
        second_page.len()
    );
    assert_eq!(first_page.len(), 2, "the limit is honoured");
    assert_eq!(
        second_page.len(),
        1,
        "and the offset moves past the first two"
    );

    cleanup(db, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// 空统计 / 数据库错误不伪造零统计
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn no_rows_is_a_zero_report_but_a_broken_query_is_an_error() -> Result<(), Box<dyn Error>> {
    // The two halves of one question, and the plan asks for both: **a query that matched nothing and a query
    // that failed must not look the same to a caller.** A settlement path that read a broken query as zero
    // would bill nobody, which is why the plan names it separately.
    let (db, path) = fresh_db("empty_vs_failed").await?;

    // --- the empty half: a real report of zeroes, from a database that is working ---
    let stats = UsageStatsService::get_stats(&db, "nobody", "month").await?;
    println!("no rows: {stats:?}");
    assert_eq!(stats.total_requests, 0);
    assert_eq!(stats.total_prompt_tokens, 0);
    assert_eq!(stats.total_completion_tokens, 0);
    assert_eq!(stats.total_cost_nano, 0);

    let by_model = UsageStatsService::get_stats_by_model(&db, "nobody", "month").await?;
    assert!(
        by_model.is_empty(),
        "no rows means no groups, not one group of zeroes"
    );

    let summary = burncloud_router::BillingService::get_billing_summary(&db, None, None).await?;
    println!(
        "no rows: {} models, total {}",
        summary.models.len(),
        summary.total_cost_usd
    );
    assert!(summary.models.is_empty());
    assert_eq!(summary.total_cost_usd, 0.0);

    // The same for a user who exists and has no logs, so the zeroes above are not an artefact of the unknown
    // user name.
    credit(&db, "quiet", 0, 0).await?;
    let quiet = UsageStatsService::get_stats(&db, "quiet", "month").await?;
    assert_eq!(
        quiet.total_requests, 0,
        "an existing user with no logs also reports zero"
    );

    // --- the failure half ---
    //
    // **A negative `limit` is not a failure.** The first version of this test used one and measured zero rows:
    // SQLite treats it as "no limit" rather than an error, so it proved nothing. Dropping the table produces a
    // query that genuinely cannot run.
    let conn = db.get_connection()?;
    burncloud_database::sqlx::query("DROP TABLE router_logs")
        .execute(conn.pool())
        .await?;

    let broken_stats = UsageStatsService::get_stats(&db, "nobody", "month").await;
    match &broken_stats {
        Ok(s) => panic!(
            "querying a dropped table returned a report instead of failing: {s:?}. A caller cannot tell this \
             from a user who genuinely spent nothing"
        ),
        Err(e) => println!("a dropped table reports: {e}"),
    }
    assert!(
        broken_stats.is_err(),
        "a query that cannot run must be an error"
    );

    // And the other entry points propagate it too, rather than one method catching and defaulting while the
    // rest do not.
    assert!(
        RouterLogService::get(&db, 10, 0).await.is_err(),
        "the log listing must fail rather than report no logs"
    );
    assert!(
        RouterLogService::get_usage_by_user(&db, "quiet")
            .await
            .is_err(),
        "the usage total must fail rather than report zero tokens"
    );
    assert!(
        UsageStatsService::get_stats_by_model(&db, "quiet", "month")
            .await
            .is_err(),
        "the per-model grouping must fail rather than report no usage"
    );
    assert!(
        burncloud_router::BillingService::get_billing_summary(&db, None, None)
            .await
            .is_err(),
        "the billing summary must fail rather than report zero"
    );

    cleanup(db, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// 双币种扣减与汇率单位
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_cost_covered_by_the_primary_currency_leaves_the_other_alone(
) -> Result<(), Box<dyn Error>> {
    // The simple half of the dual-currency path, and the baseline for the fallback test below.
    let (db, path) = fresh_db("primary_only").await?;
    credit(&db, "u1", 1_000_000_000, 500_000_000).await?;

    let ok =
        BalanceService::deduct_dual_currency(&db, "u1", 100_000_000, "USD", 7_240_000_000).await?;
    assert!(ok, "the USD balance covers the cost");

    let after = balances(&db, "u1").await?;
    println!("after a USD-covered cost: {after:?}");
    assert_eq!(
        after,
        Some((900_000_000, 500_000_000)),
        "USD is reduced by the cost and CNY is untouched"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn deduction_falls_back_to_the_second_currency_at_the_scaled_rate(
) -> Result<(), Box<dyn Error>> {
    // **The exchange-rate unit is the part most likely to be wrong**, because the rate is an integer scaled by
    // 10^9 rather than a float. The plan names "汇率单位" separately for that reason.
    //
    // A USD cost of 100 nano against a USD balance of 40 leaves 60 nano needed. At 7.24 CNY per USD, spelled
    // `7_240_000_000`, the CNY required is `60 * 7_240_000_000 / 1_000_000_000 = 434.4`, truncated to 434.
    let (db, path) = fresh_db("fallback").await?;
    credit(&db, "u1", 40, 10_000).await?;

    let ok = BalanceService::deduct_dual_currency(&db, "u1", 100, "USD", 7_240_000_000).await?;
    assert!(ok, "the two balances together cover the cost");

    let after = balances(&db, "u1").await?;
    println!("after a split deduction: {after:?}");
    assert_eq!(
        after,
        Some((0, 10_000 - 434)),
        "the USD balance is cleared and CNY pays `needed * rate / 10^9`, truncated"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn the_rate_scale_is_a_billion_not_a_float() -> Result<(), Box<dyn Error>> {
    // The same cost at two rates differing by a factor of a thousand, so a rate read as a plain integer (7
    // instead of 7_240_000_000) or as a percentage would give a visibly different deduction rather than a
    // subtly different one.
    let (db, path) = fresh_db("rate_scale").await?;
    credit(&db, "u1", 0, 1_000_000_000).await?;
    credit(&db, "u2", 0, 1_000_000_000).await?;

    let ok1 = BalanceService::deduct_dual_currency(&db, "u1", 1_000, "USD", 7_240_000_000).await?;
    assert!(ok1);
    let ok2 = BalanceService::deduct_dual_currency(&db, "u2", 1_000, "USD", 1_000_000_000).await?;
    assert!(ok2);

    let u1 = balances(&db, "u1").await?;
    let u2 = balances(&db, "u2").await?;
    println!("at 7.24 CNY/USD: {u1:?}; at 1 CNY/USD: {u2:?}");

    assert_eq!(
        u1,
        Some((0, 1_000_000_000 - 7_240)),
        "1_000 nano USD in CNY at 7.24"
    );
    assert_eq!(
        u2,
        Some((0, 1_000_000_000 - 1_000)),
        "and at parity, 1_000 nano"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn the_cny_side_of_the_split_works_the_same_way() -> Result<(), Box<dyn Error>> {
    // The mirror branch (`log.rs:1036-1090`), which the USD tests do not reach.
    let (db, path) = fresh_db("cny_side").await?;

    // **The first sub-case was missing and a mutation proved it.** The original version of this test used
    // `balance_cny = 100` against a cost of 500, which takes the *conversion* branch -- so replacing the
    // `balance_cny >= cost_nano` check with `false` changed nothing and the mutation survived. The branch it
    // guarded was never entered. This is that branch: an ample CNY balance, which must be paid from CNY alone
    // without touching USD.
    credit(&db, "rich", 1_000_000_000, 1_000_000_000).await?;
    let ok = BalanceService::deduct_dual_currency(&db, "rich", 500, "CNY", 7_240_000_000).await?;
    assert!(ok, "the CNY balance alone covers the cost");
    let rich = balances(&db, "rich").await?;
    println!("CNY-covered cost: {rich:?}");
    assert_eq!(
        rich,
        Some((1_000_000_000, 1_000_000_000 - 500)),
        "CNY pays the cost and USD is untouched, because no conversion was needed"
    );

    // And the conversion branch: 500 nano of CNY needed, 100 available, so 400 comes from USD at
    // `400 * 10^9 / 7_240_000_000 = 55.2`, truncated to 55.
    credit(&db, "u1", 1_000_000_000, 100).await?;
    let ok = BalanceService::deduct_dual_currency(&db, "u1", 500, "CNY", 7_240_000_000).await?;
    assert!(ok, "the USD balance covers the shortfall");
    let after = balances(&db, "u1").await?;
    println!("after a CNY-first split: {after:?}");
    assert_eq!(
        after,
        Some((1_000_000_000 - 55, 0)),
        "CNY is cleared and USD pays `needed_cny * 10^9 / rate`, truncated"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn a_cost_exceeding_both_balances_is_refused_and_nothing_moves() -> Result<(), Box<dyn Error>>
{
    // The refusal path. `Ok(false)` is the documented "insufficient balance", distinct from `Err`, and the
    // important part beyond the return value is that a refused deduction leaves **both** currencies as they
    // were -- a partial deduction here would take money without serving the request.
    let (db, path) = fresh_db("insufficient").await?;
    credit(&db, "u1", 100, 100).await?;
    let before = balances(&db, "u1").await?;

    let ok =
        BalanceService::deduct_dual_currency(&db, "u1", 1_000_000, "USD", 7_240_000_000).await?;

    let after = balances(&db, "u1").await?;
    println!("refused: ok={ok}, before {before:?}, after {after:?}");
    assert!(
        !ok,
        "a cost beyond both balances is refused with false, not an error"
    );
    assert_eq!(after, before, "and no part of the balance moved");

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn a_cost_of_zero_or_less_is_reported_as_a_successful_deduction_without_a_write(
) -> Result<(), Box<dyn Error>> {
    // The guard at `lib.rs:346`: `if cost_nano <= 0 { return Ok(true); }`.
    //
    // **Both readings are recorded rather than one being asserted as correct.** A zero cost succeeding without
    // a write is plainly intended -- most requests cost nothing and should not need a transaction. A
    // **negative** cost also succeeding is the part worth naming: the same `Ok(true)` covers "nothing to do"
    // and "an amount that was probably a mistake", so a caller cannot tell them apart, and a refund routed
    // through here would report success while doing nothing. That is a question for whoever owns the settlement
    // contract, which the plan says is not this task.
    let (db, path) = fresh_db("nonpositive").await?;
    credit(&db, "u1", 1_000, 1_000).await?;
    let before = balances(&db, "u1").await?;

    let zero = BalanceService::deduct_dual_currency(&db, "u1", 0, "USD", 7_240_000_000).await?;
    let negative =
        BalanceService::deduct_dual_currency(&db, "u1", -500, "USD", 7_240_000_000).await?;

    let after = balances(&db, "u1").await?;
    println!("zero -> {zero}, negative -> {negative}, before {before:?}, after {after:?}");

    assert!(
        zero,
        "a zero cost succeeds, which is the intended fast path"
    );
    assert_eq!(after, before, "and writes nothing");

    assert!(
        negative,
        "a NEGATIVE cost also returns true. Recorded, not endorsed: `Ok(true)` covers both a no-op and an \
         amount that should arguably have been rejected"
    );
    assert_eq!(
        after, before,
        "and a negative cost does not credit the balance either -- it is a no-op that reports success"
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn deducting_for_a_user_with_no_account_row_is_an_error_not_a_false(
) -> Result<(), Box<dyn Error>> {
    // **This test records a correction to my own reading of the source.** `balances.unwrap_or((0, 0))` at
    // `lib.rs:360` looks like "a missing account is treated as a zero balance", which would make the deduction
    // fail with `Ok(false)` -- indistinguishable from a user who has spent everything. The first version of
    // this file asserted exactly that, and measured
    // `Err(Query("user account not found: no-such-user"))`: something downstream checks for the row.
    //
    // So a missing account is a **failure**, which is the safer behaviour and the one worth pinning, because it
    // is the difference between "you are out of money" and "we could not find you".
    let (db, path) = fresh_db("no_account").await?;

    let result =
        BalanceService::deduct_dual_currency(&db, "no-such-user", 100, "USD", 7_240_000_000).await;

    match &result {
        Ok(ok) => panic!(
            "a missing account returned Ok({ok}); if that is false it is indistinguishable from an empty \
             wallet, and if true the deduction silently did nothing"
        ),
        Err(e) => println!("a missing account reports: {e}"),
    }
    assert!(result.is_err(), "a missing account must be an error");
    assert_eq!(
        balances(&db, "no-such-user").await?,
        None,
        "and no account row was created as a side effect"
    );

    // The single-currency paths behave the same way, so this is the model's behaviour rather than one method's.
    assert!(
        BalanceService::deduct_usd(&db, "no-such-user", 100)
            .await
            .is_err(),
        "deduct_usd must also fail for a user that does not exist"
    );

    cleanup(db, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// 时间窗口
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn the_time_window_bounds_the_summary() -> Result<(), Box<dyn Error>> {
    // The "time" half of user/model/time filtering. `start` and `end` are `YYYY-MM-DD` strings compared against
    // `strftime('%Y-%m-%d', created_at)`, so a summary that ignored them would report lifetime totals under a
    // period label.
    let (db, path) = fresh_db("time_window").await?;
    for (id, when) in [
        ("jan", "2026-01-15 12:00:00"),
        ("feb", "2026-02-15 12:00:00"),
        ("mar", "2026-03-15 12:00:00"),
    ] {
        insert_log_at(&db, id, "u1", "gpt-4o", 10_000_000, when).await?;
    }

    let all = burncloud_router::BillingService::get_billing_summary(&db, None, None).await?;
    let january = burncloud_router::BillingService::get_billing_summary(
        &db,
        Some("2026-01-01"),
        Some("2026-01-31"),
    )
    .await?;
    let quarter = burncloud_router::BillingService::get_billing_summary(
        &db,
        Some("2026-01-01"),
        Some("2026-03-31"),
    )
    .await?;

    let requests =
        |s: &burncloud_router::BillingSummary| s.models.iter().map(|m| m.requests).sum::<i64>();
    println!(
        "all {}, january {}, quarter {}",
        requests(&all),
        requests(&january),
        requests(&quarter)
    );

    assert_eq!(requests(&all), 3, "no bounds includes every row");
    assert_eq!(
        requests(&january),
        1,
        "the January window includes only January"
    );
    assert_eq!(requests(&quarter), 3, "the quarter includes all three");

    // The cost travels with the count, so a window that filtered the rows but summed every row's cost would be
    // caught. Each row costs 10_000_000 nano = 0.01 USD.
    assert!(
        (january.total_cost_usd - 0.01).abs() < 1e-9,
        "one row at 0.01 USD, got {}",
        january.total_cost_usd
    );
    assert!(
        (all.total_cost_usd - 0.03).abs() < 1e-9,
        "three rows at 0.01 USD, got {}",
        all.total_cost_usd
    );

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn the_usage_statistics_window_is_relative_to_now_and_old_rows_are_outside_it(
) -> Result<(), Box<dyn Error>> {
    // **The distinction the first version of this file got wrong.** `get_usage_stats` does not accept a window
    // from its caller: `period` chooses only the **length** (`day` is 24 hours, `week` is 7 days, anything else
    // is 30 days) and the rows are bounded against the **current** time. So a log dated last year reads as no
    // usage, which is a different statement from no rows -- and both must be independently visible.
    let (db, path) = fresh_db("stats_window").await?;

    // Inside the window: counted.
    insert_log_at(&db, "fresh", "u1", "gpt-4o", 42, &recent(3_600)).await?;
    // Outside it: stored, but not counted by a 30-day window.
    insert_log_at(&db, "stale", "u1", "gpt-4o", 7, "2020-01-01 00:00:00").await?;

    // Both rows exist, so the older one is genuinely stored rather than rejected.
    let all_rows = RouterLogService::get_filtered(&db, Some("u1"), None, None, 100, 0).await?;
    assert_eq!(all_rows.len(), 2, "both rows are stored");

    let counted = UsageStatsService::get_stats(&db, "u1", "month").await?;
    println!("30-day window: {counted:?}");
    assert_eq!(
        counted.total_requests, 1,
        "only the row inside the window is counted"
    );
    assert_eq!(counted.total_cost_nano, 42, "and only its cost");

    // A one-hour-old row is outside a `day` window only if it is older than a day; three hours is inside both,
    // so this checks the length is read rather than ignored.
    let day = UsageStatsService::get_stats(&db, "u1", "day").await?;
    assert_eq!(
        day.total_requests, 1,
        "a one-hour-old row is inside a day window too"
    );

    // The per-model grouping applies the same window, so a row outside it must not appear as a model group.
    let grouped = UsageStatsService::get_stats_by_model(&db, "u1", "month").await?;
    println!("grouped: {grouped:?}");
    assert_eq!(grouped.len(), 1, "one model in the window");
    assert_eq!(grouped[0].requests, 1);
    assert_eq!(grouped[0].cost_nano, 42);

    cleanup(db, path).await;
    Ok(())
}

#[tokio::test]
async fn a_summary_for_one_user_excludes_another_users_rows() -> Result<(), Box<dyn Error>> {
    // `get_billing_summary_for_user` against the unscoped summary, so a filter that was dropped shows up as the
    // wrong count rather than as an empty result.
    let (db, path) = fresh_db("summary_isolation").await?;
    insert_log_at(
        &db,
        "a",
        "alice",
        "gpt-4o",
        10_000_000,
        "2026-01-10 00:00:00",
    )
    .await?;
    insert_log_at(&db, "b", "bob", "gpt-4o", 20_000_000, "2026-01-11 00:00:00").await?;

    let scoped = burncloud_router::BillingService::get_billing_summary_for_user(
        &db,
        "alice",
        Some("2026-01-01"),
        Some("2026-01-31"),
    )
    .await?;
    let unscoped = burncloud_router::BillingService::get_billing_summary(
        &db,
        Some("2026-01-01"),
        Some("2026-01-31"),
    )
    .await?;

    let requests =
        |s: &burncloud_router::BillingSummary| s.models.iter().map(|m| m.requests).sum::<i64>();
    println!(
        "alice {}, everyone {}",
        requests(&scoped),
        requests(&unscoped)
    );

    assert_eq!(requests(&scoped), 1, "alice's summary counts only alice");
    assert!(
        (scoped.total_cost_usd - 0.01).abs() < 1e-9,
        "and only her 0.01 USD, got {}",
        scoped.total_cost_usd
    );
    assert_eq!(requests(&unscoped), 2, "the unscoped summary counts both");
    assert!(
        (unscoped.total_cost_usd - 0.03).abs() < 1e-9,
        "and both costs"
    );

    cleanup(db, path).await;
    Ok(())
}
