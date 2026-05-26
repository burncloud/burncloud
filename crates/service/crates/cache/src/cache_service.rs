//! Main cache service abstraction
//!
//! Provides a unified interface for caching hot data with automatic fallback
//! to database when Redis is unavailable.

use crate::error::{CacheError, CacheResult};
use redis::Client as RedisClient;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Redis URL (e.g., redis://localhost:6379)
    pub redis_url: Option<String>,
    /// Whether caching is enabled
    pub enabled: bool,
    /// Default TTL for token cache (seconds)
    pub token_ttl_secs: u64,
    /// Default TTL for channel cache (seconds)
    pub channel_ttl_secs: u64,
    /// Default TTL for price cache (seconds)
    pub price_ttl_secs: u64,
    /// Default TTL for quota cache (seconds)
    pub quota_ttl_secs: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            redis_url: std::env::var("REDIS_URL").ok(),
            enabled: std::env::var("CACHE_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            token_ttl_secs: 300,  // 5 minutes
            channel_ttl_secs: 60, // 1 minute
            price_ttl_secs: 600,  // 10 minutes
            quota_ttl_secs: 60,   // 1 minute
        }
    }
}

impl CacheConfig {
    /// Create config from environment variables
    pub fn from_env() -> Self {
        Self::default()
    }

    /// Create config for testing (in-memory, no Redis)
    pub fn for_testing() -> Self {
        Self {
            redis_url: None,
            enabled: false,
            ..Self::default()
        }
    }
}

/// Redis connection wrapper
struct RedisConnection {
    client: RedisClient,
}

impl RedisConnection {
    async fn new(url: &str) -> CacheResult<Self> {
        let client =
            RedisClient::open(url).map_err(|e| CacheError::ConnectionError(e.to_string()))?;

        // Test connection
        let mut conn = client
            .get_connection_manager()
            .await
            .map_err(|e| CacheError::ConnectionError(e.to_string()))?;

        // Ping to verify connection
        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| CacheError::ConnectionError(e.to_string()))?;

        Ok(Self { client })
    }

    async fn get_connection(&self) -> CacheResult<redis::aio::ConnectionManager> {
        self.client
            .get_connection_manager()
            .await
            .map_err(|e| CacheError::ConnectionError(e.to_string()))
    }
}

/// Main cache service
#[derive(Clone)]
pub struct CacheService {
    /// Redis client (None if cache disabled or unavailable)
    redis: Arc<RwLock<Option<RedisConnection>>>,
    /// Configuration
    config: CacheConfig,
    /// Whether Redis is available (cached for performance)
    redis_available: Arc<RwLock<bool>>,
}

impl CacheService {
    /// Create a new cache service
    pub async fn new(config: CacheConfig) -> CacheResult<Self> {
        let (redis, redis_available) = if !config.enabled {
            tracing::info!("Cache disabled by configuration");
            (None, false)
        } else if let Some(ref url) = config.redis_url {
            match RedisConnection::new(url).await {
                Ok(conn) => {
                    tracing::info!("Redis cache connected successfully");
                    (Some(conn), true)
                }
                Err(e) => {
                    tracing::warn!("Redis connection failed, using fallback: {}", e);
                    (None, false)
                }
            }
        } else {
            tracing::info!("No Redis URL configured, cache disabled");
            (None, false)
        };

        Ok(Self {
            redis: Arc::new(RwLock::new(redis)),
            config,
            redis_available: Arc::new(RwLock::new(redis_available)),
        })
    }

    /// Check if cache is available
    pub async fn is_available(&self) -> bool {
        *self.redis_available.read().await
    }

    /// Get configuration
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Get a value from cache
    pub async fn get<T>(&self, key: &str) -> CacheResult<Option<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        if !self.is_available().await {
            return Ok(None);
        }

        let redis_guard = self.redis.read().await;
        if let Some(ref redis_conn) = *redis_guard {
            let mut conn = redis_conn.get_connection().await?;
            let result: Option<String> = redis::cmd("GET")
                .arg(key)
                .query_async(&mut conn)
                .await
                .map_err(|e| {
                    tracing::warn!("Redis GET error for key {}: {}", key, e);
                    CacheError::OperationError(e.to_string())
                })?;

            match result {
                Some(json) => {
                    let value: T = serde_json::from_str(&json)?;
                    tracing::trace!("Cache hit for key: {}", key);
                    Ok(Some(value))
                }
                None => {
                    tracing::trace!("Cache miss for key: {}", key);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Set a value in cache with TTL
    pub async fn set<T>(&self, key: &str, value: &T, ttl_secs: u64) -> CacheResult<()>
    where
        T: serde::Serialize,
    {
        if !self.is_available().await {
            return Ok(());
        }

        let redis_guard = self.redis.read().await;
        if let Some(ref redis_conn) = *redis_guard {
            let json = serde_json::to_string(value)?;
            let mut conn = redis_conn.get_connection().await?;

            let _: () = redis::cmd("SET")
                .arg(key)
                .arg(&json)
                .arg("EX")
                .arg(ttl_secs)
                .query_async(&mut conn)
                .await
                .map_err(|e| {
                    tracing::warn!("Redis SET error for key {}: {}", key, e);
                    CacheError::OperationError(e.to_string())
                })?;

            tracing::trace!("Cache set for key: {} (TTL: {}s)", key, ttl_secs);
        }

        Ok(())
    }

    /// Delete a key from cache
    pub async fn delete(&self, key: &str) -> CacheResult<()> {
        if !self.is_available().await {
            return Ok(());
        }

        let redis_guard = self.redis.read().await;
        if let Some(ref redis_conn) = *redis_guard {
            let mut conn = redis_conn.get_connection().await?;

            let _: () = redis::cmd("DEL")
                .arg(key)
                .query_async(&mut conn)
                .await
                .map_err(|e| {
                    tracing::warn!("Redis DEL error for key {}: {}", key, e);
                    CacheError::OperationError(e.to_string())
                })?;

            tracing::trace!("Cache deleted for key: {}", key);
        }

        Ok(())
    }

    /// Delete all keys matching a pattern
    pub async fn delete_pattern(&self, pattern: &str) -> CacheResult<()> {
        if !self.is_available().await {
            return Ok(());
        }

        let redis_guard = self.redis.read().await;
        if let Some(ref redis_conn) = *redis_guard {
            let mut conn = redis_conn.get_connection().await?;

            // SCAN for keys matching pattern
            let keys: Vec<String> = redis::cmd("KEYS")
                .arg(pattern)
                .query_async(&mut conn)
                .await
                .map_err(|e| {
                    tracing::warn!("Redis KEYS error for pattern {}: {}", pattern, e);
                    CacheError::OperationError(e.to_string())
                })?;

            if !keys.is_empty() {
                let _: () = redis::cmd("DEL")
                    .arg(&keys)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| {
                        tracing::warn!("Redis DEL error for pattern {}: {}", pattern, e);
                        CacheError::OperationError(e.to_string())
                    })?;

                tracing::trace!(
                    "Cache deleted {} keys matching pattern: {}",
                    keys.len(),
                    pattern
                );
            }
        }

        Ok(())
    }

    /// Invalidate token cache for a specific token
    pub async fn invalidate_token(&self, token: &str) -> CacheResult<()> {
        self.delete(&format!("token:{}", token)).await
    }

    /// Invalidate channel cache
    pub async fn invalidate_channels(&self) -> CacheResult<()> {
        self.delete_pattern("channel:*").await
    }

    /// Invalidate price cache for a model
    pub async fn invalidate_price(&self, model: &str, region: Option<&str>) -> CacheResult<()> {
        match region {
            Some(r) => {
                self.delete(&format!("price:{}:{}", model.to_lowercase(), r))
                    .await
            }
            None => {
                self.delete_pattern(&format!("price:{}:*", model.to_lowercase()))
                    .await
            }
        }
    }

    /// Invalidate quota cache for a token
    pub async fn invalidate_quota(&self, token: &str) -> CacheResult<()> {
        self.delete(&format!("quota:{}", token)).await
    }
}

impl Default for CacheService {
    fn default() -> Self {
        // Create a disabled cache service
        Self {
            redis: Arc::new(RwLock::new(None)),
            config: CacheConfig::for_testing(),
            redis_available: Arc::new(RwLock::new(false)),
        }
    }
}
