//! Database models for BurnCloud
//!
//! This crate provides database model implementations for various entities.

mod ability;
mod channel;
mod common;
mod model;
mod price;
mod protocol_config;
mod tiered_price;
mod token;

// Re-export common utility
pub use common::current_timestamp;

// Re-export model types
pub use model::{ModelDatabase, ModelInfo};

// Re-export token types
pub use token::{Token, TokenInput, TokenModel, TokenUpdateInput};

// Re-export channel types
pub use channel::ChannelModel;

// Re-export ability types
pub use ability::{AbilityInput, AbilityModel};

// Re-export protocol config types
pub use protocol_config::{ProtocolConfig, ProtocolConfigInput, ProtocolConfigModel};

// Re-export tiered price model (TieredPrice and TieredPriceInput types come from burncloud_common)
pub use tiered_price::TieredPriceModel;

// Re-export price model (Price and PriceInput types come from burncloud_common)
pub use price::PriceModel;

// Re-export Price and TieredPrice types from common for convenience
pub use burncloud_common::types::{Price, PriceInput, TieredPrice, TieredPriceInput};

// Re-export database error
pub use burncloud_database::DatabaseError;
