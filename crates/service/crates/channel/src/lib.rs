//! # BurnCloud Service Channel
//!
//! Channel service layer providing business logic for upstream channel management,
//! including CRUD operations and ability synchronization.

use burncloud_database::Database;
use burncloud_database_models::ChannelProviderModel;

pub use burncloud_common::types::Channel;

type Result<T> = std::result::Result<T, burncloud_database::DatabaseError>;

/// Channel service for managing upstream API channels
pub struct ChannelService;

impl ChannelService {
    /// List channels with pagination
    pub async fn list(db: &Database, limit: i32, offset: i32) -> Result<Vec<Channel>> {
        ChannelProviderModel::list(db, limit, offset).await
    }

    /// Create a new channel. Sets `channel.id` to the newly assigned ID.
    pub async fn create(db: &Database, channel: &mut Channel) -> Result<i32> {
        ChannelProviderModel::create(db, channel).await
    }

    /// Update an existing channel
    pub async fn update(db: &Database, channel: &Channel) -> Result<()> {
        ChannelProviderModel::update(db, channel).await
    }

    /// Delete a channel by ID
    pub async fn delete(db: &Database, id: i32) -> Result<()> {
        ChannelProviderModel::delete(db, id).await
    }

    /// Get a channel by ID
    pub async fn get_by_id(db: &Database, id: i32) -> Result<Option<Channel>> {
        ChannelProviderModel::get_by_id(db, id).await
    }

    /// Sync model abilities for a channel
    pub async fn sync_abilities(db: &Database, channel: &Channel) -> Result<()> {
        ChannelProviderModel::sync_abilities(db, channel).await
    }
}
