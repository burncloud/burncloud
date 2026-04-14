//! Database operations for group management
//!
//! This crate handles all database operations related to upstream groups,
//! which are used for load balancing and routing strategies.

use burncloud_common::CrudRepository;
use burncloud_database::{adapt_sql, phs, Database, DatabaseError, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Upstream group configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbGroup {
    pub id: String,
    pub name: String,
    pub strategy: String, // "round_robin", "weighted"
    pub match_path: String,
}

/// Group member configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbGroupMember {
    pub group_id: String,
    pub upstream_id: String,
    pub weight: i32,
}

pub struct GroupModel;

impl GroupModel {
    /// Get all groups
    pub async fn get_all(db: &Database) -> Result<Vec<DbGroup>> {
        let conn = db.get_connection()?;
        let rows = sqlx::query_as::<_, DbGroup>(
            "SELECT id, name, strategy, match_path FROM router_groups",
        )
        .fetch_all(conn.pool())
        .await?;
        Ok(rows)
    }

    /// Get a single group by ID
    pub async fn get(db: &Database, id: &str) -> Result<Option<DbGroup>> {
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

    /// Create a new group
    pub async fn create(db: &Database, g: &DbGroup) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = format!(
            "INSERT INTO router_groups (id, name, strategy, match_path) VALUES ({})",
            phs(is_postgres, 4)
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

    /// Delete a group and all its members
    pub async fn delete(db: &Database, id: &str) -> Result<()> {
        let conn = db.get_connection()?;

        // Delete members first
        let sql_members = if db.kind() == "postgres" {
            "DELETE FROM router_group_members WHERE group_id = $1"
        } else {
            "DELETE FROM router_group_members WHERE group_id = ?"
        };
        sqlx::query(sql_members)
            .bind(id)
            .execute(conn.pool())
            .await?;

        // Delete group
        let sql_group = if db.kind() == "postgres" {
            "DELETE FROM router_groups WHERE id = $1"
        } else {
            "DELETE FROM router_groups WHERE id = ?"
        };
        sqlx::query(sql_group).bind(id).execute(conn.pool()).await?;
        Ok(())
    }
}

pub struct GroupMemberModel;

impl GroupMemberModel {
    /// Get all group members
    pub async fn get_all(db: &Database) -> Result<Vec<DbGroupMember>> {
        let conn = db.get_connection()?;
        let rows = sqlx::query_as::<_, DbGroupMember>(
            "SELECT group_id, upstream_id, weight FROM router_group_members",
        )
        .fetch_all(conn.pool())
        .await?;
        Ok(rows)
    }

    /// Get members for a specific group
    pub async fn get_by_group(db: &Database, group_id: &str) -> Result<Vec<DbGroupMember>> {
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

    /// Set members for a group (full replace)
    pub async fn set_for_group(
        db: &Database,
        group_id: &str,
        members: Vec<DbGroupMember>,
    ) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        // Delete existing members
        let delete_sql = adapt_sql(is_postgres, "DELETE FROM router_group_members WHERE group_id = ?");
        sqlx::query(&delete_sql)
            .bind(group_id)
            .execute(conn.pool())
            .await?;

        // Insert new members
        let insert_sql = format!(
            "INSERT INTO router_group_members (group_id, upstream_id, weight) VALUES ({})",
            phs(is_postgres, 3)
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
}

/// Repository wrapper that implements the standard [`CrudRepository`] contract for groups.
///
/// Note: `GroupModel` has no update method — groups are replaced by delete + create.
/// The `update` implementation here mirrors that: delete then create with the new data
/// and the canonical `id`.
pub struct GroupRepository<'a>(pub &'a Database);

#[async_trait::async_trait]
impl<'a> CrudRepository<DbGroup, String, DatabaseError> for GroupRepository<'a> {
    async fn find_by_id(&self, id: &String) -> Result<Option<DbGroup>> {
        GroupModel::get(self.0, id).await
    }

    async fn list(&self) -> Result<Vec<DbGroup>> {
        GroupModel::get_all(self.0).await
    }

    async fn create(&self, input: &DbGroup) -> Result<DbGroup> {
        GroupModel::create(self.0, input).await?;
        GroupModel::get(self.0, &input.id)
            .await?
            .ok_or_else(|| DatabaseError::Query("group disappeared after insert".to_string()))
    }

    async fn update(&self, id: &String, input: &DbGroup) -> Result<bool> {
        let exists = GroupModel::get(self.0, id).await?.is_some();
        if !exists {
            return Ok(false);
        }
        GroupModel::delete(self.0, id).await?;
        let mut record = input.clone();
        record.id = id.clone();
        GroupModel::create(self.0, &record).await?;
        Ok(true)
    }

    async fn delete(&self, id: &String) -> Result<bool> {
        let exists = GroupModel::get(self.0, id).await?.is_some();
        if exists {
            GroupModel::delete(self.0, id).await?;
        }
        Ok(exists)
    }
}
