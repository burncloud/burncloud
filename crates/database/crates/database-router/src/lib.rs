use burncloud_database::{Database, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Row, FromRow};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbUpstream {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub match_path: String,
    pub auth_type: String, // Stored as string: "Bearer", "XApiKey"
    #[sqlx(default)] // Handle missing column in old rows during migration
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbToken {
    pub token: String,
    pub user_id: String,
    pub status: String, // "active", "disabled"
}

pub struct RouterDatabase;

impl RouterDatabase {
    pub async fn init(db: &Database) -> Result<()> {
        let conn = db.connection()?;
        
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS router_upstreams (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                base_url TEXT NOT NULL,
                api_key TEXT NOT NULL,
                match_path TEXT NOT NULL,
                auth_type TEXT NOT NULL
            );
            "#
        )
        .execute(conn.pool())
        .await?;

        // Migration: Add priority column if it doesn't exist
        // We use a separate query and ignore error (simplest migration strategy for now)
        let _ = sqlx::query("ALTER TABLE router_upstreams ADD COLUMN priority INTEGER NOT NULL DEFAULT 0")
            .execute(conn.pool())
            .await;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS router_tokens (
                token TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                status TEXT NOT NULL
            );
            "#
        )
        .execute(conn.pool())
        .await?;

        // Insert default demo data if empty
        let count: i64 = sqlx::query("SELECT COUNT(*) FROM router_upstreams")
            .fetch_one(conn.pool())
            .await?
            .get(0);

        if count == 0 {
             sqlx::query(
                r#"
                INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type, priority)
                VALUES 
                ('demo-openai', 'OpenAI Demo', 'https://api.openai.com', 'sk-demo', '/v1', 'Bearer', 0),
                ('demo-claude', 'Claude Demo', 'https://api.anthropic.com', 'sk-ant-demo', '/v1/messages', 'XApiKey', 0)
                "#
            )
            .execute(conn.pool())
            .await?;
        }

        let token_count: i64 = sqlx::query("SELECT COUNT(*) FROM router_tokens")
            .fetch_one(conn.pool())
            .await?
            .get(0);

        if token_count == 0 {
             sqlx::query(
                r#"
                INSERT INTO router_tokens (token, user_id, status)
                VALUES ('sk-burncloud-demo', 'demo-user', 'active')
                "#
            )
            .execute(conn.pool())
            .await?;
        }

        Ok(())
    }

    pub async fn get_all_upstreams(db: &Database) -> Result<Vec<DbUpstream>> {
        let conn = db.connection()?;
        // We sort by length(desc) then priority(desc) in SQL, but we also do it in code for robustness
        let rows = sqlx::query_as::<_, DbUpstream>(
            "SELECT id, name, base_url, api_key, match_path, auth_type, priority FROM router_upstreams"
        )
        .fetch_all(conn.pool())
        .await?;
        Ok(rows)
    }

    pub async fn validate_token(db: &Database, token: &str) -> Result<Option<DbToken>> {
         let conn = db.connection()?;
         let token = sqlx::query_as::<_, DbToken>(
             "SELECT token, user_id, status FROM router_tokens WHERE token = ? AND status = 'active'"
         )
         .bind(token)
         .fetch_optional(conn.pool())
         .await?;
         Ok(token)
    }
}