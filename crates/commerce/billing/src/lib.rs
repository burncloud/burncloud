//! Commerce Billing domain implementation.
//!
//! Owns billing price persistence, price caching, usage normalization, token
//! counting, and request cost calculation. Raw Traffic request logs and
//! Identity credential quota state remain outside this crate.

mod repository;

#[expect(
    clippy::type_complexity,
    reason = "PriceCache intentionally keys prices by the (model, region) pair inside one synchronized map; a named alias is planned with the cache refactor"
)]
pub mod cache;
#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the billing calculator still carries the pre-existing request-context signature and detailed multimodal breakdown; behavior is locked by the P0 billing contract tests while the API is decomposed"
)]
pub mod calculator;
pub mod counter;
pub mod error;
pub mod types;
#[expect(
    clippy::allow_attributes_without_reason,
    reason = "provider usage parsers sit at the dynamic upstream JSON boundary and still contain legacy module-level Value exceptions; keep that debt confined to usage parsing"
)]
pub mod usage;

pub use cache::PriceCache;
pub use calculator::{CostCalculator, RequestOptions};
pub use counter::UnifiedTokenCounter;
pub use error::{BillingError, ParseError};
pub use repository::{BillingPriceModel, BillingTieredPriceModel, current_timestamp};
pub use types::{CostBreakdown, CostResult, UnifiedUsage};
pub use usage::{get_parser, parse_chunk_or_default, parse_response_or_default, UsageParser};

pub use burncloud_commerce_contracts::pricing::{
    BillingExchangeRate, BillingPrice, BillingPriceInput, BillingTieredPrice,
    BillingTieredPriceInput, ExchangeRate, Price, PriceInput, TieredPrice, TieredPriceInput,
};
pub use burncloud_database::DatabaseError;
