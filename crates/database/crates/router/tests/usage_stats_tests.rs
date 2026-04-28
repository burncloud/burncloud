#![allow(clippy::unwrap_used, clippy::expect_used)]

/// Regression tests for issue #156 fixes in log.rs:
/// - B4: time filter CAST failure (strftime/EXTRACT EPOCH)
/// - V2: deduct_usd/deduct_cny error propagation (fetch_optional + ok_or_else)
/// - B5: get_usage_stats_by_model period parameter (no longer ignored)
use burncloud_database::create_database_with_url;
use burncloud_database_router::{
    get_usage_stats, get_usage_stats_by_model, BalanceModel, RouterDatabase,
};
use tempfile::NamedTempFile;

/// Create an isolated SQLite test database with all required tables.
async fn create_test_db() -> (burncloud_database::Database, NamedTempFile) {
    let tmp = NamedTempFile::new().unwrap_or_else(|e| panic!("failed to create temp file: {e}"));
    let url = format!("sqlite://{}?mode=rwc", tmp.path().display());
    let db = create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("failed to initialize test database: {e}"));
    RouterDatabase::init(&db)
        .await
        .unwrap_or_else(|e| panic!("failed to initialize router tables: {e}"));
    (db, tmp)
}

/// Insert a router_logs row with a specific created_at timestamp.
/// `created_at` must be an ISO8601 string like "2026-04-28 12:00:00".
async fn insert_log_with_timestamp(db: &burncloud_database::Database, user_id: &str, created_at: &str) {
    let conn = db.get_connection().unwrap();
    let sql = r#"
        INSERT INTO router_logs
        (request_id, user_id, path, upstream_id, status_code, latency_ms,
         prompt_tokens, completion_tokens, cost, model, created_at)
        VALUES (?, ?, '/v1/chat/completions', 'up-1', 200, 100,
                1000, 500, 20000000, 'gpt-4o', ?)
    "#;
    sqlx::query(sql)
        .bind(format!("req-{}", uuid::Uuid::new_v4()))
        .bind(user_id)
        .bind(created_at)
        .execute(conn.pool())
        .await
        .unwrap_or_else(|e| panic!("insert_log_with_timestamp failed: {e}"));
}

/// Insert a user_accounts row with given USD and CNY balances.
async fn insert_user_account(
    db: &burncloud_database::Database,
    user_id: &str,
    balance_usd: i64,
    balance_cny: i64,
) {
    let conn = db.get_connection().unwrap();
    let sql = r#"
        INSERT OR IGNORE INTO user_accounts (id, username, password_hash, status, balance_usd, balance_cny)
        VALUES (?, ?, 'no-login', 1, ?, ?)
    "#;
    sqlx::query(sql)
        .bind(user_id)
        .bind(user_id)
        .bind(balance_usd)
        .bind(balance_cny)
        .execute(conn.pool())
        .await
        .unwrap_or_else(|e| panic!("insert_user_account failed: {e}"));
}

// ---------------------------------------------------------------------------
// B4: Time filter regression tests
// ---------------------------------------------------------------------------

/// B4 regression: get_usage_stats with period="day" returns non-zero stats
/// for logs inserted within the last 24 hours.
///
/// Before the fix, CAST(created_at AS INTEGER) extracted only the year prefix
/// (e.g. 2026), which is always < Unix epoch threshold, so all periods returned 0.
/// After the fix, strftime('%s', created_at) correctly converts to epoch seconds.
#[tokio::test]
async fn test_b4_time_filter_day_returns_recent_data() {
    let (db, _tmp) = create_test_db().await;
    let user_id = "b4-test-user-day";

    // Insert a log with current timestamp (within the last day)
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    insert_log_with_timestamp(&db, user_id, &now).await;

    let stats = get_usage_stats(&db, user_id, "day")
        .await
        .expect("get_usage_stats should succeed");

    assert!(
        stats.total_requests > 0,
        "B4: day-period stats should return non-zero requests for recent data, got {}",
        stats.total_requests
    );
    assert!(
        stats.total_prompt_tokens > 0,
        "B4: day-period stats should return non-zero prompt_tokens"
    );
}

/// B4 regression: get_usage_stats with period="week" returns non-zero stats
/// for logs inserted within the last 7 days.
#[tokio::test]
async fn test_b4_time_filter_week_returns_recent_data() {
    let (db, _tmp) = create_test_db().await;
    let user_id = "b4-test-user-week";

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    insert_log_with_timestamp(&db, user_id, &now).await;

    let stats = get_usage_stats(&db, user_id, "week")
        .await
        .expect("get_usage_stats should succeed");

    assert!(
        stats.total_requests > 0,
        "B4: week-period stats should return non-zero requests for recent data, got {}",
        stats.total_requests
    );
}

/// B4 regression: get_usage_stats with period="month" returns non-zero stats
/// for logs inserted within the last 30 days.
#[tokio::test]
async fn test_b4_time_filter_month_returns_recent_data() {
    let (db, _tmp) = create_test_db().await;
    let user_id = "b4-test-user-month";

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    insert_log_with_timestamp(&db, user_id, &now).await;

    let stats = get_usage_stats(&db, user_id, "month")
        .await
        .expect("get_usage_stats should succeed");

    assert!(
        stats.total_requests > 0,
        "B4: month-period stats should return non-zero requests for recent data, got {}",
        stats.total_requests
    );
}

/// B4 regression: get_usage_stats returns zero for old data outside the period.
#[tokio::test]
async fn test_b4_time_filter_old_data_excluded() {
    let (db, _tmp) = create_test_db().await;
    let user_id = "b4-test-user-old";

    // Insert a log from 90 days ago — should be outside day/week/month windows
    let old_time = (chrono::Utc::now() - chrono::Duration::days(90))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    insert_log_with_timestamp(&db, user_id, &old_time).await;

    let stats = get_usage_stats(&db, user_id, "day")
        .await
        .expect("get_usage_stats should succeed");

    assert_eq!(
        stats.total_requests, 0,
        "B4: day-period stats should return 0 for data older than 24 hours"
    );
}

// ---------------------------------------------------------------------------
// V2: deduct error propagation regression tests
// ---------------------------------------------------------------------------

/// V2 regression: deduct_usd returns Err (not Ok(false)) when user does not exist.
///
/// Before the fix, unwrap_or(0) silently treated missing users as having zero balance,
/// so deduct_usd returned Ok(false) ("insufficient balance") instead of an error.
/// After the fix, fetch_optional + ok_or_else returns Err(DatabaseError::Query).
#[tokio::test]
async fn test_v2_deduct_usd_user_not_found_returns_err() {
    let (db, _tmp) = create_test_db().await;

    let result = BalanceModel::deduct_usd(&db, "nonexistent-user-v2", 1_000_000).await;

    assert!(
        result.is_err(),
        "V2: deduct_usd should return Err for nonexistent user, got {:?}",
        result
    );
}

/// V2 regression: deduct_cny returns Err (not Ok(false)) when user does not exist.
#[tokio::test]
async fn test_v2_deduct_cny_user_not_found_returns_err() {
    let (db, _tmp) = create_test_db().await;

    let result = BalanceModel::deduct_cny(&db, "nonexistent-user-v2", 1_000_000).await;

    assert!(
        result.is_err(),
        "V2: deduct_cny should return Err for nonexistent user, got {:?}",
        result
    );
}

/// V2 complement: deduct_usd returns Ok(false) for existing user with insufficient balance.
#[tokio::test]
async fn test_v2_deduct_usd_insufficient_balance_returns_ok_false() {
    let (db, _tmp) = create_test_db().await;
    let user_id = "v2-poor-user";

    // User has 1M nanodollars ($0.001)
    insert_user_account(&db, user_id, 1_000_000, 0).await;

    // Try to deduct 10M nanodollars ($0.01) — insufficient
    let result = BalanceModel::deduct_usd(&db, user_id, 10_000_000)
        .await
        .expect("deduct_usd should succeed for existing user");

    assert!(
        !result,
        "V2: deduct_usd should return Ok(false) for insufficient balance"
    );
}

/// V2 complement: deduct_usd returns Ok(true) for existing user with sufficient balance.
#[tokio::test]
async fn test_v2_deduct_usd_sufficient_balance_returns_ok_true() {
    let (db, _tmp) = create_test_db().await;
    let user_id = "v2-rich-user";

    // User has 100M nanodollars ($0.1)
    insert_user_account(&db, user_id, 100_000_000, 0).await;

    // Try to deduct 1M nanodollars ($0.001) — sufficient
    let result = BalanceModel::deduct_usd(&db, user_id, 1_000_000)
        .await
        .expect("deduct_usd should succeed for existing user");

    assert!(
        result,
        "V2: deduct_usd should return Ok(true) for sufficient balance"
    );
}

// ---------------------------------------------------------------------------
// B5: period parameter regression tests
// ---------------------------------------------------------------------------

/// B5 regression: get_usage_stats_by_model respects the period parameter.
///
/// Before the fix, _period was ignored and the function returned all-time data.
/// After the fix, period="day" filters to the last 24 hours.
#[tokio::test]
async fn test_b5_period_day_filters_by_model() {
    let (db, _tmp) = create_test_db().await;
    let user_id = "b5-test-user-day";

    // Insert a recent log
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    insert_log_with_timestamp(&db, user_id, &now).await;

    let stats = get_usage_stats_by_model(&db, user_id, "day")
        .await
        .expect("get_usage_stats_by_model should succeed");

    assert!(
        !stats.is_empty(),
        "B5: day-period by-model stats should return data for recent logs"
    );
    assert!(
        stats.iter().any(|s| s.requests > 0),
        "B5: at least one model should have non-zero requests"
    );
}

/// B5 regression: get_usage_stats_by_model with period="week" returns recent data.
#[tokio::test]
async fn test_b5_period_week_filters_by_model() {
    let (db, _tmp) = create_test_db().await;
    let user_id = "b5-test-user-week";

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    insert_log_with_timestamp(&db, user_id, &now).await;

    let stats = get_usage_stats_by_model(&db, user_id, "week")
        .await
        .expect("get_usage_stats_by_model should succeed");

    assert!(
        !stats.is_empty(),
        "B5: week-period by-model stats should return data for recent logs"
    );
}

/// B5 regression: get_usage_stats_by_model excludes old data outside the period.
#[tokio::test]
async fn test_b5_period_excludes_old_data_by_model() {
    let (db, _tmp) = create_test_db().await;
    let user_id = "b5-test-user-old";

    // Insert a log from 90 days ago
    let old_time = (chrono::Utc::now() - chrono::Duration::days(90))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    insert_log_with_timestamp(&db, user_id, &old_time).await;

    let stats = get_usage_stats_by_model(&db, user_id, "day")
        .await
        .expect("get_usage_stats_by_model should succeed");

    assert!(
        stats.is_empty() || stats.iter().all(|s| s.requests == 0),
        "B5: day-period by-model stats should exclude data older than 24 hours"
    );
}
