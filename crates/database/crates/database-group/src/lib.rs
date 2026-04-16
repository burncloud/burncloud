//! Database operations for group management
//!
//! This crate handles all database operations related to upstream groups,
//! which are used for load balancing and routing strategies.

use burncloud_common::CrudRepository;
use burncloud_database::{adapt_sql, ph, phs, Database, DatabaseError, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Upstream group configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RouterGroup {
    pub id: String,
    pub name: String,
    pub strategy: String, // "round_robin", "weighted"
    pub match_path: String,
}

/// Group member configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RouterGroupMember {
    pub group_id: String,
    pub upstream_id: String,
    pub weight: i32,
}

pub struct RouterGroupModel;

impl RouterGroupModel {
    /// Get all groups
    pub async fn get_all(db: &Database) -> Result<Vec<RouterGroup>> {
        let conn = db.get_connection()?;
        let rows = sqlx::query_as::<_, RouterGroup>(
            "SELECT id, name, strategy, match_path FROM router_groups",
        )
        .fetch_all(conn.pool())
        .await?;
        Ok(rows)
    }

    /// Get a single group by ID
    pub async fn get(db: &Database, id: &str) -> Result<Option<RouterGroup>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = format!(
            "SELECT id, name, strategy, match_path FROM router_groups WHERE id = {}",
            ph(is_postgres, 1)
        );
        let group = sqlx::query_as::<_, RouterGroup>(&sql)
            .bind(id)
            .fetch_optional(conn.pool())
            .await?;
        Ok(group)
    }

    /// Create a new group
    pub async fn create(db: &Database, g: &RouterGroup) -> Result<()> {
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
        let is_postgres = db.kind() == "postgres";

        // Delete members first
        let sql_members = format!(
            "DELETE FROM router_group_members WHERE group_id = {}",
            ph(is_postgres, 1)
        );
        sqlx::query(&sql_members)
            .bind(id)
            .execute(conn.pool())
            .await?;

        // Delete group
        let sql_group = format!(
            "DELETE FROM router_groups WHERE id = {}",
            ph(is_postgres, 1)
        );
        sqlx::query(&sql_group)
            .bind(id)
            .execute(conn.pool())
            .await?;
        Ok(())
    }
}

pub struct RouterGroupMemberModel;

impl RouterGroupMemberModel {
    /// Get all group members
    pub async fn get_all(db: &Database) -> Result<Vec<RouterGroupMember>> {
        let conn = db.get_connection()?;
        let rows = sqlx::query_as::<_, RouterGroupMember>(
            "SELECT group_id, upstream_id, weight FROM router_group_members",
        )
        .fetch_all(conn.pool())
        .await?;
        Ok(rows)
    }

    /// Get members for a specific group
    pub async fn get_by_group(db: &Database, group_id: &str) -> Result<Vec<RouterGroupMember>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = format!(
            "SELECT group_id, upstream_id, weight FROM router_group_members WHERE group_id = {}",
            ph(is_postgres, 1)
        );
        let rows = sqlx::query_as::<_, RouterGroupMember>(&sql)
            .bind(group_id)
            .fetch_all(conn.pool())
            .await?;
        Ok(rows)
    }

    /// Set members for a group (full replace)
    pub async fn set_for_group(
        db: &Database,
        group_id: &str,
        members: Vec<RouterGroupMember>,
    ) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        // Delete existing members
        let delete_sql = adapt_sql(
            is_postgres,
            "DELETE FROM router_group_members WHERE group_id = ?",
        );
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
/// Note: `RouterGroupModel` has no update method — groups are replaced by delete + create.
/// The `update` implementation here mirrors that: delete then create with the new data
/// and the canonical `id`.
pub struct RouterGroupRepository<'a>(pub &'a Database);

#[async_trait::async_trait]
impl<'a> CrudRepository<RouterGroup, String, DatabaseError> for RouterGroupRepository<'a> {
    async fn find_by_id(&self, id: &String) -> Result<Option<RouterGroup>> {
        RouterGroupModel::get(self.0, id).await
    }

    async fn list(&self) -> Result<Vec<RouterGroup>> {
        RouterGroupModel::get_all(self.0).await
    }

    async fn create(&self, input: &RouterGroup) -> Result<RouterGroup> {
        RouterGroupModel::create(self.0, input).await?;
        RouterGroupModel::get(self.0, &input.id)
            .await?
            .ok_or_else(|| DatabaseError::Query("group disappeared after insert".to_string()))
    }

    async fn update(&self, id: &String, input: &RouterGroup) -> Result<bool> {
        let exists = RouterGroupModel::get(self.0, id).await?.is_some();
        if !exists {
            return Ok(false);
        }
        RouterGroupModel::delete(self.0, id).await?;
        let mut record = input.clone();
        record.id = id.clone();
        RouterGroupModel::create(self.0, &record).await?;
        Ok(true)
    }

    async fn delete(&self, id: &String) -> Result<bool> {
        let exists = RouterGroupModel::get(self.0, id).await?.is_some();
        if exists {
            RouterGroupModel::delete(self.0, id).await?;
        }
        Ok(exists)
    }
}
