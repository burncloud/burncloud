//! Channel cache for caching channel configuration
//!
//! Reduces database load for channel list and weight queries.

use crate::{CacheResult, CacheService};
use burncloud_common::types::Channel;
use burncloud_database::Database;
use burncloud_database_channel::ChannelProviderModel;

/// Channel list cache key
const CHANNEL_LIST_KEY: &str = "channel:list";
/// Channel by ID cache key prefix
const CHANNEL_BY_ID_PREFIX: &str = "channel:id:";

/// Channel cache wrapper
pub struct ChannelCache {
    cache: CacheService,
}

impl ChannelCache {
    /// Create a new channel cache
    pub fn new(cache: CacheService) -> Self {
        Self { cache }
    }

    /// Get all channels from cache or database
    pub async fn get_all_or_fetch(&self, db: &Database) -> CacheResult<Vec<Channel>> {
        // Try cache first
        if let Some(cached) = self.cache.get::<Vec<Channel>>(CHANNEL_LIST_KEY).await? {
            tracing::trace!("Channel list cache hit");
            return Ok(cached);
        }

        // Cache miss - fetch from database
        tracing::trace!("Channel list cache miss");
        // Fetch all channels (no limit)
        let channels = ChannelProviderModel::list(db, i32::MAX, 0).await?;

        // Cache the result
        let ttl = self.cache.config().channel_ttl_secs;
        if let Err(e) = self.cache.set(CHANNEL_LIST_KEY, &channels, ttl).await {
            tracing::warn!("Failed to cache channel list: {}", e);
        }

        Ok(channels)
    }

    /// Get channel by ID from cache or database
    pub async fn get_by_id_or_fetch(
        &self,
        db: &Database,
        channel_id: i32,
    ) -> CacheResult<Option<Channel>> {
        let cache_key = format!("{}{}", CHANNEL_BY_ID_PREFIX, channel_id);

        // Try cache first
        if let Some(cached) = self.cache.get::<Channel>(&cache_key).await? {
            tracing::trace!("Channel cache hit for ID: {}", channel_id);
            return Ok(Some(cached));
        }

        // Cache miss - fetch from database
        tracing::trace!("Channel cache miss for ID: {}", channel_id);
        let channel = ChannelProviderModel::get_by_id(db, channel_id).await?;

        // Cache the result if found
        if let Some(ref ch) = channel {
            let ttl = self.cache.config().channel_ttl_secs;
            if let Err(e) = self.cache.set(&cache_key, ch, ttl).await {
                tracing::warn!("Failed to cache channel {}: {}", channel_id, e);
            }
        }

        Ok(channel)
    }

    /// Invalidate all channel cache
    pub async fn invalidate_all(&self) -> CacheResult<()> {
        self.cache.delete(CHANNEL_LIST_KEY).await?;
        self.cache.delete_pattern(CHANNEL_BY_ID_PREFIX).await?;
        Ok(())
    }

    /// Invalidate specific channel cache
    pub async fn invalidate(&self, channel_id: i32) -> CacheResult<()> {
        let cache_key = format!("{}{}", CHANNEL_BY_ID_PREFIX, channel_id);
        self.cache.delete(&cache_key).await?;
        // Also invalidate the list cache
        self.cache.delete(CHANNEL_LIST_KEY).await?;
        Ok(())
    }

    /// Check if cache is available
    pub async fn is_available(&self) -> bool {
        self.cache.is_available().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_cache_keys() {
        assert_eq!(CHANNEL_LIST_KEY, "channel:list");
        assert!(CHANNEL_BY_ID_PREFIX.starts_with("channel:id:"));
    }
}
