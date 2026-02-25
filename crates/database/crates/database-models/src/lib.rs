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

// Re-export tiered price types
pub use tiered_price::{TieredPrice, TieredPriceInput, TieredPriceModel};

// Re-export price types
pub use price::{Price, PriceInput, PriceModel};

// Re-export database error
pub use burncloud_database::DatabaseError;
