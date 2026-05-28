//! Redis cache integration for BurnCloud.
//!
//! Provides a caching layer for hot data to reduce database load and improve response times.
//!
//! # Cache Targets
//! - Token information: TTL 5 minutes
//! - Channel configuration: TTL 1 minute
//! - Model prices: TTL 10 minutes
//! - User quota balance: TTL 1 minute
//!
//! # Environment Variables
//! - `REDIS_URL`: Redis connection URL (default: none, cache disabled)
//! - `CACHE_ENABLED`: Enable/disable cache (default: false)
//!
//! # Usage
//! ```rust,no_run
//! use burncloud_service_cache::CacheService;
//!
//! // Initialize at startup
//! let cache = CacheService::new().await?;
//!
//! // Get cached token
//! let token = cache.get_token(&key).await?;
//!
//! // Set cached value
//! cache.set_token(&key, &token_data).await?;
//! ```

mod error;
mod service;

pub use error::{CacheError, CacheResult};
pub use service::{CacheService, CachedChannel, CachedQuota, CachedToken};
