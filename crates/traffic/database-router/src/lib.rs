//! Database operations for router configuration
//!
//! This crate aggregates all router-related database operations and provides
//! database initialization functionality.

use burncloud_database::{adapt_sql, phs, Database, Result};

pub mod log;
pub mod router_video_task;
pub mod token;

pub use log::{
    get_billing_summary, get_billing_summary_for_user, get_usage_stats, get_usage_stats_by_model,
    BalanceModel, BillingModelSummary, BillingSummary, CandidateInfo, FailoverAttempt,
    ModelUsageStats, RouterLog, RouterLogModel, RouterRequestLog, RouterRequestLogModel,
    StoragePolicy, UsageStats,
};
pub use router_video_task::{RouterVideoTask, RouterVideoTaskModel};
pub use token::{
    RouterToken, RouterTokenModel, RouterTokenRepository, RouterTokenValidationResult,
    TokenRotationResult,
};

#[derive(Debug, Clone)]
pub struct TokenValidationInfo {
    pub user_id: String,
    pub group: String,
    pub remain_quota: i64,
    pub used_quota: i64,
    pub order_type: Option<String>,
    pub price_cap: Option<i64>,
}

type TokenValidationRow = (String, String, i64, i64, Option<String>, Option<i64>);

pub struct RouterDatabase;

fn router_tokens_table_sql(kind: &str) -> &'static str {
    match kind {
        "sqlite" => {
            r#"
            CREATE TABLE IF NOT EXISTS router_tokens (
                token TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                status TEXT NOT NULL,
                quota_limit INTEGER NOT NULL DEFAULT -1,
                used_quota INTEGER NOT NULL DEFAULT 0,
                expired_time INTEGER NOT NULL DEFAULT -1,
                accessed_time INTEGER NOT NULL DEFAULT 0
            );
            "#
        }
        "postgres" => {
            r#"
            CREATE TABLE IF NOT EXISTS router_tokens (
                token TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                status TEXT NOT NULL,
                quota_limit BIGINT NOT NULL DEFAULT -1,
                used_quota BIGINT NOT NULL DEFAULT 0,
                expired_time BIGINT NOT NULL DEFAULT -1,
                accessed_time BIGINT NOT NULL DEFAULT 0
            );
            "#
        }
        _ => unreachable!("Unsupported database kind"),
    }
}

async fn execute_sqlite_add_column(db: &Database, sql: &str) -> Result<()> {
    let conn = db.get_connection()?;
    match sqlx::query(sql).execute(conn.pool()).await {
        Ok(_) => Ok(()),
        Err(error) if error.to_string().contains("duplicate column name") => {
            tracing::debug!(%error, "router database migration column already exists");
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

async fn migrate_router_schema(db: &Database) -> Result<()> {
    if db.kind() != "sqlite" {
        return Ok(());
    }

    const MIGRATIONS: &[&str] = &[
        "ALTER TABLE router_tokens ADD COLUMN quota_limit INTEGER NOT NULL DEFAULT -1",
        "ALTER TABLE router_tokens ADD COLUMN used_quota INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE router_tokens ADD COLUMN expired_time INTEGER NOT NULL DEFAULT -1",
        "ALTER TABLE router_tokens ADD COLUMN accessed_time INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE router_logs ADD COLUMN cost REAL NOT NULL DEFAULT 0",
    ];
    for sql in MIGRATIONS {
        execute_sqlite_add_column(db, sql).await?;
    }
    Ok(())
}

impl RouterDatabase {
    /// Initialize router database tables.
    pub async fn init(db: &Database) -> Result<()> {
        let conn = db.get_connection()?;
        sqlx::query(router_tokens_table_sql(db.kind().as_str()))
            .execute(conn.pool())
            .await?;
        migrate_router_schema(db).await
    }

    pub async fn list_tokens(db: &Database) -> Result<Vec<RouterToken>> {
        RouterTokenModel::list(db).await
    }

    pub async fn create_token(db: &Database, token: &RouterToken) -> Result<()> {
        RouterTokenModel::create(db, token).await
    }

    pub async fn delete_token(db: &Database, token: &str) -> Result<()> {
        RouterTokenModel::delete(db, token).await
    }

    pub async fn update_token_status(db: &Database, token: &str, status: &str) -> Result<()> {
        RouterTokenModel::update_status(db, token, status).await
    }

    pub async fn validate_token(db: &Database, token: &str) -> Result<Option<RouterToken>> {
        RouterTokenModel::validate(db, token).await
    }

    pub async fn validate_token_detailed(
        db: &Database,
        token: &str,
    ) -> Result<RouterTokenValidationResult> {
        RouterTokenModel::validate_detailed(db, token).await
    }

    pub async fn update_token_accessed_time(db: &Database, token: &str) -> Result<()> {
        RouterTokenModel::update_accessed_time(db, token).await
    }

    pub async fn check_quota(db: &Database, token: &str, cost: i64) -> Result<bool> {
        RouterTokenModel::check_quota(db, token, cost).await
    }

    pub async fn deduct_quota(
        db: &Database,
        _user_id: &str,
        token: &str,
        cost: i64,
    ) -> Result<bool> {
        RouterTokenModel::deduct_quota(db, token, cost).await
    }

    pub async fn validate_token_and_get_info(
        db: &Database,
        token: &str,
    ) -> Result<Option<TokenValidationInfo>> {
        let conn = db.get_connection()?;
        let group_col = if db.kind() == "postgres" {
            "\"group\""
        } else {
            "`group`"
        };
        let placeholder = if db.kind() == "postgres" { "$1" } else { "?" };
        let query = format!(
            r#"
            SELECT u.id, u.{}, t.remain_quota, t.used_quota,
                   rt.order_type, rt.price_cap_nanodollars
            FROM user_api_keys t
            JOIN user_accounts u ON t.user_id = u.id
            LEFT JOIN router_tokens rt ON rt.token = t.key
            WHERE t.key = {} AND t.status = 1 AND u.status = 1
            "#,
            group_col, placeholder
        );
        let row: Option<TokenValidationRow> = sqlx::query_as(&query)
            .bind(token)
            .fetch_optional(conn.pool())
            .await?;
        Ok(row.map(
            |(user_id, group, remain_quota, used_quota, order_type, price_cap)| {
                TokenValidationInfo {
                    user_id,
                    group,
                    remain_quota,
                    used_quota,
                    order_type,
                    price_cap,
                }
            },
        ))
    }

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

    pub async fn deduct_usd(db: &Database, user_id: &str, cost_nano: i64) -> Result<bool> {
        BalanceModel::deduct_usd(db, user_id, cost_nano).await
    }

    pub async fn deduct_cny(db: &Database, user_id: &str, cost_nano: i64) -> Result<bool> {
        BalanceModel::deduct_cny(db, user_id, cost_nano).await
    }

    async fn balances(db: &Database, user_id: &str) -> Result<(i64, i64)> {
        let conn = db.get_connection()?;
        let sql = adapt_sql(
            db.kind() == "postgres",
            "SELECT COALESCE(balance_usd, 0), COALESCE(balance_cny, 0) FROM user_accounts WHERE id = ?",
        );
        Ok(sqlx::query_as(&sql)
            .bind(user_id)
            .fetch_optional(conn.pool())
            .await?
            .unwrap_or((0, 0)))
    }

    async fn deduct_cny_then_usd(
        db: &Database,
        user_id: &str,
        cost_nano: i64,
        exchange_rate_nano: i64,
        balance_usd: i64,
        balance_cny: i64,
    ) -> Result<bool> {
        if balance_cny >= cost_nano {
            return Self::deduct_cny(db, user_id, cost_nano).await;
        }
        let required_cny = cost_nano - balance_cny;
        let required_usd =
            (i128::from(required_cny) * 1_000_000_000) / i128::from(exchange_rate_nano);
        if required_usd > i128::from(balance_usd) {
            return Ok(false);
        }

        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let mut tx = conn.pool().begin().await?;
        if balance_cny > 0 {
            let clear_cny = adapt_sql(
                is_postgres,
                "UPDATE user_accounts SET balance_cny = 0 WHERE id = ?",
            );
            sqlx::query(&clear_cny)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
        }
        let usd_to_deduct = required_usd as i64;
        let deduct_usd = adapt_sql(
            is_postgres,
            "UPDATE user_accounts SET balance_usd = balance_usd - ? WHERE id = ? AND balance_usd >= ?",
        );
        sqlx::query(&deduct_usd)
            .bind(usd_to_deduct)
            .bind(user_id)
            .bind(usd_to_deduct)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }

    async fn deduct_usd_then_cny(
        db: &Database,
        user_id: &str,
        cost_nano: i64,
        exchange_rate_nano: i64,
        balance_usd: i64,
        balance_cny: i64,
    ) -> Result<bool> {
        if balance_usd >= cost_nano {
            return Self::deduct_usd(db, user_id, cost_nano).await;
        }
        let required_usd = cost_nano - balance_usd;
        let required_cny =
            (i128::from(required_usd) * i128::from(exchange_rate_nano)) / 1_000_000_000;
        if required_cny > i128::from(balance_cny) {
            return Ok(false);
        }

        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let mut tx = conn.pool().begin().await?;
        if balance_usd > 0 {
            let clear_usd = adapt_sql(
                is_postgres,
                "UPDATE user_accounts SET balance_usd = 0 WHERE id = ?",
            );
            sqlx::query(&clear_usd)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
        }
        let cny_to_deduct = required_cny as i64;
        let deduct_cny = adapt_sql(
            is_postgres,
            "UPDATE user_accounts SET balance_cny = balance_cny - ? WHERE id = ? AND balance_cny >= ?",
        );
        sqlx::query(&deduct_cny)
            .bind(cny_to_deduct)
            .bind(user_id)
            .bind(cny_to_deduct)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }

    pub async fn deduct_dual_currency(
        db: &Database,
        user_id: &str,
        cost_nano: i64,
        cost_currency: &str,
        exchange_rate_nano: i64,
    ) -> Result<bool> {
        if cost_nano <= 0 {
            return Ok(true);
        }
        if exchange_rate_nano <= 0 {
            return Ok(false);
        }
        let (balance_usd, balance_cny) = Self::balances(db, user_id).await?;
        if cost_currency == "CNY" {
            Self::deduct_cny_then_usd(
                db,
                user_id,
                cost_nano,
                exchange_rate_nano,
                balance_usd,
                balance_cny,
            )
            .await
        } else {
            Self::deduct_usd_then_cny(
                db,
                user_id,
                cost_nano,
                exchange_rate_nano,
                balance_usd,
                balance_cny,
            )
            .await
        }
    }
}

/// Get aggregated usage statistics by token key over a time period.
pub async fn get_usage_stats_by_token(
    db: &Database,
    token_key: &str,
    period: &str,
) -> Result<Option<(String, UsageStats)>> {
    let conn = db.get_connection()?;
    let sql = adapt_sql(
        db.kind() == "postgres",
        "SELECT user_id FROM user_api_keys WHERE key = ? AND status = 1",
    );
    let user_id: Option<String> = sqlx::query_scalar(&sql)
        .bind(token_key)
        .fetch_optional(conn.pool())
        .await?;
    match user_id {
        None => Ok(None),
        Some(user_id) => {
            let stats = get_usage_stats(db, &user_id, period).await?;
            Ok(Some((user_id, stats)))
        }
    }
}
