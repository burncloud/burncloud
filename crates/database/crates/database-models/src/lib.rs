//! Database models for BurnCloud
//!
//! This crate provides database model implementations for various entities.

mod billing_price;
mod billing_tiered_price;
mod channel_ability;
mod channel_protocol_config;
mod channel_provider;
mod common;
mod model_capability;
mod router_video_task;
mod user_api_key;

// Re-export common utility
pub use common::current_timestamp;

// Re-export model types
pub use model_capability::{ModelDatabase, ModelInfo};

// Re-export user API key types
pub use user_api_key::{UserApiKey, UserApiKeyInput, UserApiKeyModel, UserApiKeyUpdateInput};

// Re-export channel provider types
pub use channel_provider::ChannelProviderModel;

// Re-export channel ability types
pub use channel_ability::{ChannelAbilityInput, ChannelAbilityModel};

// Re-export channel protocol config types
pub use channel_protocol_config::{
    ChannelProtocolConfig, ChannelProtocolConfigInput, ChannelProtocolConfigModel,
};

// Re-export billing tiered price model (TieredPrice and TieredPriceInput types come from burncloud_common)
pub use billing_tiered_price::BillingTieredPriceModel;

// Re-export billing price model (Price and PriceInput types come from burncloud_common)
pub use billing_price::BillingPriceModel;

// Re-export Price and TieredPrice types from common for convenience
pub use burncloud_common::types::{Price, PriceInput, TieredPrice, TieredPriceInput};

// Re-export router video task types
pub use router_video_task::{RouterVideoTask, RouterVideoTaskModel};

// Re-export database error
pub use burncloud_database::DatabaseError;
