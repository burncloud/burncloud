//! Database operations for upstream/channel configuration
//!
//! This crate handles all database operations related to upstream services
//! (also known as channels in the API layer).

use burncloud_database::{Database, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Upstream service configuration
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

pub struct UpstreamModel;

impl UpstreamModel {
    /// Get all upstreams
    pub async fn get_all(db: &Database) -> Result<Vec<DbUpstream>> {
        let conn = db.get_connection()?;
        let rows = sqlx::query_as::<_, DbUpstream>(
            "SELECT id, name, base_url, api_key, match_path, auth_type, priority, protocol, param_override, header_override, api_version FROM router_upstreams"
        )
        .fetch_all(conn.pool())
        .await?;
        Ok(rows)
    }

    /// Get a single upstream by ID
    pub async fn get(db: &Database, id: &str) -> Result<Option<DbUpstream>> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT id, name, base_url, api_key, match_path, auth_type, priority, protocol, param_override, header_override, api_version FROM router_upstreams WHERE id = $1"
        } else {
            "SELECT id, name, base_url, api_key, match_path, auth_type, priority, protocol, param_override, header_override, api_version FROM router_upstreams WHERE id = ?"
        };
        let upstream = sqlx::query_as::<_, DbUpstream>(sql)
            .bind(id)
            .fetch_optional(conn.pool())
            .await?;
        Ok(upstream)
    }

    /// Create a new upstream
    pub async fn create(db: &Database, u: &DbUpstream) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let placeholders = if is_postgres {
            "$1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11"
        } else {
            "?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?"
        };
        let sql = format!(
            "INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type, priority, protocol, param_override, header_override, api_version) VALUES ({})",
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
            .bind(&u.api_version)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    /// Update an existing upstream
    pub async fn update(db: &Database, u: &DbUpstream) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "UPDATE router_upstreams SET name=$1, base_url=$2, api_key=$3, match_path=$4, auth_type=$5, priority=$6, protocol=$7, param_override=$8, header_override=$9, api_version=$10 WHERE id=$11"
        } else {
            "UPDATE router_upstreams SET name=?, base_url=?, api_key=?, match_path=?, auth_type=?, priority=?, protocol=?, param_override=?, header_override=?, api_version=? WHERE id=?"
        };
        sqlx::query(sql)
            .bind(&u.name)
            .bind(&u.base_url)
            .bind(&u.api_key)
            .bind(&u.match_path)
            .bind(&u.auth_type)
            .bind(u.priority)
            .bind(&u.protocol)
            .bind(&u.param_override)
            .bind(&u.header_override)
            .bind(&u.api_version)
            .bind(&u.id)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    /// Delete an upstream
    pub async fn delete(db: &Database, id: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "DELETE FROM router_upstreams WHERE id = $1"
        } else {
            "DELETE FROM router_upstreams WHERE id = ?"
        };
        sqlx::query(sql).bind(id).execute(conn.pool()).await?;
        Ok(())
    }
}
