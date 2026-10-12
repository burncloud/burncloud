//! Database operations for router configuration
//!
//! This module aggregates all router-related database operations and provides
//! database initialization functionality.
//!
//! # Modules
//! - [`log`] - Router logs, usage stats and balance deduction (RouterLog, RouterLogModel, BalanceModel)
//! - [`router_video_task`] - Router video task persistence (RouterVideoTask, RouterVideoTaskModel)

use burncloud_database::{adapt_sql, phs, Database, Result};
use burncloud_identity_token::TokenService;

pub mod log;
pub mod router_video_task;
// Re-export common types.
pub use burncloud_identity_token::{
    RouterToken, RouterTokenModel, RouterTokenValidationResult, TokenRotationResult,
};
pub use log::{
    get_billing_summary, get_billing_summary_for_user, get_usage_stats, get_usage_stats_by_model,
    BalanceModel, BillingModelSummary, BillingSummary, CandidateInfo, FailoverAttempt,
    ModelUsageStats, RouterLog, RouterLogModel, RouterRequestLog, RouterRequestLogModel,
    StoragePolicy, UsageStats,
};
pub use router_video_task::{RouterVideoTask, RouterVideoTaskModel};

/// Result of [`RouterDatabase::validate_token_and_get_info`].
///
/// Carries user identity, quota state, and the L1 Classifier inputs
/// (`order_type` / `price_cap`) needed by `proxy_logic` to construct a real
/// `SchedulingRequest`.
///
/// `order_type` and `price_cap` are `Option` because:
/// - `user_api_keys` (the primary token table) does not store them; the values
///   live in `router_tokens` and are pulled via `LEFT JOIN`. Tokens that don't
///   have a matching `router_tokens` row read as `None`.
/// - `price_cap_nanodollars` is also nullable inside `router_tokens` itself
///   (per migration 0011 — only `order_type` carries a default).
///
/// Callers should fall back to `OrderType::default()` when either is `None`.
#[derive(Debug, Clone)]
pub struct TokenValidationInfo {
    pub user_id: String,
    pub group: String,
    pub remain_quota: i64,
    pub used_quota: i64,
    pub order_type: Option<String>,
    pub price_cap: Option<i64>,
}

/// Run a migration statement whose failure must not abort initialization.
///
/// `ALTER TABLE ... ADD COLUMN` fails once the column already exists —
/// "duplicate column name" on SQLite, "column already exists" on PostgreSQL —
/// which is the expected outcome on every re-run. This crate carries no logging
/// dependency, so the expected error is dropped here instead of propagated.
async fn best_effort_execute(pool: &sqlx::AnyPool, sql: &str) {
    // Failure is expected on re-runs ("duplicate column name") and is intentionally discarded.
    sqlx::query(sql).execute(pool).await.ok();
}

/// Router database operations
pub struct RouterDatabase;

impl RouterDatabase {
    /// Initialize router database tables
    pub async fn init(db: &Database) -> Result<()> {
        TokenService::init(db).await?;

        if db.kind() == "sqlite" {
            let conn = db.get_connection()?;
            best_effort_execute(
                conn.pool(),
                "ALTER TABLE router_logs ADD COLUMN cost REAL NOT NULL DEFAULT 0",
            )
            .await;
        }

        Ok(())
    }

    // ============== Identity-owned Token capability delegations ==============

    pub async fn list_tokens(db: &Database) -> Result<Vec<RouterToken>> {
        TokenService::list(db).await
    }

    pub async fn create_token(db: &Database, t: &RouterToken) -> Result<()> {
        TokenService::create(db, t).await
    }

    pub async fn delete_token(db: &Database, token: &str) -> Result<()> {
        TokenService::delete(db, token).await
    }

    pub async fn update_token_status(db: &Database, token: &str, status: &str) -> Result<()> {
        TokenService::update_status(db, token, status).await
    }

    pub async fn validate_token(db: &Database, token: &str) -> Result<Option<RouterToken>> {
        TokenService::validate(db, token).await
    }

    pub async fn validate_token_detailed(
        db: &Database,
        token: &str,
    ) -> Result<RouterTokenValidationResult> {
        TokenService::validate_detailed(db, token).await
    }

    pub async fn update_token_accessed_time(db: &Database, token: &str) -> Result<()> {
        TokenService::update_accessed_time(db, token).await
    }

    pub async fn check_quota(db: &Database, token: &str, cost: i64) -> Result<bool> {
        TokenService::check_quota(db, token, cost).await
    }

    pub async fn deduct_quota(
        db: &Database,
        _user_id: &str,
        token: &str,
        cost: i64,
    ) -> Result<bool> {
        TokenService::deduct_quota(db, token, cost).await
    }

    /// Validates a token and returns the [`TokenValidationInfo`] payload
    /// (user identity, quota state, and L1 Classifier inputs) when the token
    /// is active. Returns `Ok(None)` on no match — the proxy entrypoint then
    /// falls back to the legacy `validate_token_detailed` path.
    ///
    /// `order_type` and `price_cap_nanodollars` come from `router_tokens` via
    /// `LEFT JOIN` on the token string. Tokens that exist only in
    /// `user_api_keys` (no matching `router_tokens` row) get `None` for both
    /// fields and the L1 Classifier falls back to `OrderType::default()`.
    pub async fn validate_token_and_get_info(
        db: &Database,
        token: &str,
    ) -> Result<Option<TokenValidationInfo>> {
        let identity = match TokenService::active_api_key_identity(db, token).await? {
            Some(identity) => identity,
            None => return Ok(None),
        };

        // Only the two Traffic-owned routing projection columns are read here.
        // Absence of a router_tokens row retains historical None/None defaults.
        let conn = db.get_connection()?;
        let sql = adapt_sql(
            db.kind() == "postgres",
            "SELECT order_type, price_cap_nanodollars FROM router_tokens WHERE token = ?",
        );
        let projection: Option<(Option<String>, Option<i64>)> = sqlx::query_as(&sql)
            .bind(token)
            .fetch_optional(conn.pool())
            .await?;
        let (order_type, price_cap) = projection.unwrap_or((None, None));
        Ok(Some(TokenValidationInfo {
            user_id: identity.user_id,
            group: identity.group,
            remain_quota: identity.remain_quota,
            used_quota: identity.used_quota,
            order_type,
            price_cap,
        }))
    }

    // ============== Log delegations ==============

    /// Insert an immutable usage/billing log entry.
    ///
    /// Logging is intentionally side-effect free with respect to credential
    /// spend quota. Spend settlement is owned by `deduct_quota` and is scoped
    /// to the credential that actually authorized the request.
    pub async fn insert_log(db: &Database, log: &RouterLog) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = format!(
            r#"
            INSERT INTO router_logs
            (request_id, user_id, path, upstream_id, status_code, latency_ms,
             prompt_tokens, completion_tokens, cost,
             model, cache_read_tokens, reasoning_tokens, pricing_region, video_tokens,
             cache_write_tokens, audio_input_tokens, audio_output_tokens, image_tokens, embedding_tokens,
             input_cost, output_cost, cache_read_cost, cache_write_cost,
             audio_cost, image_cost, video_cost, reasoning_cost, embedding_cost,
             layer_decision, traffic_color, cost_status, error_type)
            VALUES ({})
            "#,
            phs(is_postgres, 32)
        );

        sqlx::query(&sql)
            .bind(&log.request_id)
            .bind(&log.user_id)
            .bind(&log.path)
            .bind(&log.upstream_id)
            .bind(log.status_code)
            .bind(log.latency_ms)
            .bind(log.prompt_tokens)
            .bind(log.completion_tokens)
            .bind(log.cost)
            .bind(&log.model)
            .bind(log.cache_read_tokens)
            .bind(log.reasoning_tokens)
            .bind(&log.pricing_region)
            .bind(log.video_tokens)
            .bind(log.cache_write_tokens)
            .bind(log.audio_input_tokens)
            .bind(log.audio_output_tokens)
            .bind(log.image_tokens)
            .bind(log.embedding_tokens)
            .bind(log.input_cost)
            .bind(log.output_cost)
            .bind(log.cache_read_cost)
            .bind(log.cache_write_cost)
            .bind(log.audio_cost)
            .bind(log.image_cost)
            .bind(log.video_cost)
            .bind(log.reasoning_cost)
            .bind(log.embedding_cost)
            .bind(&log.layer_decision)
            .bind(&log.traffic_color)
            .bind(&log.cost_status)
            .bind(&log.error_type)
            .execute(conn.pool())
            .await?;

        Ok(())
    }

    pub async fn get_logs(db: &Database, limit: i32, offset: i32) -> Result<Vec<RouterLog>> {
        RouterLogModel::get(db, limit, offset).await
    }

    pub async fn get_logs_filtered(
        db: &Database,
        user_id: Option<&str>,
        upstream_id: Option<&str>,
        model: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<RouterLog>> {
        RouterLogModel::get_filtered(db, user_id, upstream_id, model, limit, offset).await
    }

    pub async fn get_usage_by_user(db: &Database, user_id: &str) -> Result<(i64, i64)> {
        RouterLogModel::get_usage_by_user(db, user_id).await
    }

    // ============== Request Log delegations ==============

    pub async fn insert_request_log(db: &Database, log: &RouterRequestLog) -> Result<()> {
        RouterRequestLogModel::insert(db, log).await
    }

    pub async fn get_request_log_by_id(
        db: &Database,
        request_id: &str,
    ) -> Result<Option<RouterRequestLog>> {
        RouterRequestLogModel::get_by_request_id(db, request_id).await
    }

    pub async fn delete_old_request_logs(db: &Database, days: i32) -> Result<u64> {
        RouterRequestLogModel::delete_old_logs(db, days).await
    }

    // ============== Balance delegations ==============

    pub async fn deduct_usd(db: &Database, user_id: &str, cost_nano: i64) -> Result<bool> {
        BalanceModel::deduct_usd(db, user_id, cost_nano).await
    }

    pub async fn deduct_cny(db: &Database, user_id: &str, cost_nano: i64) -> Result<bool> {
        BalanceModel::deduct_cny(db, user_id, cost_nano).await
    }

    pub async fn deduct_dual_currency(
        db: &Database,
        user_id: &str,
        cost_nano: i64,
        cost_currency: &str,
        exchange_rate_nano: i64,
    ) -> Result<bool> {
        BalanceModel::deduct_dual_currency_router_legacy(
            db,
            user_id,
            cost_nano,
            cost_currency,
            exchange_rate_nano,
        )
        .await
    }
}

/// Get aggregated usage statistics by token key over a time period.
/// Looks up the user_id from user_api_keys, then queries router_logs.
pub async fn get_usage_stats_by_token(
    db: &Database,
    token_key: &str,
    period: &str,
) -> Result<Option<(String, UsageStats)>> {
    let user_id = TokenService::active_api_key_user_id(db, token_key).await?;

    match user_id {
        None => Ok(None),
        Some(uid) => {
            let stats = get_usage_stats(db, &uid, period).await?;
            Ok(Some((uid, stats)))
        }
    }
}
