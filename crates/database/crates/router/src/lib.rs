//! Database operations for router configuration
//!
//! This crate aggregates all router-related database operations and provides
//! database initialization functionality.
//!
//! # Modules
//! - [`upstream`] - Upstream/channel configuration (RouterUpstream, RouterUpstreamModel)
//! - [`token`] - Token management (RouterToken, RouterTokenModel)
//! - [`group`] - Group management (RouterGroup, RouterGroupModel, RouterGroupMemberModel)
//! - [`log`] - Router logs, usage stats and balance deduction (RouterLog, RouterLogModel, BalanceModel)
//! - [`router_video_task`] - Router video task persistence (RouterVideoTask, RouterVideoTaskModel)

use burncloud_database::{adapt_sql, Database, Result};
use sqlx::Row;

pub mod group;
pub mod log;
pub mod router_video_task;
pub mod token;
pub mod upstream;

// Re-export common types.
pub use group::{
    RouterGroup, RouterGroupMember, RouterGroupMemberModel, RouterGroupModel, RouterGroupRepository,
};
pub use log::{
    get_billing_summary, get_usage_stats, get_usage_stats_by_model, BalanceModel,
    BillingModelSummary, BillingSummary, ModelUsageStats, RouterLog, RouterLogModel, UsageStats,
};
pub use router_video_task::{RouterVideoTask, RouterVideoTaskModel};
pub use token::{
    RouterToken, RouterTokenModel, RouterTokenRepository, RouterTokenValidationResult,
};
pub use upstream::{RouterUpstream, RouterUpstreamModel, RouterUpstreamRepository};

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

/// Tuple shape of the SELECT inside [`RouterDatabase::validate_token_and_get_info`].
/// Aliased so the row type does not trip `clippy::type_complexity`.
type TokenValidationRow = (String, String, i64, i64, Option<String>, Option<i64>);

/// Router database operations
pub struct RouterDatabase;

impl RouterDatabase {
    /// Initialize router database tables
    pub async fn init(db: &Database) -> Result<()> {
        let conn = db.get_connection()?;
        let kind = db.kind();

        // Table definitions
        let (upstreams_sql, tokens_sql, groups_sql, members_sql) = match kind.as_str() {
            "sqlite" => (
                r#"
                CREATE TABLE IF NOT EXISTS router_upstreams (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    base_url TEXT NOT NULL,
                    api_key TEXT NOT NULL,
                    match_path TEXT NOT NULL,
                    auth_type TEXT NOT NULL,
                    priority INTEGER NOT NULL DEFAULT 0,
                    protocol TEXT NOT NULL DEFAULT 'openai',
                    param_override TEXT,
                    header_override TEXT
                );
                "#,
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
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS router_groups (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    strategy TEXT NOT NULL DEFAULT 'round_robin',
                    match_path TEXT NOT NULL
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS router_group_members (
                    group_id TEXT NOT NULL,
                    upstream_id TEXT NOT NULL,
                    weight INTEGER NOT NULL DEFAULT 1,
                    PRIMARY KEY (group_id, upstream_id)
                );
                "#,
            ),
            "postgres" => (
                r#"
                CREATE TABLE IF NOT EXISTS router_upstreams (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    base_url TEXT NOT NULL,
                    api_key TEXT NOT NULL,
                    match_path TEXT NOT NULL,
                    auth_type TEXT NOT NULL,
                    priority INTEGER NOT NULL DEFAULT 0,
                    protocol TEXT NOT NULL DEFAULT 'openai',
                    param_override TEXT,
                    header_override TEXT
                );
                "#,
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
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS router_groups (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    strategy TEXT NOT NULL DEFAULT 'round_robin',
                    match_path TEXT NOT NULL
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS router_group_members (
                    group_id TEXT NOT NULL,
                    upstream_id TEXT NOT NULL,
                    weight INTEGER NOT NULL DEFAULT 1,
                    PRIMARY KEY (group_id, upstream_id)
                );
                "#,
            ),
            _ => unreachable!("Unsupported database kind"),
        };

        sqlx::query(upstreams_sql).execute(conn.pool()).await?;
        sqlx::query(tokens_sql).execute(conn.pool()).await?;
        sqlx::query(groups_sql).execute(conn.pool()).await?;
        sqlx::query(members_sql).execute(conn.pool()).await?;

        // Migrations
        if kind == "sqlite" {
            let _ = sqlx::query(
                "ALTER TABLE router_upstreams ADD COLUMN priority INTEGER NOT NULL DEFAULT 0",
            )
            .execute(conn.pool())
            .await;
            let _ = sqlx::query(
                "ALTER TABLE router_upstreams ADD COLUMN protocol TEXT NOT NULL DEFAULT 'openai'",
            )
            .execute(conn.pool())
            .await;
            let _ = sqlx::query("ALTER TABLE router_upstreams ADD COLUMN param_override TEXT")
                .execute(conn.pool())
                .await;
            let _ = sqlx::query("ALTER TABLE router_upstreams ADD COLUMN header_override TEXT")
                .execute(conn.pool())
                .await;
            let _ = sqlx::query(
                "ALTER TABLE router_tokens ADD COLUMN quota_limit INTEGER NOT NULL DEFAULT -1",
            )
            .execute(conn.pool())
            .await;
            let _ = sqlx::query(
                "ALTER TABLE router_tokens ADD COLUMN used_quota INTEGER NOT NULL DEFAULT 0",
            )
            .execute(conn.pool())
            .await;
            let _ = sqlx::query(
                "ALTER TABLE router_tokens ADD COLUMN expired_time INTEGER NOT NULL DEFAULT -1",
            )
            .execute(conn.pool())
            .await;
            let _ = sqlx::query(
                "ALTER TABLE router_tokens ADD COLUMN accessed_time INTEGER NOT NULL DEFAULT 0",
            )
            .execute(conn.pool())
            .await;
            let _ = sqlx::query("ALTER TABLE router_logs ADD COLUMN cost REAL NOT NULL DEFAULT 0")
                .execute(conn.pool())
                .await;
            // Add api_version column for protocol adaptation
            let _ = sqlx::query("ALTER TABLE router_upstreams ADD COLUMN api_version TEXT")
                .execute(conn.pool())
                .await;
            // Note: router_logs migration moved to schema.rs for CLI compatibility
        } else if kind == "postgres" {
            let _ = sqlx::query(
                "ALTER TABLE router_upstreams ADD COLUMN IF NOT EXISTS api_version TEXT",
            )
            .execute(conn.pool())
            .await;
        }

        // Insert default demo data if empty
        let count: i64 = sqlx::query("SELECT COUNT(*) FROM router_upstreams")
            .fetch_one(conn.pool())
            .await?
            .get(0);

        if count == 0 {
            sqlx::query(
                r#"
                INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type, priority, protocol)
                VALUES
                ('demo-openai', 'OpenAI Demo', 'https://api.openai.com', 'sk-demo', '/v1/chat/completions', 'Bearer', 0, 'openai'),
                ('demo-claude', 'Claude Demo', 'https://api.anthropic.com', 'sk-ant-demo', '/v1/messages', 'XApiKey', 0, 'claude')
                "#
            )
            .execute(conn.pool())
            .await?;
        }

        Ok(())
    }

    // ============== Upstream delegations ==============

    pub async fn get_all_upstreams(db: &Database) -> Result<Vec<RouterUpstream>> {
        RouterUpstreamModel::get_all(db).await
    }

    pub async fn get_upstream(db: &Database, id: &str) -> Result<Option<RouterUpstream>> {
        RouterUpstreamModel::get(db, id).await
    }

    pub async fn create_upstream(db: &Database, u: &RouterUpstream) -> Result<()> {
        RouterUpstreamModel::create(db, u).await
    }

    pub async fn update_upstream(db: &Database, u: &RouterUpstream) -> Result<()> {
        RouterUpstreamModel::update(db, u).await
    }

    pub async fn delete_upstream(db: &Database, id: &str) -> Result<()> {
        RouterUpstreamModel::delete(db, id).await
    }

    // ============== Token delegations ==============

    pub async fn list_tokens(db: &Database) -> Result<Vec<RouterToken>> {
        RouterTokenModel::list(db).await
    }

    pub async fn create_token(db: &Database, t: &RouterToken) -> Result<()> {
        RouterTokenModel::create(db, t).await
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

        Ok(
            row.map(
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
            ),
        )
    }

    // ============== Group delegations ==============

    pub async fn get_all_groups(db: &Database) -> Result<Vec<RouterGroup>> {
        RouterGroupModel::get_all(db).await
    }

    pub async fn get_group_by_id(db: &Database, id: &str) -> Result<Option<RouterGroup>> {
        RouterGroupModel::get(db, id).await
    }

    pub async fn create_group(db: &Database, g: &RouterGroup) -> Result<()> {
        RouterGroupModel::create(db, g).await
    }

    pub async fn delete_group(db: &Database, id: &str) -> Result<()> {
        RouterGroupModel::delete(db, id).await
    }

    // ============== Group member delegations ==============

    pub async fn get_group_members(db: &Database) -> Result<Vec<RouterGroupMember>> {
        RouterGroupMemberModel::get_all(db).await
    }

    pub async fn get_group_members_by_group(
        db: &Database,
        group_id: &str,
    ) -> Result<Vec<RouterGroupMember>> {
        RouterGroupMemberModel::get_by_group(db, group_id).await
    }

    pub async fn set_group_members(
        db: &Database,
        group_id: &str,
        members: Vec<RouterGroupMember>,
    ) -> Result<()> {
        RouterGroupMemberModel::set_for_group(db, group_id, members).await
    }

    // ============== Log delegations ==============

    pub async fn insert_log(db: &Database, log: &RouterLog) -> Result<()> {
        RouterLogModel::insert(db, log).await
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
        if cost_nano <= 0 {
            return Ok(true);
        }

        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        // Get current balances
        let balances_sql = adapt_sql(is_postgres, "SELECT COALESCE(balance_usd, 0), COALESCE(balance_cny, 0) FROM user_accounts WHERE id = ?");
        let balances: Option<(i64, i64)> = sqlx::query_as(&balances_sql)
            .bind(user_id)
            .fetch_optional(conn.pool())
            .await?;

        let (balance_usd, balance_cny) = balances.unwrap_or((0, 0));

        if cost_currency == "CNY" {
            // CNY model: prioritize CNY balance
            if balance_cny >= cost_nano {
                // Sufficient CNY balance
                return Self::deduct_cny(db, user_id, cost_nano).await;
            }

            // Need to convert USD to CNY
            // Required CNY = cost_nano - balance_cny
            // Required USD in nanodollars = required_cny * 10^9 / exchange_rate_nano
            // Using i128 for intermediate calculation to avoid overflow
            let required_cny = cost_nano - balance_cny;
            let required_usd: i128 =
                (required_cny as i128 * 1_000_000_000) / exchange_rate_nano as i128;

            if required_usd > balance_usd as i128 {
                // Insufficient total balance
                return Ok(false);
            }

            // Deduct from both currencies atomically
            let mut tx = conn.pool().begin().await?;

            let clear_cny_sql = adapt_sql(
                is_postgres,
                "UPDATE user_accounts SET balance_cny = 0 WHERE id = ?",
            );

            // Deduct remaining CNY
            if balance_cny > 0 {
                sqlx::query(&clear_cny_sql)
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;
            }

            // Deduct required USD (already integer, no need to round)
            let usd_to_deduct = required_usd as i64;
            let deduct_usd_sql = adapt_sql(is_postgres, "UPDATE user_accounts SET balance_usd = balance_usd - ? WHERE id = ? AND balance_usd >= ?");
            sqlx::query(&deduct_usd_sql)
                .bind(usd_to_deduct)
                .bind(user_id)
                .bind(usd_to_deduct)
                .execute(&mut *tx)
                .await?;

            tx.commit().await?;
            Ok(true)
        } else {
            // USD model (default): prioritize USD balance
            if balance_usd >= cost_nano {
                // Sufficient USD balance
                return Self::deduct_usd(db, user_id, cost_nano).await;
            }

            // Need to convert CNY to USD
            // Required USD = cost_nano - balance_usd
            // Required CNY in nanodollars = required_usd * exchange_rate_nano / 10^9
            // Using i128 for intermediate calculation to avoid overflow
            let required_usd = cost_nano - balance_usd;
            let required_cny: i128 =
                (required_usd as i128 * exchange_rate_nano as i128) / 1_000_000_000;

            if required_cny > balance_cny as i128 {
                // Insufficient total balance
                return Ok(false);
            }

            // Deduct from both currencies atomically
            let mut tx = conn.pool().begin().await?;

            let clear_usd_sql = adapt_sql(
                is_postgres,
                "UPDATE user_accounts SET balance_usd = 0 WHERE id = ?",
            );

            // Deduct remaining USD
            if balance_usd > 0 {
                sqlx::query(&clear_usd_sql)
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;
            }

            // Deduct required CNY (already integer, no need to round)
            let cny_to_deduct = required_cny as i64;
            let deduct_cny_sql = adapt_sql(is_postgres, "UPDATE user_accounts SET balance_cny = balance_cny - ? WHERE id = ? AND balance_cny >= ?");
            sqlx::query(&deduct_cny_sql)
                .bind(cny_to_deduct)
                .bind(user_id)
                .bind(cny_to_deduct)
                .execute(&mut *tx)
                .await?;

            tx.commit().await?;
            Ok(true)
        }
    }
}

/// Get aggregated usage statistics by token key over a time period.
/// Looks up the user_id from router_tokens, then queries router_logs.
pub async fn get_usage_stats_by_token(
    db: &Database,
    token_key: &str,
    period: &str,
) -> Result<Option<(String, UsageStats)>> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    let sql = adapt_sql(
        is_postgres,
        "SELECT user_id FROM router_tokens WHERE token = ? AND status = 'active'",
    );
    let user_id: Option<String> = sqlx::query_scalar(&sql)
        .bind(token_key)
        .fetch_optional(conn.pool())
        .await?;

    match user_id {
        None => Ok(None),
        Some(uid) => {
            let stats = get_usage_stats(db, &uid, period).await?;
            Ok(Some((uid, stats)))
        }
    }
}
