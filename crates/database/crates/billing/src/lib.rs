//! Database billing crate for BurnCloud
//!
//! This crate aggregates all billing_ domain tables: billing_prices,
//! billing_tiered_prices, billing_exchange_rates.

mod billing_exchange_rate;
mod billing_price;
mod billing_tiered_price;
mod common;

pub use billing_exchange_rate::BillingExchangeRateModel;
pub use billing_price::BillingPriceModel;
pub use billing_tiered_price::BillingTieredPriceModel;
pub use common::current_timestamp;

// Re-export types from burncloud-common for convenience (spec-aligned aliases + originals)
pub use burncloud_common::types::{
    BillingExchangeRate, BillingPrice, BillingPriceInput, BillingTieredPrice,
    BillingTieredPriceInput, ExchangeRate, Price, PriceInput, TieredPrice, TieredPriceInput,
};

pub use burncloud_database::DatabaseError;
