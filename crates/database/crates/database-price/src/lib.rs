//! Database price crate for BurnCloud
//!
//! This crate provides database model implementations for price and tiered pricing,
//! extracted from `database-models` to align with the Database ↔ Service matrix (spec §1.6).

mod billing_price;
mod billing_tiered_price;
mod common;

pub use billing_price::BillingPriceModel;
pub use billing_tiered_price::BillingTieredPriceModel;
pub use common::current_timestamp;

// Re-export types from burncloud-common for convenience
pub use burncloud_common::types::{Price, PriceInput, TieredPrice, TieredPriceInput};

pub use burncloud_database::DatabaseError;
