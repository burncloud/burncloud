//! Cache service implementation.

use crate::{CacheError, CacheResult};
use redis::{AsyncCommands, Client};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache configuration.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Redis URL
    pub redis_url: Option<String>,
    /// Whether cache is enabled
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            redis_url: std::env::var("REDIS_URL").ok(),
            enabled: std::env::var("CACHE_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        }
    }
}

/// Redis cache service.
#[derive(Clone)]
pub struct CacheService {
    /// Redis client (None if cache disabled)
    client: Option<Arc<Client>>,
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
                connected: Arc::new(RwLock::new(false)),
            });
        }

        let redis_url = config
            .redis_url
            .as_ref()
            .ok_or(CacheError::Connection)?;

        tracing::info!(backend = "redis", "connecting to cache backend");

        let client =
            Client::open(redis_url.as_str()).map_err(|_| CacheError::Connection)?;

        // Test connection
        let mut conn = client
            .get_connection_manager()
            .await
            .map_err(|_| CacheError::Connection)?;

        // Ping to verify connection
        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|_| CacheError::Connection)?;

        tracing::info!("Redis connection established");

        Ok(Self {
            client: Some(Arc::new(client)),
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
            .map_err(|_| CacheError::Connection)?;

        Ok(Some(conn))
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
                .map_err(|_| CacheError::Operation)?;

            if !keys.is_empty() {
                let _: () = conn
                    .del(&keys)
                    .await
                    .map_err(|_| CacheError::Operation)?;
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
                .map_err(|_| CacheError::Operation)?;
            stats.key_count = keys.len();

            // Get memory usage (approximate)
            let info: String = redis::cmd("INFO")
                .arg("memory")
                .query_async(&mut conn)
                .await
                .map_err(|_| CacheError::Operation)?;

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
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert!(!config.enabled); // Disabled by default without REDIS_URL
    }

    #[tokio::test]
    async fn test_disabled_cache_is_unavailable() {
        let config = CacheConfig {
            redis_url: None,
            enabled: false,
            ..Default::default()
        };
        let cache = CacheService::with_config(config).await.unwrap();

        assert!(!cache.is_available().await);

        let stats = cache.stats().await.unwrap();
        assert!(!stats.enabled);
        assert!(!stats.connected);
        assert_eq!(stats.key_count, 0);
        assert_eq!(stats.memory_usage, 0);
    }

    #[tokio::test]
    async fn test_connection_error_redacts_redis_url() {
        let redis_url =
            "redis://secret-user:secret-password@internal-cache.example:not-a-port/0";
        let config = CacheConfig {
            redis_url: Some(redis_url.to_string()),
            enabled: true,
            ..Default::default()
        };

        let error = match CacheService::with_config(config).await {
            Ok(_) => panic!("invalid Redis URL unexpectedly connected"),
            Err(error) => error,
        };
        let message = error.to_string();

        assert_eq!(message, "cache connection failed");
        for secret in [
            "secret-user",
            "secret-password",
            "internal-cache.example",
            redis_url,
        ] {
            assert!(
                !message.contains(secret),
                "public cache error leaked sensitive Redis connection data"
            );
        }
    }
}
