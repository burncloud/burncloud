//! Database billing crate for BurnCloud
//!
//! This crate aggregates the active billing persistence for prices, tiered prices and exchange
//! rates.
//!
//! Migration `0015_monthly_quota.sql` also contains `billing_plans` and
//! `billing_subscriptions`, but those tables do not currently have Rust domain types, modules or
//! a runtime service/API. The old uncompiled source files were removed in #653; implementing that
//! feature is tracked separately by #652. Their schema must not be treated as an active capability.

#[expect(
    clippy::too_many_lines,
    reason = "billing_price upsert paths are legacy SQL bind sequences slightly above the line budget; behavior is covered by billing tests and decomposition is separate maintainability work"
)]
mod billing_price;
mod billing_tiered_price;
mod common;
mod rows;

pub use billing_price::BillingPriceModel;
pub use billing_tiered_price::BillingTieredPriceModel;
pub use common::current_timestamp;

// Re-export types from burncloud-common for convenience (spec-aligned aliases + originals)
pub use burncloud_commerce_contracts::pricing::{
    BillingExchangeRate, BillingPrice, BillingPriceInput, BillingTieredPrice,
    BillingTieredPriceInput, ExchangeRate, Price, PriceInput, TieredPrice, TieredPriceInput,
};

pub use burncloud_database::DatabaseError;
