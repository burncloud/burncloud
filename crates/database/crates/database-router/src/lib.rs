use burncloud_database::{Database, Result};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};

/// Helper module for PostgreSQL/SQLite SQL compatibility
mod sql_compat {
    /// Generate parameterized placeholders for SQL queries
    /// Returns ($1, $2, ...) for PostgreSQL or (?, ?, ...) for SQLite
    pub fn placeholders(db_kind: &str, count: usize) -> String {
        if db_kind == "postgres" {
            (1..=count)
                .map(|i| format!("${}", i))
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            vec!["?"; count].join(", ")
        }
    }

    /// Generate LIMIT OFFSET placeholders
    /// PostgreSQL: LIMIT $n OFFSET $m, SQLite: LIMIT ? OFFSET ?
    pub fn limit_offset_placeholders(db_kind: &str, start_index: usize) -> String {
        if db_kind == "postgres" {
            format!("LIMIT ${} OFFSET ${}", start_index, start_index + 1)
        } else {
            "LIMIT ? OFFSET ?".to_string()
        }
    }
}

/// Token validation result that distinguishes between invalid and expired tokens
#[derive(Debug, Clone)]
pub enum TokenValidationResult {
    Valid(DbToken),
    Invalid,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbUpstream {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub match_path: String,
    pub auth_type: String, // Stored as string: "Bearer", "XApiKey"
    #[sqlx(default)]
    pub priority: i32,
    #[sqlx(default)]
    pub protocol: String, // "openai", "gemini", "claude"
    pub param_override: Option<String>,
    pub header_override: Option<String>,
    #[sqlx(default)]
    pub api_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbToken {
    pub token: String,
    pub user_id: String,
    pub status: String, // "active", "disabled"
    #[sqlx(default)]
    pub quota_limit: i64, // -1 for unlimited
    #[sqlx(default)]
    pub used_quota: i64,
    #[sqlx(default)]
    pub expired_time: i64, // -1 for never expire, >0 = unix timestamp
    #[sqlx(default)]
    pub accessed_time: i64, // unix timestamp of last access
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbGroup {
    pub id: String,
    pub name: String,
    pub strategy: String, // "round_robin", "weighted"
    pub match_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbGroupMember {
    pub group_id: String,
    pub upstream_id: String,
    pub weight: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbRouterLog {
    pub request_id: String,
    pub user_id: Option<String>,
    pub path: String,
    pub upstream_id: Option<String>,
    pub status_code: i32,
    pub latency_ms: i64,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    #[sqlx(default)]
    /// Cost in nanodollars (9 decimal precision)
    pub cost: i64,
    // created_at is handled by DB default
}

// DbUser etc are moved to burncloud-database-user

pub struct RouterDatabase;

impl RouterDatabase {
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
                    cost REAL DEFAULT 0,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
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

        // Default demo token seeding removed to prevent "sk-burncloud-demo" from reappearing
        // whenever the user clears their keys.

        Ok(())
    }

    pub async fn insert_log(db: &Database, log: &DbRouterLog) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let placeholders = sql_compat::placeholders(&db.kind(), 9);
        let sql = format!(
            r#"
            INSERT INTO router_logs
            (request_id, user_id, path, upstream_id, status_code, latency_ms, prompt_tokens, completion_tokens, cost)
            VALUES ({})
            "#,
            placeholders
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
            .execute(conn.pool())
            .await?;

        if let Some(user_id) = &log.user_id {
            let total_tokens = log.prompt_tokens + log.completion_tokens;
            if total_tokens > 0 {
                let update_sql = if is_postgres {
                    "UPDATE router_tokens SET used_quota = used_quota + $1 WHERE user_id = $2"
                } else {
                    "UPDATE router_tokens SET used_quota = used_quota + ? WHERE user_id = ?"
                };
                sqlx::query(update_sql)
                    .bind(total_tokens)
                    .bind(user_id)
                    .execute(conn.pool())
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn get_all_upstreams(db: &Database) -> Result<Vec<DbUpstream>> {
        let conn = db.get_connection()?;
        let rows = sqlx::query_as::<_, DbUpstream>(
            "SELECT id, name, base_url, api_key, match_path, auth_type, priority, protocol, param_override, header_override FROM router_upstreams"
        )
        .fetch_all(conn.pool())
        .await?;
        Ok(rows)
    }

    pub async fn get_all_groups(db: &Database) -> Result<Vec<DbGroup>> {
        let conn = db.get_connection()?;
        let rows = sqlx::query_as::<_, DbGroup>(
            "SELECT id, name, strategy, match_path FROM router_groups",
        )
        .fetch_all(conn.pool())
        .await?;
        Ok(rows)
    }

    pub async fn get_group_by_id(db: &Database, id: &str) -> Result<Option<DbGroup>> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT id, name, strategy, match_path FROM router_groups WHERE id = $1"
        } else {
            "SELECT id, name, strategy, match_path FROM router_groups WHERE id = ?"
        };
        let group = sqlx::query_as::<_, DbGroup>(sql)
            .bind(id)
            .fetch_optional(conn.pool())
            .await?;
        Ok(group)
    }

    pub async fn get_group_members(db: &Database) -> Result<Vec<DbGroupMember>> {
        let conn = db.get_connection()?;
        let rows = sqlx::query_as::<_, DbGroupMember>(
            "SELECT group_id, upstream_id, weight FROM router_group_members",
        )
        .fetch_all(conn.pool())
        .await?;
        Ok(rows)
    }

    pub async fn get_group_members_by_group(
        db: &Database,
        group_id: &str,
    ) -> Result<Vec<DbGroupMember>> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT group_id, upstream_id, weight FROM router_group_members WHERE group_id = $1"
        } else {
            "SELECT group_id, upstream_id, weight FROM router_group_members WHERE group_id = ?"
        };
        let rows = sqlx::query_as::<_, DbGroupMember>(sql)
            .bind(group_id)
            .fetch_all(conn.pool())
            .await?;
        Ok(rows)
    }

    pub async fn validate_token(db: &Database, token: &str) -> Result<Option<DbToken>> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time FROM router_tokens WHERE token = $1 AND status = 'active'"
        } else {
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time FROM router_tokens WHERE token = ? AND status = 'active'"
        };
        let token = sqlx::query_as::<_, DbToken>(sql)
            .bind(token)
            .fetch_optional(conn.pool())
            .await?;

        // Check if token is expired
        if let Some(ref t) = token {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            // expired_time > 0 means it has an expiration time
            // If current time exceeds expiration, token is expired
            if t.expired_time > 0 && now > t.expired_time {
                return Ok(None); // Token expired
            }
        }

        Ok(token)
    }

    /// Validates a token and returns detailed result distinguishing between invalid and expired
    pub async fn validate_token_detailed(
        db: &Database,
        token: &str,
    ) -> Result<TokenValidationResult> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time FROM router_tokens WHERE token = $1 AND status = 'active'"
        } else {
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time FROM router_tokens WHERE token = ? AND status = 'active'"
        };
        let token = sqlx::query_as::<_, DbToken>(sql)
            .bind(token)
            .fetch_optional(conn.pool())
            .await?;

        match token {
            Some(t) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;

                // expired_time > 0 means it has an expiration time
                // If current time exceeds expiration, token is expired
                if t.expired_time > 0 && now > t.expired_time {
                    Ok(TokenValidationResult::Expired)
                } else {
                    Ok(TokenValidationResult::Valid(t))
                }
            }
            None => Ok(TokenValidationResult::Invalid),
        }
    }

    /// Update the accessed_time for a token (non-blocking, best-effort)
    pub async fn update_token_accessed_time(db: &Database, token: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let sql = if db.kind() == "postgres" {
            "UPDATE router_tokens SET accessed_time = $1 WHERE token = $2"
        } else {
            "UPDATE router_tokens SET accessed_time = ? WHERE token = ?"
        };

        sqlx::query(sql)
            .bind(now)
            .bind(token)
            .execute(conn.pool())
            .await?;
        Ok(())
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

    // CRUD for Upstreams
    pub async fn create_upstream(db: &Database, u: &DbUpstream) -> Result<()> {
        let conn = db.get_connection()?;
        let placeholders = sql_compat::placeholders(&db.kind(), 10);
        let sql = format!(
            "INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type, priority, protocol, param_override, header_override) VALUES ({})",
            placeholders
        );
        sqlx::query(&sql)
            .bind(&u.id)
            .bind(&u.name)
            .bind(&u.base_url)
            .bind(&u.api_key)
            .bind(&u.match_path)
            .bind(&u.auth_type)
            .bind(u.priority)
            .bind(&u.protocol)
            .bind(&u.param_override)
            .bind(&u.header_override)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    pub async fn get_upstream(db: &Database, id: &str) -> Result<Option<DbUpstream>> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT id, name, base_url, api_key, match_path, auth_type, priority, protocol, param_override, header_override FROM router_upstreams WHERE id = $1"
        } else {
            "SELECT id, name, base_url, api_key, match_path, auth_type, priority, protocol, param_override, header_override FROM router_upstreams WHERE id = ?"
        };
        let upstream = sqlx::query_as::<_, DbUpstream>(sql)
            .bind(id)
            .fetch_optional(conn.pool())
            .await?;
        Ok(upstream)
    }

    pub async fn update_upstream(db: &Database, u: &DbUpstream) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "UPDATE router_upstreams SET name=$1, base_url=$2, api_key=$3, match_path=$4, auth_type=$5, priority=$6, protocol=$7 WHERE id=$8"
        } else {
            "UPDATE router_upstreams SET name=?, base_url=?, api_key=?, match_path=?, auth_type=?, priority=?, protocol=? WHERE id=?"
        };
        sqlx::query(sql)
            .bind(&u.name)
            .bind(&u.base_url)
            .bind(&u.api_key)
            .bind(&u.match_path)
            .bind(&u.auth_type)
            .bind(u.priority)
            .bind(&u.protocol)
            .bind(&u.id)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_upstream(db: &Database, id: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "DELETE FROM router_upstreams WHERE id = $1"
        } else {
            "DELETE FROM router_upstreams WHERE id = ?"
        };
        sqlx::query(sql).bind(id).execute(conn.pool()).await?;
        Ok(())
    }

    // CRUD for Groups
    pub async fn create_group(db: &Database, g: &DbGroup) -> Result<()> {
        let conn = db.get_connection()?;
        let placeholders = sql_compat::placeholders(&db.kind(), 4);
        let sql = format!(
            "INSERT INTO router_groups (id, name, strategy, match_path) VALUES ({})",
            placeholders
        );
        sqlx::query(&sql)
            .bind(&g.id)
            .bind(&g.name)
            .bind(&g.strategy)
            .bind(&g.match_path)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_group(db: &Database, id: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let sql_members = if db.kind() == "postgres" {
            "DELETE FROM router_group_members WHERE group_id = $1"
        } else {
            "DELETE FROM router_group_members WHERE group_id = ?"
        };
        sqlx::query(sql_members)
            .bind(id)
            .execute(conn.pool())
            .await?;

        let sql_group = if db.kind() == "postgres" {
            "DELETE FROM router_groups WHERE id = $1"
        } else {
            "DELETE FROM router_groups WHERE id = ?"
        };
        sqlx::query(sql_group).bind(id).execute(conn.pool()).await?;
        Ok(())
    }

    // Full replace of members for a group
    pub async fn set_group_members(
        db: &Database,
        group_id: &str,
        members: Vec<DbGroupMember>,
    ) -> Result<()> {
        let conn = db.get_connection()?;
        let delete_sql = if db.kind() == "postgres" {
            "DELETE FROM router_group_members WHERE group_id = $1"
        } else {
            "DELETE FROM router_group_members WHERE group_id = ?"
        };
        sqlx::query(delete_sql)
            .bind(group_id)
            .execute(conn.pool())
            .await?;

        let insert_placeholders = sql_compat::placeholders(&db.kind(), 3);
        let insert_sql = format!(
            "INSERT INTO router_group_members (group_id, upstream_id, weight) VALUES ({})",
            insert_placeholders
        );

        for m in members {
            sqlx::query(&insert_sql)
                .bind(group_id)
                .bind(&m.upstream_id)
                .bind(m.weight)
                .execute(conn.pool())
                .await?;
        }
        Ok(())
    }

    // CRUD for Tokens
    pub async fn list_tokens(db: &Database) -> Result<Vec<DbToken>> {
        let conn = db.get_connection()?;
        let tokens = sqlx::query_as::<_, DbToken>(
            "SELECT token, user_id, status, quota_limit, used_quota, expired_time, accessed_time FROM router_tokens",
        )
        .fetch_all(conn.pool())
        .await?;
        Ok(tokens)
    }

    pub async fn create_token(db: &Database, t: &DbToken) -> Result<()> {
        let conn = db.get_connection()?;
        let placeholders = sql_compat::placeholders(&db.kind(), 7);
        let sql = format!(
            "INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota, expired_time, accessed_time) VALUES ({})",
            placeholders
        );
        sqlx::query(&sql)
            .bind(&t.token)
            .bind(&t.user_id)
            .bind(&t.status)
            .bind(t.quota_limit)
            .bind(t.used_quota)
            .bind(t.expired_time)
            .bind(t.accessed_time)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_token(db: &Database, token: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "DELETE FROM router_tokens WHERE token = $1"
        } else {
            "DELETE FROM router_tokens WHERE token = ?"
        };
        sqlx::query(sql).bind(token).execute(conn.pool()).await?;
        Ok(())
    }

    pub async fn update_token_status(db: &Database, token: &str, status: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "UPDATE router_tokens SET status = $1 WHERE token = $2"
        } else {
            "UPDATE router_tokens SET status = ? WHERE token = ?"
        };
        sqlx::query(sql)
            .bind(status)
            .bind(token)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    pub async fn get_logs(db: &Database, limit: i32, offset: i32) -> Result<Vec<DbRouterLog>> {
        let conn = db.get_connection()?;
        let limit_offset = sql_compat::limit_offset_placeholders(&db.kind(), 1);
        let sql = format!(
            "SELECT request_id, user_id, path, upstream_id, status_code, latency_ms, prompt_tokens, completion_tokens, cost FROM router_logs ORDER BY created_at DESC {}",
            limit_offset
        );
        let logs = sqlx::query_as::<_, DbRouterLog>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(conn.pool())
            .await?;
        Ok(logs)
    }

    pub async fn get_usage_by_user(db: &Database, user_id: &str) -> Result<(i64, i64)> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT SUM(prompt_tokens), SUM(completion_tokens) FROM router_logs WHERE user_id = $1"
        } else {
            "SELECT SUM(prompt_tokens), SUM(completion_tokens) FROM router_logs WHERE user_id = ?"
        };
        let row: (Option<i64>, Option<i64>) = sqlx::query_as(sql)
            .bind(user_id)
            .fetch_one(conn.pool())
            .await?;

        Ok((row.0.unwrap_or(0), row.1.unwrap_or(0)))
    }

    /// Deduct quota from both user and token atomically.
    /// Cost is in quota units (typically 1 quota = 1 token, or can be scaled).
    /// Returns Ok(true) if deduction successful, Ok(false) if insufficient quota.
    /// Cost parameter uses i64 nanodollars for precision.
    pub async fn deduct_quota(
        db: &Database,
        _user_id: &str,
        token: &str,
        cost: i64,
    ) -> Result<bool> {
        let conn = db.get_connection()?;
        let cost_i64 = cost;
        let is_postgres = db.kind() == "postgres";

        if cost_i64 <= 0 {
            return Ok(true);
        }

        // Start transaction
        let mut tx = conn.pool().begin().await?;

        // Check if token has unlimited quota
        let unlimited_sql = if is_postgres {
            "SELECT COALESCE(unlimited_quota, 0) FROM router_tokens WHERE token = $1"
        } else {
            "SELECT COALESCE(unlimited_quota, 0) FROM router_tokens WHERE token = ?"
        };
        let unlimited: bool = sqlx::query_scalar(unlimited_sql)
            .bind(token)
            .fetch_one(&mut *tx)
            .await
            .unwrap_or(0)
            != 0;

        if unlimited {
            // Unlimited quota - just update used_quota for tracking
            let update_sql = if is_postgres {
                "UPDATE router_tokens SET used_quota = used_quota + $1 WHERE token = $2"
            } else {
                "UPDATE router_tokens SET used_quota = used_quota + ? WHERE token = ?"
            };
            sqlx::query(update_sql)
                .bind(cost_i64)
                .bind(token)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok(true);
        }

        // Check token quota
        let quota_sql = if is_postgres {
            "SELECT quota_limit, used_quota FROM router_tokens WHERE token = $1"
        } else {
            "SELECT quota_limit, used_quota FROM router_tokens WHERE token = ?"
        };
        let token_quota: Option<(i64, i64)> = sqlx::query_as(quota_sql)
            .bind(token)
            .fetch_optional(&mut *tx)
            .await?;

        if let Some((quota_limit, used_quota)) = token_quota {
            // quota_limit = -1 means unlimited for token
            if quota_limit >= 0 && used_quota + cost_i64 > quota_limit {
                tx.rollback().await?;
                return Ok(false);
            }
        }

        // Check user quota from users table (if it exists)
        // Note: users table is managed by database-user crate
        // For now, we just check token quota
        // TODO(issue): Integrate with user-level quota checking
        //   - Requires architectural decision on cross-crate data access
        //   - Options: 1) Add database-router -> database-user dependency
        //             2) Create shared quota service in service layer
        //             3) Pass user quota as parameter from router layer

        // Deduct from token
        let deduct_sql = if is_postgres {
            "UPDATE router_tokens SET used_quota = used_quota + $1 WHERE token = $2"
        } else {
            "UPDATE router_tokens SET used_quota = used_quota + ? WHERE token = ?"
        };
        sqlx::query(deduct_sql)
            .bind(cost_i64)
            .bind(token)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(true)
    }

    /// Check if quota is sufficient without deducting.
    /// Cost parameter uses i64 nanodollars for precision.
    pub async fn check_quota(db: &Database, token: &str, cost: i64) -> Result<bool> {
        let conn = db.get_connection()?;
        let cost_i64 = cost;
        let is_postgres = db.kind() == "postgres";

        if cost_i64 <= 0 {
            return Ok(true);
        }

        // Check if token has unlimited quota
        let unlimited_sql = if is_postgres {
            "SELECT COALESCE(unlimited_quota, 0) FROM router_tokens WHERE token = $1"
        } else {
            "SELECT COALESCE(unlimited_quota, 0) FROM router_tokens WHERE token = ?"
        };
        let unlimited: bool = sqlx::query_scalar(unlimited_sql)
            .bind(token)
            .fetch_one(conn.pool())
            .await
            .unwrap_or(0)
            != 0;

        if unlimited {
            return Ok(true);
        }

        // Check token quota
        let quota_sql = if is_postgres {
            "SELECT quota_limit, used_quota FROM router_tokens WHERE token = $1"
        } else {
            "SELECT quota_limit, used_quota FROM router_tokens WHERE token = ?"
        };
        let token_quota: Option<(i64, i64)> = sqlx::query_as(quota_sql)
            .bind(token)
            .fetch_optional(conn.pool())
            .await?;

        if let Some((quota_limit, used_quota)) = token_quota {
            // quota_limit = -1 means unlimited
            if quota_limit >= 0 && used_quota + cost_i64 > quota_limit {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Deduct from USD balance.
    /// Cost is in nanodollars (i64).
    /// Returns Ok(true) if deduction successful, Ok(false) if insufficient balance.
    pub async fn deduct_usd(db: &Database, user_id: &str, cost_nano: i64) -> Result<bool> {
        if cost_nano <= 0 {
            return Ok(true);
        }

        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        // Check current balance
        let balance_sql = if is_postgres {
            "SELECT COALESCE(balance_usd, 0) FROM users WHERE id = $1"
        } else {
            "SELECT COALESCE(balance_usd, 0) FROM users WHERE id = ?"
        };
        let balance: i64 = sqlx::query_scalar(balance_sql)
            .bind(user_id)
            .fetch_one(conn.pool())
            .await
            .unwrap_or(0);

        if balance < cost_nano {
            return Ok(false);
        }

        // Deduct
        let deduct_sql = if is_postgres {
            "UPDATE users SET balance_usd = balance_usd - $1 WHERE id = $2 AND balance_usd >= $3"
        } else {
            "UPDATE users SET balance_usd = balance_usd - ? WHERE id = ? AND balance_usd >= ?"
        };
        let rows_affected = sqlx::query(deduct_sql)
            .bind(cost_nano)
            .bind(user_id)
            .bind(cost_nano)
            .execute(conn.pool())
            .await?
            .rows_affected();

        Ok(rows_affected > 0)
    }

    /// Deduct from CNY balance.
    /// Cost is in nanodollars (i64).
    /// Returns Ok(true) if deduction successful, Ok(false) if insufficient balance.
    pub async fn deduct_cny(db: &Database, user_id: &str, cost_nano: i64) -> Result<bool> {
        if cost_nano <= 0 {
            return Ok(true);
        }

        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        // Check current balance
        let balance_sql = if is_postgres {
            "SELECT COALESCE(balance_cny, 0) FROM users WHERE id = $1"
        } else {
            "SELECT COALESCE(balance_cny, 0) FROM users WHERE id = ?"
        };
        let balance: i64 = sqlx::query_scalar(balance_sql)
            .bind(user_id)
            .fetch_one(conn.pool())
            .await
            .unwrap_or(0);

        if balance < cost_nano {
            return Ok(false);
        }

        // Deduct
        let deduct_sql = if is_postgres {
            "UPDATE users SET balance_cny = balance_cny - $1 WHERE id = $2 AND balance_cny >= $3"
        } else {
            "UPDATE users SET balance_cny = balance_cny - ? WHERE id = ? AND balance_cny >= ?"
        };
        let rows_affected = sqlx::query(deduct_sql)
            .bind(cost_nano)
            .bind(user_id)
            .bind(cost_nano)
            .execute(conn.pool())
            .await?
            .rows_affected();

        Ok(rows_affected > 0)
    }

    /// Deduct cost from dual-currency wallet.
    /// Uses the primary currency first (based on cost_currency), then converts from secondary if needed.
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `user_id` - User ID to deduct from
    /// * `cost_nano` - Cost in nanodollars (i64)
    /// * `cost_currency` - Currency of the cost ("USD" or "CNY")
    /// * `exchange_rate_nano` - Exchange rate scaled by 10^9 (e.g., 7.24 CNY/USD = 7_240_000_000)
    ///
    /// # Returns
    /// Ok(true) if deduction successful, Ok(false) if insufficient balance across both currencies.
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
