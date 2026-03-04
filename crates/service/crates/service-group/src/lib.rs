//! # BurnCloud Service Group
//!
//! Group service layer providing business logic for upstream groups,
//! which are used for load balancing and routing strategies.

use burncloud_database::Database;
use burncloud_database_group::{GroupMemberModel, GroupModel};

pub use burncloud_database_group::{DbGroup, DbGroupMember};

type Result<T> = std::result::Result<T, burncloud_database::DatabaseError>;

/// Group service for managing upstream groups
pub struct GroupService;

impl GroupService {
    /// Get all groups
    pub async fn get_all(db: &Database) -> Result<Vec<DbGroup>> {
        GroupModel::get_all(db).await
    }

    /// Get a single group by ID
    pub async fn get(db: &Database, id: &str) -> Result<Option<DbGroup>> {
        GroupModel::get(db, id).await
    }

    /// Create a new group
    pub async fn create(db: &Database, group: &DbGroup) -> Result<()> {
        GroupModel::create(db, group).await
    }

    /// Delete a group and all its members
    pub async fn delete(db: &Database, id: &str) -> Result<()> {
        GroupModel::delete(db, id).await
    }
}

/// Group member service for managing group memberships
pub struct GroupMemberService;

impl GroupMemberService {
    /// Get all group members
    pub async fn get_all(db: &Database) -> Result<Vec<DbGroupMember>> {
        GroupMemberModel::get_all(db).await
    }

    /// Get members for a specific group
    pub async fn get_by_group(db: &Database, group_id: &str) -> Result<Vec<DbGroupMember>> {
        GroupMemberModel::get_by_group(db, group_id).await
    }

    /// Set members for a group (full replace)
    pub async fn set_for_group(
        db: &Database,
        group_id: &str,
        members: Vec<DbGroupMember>,
    ) -> Result<()> {
        GroupMemberModel::set_for_group(db, group_id, members).await
    }
}
