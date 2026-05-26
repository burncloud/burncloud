//! # BurnCloud Service Cache
//!
//! Redis cache integration for hot data, reducing database pressure and
//! improving response times.
//!
//! ## Cache Targets
//!
//! - **Token Info**: User authentication data (TTL 5 min)
//! - **Channel Config**: Channel list and weights (TTL 1 min)
//! - **Model Prices**: Price configuration (TTL 10 min)
//! - **User Quota**: Quota balance (TTL 1 min)
//!
//! ## Cache Strategy
//!
//! - **Cache-Aside**: Read from cache first, fallback to DB on miss
//! - **Write-Through**: Update cache and DB together on write
//! - **LRU Eviction**: Handled by Redis TTL

mod cache_service;
mod channel_cache;
mod error;
mod price_cache;
mod quota_cache;
mod token_cache;

pub use cache_service::{CacheConfig, CacheService};
pub use channel_cache::ChannelCache;
pub use error::{CacheError, CacheResult};
pub use price_cache::PriceCache;
pub use quota_cache::QuotaCache;
pub use token_cache::TokenCache;

// Re-export types for convenience
pub use burncloud_common::types::{Channel, Price};
pub use burncloud_database_router::RouterToken;
