//! # BurnCloud Service Group
//!
//! Group service layer providing business logic for channel group management,
//! including CRUD operations for groups and their members.

use burncloud_database::Database;
use burncloud_database_group::{GroupMember, GroupMemberInput, GroupMemberModel, RouterGroup, RouterGroupModel};

type Result<T> = std::result::Result<T, burncloud_database::DatabaseError>;

/// Group service for managing channel groups
pub struct GroupService;

impl GroupService {
    /// List all groups
    pub async fn list(db: &Database) -> Result<Vec<RouterGroup>> {
        RouterGroupModel::list(db).await
    }

    /// Get a group by ID
    pub async fn get_by_id(db: &Database, id: &str) -> Result<Option<RouterGroup>> {
        RouterGroupModel::get_by_id(db, id).await
    }

    /// Create a new group
    pub async fn create(db: &Database, group: &RouterGroup) -> Result<()> {
        RouterGroupModel::create(db, group).await
    }

    /// Update a group
    pub async fn update(db: &Database, group: &RouterGroup) -> Result<()> {
        RouterGroupModel::update(db, group).await
    }

    /// Delete a group by ID
    pub async fn delete(db: &Database, id: &str) -> Result<()> {
        RouterGroupModel::delete(db, id).await
    }

    /// Get members for a group
    pub async fn get_members(db: &Database, group_id: &str) -> Result<Vec<GroupMember>> {
        GroupMemberModel::list_by_group(db, group_id).await
    }

    /// Update members for a group
    pub async fn update_members(db: &Database, group_id: &str, members: &[GroupMemberInput]) -> Result<()> {
        GroupMemberModel::update_members(db, group_id, members).await
    }
}
