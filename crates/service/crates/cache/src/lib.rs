//! Redis cache integration for BurnCloud.
//!
//! Provides a caching layer for hot data to reduce database load and improve response times.
//!
//! # Cache Targets
//! - Model prices: TTL 10 minutes
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
//! // Read cache backend statistics
//! let _stats = cache.stats().await?;
//! ```

mod error;
mod service;

pub use error::{CacheError, CacheResult};
pub use service::CacheService;
