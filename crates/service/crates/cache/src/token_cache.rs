//! Token cache for caching token validation results
//!
//! Reduces database load for frequently validated tokens.

use crate::{CacheResult, CacheService};
use burncloud_database::Database;
use burncloud_database_router::{RouterToken, RouterTokenModel};

/// Token cache wrapper
pub struct TokenCache {
    cache: CacheService,
}

impl TokenCache {
    /// Create a new token cache
    pub fn new(cache: CacheService) -> Self {
        Self { cache }
    }

    /// Get token from cache or database
    pub async fn get_or_fetch(
        &self,
        db: &Database,
        token: &str,
    ) -> CacheResult<Option<RouterToken>> {
        let cache_key = format!("token:{}", token);

        // Try cache first
        if let Some(cached) = self.cache.get::<RouterToken>(&cache_key).await? {
            tracing::trace!("Token cache hit for: {}", token);
            return Ok(Some(cached));
        }

        // Cache miss - fetch from database
        tracing::trace!("Token cache miss for: {}", token);
        let result = RouterTokenModel::validate(db, token).await?;

        // Cache the result if valid
        if let Some(ref token_data) = result {
            let ttl = self.cache.config().token_ttl_secs;
            if let Err(e) = self.cache.set(&cache_key, token_data, ttl).await {
                tracing::warn!("Failed to cache token {}: {}", token, e);
            }
        }

        Ok(result)
    }

    /// Validate token (cached)
    pub async fn validate(&self, db: &Database, token: &str) -> CacheResult<Option<RouterToken>> {
        self.get_or_fetch(db, token).await
    }

    /// Invalidate token cache
    pub async fn invalidate(&self, token: &str) -> CacheResult<()> {
        self.cache.invalidate_token(token).await
    }

    /// Update token in cache (write-through)
    pub async fn update(&self, token: &RouterToken) -> CacheResult<()> {
        let cache_key = format!("token:{}", token.token);
        let ttl = self.cache.config().token_ttl_secs;
        self.cache.set(&cache_key, token, ttl).await
    }

    /// Check if cache is available
    pub async fn is_available(&self) -> bool {
        self.cache.is_available().await
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_token_cache_key_format() {
        let cache_key = format!("token:abc123");
        assert!(cache_key.starts_with("token:"));
        assert!(cache_key.ends_with("abc123"));
    }
}
