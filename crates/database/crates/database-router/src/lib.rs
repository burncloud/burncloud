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

        // Create Groups Tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS router_groups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                strategy TEXT NOT NULL DEFAULT 'round_robin',
                match_path TEXT NOT NULL
            );
            "#
        )
        .execute(conn.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS router_group_members (
                group_id TEXT NOT NULL,
                upstream_id TEXT NOT NULL,
                weight INTEGER NOT NULL DEFAULT 1,
                PRIMARY KEY (group_id, upstream_id)
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
                ('demo-openai', 'OpenAI Demo', 'https://api.openai.com', 'sk-demo', '/v1/chat/completions', 'Bearer', 0),
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
        let rows = sqlx::query_as::<_, DbUpstream>(
            "SELECT id, name, base_url, api_key, match_path, auth_type, priority FROM router_upstreams"
        )
        .fetch_all(conn.pool())
        .await?;
        Ok(rows)
    }

    pub async fn get_all_groups(db: &Database) -> Result<Vec<DbGroup>> {
        let conn = db.connection()?;
        let rows = sqlx::query_as::<_, DbGroup>(
            "SELECT id, name, strategy, match_path FROM router_groups"
        )
        .fetch_all(conn.pool())
        .await?;
        Ok(rows)
    }

    pub async fn get_group_members(db: &Database) -> Result<Vec<DbGroupMember>> {
        let conn = db.connection()?;
        let rows = sqlx::query_as::<_, DbGroupMember>(
            "SELECT group_id, upstream_id, weight FROM router_group_members"
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

    // CRUD for Upstreams
    pub async fn create_upstream(db: &Database, u: &DbUpstream) -> Result<()> {
        let conn = db.connection()?;
        sqlx::query(
            "INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type, priority) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&u.id).bind(&u.name).bind(&u.base_url).bind(&u.api_key).bind(&u.match_path).bind(&u.auth_type).bind(u.priority)
        .execute(conn.pool())
        .await?;
        Ok(())
    }

    pub async fn get_upstream(db: &Database, id: &str) -> Result<Option<DbUpstream>> {
        let conn = db.connection()?;
        let upstream = sqlx::query_as::<_, DbUpstream>(
            "SELECT id, name, base_url, api_key, match_path, auth_type, priority FROM router_upstreams WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(conn.pool())
        .await?;
        Ok(upstream)
    }

    pub async fn update_upstream(db: &Database, u: &DbUpstream) -> Result<()> {
        let conn = db.connection()?;
        sqlx::query(
            "UPDATE router_upstreams SET name=?, base_url=?, api_key=?, match_path=?, auth_type=?, priority=? WHERE id=?"
        )
        .bind(&u.name).bind(&u.base_url).bind(&u.api_key).bind(&u.match_path).bind(&u.auth_type).bind(u.priority).bind(&u.id)
        .execute(conn.pool())
        .await?;
        Ok(())
    }

    pub async fn delete_upstream(db: &Database, id: &str) -> Result<()> {
        let conn = db.connection()?;
        sqlx::query("DELETE FROM router_upstreams WHERE id = ?")
            .bind(id)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    // CRUD for Groups
    pub async fn create_group(db: &Database, g: &DbGroup) -> Result<()> {
        let conn = db.connection()?;
        sqlx::query(
            "INSERT INTO router_groups (id, name, strategy, match_path) VALUES (?, ?, ?, ?)"
        )
        .bind(&g.id).bind(&g.name).bind(&g.strategy).bind(&g.match_path)
        .execute(conn.pool())
        .await?;
        Ok(())
    }

    pub async fn delete_group(db: &Database, id: &str) -> Result<()> {
        let conn = db.connection()?;
        // Transaction would be better, but for now explicit order
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
    pub async fn set_group_members(db: &Database, group_id: &str, members: Vec<DbGroupMember>) -> Result<()> {
        let conn = db.connection()?;
        // 1. Clear existing
        sqlx::query("DELETE FROM router_group_members WHERE group_id = ?")
            .bind(group_id)
            .execute(conn.pool())
            .await?;
        
        // 2. Insert new
        for m in members {
            sqlx::query(
                "INSERT INTO router_group_members (group_id, upstream_id, weight) VALUES (?, ?, ?)"
            )
            .bind(group_id).bind(&m.upstream_id).bind(m.weight)
            .execute(conn.pool())
            .await?;
        }
        Ok(())
    }
}