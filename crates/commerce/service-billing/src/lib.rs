//! Unified billing module for BurnCloud.
//!
//! Provides:
//! - [`UnifiedUsage`] — multi-modal token counting struct
//! - [`UsageParser`] trait + [`get_parser`] factory per provider
//! - [`UnifiedTokenCounter`] — thread-safe streaming token counter
//! - [`PriceCache`] — in-memory price lookup (loaded at startup, refreshed on write)
//! - [`CostCalculator`] — preflight check + cost calculation
//!
//! # Usage
//! 1. `PriceCache::load(&db).await?` — load at startup
//! 2. `calculator.preflight(&model, region)` — check before forwarding (503 if missing)
//! 3. `get_parser(channel_type).parse_response(&json)` — extract usage
//! 4. `calculator.calculate(&model, &usage, &req_id, is_batch, is_priority, region)` — compute cost

#[expect(
    clippy::type_complexity,
    reason = "PriceCache intentionally keys prices by the `(model, region)` pair inside one synchronized map; a named alias is planned with the cache refactor"
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
pub mod summary;
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
pub use summary::{BillingModelSummary, BillingService, BillingSummary};
pub use types::{CostBreakdown, CostResult, UnifiedUsage};
pub use usage::{get_parser, parse_chunk_or_default, parse_response_or_default, UsageParser};
