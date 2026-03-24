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
//! 2. `calculator.preflight(&model)` — check before forwarding (503 if missing)
//! 3. `get_parser(channel_type).parse_response(&json)` — extract usage
//! 4. `calculator.calculate(&model, &usage, &req_id, is_batch, is_priority)` — compute cost

pub mod cache;
pub mod calculator;
pub mod counter;
pub mod error;
pub mod types;
pub mod usage;

pub use cache::PriceCache;
pub use calculator::CostCalculator;
pub use counter::UnifiedTokenCounter;
pub use error::{BillingError, ParseError};
pub use types::{CostBreakdown, CostResult, UnifiedUsage};
pub use usage::{
    get_parser, parse_chunk_or_default, parse_response_or_default, UsageParser,
};
