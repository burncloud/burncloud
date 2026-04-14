//! Database price crate for BurnCloud
//!
//! This crate provides database model implementations for price and tiered pricing,
//! extracted from `database-models` to align with the Database ↔ Service matrix (spec §1.6).

mod common;
mod price;
mod tiered_price;

pub use common::current_timestamp;
pub use price::PriceModel;
pub use tiered_price::TieredPriceModel;

// Re-export types from burncloud-common for convenience
pub use burncloud_common::types::{Price, PriceInput, TieredPrice, TieredPriceInput};

pub use burncloud_database::DatabaseError;
