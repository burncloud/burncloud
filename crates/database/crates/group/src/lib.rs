//! Database group crate for BurnCloud
//!
//! This crate provides database model implementations for group management,
//! including router_groups and group_members tables.

use burncloud_database::{adapt_sql, Database, DatabaseError, Result};
use serde::{Deserialize, Serialize};
use sqlx::Row;

/// Group model for router_groups table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterGroup {
    pub id: String,
    pub name: String,
    pub strategy: String,
    pub match_path: String,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// Group member model for group_members table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub id: i64,
    pub group_id: String,
    pub upstream_id: i32,
    pub weight: i32,
    pub created_at: Option<i64>,
}

/// Input type for creating/updating group members
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMemberInput {
    pub upstream_id: i32,
    pub weight: i32,
}

fn current_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Database operations for router_groups
pub struct RouterGroupModel;

impl RouterGroupModel {
    /// Initialize group tables
    pub async fn init(db: &Database) -> Result<()> {
        let conn = db.get_connection()?;
        let kind = db.kind();

        let groups_sql = match kind.as_str() {
            "sqlite" => {
                r#"
                CREATE TABLE IF NOT EXISTS router_groups (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    strategy TEXT NOT NULL DEFAULT 'round_robin',
                    match_path TEXT NOT NULL DEFAULT '/v1/chat/completions',
                    created_at INTEGER DEFAULT (strftime('%s', 'now')),
                    updated_at INTEGER DEFAULT (strftime('%s', 'now'))
                );
                CREATE INDEX IF NOT EXISTS idx_router_groups_name ON router_groups(name);
                "#
            }
            "postgres" => {
                r#"
                CREATE TABLE IF NOT EXISTS router_groups (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    strategy TEXT NOT NULL DEFAULT 'round_robin',
                    match_path TEXT NOT NULL DEFAULT '/v1/chat/completions',
                    created_at BIGINT DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT),
                    updated_at BIGINT DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT)
                );
                CREATE INDEX IF NOT EXISTS idx_router_groups_name ON router_groups(name);
                "#
            }
            _ => unreachable!("Unsupported database kind"),
        };

        let members_sql = match kind.as_str() {
            "sqlite" => {
                r#"
                CREATE TABLE IF NOT EXISTS group_members (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    group_id TEXT NOT NULL,
                    upstream_id INTEGER NOT NULL,
                    weight INTEGER NOT NULL DEFAULT 1,
                    created_at INTEGER DEFAULT (strftime('%s', 'now')),
                    FOREIGN KEY (group_id) REFERENCES router_groups(id) ON DELETE CASCADE,
                    FOREIGN KEY (upstream_id) REFERENCES channel_providers(id) ON DELETE CASCADE
                );
                CREATE INDEX IF NOT EXISTS idx_group_members_group_id ON group_members(group_id);
                CREATE INDEX IF NOT EXISTS idx_group_members_upstream_id ON group_members(upstream_id);
                "#
            }
            "postgres" => {
                r#"
                CREATE TABLE IF NOT EXISTS group_members (
                    id BIGSERIAL PRIMARY KEY,
                    group_id TEXT NOT NULL,
                    upstream_id INTEGER NOT NULL,
                    weight INTEGER NOT NULL DEFAULT 1,
                    created_at BIGINT DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT),
                    FOREIGN KEY (group_id) REFERENCES router_groups(id) ON DELETE CASCADE,
                    FOREIGN KEY (upstream_id) REFERENCES channel_providers(id) ON DELETE CASCADE
                );
                CREATE INDEX IF NOT EXISTS idx_group_members_group_id ON group_members(group_id);
                CREATE INDEX IF NOT EXISTS idx_group_members_upstream_id ON group_members(upstream_id);
                "#
            }
            _ => unreachable!("Unsupported database kind"),
        };

        sqlx::query(groups_sql).execute(conn.pool()).await?;
        sqlx::query(members_sql).execute(conn.pool()).await?;

        Ok(())
    }

    /// List all groups
    pub async fn list(db: &Database) -> Result<Vec<RouterGroup>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = adapt_sql(is_postgres, "SELECT id, name, strategy, match_path, created_at, updated_at FROM router_groups ORDER BY name");
        let rows = sqlx::query(&sql)
            .fetch_all(conn.pool())
            .await?;
        
        let groups: Vec<RouterGroup> = rows
            .into_iter()
            .map(|row| RouterGroup {
                id: row.get("id"),
                name: row.get("name"),
                strategy: row.get("strategy"),
                match_path: row.get("match_path"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();
        
        Ok(groups)
    }

    /// Get a group by ID
    pub async fn get_by_id(db: &Database, id: &str) -> Result<Option<RouterGroup>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = adapt_sql(is_postgres, "SELECT id, name, strategy, match_path, created_at, updated_at FROM router_groups WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(conn.pool())
            .await?;
        
        Ok(row.map(|r| RouterGroup {
            id: r.get("id"),
            name: r.get("name"),
            strategy: r.get("strategy"),
            match_path: r.get("match_path"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    /// Create a new group
    pub async fn create(db: &Database, group: &RouterGroup) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let now = current_timestamp();
        let sql = adapt_sql(is_postgres, "INSERT INTO router_groups (id, name, strategy, match_path, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)");
        sqlx::query(&sql)
            .bind(&group.id)
            .bind(&group.name)
            .bind(&group.strategy)
            .bind(&group.match_path)
            .bind(now)
            .bind(now)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    /// Update a group
    pub async fn update(db: &Database, group: &RouterGroup) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let now = current_timestamp();
        let sql = adapt_sql(is_postgres, "UPDATE router_groups SET name = ?, strategy = ?, match_path = ?, updated_at = ? WHERE id = ?");
        let result = sqlx::query(&sql)
            .bind(&group.name)
            .bind(&group.strategy)
            .bind(&group.match_path)
            .bind(now)
            .bind(&group.id)
            .execute(conn.pool())
            .await?;
        if result.rows_affected() == 0 {
            return Err(DatabaseError::Query(format!("Group not found: {}", group.id)));
        }
        Ok(())
    }

    /// Delete a group by ID
    pub async fn delete(db: &Database, id: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = adapt_sql(is_postgres, "DELETE FROM router_groups WHERE id = ?");
        let result = sqlx::query(&sql)
            .bind(id)
            .execute(conn.pool())
            .await?;
        if result.rows_affected() == 0 {
            return Err(DatabaseError::Query(format!("Group not found: {}", id)));
        }
        Ok(())
    }
}

/// Database operations for group_members
pub struct GroupMemberModel;

impl GroupMemberModel {
    /// List all members for a group
    pub async fn list_by_group(db: &Database, group_id: &str) -> Result<Vec<GroupMember>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = adapt_sql(is_postgres, "SELECT id, group_id, upstream_id, weight, created_at FROM group_members WHERE group_id = ? ORDER BY upstream_id");
        let rows = sqlx::query(&sql)
            .bind(group_id)
            .fetch_all(conn.pool())
            .await?;
        
        let members: Vec<GroupMember> = rows
            .into_iter()
            .map(|row| GroupMember {
                id: row.get("id"),
                group_id: row.get("group_id"),
                upstream_id: row.get("upstream_id"),
                weight: row.get("weight"),
                created_at: row.get("created_at"),
            })
            .collect();
        Ok(members)
    }

    /// Delete all members for a group
    pub async fn delete_by_group(db: &Database, group_id: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = adapt_sql(is_postgres, "DELETE FROM group_members WHERE group_id = ?");
        sqlx::query(&sql)
            .bind(group_id)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    /// Add members to a group
    pub async fn add_members(db: &Database, group_id: &str, members: &[GroupMemberInput]) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let now = current_timestamp();
        for member in members {
            let sql = adapt_sql(is_postgres, "INSERT INTO group_members (group_id, upstream_id, weight, created_at) VALUES (?, ?, ?, ?)");
            sqlx::query(&sql)
                .bind(group_id)
                .bind(member.upstream_id)
                .bind(member.weight)
                .bind(now)
                .execute(conn.pool())
                .await?;
        }
        Ok(())
    }

    /// Update members for a group (delete all and re-add)
    pub async fn update_members(db: &Database, group_id: &str, members: &[GroupMemberInput]) -> Result<()> {
        Self::delete_by_group(db, group_id).await?;
        Self::add_members(db, group_id, members).await?;
        Ok(())
    }
}
