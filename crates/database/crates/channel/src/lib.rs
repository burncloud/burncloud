//! Database channel crate for BurnCloud
//!
//! This crate provides database model implementations for channel and ability management,
//! aggregating all channel_ domain tables: channel_providers, channel_abilities,
//! channel_protocol_configs.

mod channel_ability;
mod channel_protocol_config;
mod channel_provider;
mod common;

pub use channel_ability::{ChannelAbilityInput, ChannelAbilityModel};
pub use channel_protocol_config::{
    ChannelProtocolConfig, ChannelProtocolConfigInput, ChannelProtocolConfigModel,
};
pub use channel_provider::ChannelProviderModel;

// Re-export spec-aligned row types from burncloud-common for convenience
pub use burncloud_common::types::{Ability, Channel, ChannelAbility, ChannelProvider};

pub use burncloud_database::DatabaseError;
