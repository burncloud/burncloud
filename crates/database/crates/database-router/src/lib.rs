//! Database operations for router configuration
//!
//! This crate aggregates all router-related database operations and provides
//! database initialization functionality.
//!
//! # Modules
//! - [`upstream`] - Upstream/channel configuration (DbUpstream, UpstreamModel)
//! - [`token`] - Token management (DbToken, TokenModel)
//! - [`group`] - Group management (DbGroup, GroupModel, GroupMemberModel)
//! - [`log`] - Router logs and usage stats (DbRouterLog, RouterLogModel)

use burncloud_database::{Database, DatabaseError, Result};
use serde::{Deserialize, Serialize};
use sqlx::Row;

// Re-export sub-crates as modules
pub use burncloud_database_group as group;
pub use burncloud_database_router_log as log;
pub use burncloud_database_token as token;
pub use burncloud_database_upstream as upstream;

// Re-export common types for backward compatibility
pub use burncloud_database_group::{DbGroup, DbGroupMember, GroupMemberModel, GroupModel};
pub use burncloud_database_router_log::{
    BalanceModel, BillingModelSummary, BillingSummary, DbRouterLog, RouterLogModel,
    get_billing_summary,
};
pub use burncloud_database_token::{DbToken, TokenModel, TokenValidationResult};
pub use burncloud_database_upstream::{DbUpstream, UpstreamModel};

/// Router database operations
pub struct RouterDatabase;

impl RouterDatabase {
    /// Initialize router database tables
    pub async fn init(db: &Database) -> Result<()> {
        let conn = db.get_connection()?;
        let kind = db.kind();

        // Table definitions
        let (upstreams_sql, tokens_sql, groups_sql, members_sql, logs_sql) = match kind.as_str() {
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
                r#"
                CREATE TABLE IF NOT EXISTS router_logs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    request_id TEXT NOT NULL,
                    user_id TEXT,
                    path TEXT NOT NULL,
                    upstream_id TEXT,
                    status_code INTEGER NOT NULL,
                    latency_ms INTEGER NOT NULL,
                    prompt_tokens INTEGER DEFAULT 0,
                    completion_tokens INTEGER DEFAULT 0,
                    cost INTEGER DEFAULT 0,
                    created_at TEXT DEFAULT CURRENT_TIMESTAMP
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
                r#"
                CREATE TABLE IF NOT EXISTS router_logs (
                    id SERIAL PRIMARY KEY,
                    request_id TEXT NOT NULL,
                    user_id TEXT,
                    path TEXT NOT NULL,
                    upstream_id TEXT,
                    status_code INTEGER NOT NULL,
                    latency_ms BIGINT NOT NULL,
                    prompt_tokens INTEGER DEFAULT 0,
                    completion_tokens INTEGER DEFAULT 0,
                    cost DOUBLE PRECISION DEFAULT 0,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );
                "#,
            ),
            _ => unreachable!("Unsupported database kind"),
        };

        sqlx::query(upstreams_sql).execute(conn.pool()).await?;
        sqlx::query(tokens_sql).execute(conn.pool()).await?;
        sqlx::query(groups_sql).execute(conn.pool()).await?;
        sqlx::query(members_sql).execute(conn.pool()).await?;
        sqlx::query(logs_sql).execute(conn.pool()).await?;

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

    pub async fn get_all_upstreams(db: &Database) -> Result<Vec<DbUpstream>> {
        UpstreamModel::get_all(db).await
    }

    pub async fn get_upstream(db: &Database, id: &str) -> Result<Option<DbUpstream>> {
        UpstreamModel::get(db, id).await
    }

    pub async fn create_upstream(db: &Database, u: &DbUpstream) -> Result<()> {
        UpstreamModel::create(db, u).await
    }

    pub async fn update_upstream(db: &Database, u: &DbUpstream) -> Result<()> {
        UpstreamModel::update(db, u).await
    }

    pub async fn delete_upstream(db: &Database, id: &str) -> Result<()> {
        UpstreamModel::delete(db, id).await
    }

    // ============== Token delegations ==============

    pub async fn list_tokens(db: &Database) -> Result<Vec<DbToken>> {
        TokenModel::list(db).await
    }

    pub async fn create_token(db: &Database, t: &DbToken) -> Result<()> {
        TokenModel::create(db, t).await
    }

    pub async fn delete_token(db: &Database, token: &str) -> Result<()> {
        TokenModel::delete(db, token).await
    }

    pub async fn update_token_status(db: &Database, token: &str, status: &str) -> Result<()> {
        TokenModel::update_status(db, token, status).await
    }

    pub async fn validate_token(db: &Database, token: &str) -> Result<Option<DbToken>> {
        TokenModel::validate(db, token).await
    }

    pub async fn validate_token_detailed(
        db: &Database,
        token: &str,
    ) -> Result<TokenValidationResult> {
        TokenModel::validate_detailed(db, token).await
    }

    pub async fn update_token_accessed_time(db: &Database, token: &str) -> Result<()> {
        TokenModel::update_accessed_time(db, token).await
    }

    pub async fn check_quota(db: &Database, token: &str, cost: i64) -> Result<bool> {
        TokenModel::check_quota(db, token, cost).await
    }

    pub async fn deduct_quota(
        db: &Database,
        _user_id: &str,
        token: &str,
        cost: i64,
    ) -> Result<bool> {
        TokenModel::deduct_quota(db, token, cost).await
    }

    /// Validates a token and returns (user_id, group, token_quota_limit, token_used_quota)
    pub async fn validate_token_and_get_info(
        db: &Database,
        token: &str,
    ) -> Result<Option<(String, String, i64, i64)>> {
        let conn = db.get_connection()?;
        let group_col = if db.kind() == "postgres" {
            "\"group\""
        } else {
            "`group`"
        };

        let placeholder = if db.kind() == "postgres" { "$1" } else { "?" };

        // Assuming tokens.key matches the bearer token
        // And tokens.user_id links to users.id
        let query = format!(
            r#"
            SELECT u.id, u.{}, t.remain_quota, t.used_quota
            FROM tokens t
            JOIN users u ON t.user_id = u.id
            WHERE t.key = {} AND t.status = 1 AND u.status = 1
            "#,
            group_col, placeholder
        );

        // Use query_as to map to a tuple
        let result: Option<(String, String, i64, i64)> = sqlx::query_as(&query)
            .bind(token)
            .fetch_optional(conn.pool())
            .await?;

        Ok(result)
    }

    // ============== Group delegations ==============

    pub async fn get_all_groups(db: &Database) -> Result<Vec<DbGroup>> {
        GroupModel::get_all(db).await
    }

    pub async fn get_group_by_id(db: &Database, id: &str) -> Result<Option<DbGroup>> {
        GroupModel::get(db, id).await
    }

    pub async fn create_group(db: &Database, g: &DbGroup) -> Result<()> {
        GroupModel::create(db, g).await
    }

    pub async fn delete_group(db: &Database, id: &str) -> Result<()> {
        GroupModel::delete(db, id).await
    }

    // ============== Group member delegations ==============

    pub async fn get_group_members(db: &Database) -> Result<Vec<DbGroupMember>> {
        GroupMemberModel::get_all(db).await
    }

    pub async fn get_group_members_by_group(
        db: &Database,
        group_id: &str,
    ) -> Result<Vec<DbGroupMember>> {
        GroupMemberModel::get_by_group(db, group_id).await
    }

    pub async fn set_group_members(
        db: &Database,
        group_id: &str,
        members: Vec<DbGroupMember>,
    ) -> Result<()> {
        GroupMemberModel::set_for_group(db, group_id, members).await
    }

    // ============== Log delegations ==============

    pub async fn insert_log(db: &Database, log: &DbRouterLog) -> Result<()> {
        RouterLogModel::insert(db, log).await
    }

    pub async fn get_logs(db: &Database, limit: i32, offset: i32) -> Result<Vec<DbRouterLog>> {
        RouterLogModel::get(db, limit, offset).await
    }

    pub async fn get_logs_filtered(
        db: &Database,
        user_id: Option<&str>,
        upstream_id: Option<&str>,
        model: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<DbRouterLog>> {
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
        let balances_sql = if is_postgres {
            "SELECT COALESCE(balance_usd, 0), COALESCE(balance_cny, 0) FROM users WHERE id = $1"
        } else {
            "SELECT COALESCE(balance_usd, 0), COALESCE(balance_cny, 0) FROM users WHERE id = ?"
        };
        let balances: Option<(i64, i64)> = sqlx::query_as(balances_sql)
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

            let clear_cny_sql = if is_postgres {
                "UPDATE users SET balance_cny = 0 WHERE id = $1"
            } else {
                "UPDATE users SET balance_cny = 0 WHERE id = ?"
            };

            // Deduct remaining CNY
            if balance_cny > 0 {
                sqlx::query(clear_cny_sql)
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;
            }

            // Deduct required USD (already integer, no need to round)
            let usd_to_deduct = required_usd as i64;
            let deduct_usd_sql = if is_postgres {
                "UPDATE users SET balance_usd = balance_usd - $1 WHERE id = $2 AND balance_usd >= $3"
            } else {
                "UPDATE users SET balance_usd = balance_usd - ? WHERE id = ? AND balance_usd >= ?"
            };
            sqlx::query(deduct_usd_sql)
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

            let clear_usd_sql = if is_postgres {
                "UPDATE users SET balance_usd = 0 WHERE id = $1"
            } else {
                "UPDATE users SET balance_usd = 0 WHERE id = ?"
            };

            // Deduct remaining USD
            if balance_usd > 0 {
                sqlx::query(clear_usd_sql)
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;
            }

            // Deduct required CNY (already integer, no need to round)
            let cny_to_deduct = required_cny as i64;
            let deduct_cny_sql = if is_postgres {
                "UPDATE users SET balance_cny = balance_cny - $1 WHERE id = $2 AND balance_cny >= $3"
            } else {
                "UPDATE users SET balance_cny = balance_cny - ? WHERE id = ? AND balance_cny >= ?"
            };
            sqlx::query(deduct_cny_sql)
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

/// Usage statistics for a user over a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub total_requests: i64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    /// Total cost in nanodollars
    pub total_cost_nano: i64,
}

impl Default for UsageStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            total_prompt_tokens: 0,
            total_completion_tokens: 0,
            total_cost_nano: 0,
        }
    }
}

/// Usage statistics grouped by model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageStats {
    pub model: String,
    pub requests: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cache_read_tokens: i64,
    pub reasoning_tokens: i64,
    /// Cost in nanodollars
    pub cost_nano: i64,
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

    let sql = if is_postgres {
        "SELECT user_id FROM router_tokens WHERE token = $1 AND status = 'active'"
    } else {
        "SELECT user_id FROM router_tokens WHERE token = ? AND status = 'active'"
    };
    let user_id: Option<String> = sqlx::query_scalar(sql)
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

/// Get aggregated usage statistics for a user over a time period
/// Period can be: "day", "week", "month"
pub async fn get_usage_stats(db: &Database, user_id: &str, period: &str) -> Result<UsageStats> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    // Calculate time threshold based on period
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| DatabaseError::Query(format!("Time error: {}", e)))?
        .as_secs() as i64;

    let threshold = match period {
        "day" => now - 24 * 60 * 60,
        "week" => now - 7 * 24 * 60 * 60,
        "month" | _ => now - 30 * 24 * 60 * 60, // Default to month
    };

    let sql = if is_postgres {
        r#"
        SELECT
            COUNT(*) as total_requests,
            COALESCE(SUM(prompt_tokens), 0) as total_prompt_tokens,
            COALESCE(SUM(completion_tokens), 0) as total_completion_tokens,
            COALESCE(SUM(cost), 0) as total_cost
        FROM router_logs
        WHERE user_id = $1 AND created_at IS NOT NULL AND CAST(created_at AS BIGINT) >= $2
        "#
    } else {
        // SQLite: created_at is stored as datetime string "2026-03-21 08:18:27"
        // Use strftime('%s', ...) to convert to unix timestamp for comparison
        r#"
        SELECT
            COUNT(*) as total_requests,
            COALESCE(SUM(prompt_tokens), 0) as total_prompt_tokens,
            COALESCE(SUM(completion_tokens), 0) as total_completion_tokens,
            COALESCE(SUM(cost), 0) as total_cost
        FROM router_logs
        WHERE user_id = ? AND created_at IS NOT NULL AND CAST(strftime('%s', created_at) AS INTEGER) >= ?
        "#
    };

    // Note: created_at is stored as string, need to handle carefully
    // We'll parse the timestamp from the string format
    let row: (Option<i64>, Option<i64>, Option<i64>, Option<i64>) = sqlx::query_as(sql)
        .bind(user_id)
        .bind(threshold.to_string())
        .fetch_one(conn.pool())
        .await?;

    Ok(UsageStats {
        total_requests: row.0.unwrap_or(0),
        total_prompt_tokens: row.1.unwrap_or(0),
        total_completion_tokens: row.2.unwrap_or(0),
        total_cost_nano: row.3.unwrap_or(0),
    })
}

/// Get usage statistics grouped by model for a user over a time period
pub async fn get_usage_stats_by_model(
    db: &Database,
    user_id: &str,
    _period: &str,
) -> Result<Vec<ModelUsageStats>> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    let sql = if is_postgres {
        r#"
        SELECT
            COALESCE(model, 'Unknown') as model,
            COUNT(*) as requests,
            COALESCE(SUM(prompt_tokens), 0) as prompt_tokens,
            COALESCE(SUM(completion_tokens), 0) as completion_tokens,
            COALESCE(SUM(cache_read_tokens), 0) as cache_read_tokens,
            COALESCE(SUM(reasoning_tokens), 0) as reasoning_tokens,
            COALESCE(SUM(cost), 0) as cost
        FROM router_logs
        WHERE user_id = $1
        GROUP BY model
        ORDER BY cost DESC
        "#
    } else {
        r#"
        SELECT
            COALESCE(model, 'Unknown') as model,
            COUNT(*) as requests,
            COALESCE(SUM(prompt_tokens), 0) as prompt_tokens,
            COALESCE(SUM(completion_tokens), 0) as completion_tokens,
            COALESCE(SUM(cache_read_tokens), 0) as cache_read_tokens,
            COALESCE(SUM(reasoning_tokens), 0) as reasoning_tokens,
            COALESCE(SUM(cost), 0) as cost
        FROM router_logs
        WHERE user_id = ?
        GROUP BY model
        ORDER BY cost DESC
        "#
    };

    let rows: Vec<(String, i64, i64, i64, i64, i64, i64)> = sqlx::query_as(sql)
        .bind(user_id)
        .fetch_all(conn.pool())
        .await?;

    Ok(rows
        .into_iter()
        .map(
            |(model, requests, prompt_tokens, completion_tokens, cache_read_tokens, reasoning_tokens, cost_nano)| ModelUsageStats {
                model,
                requests,
                prompt_tokens,
                completion_tokens,
                cache_read_tokens,
                reasoning_tokens,
                cost_nano,
            },
        )
        .collect())
}
