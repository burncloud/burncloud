//! Price cache for caching model prices
//!
//! Reduces database load for price lookup queries.

use crate::{CacheResult, CacheService};
use burncloud_common::types::Price;
use burncloud_database::Database;
use burncloud_database_billing::BillingPriceModel;

/// Price cache key prefix
const PRICE_KEY_PREFIX: &str = "price:";
/// Price list cache key
const PRICE_LIST_KEY: &str = "price:list";

/// Price cache wrapper
pub struct PriceCache {
    cache: CacheService,
}

impl PriceCache {
    /// Create a new price cache
    pub fn new(cache: CacheService) -> Self {
        Self { cache }
    }

    /// Get price for a model from cache or database
    pub async fn get_or_fetch(
        &self,
        db: &Database,
        model: &str,
        region: Option<&str>,
    ) -> CacheResult<Option<Price>> {
        let region_str = region.unwrap_or("");
        let cache_key = format!(
            "{}{}:{}",
            PRICE_KEY_PREFIX,
            model.to_lowercase(),
            region_str
        );

        // Try cache first
        if let Some(cached) = self.cache.get::<Price>(&cache_key).await? {
            tracing::trace!("Price cache hit for model: {} region: {:?}", model, region);
            return Ok(Some(cached));
        }

        // Cache miss - fetch from database
        tracing::trace!("Price cache miss for model: {} region: {:?}", model, region);
        let prices = BillingPriceModel::list(db, i32::MAX, 0, Some(model), None).await?;

        // Find matching price (region-specific first, then universal)
        let price = prices
            .iter()
            .find(|p| {
                let p_region = p.region.as_deref().unwrap_or("");
                if region.is_some() && p_region == region_str {
                    return true;
                }
                if region.is_none() && p_region.is_empty() {
                    return true;
                }
                false
            })
            .cloned();

        // Cache the result if found
        if let Some(ref p) = price {
            let ttl = self.cache.config().price_ttl_secs;
            if let Err(e) = self.cache.set(&cache_key, p, ttl).await {
                tracing::warn!("Failed to cache price for {}: {}", model, e);
            }
        }

        Ok(price)
    }

    /// Get all prices from cache or database
    pub async fn get_all_or_fetch(&self, db: &Database) -> CacheResult<Vec<Price>> {
        // Try cache first
        if let Some(cached) = self.cache.get::<Vec<Price>>(PRICE_LIST_KEY).await? {
            tracing::trace!("Price list cache hit");
            return Ok(cached);
        }

        // Cache miss - fetch from database
        tracing::trace!("Price list cache miss");
        let prices = BillingPriceModel::list(db, i32::MAX, 0, None, None).await?;

        // Cache the result
        let ttl = self.cache.config().price_ttl_secs;
        if let Err(e) = self.cache.set(PRICE_LIST_KEY, &prices, ttl).await {
            tracing::warn!("Failed to cache price list: {}", e);
        }

        Ok(prices)
    }

    /// Invalidate price cache for a model
    pub async fn invalidate(&self, model: &str, region: Option<&str>) -> CacheResult<()> {
        self.cache.invalidate_price(model, region).await?;
        // Also invalidate the list cache
        self.cache.delete(PRICE_LIST_KEY).await?;
        Ok(())
    }

    /// Invalidate all price cache
    pub async fn invalidate_all(&self) -> CacheResult<()> {
        self.cache.delete_pattern("price:*").await?;
        self.cache.delete(PRICE_LIST_KEY).await?;
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
    fn test_price_cache_key_format() {
        let cache_key = format!("{}gpt-4:us-east", PRICE_KEY_PREFIX);
        assert!(cache_key.starts_with("price:"));
        assert!(cache_key.contains("gpt-4"));
    }
}
