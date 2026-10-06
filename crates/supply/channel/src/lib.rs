//! # BurnCloud Supply Channel
//!
//! Supply-owned implementation for upstream channel management. This crate owns the
//! `channel_providers`, `channel_abilities`, and `channel_protocol_configs` persistence
//! behavior together with the Channel use-case facade.
//!
//! Cross-domain value objects remain in `burncloud-supply-contracts`; database row mapping
//! and SQL remain implementation details of this crate.

mod channel_ability;
mod channel_protocol_config;
#[expect(
    clippy::cognitive_complexity,
    reason = "channel ability synchronization intentionally reconciles create/update/delete cases in one legacy routine; contract tests cover its behavior while decomposition remains maintainability work"
)]
mod channel_provider;
mod common;
mod rows;

use burncloud_database::Database;

pub use channel_ability::{ChannelAbilityInput, ChannelAbilityModel};
pub use channel_protocol_config::{
    ChannelProtocolConfig, ChannelProtocolConfigInput, ChannelProtocolConfigModel,
};
pub use channel_provider::ChannelProviderModel;

// The Supply contract remains the public domain-type boundary.
pub use burncloud_supply_contracts::{Ability, Channel, ChannelAbility, ChannelProvider};
pub use burncloud_database::DatabaseError;

type Result<T> = std::result::Result<T, DatabaseError>;

/// Supply-owned facade for upstream Channel use cases.
///
/// These methods intentionally preserve the former `burncloud-service-channel` behavior:
/// they delegate to the same persistence implementation without changing SQL, error, or
/// synchronization semantics.
pub struct ChannelService;

impl ChannelService {
    /// List channels with pagination.
    pub async fn list(db: &Database, limit: i32, offset: i32) -> Result<Vec<Channel>> {
        ChannelProviderModel::list(db, limit, offset).await
    }

    /// Create a new channel. Sets `channel.id` to the newly assigned ID.
    pub async fn create(db: &Database, channel: &mut Channel) -> Result<i32> {
        ChannelProviderModel::create(db, channel).await
    }

    /// Update an existing channel.
    pub async fn update(db: &Database, channel: &Channel) -> Result<()> {
        ChannelProviderModel::update(db, channel).await
    }

    /// Delete a channel by ID.
    pub async fn delete(db: &Database, id: i32) -> Result<()> {
        ChannelProviderModel::delete(db, id).await
    }

    /// Get a channel by ID.
    pub async fn get_by_id(db: &Database, id: i32) -> Result<Option<Channel>> {
        ChannelProviderModel::get_by_id(db, id).await
    }

    /// Synchronize model abilities for a channel.
    pub async fn sync_abilities(db: &Database, channel: &Channel) -> Result<()> {
        ChannelProviderModel::sync_abilities(db, channel).await
    }
}
