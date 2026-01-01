use burncloud_database::{Database, Result};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};

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
                    protocol TEXT NOT NULL DEFAULT 'openai'
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS router_tokens (
                    token TEXT PRIMARY KEY,
                    user_id TEXT NOT NULL,
                    status TEXT NOT NULL,
                    quota_limit INTEGER NOT NULL DEFAULT -1,
                    used_quota INTEGER NOT NULL DEFAULT 0
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
                    protocol TEXT NOT NULL DEFAULT 'openai'
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS router_tokens (
                    token TEXT PRIMARY KEY,
                    user_id TEXT NOT NULL,
                    status TEXT NOT NULL,
                    quota_limit BIGINT NOT NULL DEFAULT -1,
                    used_quota BIGINT NOT NULL DEFAULT 0
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

        sqlx::query(
            r#"
            INSERT INTO router_logs 
            (request_id, user_id, path, upstream_id, status_code, latency_ms, prompt_tokens, completion_tokens) 
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&log.request_id)
        .bind(&log.user_id)
        .bind(&log.path)
        .bind(&log.upstream_id)
        .bind(log.status_code)
        .bind(log.latency_ms)
        .bind(log.prompt_tokens)
        .bind(log.completion_tokens)
        .execute(conn.pool())
        .await?;

        if let Some(user_id) = &log.user_id {
            let total_tokens = log.prompt_tokens + log.completion_tokens;
            if total_tokens > 0 {
                sqlx::query(
                    "UPDATE router_tokens SET used_quota = used_quota + ? WHERE user_id = ?",
                )
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
            "SELECT id, name, base_url, api_key, match_path, auth_type, priority, protocol FROM router_upstreams"
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
        let rows = sqlx::query_as::<_, DbGroupMember>(
            "SELECT group_id, upstream_id, weight FROM router_group_members WHERE group_id = ?",
        )
        .bind(group_id)
        .fetch_all(conn.pool())
        .await?;
        Ok(rows)
    }

    pub async fn validate_token(db: &Database, token: &str) -> Result<Option<DbToken>> {
        let conn = db.get_connection()?;
        let token = sqlx::query_as::<_, DbToken>(
             "SELECT token, user_id, status, quota_limit, used_quota FROM router_tokens WHERE token = ? AND status = 'active'"
         )
         .bind(token)
         .fetch_optional(conn.pool())
         .await?;
        Ok(token)
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

        // Assuming tokens.key matches the bearer token
        // And tokens.user_id links to users.id
        let query = format!(
            r#"
            SELECT u.id, u.{}, t.remain_quota, t.used_quota 
            FROM tokens t 
            JOIN users u ON t.user_id = u.id 
            WHERE t.key = ? AND t.status = 1 AND u.status = 1
            "#,
            group_col
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
        sqlx::query(
            "INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type, priority, protocol) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&u.id).bind(&u.name).bind(&u.base_url).bind(&u.api_key).bind(&u.match_path).bind(&u.auth_type).bind(u.priority).bind(&u.protocol)
        .execute(conn.pool())
        .await?;
        Ok(())
    }

    pub async fn get_upstream(db: &Database, id: &str) -> Result<Option<DbUpstream>> {
        let conn = db.get_connection()?;
        let upstream = sqlx::query_as::<_, DbUpstream>(
            "SELECT id, name, base_url, api_key, match_path, auth_type, priority, protocol FROM router_upstreams WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(conn.pool())
        .await?;
        Ok(upstream)
    }

    pub async fn update_upstream(db: &Database, u: &DbUpstream) -> Result<()> {
        let conn = db.get_connection()?;
        sqlx::query(
            "UPDATE router_upstreams SET name=?, base_url=?, api_key=?, match_path=?, auth_type=?, priority=?, protocol=? WHERE id=?"
        )
        .bind(&u.name).bind(&u.base_url).bind(&u.api_key).bind(&u.match_path).bind(&u.auth_type).bind(u.priority).bind(&u.protocol).bind(&u.id)
        .execute(conn.pool())
        .await?;
        Ok(())
    }

    pub async fn delete_upstream(db: &Database, id: &str) -> Result<()> {
        let conn = db.get_connection()?;
        sqlx::query("DELETE FROM router_upstreams WHERE id = ?")
            .bind(id)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    // CRUD for Groups
    pub async fn create_group(db: &Database, g: &DbGroup) -> Result<()> {
        let conn = db.get_connection()?;
        sqlx::query(
            "INSERT INTO router_groups (id, name, strategy, match_path) VALUES (?, ?, ?, ?)",
        )
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
        sqlx::query("DELETE FROM router_group_members WHERE group_id = ?")
            .bind(id)
            .execute(conn.pool())
            .await?;

        sqlx::query("DELETE FROM router_groups WHERE id = ?")
            .bind(id)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    // Full replace of members for a group
    pub async fn set_group_members(
        db: &Database,
        group_id: &str,
        members: Vec<DbGroupMember>,
    ) -> Result<()> {
        let conn = db.get_connection()?;
        sqlx::query("DELETE FROM router_group_members WHERE group_id = ?")
            .bind(group_id)
            .execute(conn.pool())
            .await?;

        for m in members {
            sqlx::query(
                "INSERT INTO router_group_members (group_id, upstream_id, weight) VALUES (?, ?, ?)",
            )
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
            "SELECT token, user_id, status, quota_limit, used_quota FROM router_tokens",
        )
        .fetch_all(conn.pool())
        .await?;
        Ok(tokens)
    }

    pub async fn create_token(db: &Database, t: &DbToken) -> Result<()> {
        let conn = db.get_connection()?;
        sqlx::query(
            "INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&t.token).bind(&t.user_id).bind(&t.status).bind(t.quota_limit).bind(t.used_quota)
        .execute(conn.pool())
        .await?;
        Ok(())
    }

    pub async fn delete_token(db: &Database, token: &str) -> Result<()> {
        let conn = db.get_connection()?;
        sqlx::query("DELETE FROM router_tokens WHERE token = ?")
            .bind(token)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    pub async fn update_token_status(db: &Database, token: &str, status: &str) -> Result<()> {
        let conn = db.get_connection()?;
        sqlx::query("UPDATE router_tokens SET status = ? WHERE token = ?")
            .bind(status)
            .bind(token)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    pub async fn get_logs(db: &Database, limit: i32, offset: i32) -> Result<Vec<DbRouterLog>> {
        let conn = db.get_connection()?;
        let logs = sqlx::query_as::<_, DbRouterLog>(
            "SELECT request_id, user_id, path, upstream_id, status_code, latency_ms, prompt_tokens, completion_tokens FROM router_logs ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(conn.pool())
        .await?;
        Ok(logs)
    }

    pub async fn get_usage_by_user(db: &Database, user_id: &str) -> Result<(i64, i64)> {
        let conn = db.get_connection()?;
        let row: (Option<i64>, Option<i64>) = sqlx::query_as(
            "SELECT SUM(prompt_tokens), SUM(completion_tokens) FROM router_logs WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_one(conn.pool())
        .await?;

        Ok((row.0.unwrap_or(0), row.1.unwrap_or(0)))
    }
}
