//! Database channel crate for BurnCloud
//!
//! This crate provides database model implementations for channel and ability management,
//! aggregating all channel_ domain tables: channel_providers, channel_abilities,
//! channel_protocol_configs.

mod channel_ability;
mod channel_protocol_config;
#[expect(
    clippy::cognitive_complexity,
    reason = "channel ability synchronization intentionally reconciles create/update/delete cases in one legacy routine; contract tests cover its behavior while decomposition remains maintainability work"
)]
mod channel_provider;
mod common;
mod rows;

pub use channel_ability::{ChannelAbilityInput, ChannelAbilityModel};
pub use channel_protocol_config::{
    ChannelProtocolConfig, ChannelProtocolConfigInput, ChannelProtocolConfigModel,
};
pub use channel_provider::ChannelProviderModel;

// Re-export the Supply contract directly: this crate must not route consumers through Common
// (Supply -> Common -> Supply contract would keep the dependency the migration removed).
pub use burncloud_supply_contracts::{Ability, Channel, ChannelAbility, ChannelProvider};

pub use burncloud_database::DatabaseError;
