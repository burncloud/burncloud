//! Cache service implementation.

use crate::{CacheError, CacheResult};
use redis::{AsyncCommands, Client};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cached token information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedToken {
    /// Token ID
    pub id: i64,
    /// Token string (hashed for security)
    pub token_hash: String,
    /// User ID
    pub user_id: String,
    /// Quota balance in nanodollars
    pub quota_balance: i64,
    /// Token status
    pub status: String,
    /// Traffic class for routing
    pub traffic_class: Option<String>,
}

/// Cached channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedChannel {
    /// Channel ID
    pub id: i32,
    /// Channel name
    pub name: String,
    /// Provider type (openai, anthropic, etc.)
    pub provider: String,
    /// Base URL
    pub base_url: String,
    /// API key (encrypted)
    pub api_key: String,
    /// Weight for load balancing
    pub weight: i32,
    /// Status
    pub status: String,
    /// Supported models list
    pub models: Vec<String>,
}

/// Cached quota balance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedQuota {
    /// User ID
    pub user_id: String,
    /// Quota balance in nanodollars
    pub balance: i64,
    /// Currency
    pub currency: String,
}

/// Cache configuration.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Redis URL
    pub redis_url: Option<String>,
    /// Whether cache is enabled
    pub enabled: bool,
    /// Token TTL in seconds
    pub token_ttl: u64,
    /// Channel TTL in seconds
    pub channel_ttl: u64,
    /// Price TTL in seconds
    pub price_ttl: u64,
    /// Quota TTL in seconds
    pub quota_ttl: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            redis_url: std::env::var("REDIS_URL").ok(),
            enabled: std::env::var("CACHE_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            token_ttl: 300,  // 5 minutes
            channel_ttl: 60, // 1 minute
            price_ttl: 600,  // 10 minutes
            quota_ttl: 60,   // 1 minute
        }
    }
}

/// Cache key prefixes.
mod keys {
    pub const TOKEN: &str = "bc:token:";
    pub const CHANNEL: &str = "bc:channel:";
    pub const CHANNEL_LIST: &str = "bc:channels:all";
    pub const PRICE: &str = "bc:price:";
    pub const QUOTA: &str = "bc:quota:";
}

/// Redis cache service.
#[derive(Clone)]
pub struct CacheService {
    /// Redis client (None if cache disabled)
    client: Option<Arc<Client>>,
    /// Cache configuration
    config: CacheConfig,
    /// Whether cache is connected
    connected: Arc<RwLock<bool>>,
}

impl CacheService {
    /// Create a new cache service.
    pub async fn new() -> CacheResult<Self> {
        Self::with_config(CacheConfig::default()).await
    }

    /// Create a new cache service with custom config.
    pub async fn with_config(config: CacheConfig) -> CacheResult<Self> {
        if !config.enabled {
            tracing::info!("Cache disabled by configuration");
            return Ok(Self {
                client: None,
                config,
                connected: Arc::new(RwLock::new(false)),
            });
        }

        let redis_url = config
            .redis_url
            .as_ref()
            .ok_or_else(|| CacheError::Connection("REDIS_URL not configured".to_string()))?;

        tracing::info!("Connecting to Redis at {}", redis_url);

        let client =
            Client::open(redis_url.as_str()).map_err(|e| CacheError::Connection(e.to_string()))?;

        // Test connection
        let mut conn = client
            .get_connection_manager()
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;

        // Ping to verify connection
        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;

        tracing::info!("Redis connection established");

        Ok(Self {
            client: Some(Arc::new(client)),
            config,
            connected: Arc::new(RwLock::new(true)),
        })
    }

    /// Check if cache is enabled and connected.
    pub async fn is_available(&self) -> bool {
        if self.client.is_none() {
            return false;
        }
        *self.connected.read().await
    }

    /// Get a Redis connection.
    async fn get_connection(&self) -> CacheResult<Option<redis::aio::ConnectionManager>> {
        if !self.is_available().await {
            return Ok(None);
        }

        let client = self.client.as_ref().ok_or(CacheError::Disabled)?;
        let conn = client
            .get_connection_manager()
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;

        Ok(Some(conn))
    }

    /// Get a value from cache.
    async fn get<T: DeserializeOwned>(&self, key: &str) -> CacheResult<Option<T>> {
        let conn = self.get_connection().await?;

        if let Some(mut conn) = conn {
            let data: Option<String> = conn
                .get(key)
                .await
                .map_err(|e| CacheError::Operation(e.to_string()))?;

            if let Some(data) = data {
                let value: T = serde_json::from_str(&data)
                    .map_err(|e| CacheError::Serialization(e.to_string()))?;
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    /// Set a value in cache with TTL.
    async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: u64) -> CacheResult<()> {
        let conn = self.get_connection().await?;

        if let Some(mut conn) = conn {
            let data = serde_json::to_string(value)
                .map_err(|e| CacheError::Serialization(e.to_string()))?;

            let _: () = conn
                .set_ex(key, data, ttl)
                .await
                .map_err(|e| CacheError::Operation(e.to_string()))?;
        }

        Ok(())
    }

    /// Delete a value from cache.
    async fn delete(&self, key: &str) -> CacheResult<()> {
        let conn = self.get_connection().await?;

        if let Some(mut conn) = conn {
            let _: () = conn
                .del(key)
                .await
                .map_err(|e| CacheError::Operation(e.to_string()))?;
        }

        Ok(())
    }

    // === Token Operations ===

    /// Get cached token by key.
    pub async fn get_token(&self, token_key: &str) -> CacheResult<Option<CachedToken>> {
        let key = format!("{}{}", keys::TOKEN, token_key);
        self.get::<CachedToken>(&key).await
    }

    /// Cache token information.
    pub async fn set_token(&self, token_key: &str, token: &CachedToken) -> CacheResult<()> {
        let key = format!("{}{}", keys::TOKEN, token_key);
        self.set(&key, token, self.config.token_ttl).await
    }

    /// Invalidate token cache.
    pub async fn invalidate_token(&self, token_key: &str) -> CacheResult<()> {
        let key = format!("{}{}", keys::TOKEN, token_key);
        self.delete(&key).await
    }

    // === Channel Operations ===

    /// Get cached channel by ID.
    pub async fn get_channel(&self, channel_id: i32) -> CacheResult<Option<CachedChannel>> {
        let key = format!("{}{}", keys::CHANNEL, channel_id);
        self.get::<CachedChannel>(&key).await
    }

    /// Cache channel information.
    pub async fn set_channel(&self, channel_id: i32, channel: &CachedChannel) -> CacheResult<()> {
        let key = format!("{}{}", keys::CHANNEL, channel_id);
        self.set(&key, channel, self.config.channel_ttl).await
    }

    /// Get all cached channels list.
    pub async fn get_all_channels(&self) -> CacheResult<Option<Vec<CachedChannel>>> {
        self.get::<Vec<CachedChannel>>(keys::CHANNEL_LIST).await
    }

    /// Cache all channels list.
    pub async fn set_all_channels(&self, channels: &Vec<CachedChannel>) -> CacheResult<()> {
        self.set(keys::CHANNEL_LIST, channels, self.config.channel_ttl)
            .await
    }

    /// Invalidate channel cache.
    pub async fn invalidate_channel(&self, channel_id: i32) -> CacheResult<()> {
        let key = format!("{}{}", keys::CHANNEL, channel_id);
        self.delete(&key).await?;
        // Also invalidate the all-channels list
        self.delete(keys::CHANNEL_LIST).await?;
        Ok(())
    }

    // === Price Operations ===

    /// Get cached price by model name.
    pub async fn get_price<T: DeserializeOwned>(&self, model: &str) -> CacheResult<Option<T>> {
        let key = format!("{}{}", keys::PRICE, model.to_lowercase());
        self.get::<T>(&key).await
    }

    /// Cache price information.
    pub async fn set_price<T: Serialize>(&self, model: &str, price: &T) -> CacheResult<()> {
        let key = format!("{}{}", keys::PRICE, model.to_lowercase());
        self.set(&key, price, self.config.price_ttl).await
    }

    /// Invalidate price cache.
    pub async fn invalidate_price(&self, model: &str) -> CacheResult<()> {
        let key = format!("{}{}", keys::PRICE, model.to_lowercase());
        self.delete(&key).await
    }

    // === Quota Operations ===

    /// Get cached quota balance.
    pub async fn get_quota(&self, user_id: &str) -> CacheResult<Option<CachedQuota>> {
        let key = format!("{}{}", keys::QUOTA, user_id);
        self.get::<CachedQuota>(&key).await
    }

    /// Cache quota balance.
    pub async fn set_quota(&self, user_id: &str, quota: &CachedQuota) -> CacheResult<()> {
        let key = format!("{}{}", keys::QUOTA, user_id);
        self.set(&key, quota, self.config.quota_ttl).await
    }

    /// Invalidate quota cache.
    pub async fn invalidate_quota(&self, user_id: &str) -> CacheResult<()> {
        let key = format!("{}{}", keys::QUOTA, user_id);
        self.delete(&key).await
    }

    // === Utility Operations ===

    /// Clear all BurnCloud cache keys.
    pub async fn clear_all(&self) -> CacheResult<()> {
        let conn = self.get_connection().await?;

        if let Some(mut conn) = conn {
            // Delete all keys matching bc:*
            let keys: Vec<String> = redis::cmd("KEYS")
                .arg("bc:*")
                .query_async(&mut conn)
                .await
                .map_err(|e| CacheError::Operation(e.to_string()))?;

            if !keys.is_empty() {
                let _: () = conn
                    .del(&keys)
                    .await
                    .map_err(|e| CacheError::Operation(e.to_string()))?;
                tracing::info!("Cleared {} cache keys", keys.len());
            }
        }

        Ok(())
    }

    /// Get cache statistics.
    pub async fn stats(&self) -> CacheResult<CacheStats> {
        let conn = self.get_connection().await?;

        let mut stats = CacheStats {
            enabled: self.is_available().await,
            connected: false,
            key_count: 0,
            memory_usage: 0,
        };

        if let Some(mut conn) = conn {
            stats.connected = true;

            // Get key count for bc:* keys
            let keys: Vec<String> = redis::cmd("KEYS")
                .arg("bc:*")
                .query_async(&mut conn)
                .await
                .map_err(|e| CacheError::Operation(e.to_string()))?;
            stats.key_count = keys.len();

            // Get memory usage (approximate)
            let info: String = redis::cmd("INFO")
                .arg("memory")
                .query_async(&mut conn)
                .await
                .map_err(|e| CacheError::Operation(e.to_string()))?;

            // Parse used_memory from INFO output
            for line in info.lines() {
                if line.starts_with("used_memory:") {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 2 {
                        stats.memory_usage = parts[1].trim().parse().unwrap_or(0);
                    }
                }
            }
        }

        Ok(stats)
    }
}

/// Cache statistics.
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    /// Whether cache is enabled
    pub enabled: bool,
    /// Whether connected to Redis
    pub connected: bool,
    /// Number of cached keys (bc:*)
    pub key_count: usize,
    /// Memory usage in bytes
    pub memory_usage: u64,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert!(!config.enabled); // Disabled by default without REDIS_URL
        assert_eq!(config.token_ttl, 300);
        assert_eq!(config.channel_ttl, 60);
    }

    #[tokio::test]
    async fn test_disabled_cache_returns_none() {
        let config = CacheConfig {
            redis_url: None,
            enabled: false,
            ..Default::default()
        };
        let cache = CacheService::with_config(config).await.unwrap();
        assert!(!cache.is_available().await);

        let result = cache.get_token("test-token").await;
        assert!(result.unwrap().is_none());
    }
}
