use burncloud_common::types::Price;
use burncloud_database::Database;
use burncloud_database_billing::BillingPriceModel;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory price cache.
///
/// Loaded at server startup and refreshed atomically whenever `POST /api/v1/prices`
/// is called — no background polling needed.
///
/// Cache key: `(lowercase_model, region)` where region is `""` for the universal price.
/// Lookup via `get(model, region)` tries the region-specific entry first, then falls
/// back to the universal entry (region = `""`).
#[derive(Clone)]
pub struct PriceCache {
    pub(crate) inner: Arc<RwLock<HashMap<(String, String), Price>>>,
}

impl PriceCache {
    /// Create an empty cache (useful for testing).
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Cache key: `(lowercase_model, region)`.
    fn make_key(model: &str, region: &str) -> (String, String) {
        (model.to_lowercase(), region.to_string())
    }

    /// Load all prices from the database.
    pub async fn load(db: &Database) -> Result<Self, burncloud_database::DatabaseError> {
        let cache = Self::empty();
        cache.refresh(db).await?;
        Ok(cache)
    }

    /// Reload all prices from the database, replacing the current cache atomically.
    pub async fn refresh(&self, db: &Database) -> Result<(), burncloud_database::DatabaseError> {
        // No limit: load all prices regardless of count to avoid silent truncation.
        let prices = BillingPriceModel::list(db, i32::MAX, 0, None, None).await?;

        let mut map = HashMap::with_capacity(prices.len());
        for price in prices {
            // Key: (lowercase_model, region). region stored as "" for universal entries.
            let region = price.region.clone().unwrap_or_default();
            map.entry(Self::make_key(&price.model, &region))
                .or_insert(price);
        }

        let mut guard = self.inner.write().await;
        *guard = map;
        tracing::info!(count = guard.len(), "PriceCache refreshed");
        Ok(())
    }

    /// Look up a price by model name and optional region (case-insensitive).
    ///
    /// Lookup order (for each candidate model name):
    /// 1. Region-specific entry: `(model, region)`
    /// 2. Universal entry fallback: `(model, "")`
    ///
    /// Model name matching:
    /// 1. Exact match (e.g. `gpt-4o-2024-11-20`)
    /// 2. Prefix match: strip the last `-<segment>` until a match is found
    ///    (e.g. `gpt-4o-2024-11-20` → `gpt-4o-2024-11` → `gpt-4o`)
    ///
    /// Returns `None` when no match is found.
    pub async fn get(&self, model: &str, region: Option<&str>) -> Option<Price> {
        let key = model.to_lowercase();
        let region = region.unwrap_or("");
        let guard = self.inner.read().await;

        // Helper: try region-specific then universal for a given model key
        let lookup = |m: &str| -> Option<Price> {
            // Region-specific first
            if !region.is_empty() {
                if let Some(p) = guard.get(&(m.to_string(), region.to_string())) {
                    return Some(p.clone());
                }
            }
            // Universal fallback
            guard.get(&(m.to_string(), String::new())).cloned()
        };

        // 1. Exact match
        if let Some(price) = lookup(&key) {
            return Some(price);
        }

        // 2. Prefix match: iteratively strip last `-segment`
        let mut prefix = key.as_str();
        while let Some(idx) = prefix.rfind('-') {
            prefix = &prefix[..idx];
            if let Some(price) = lookup(prefix) {
                return Some(price);
            }
        }

        None
    }

    /// Number of entries in the cache.
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_cache_returns_none() {
        let cache = PriceCache::empty();
        assert!(cache.get("gpt-4o", None).await.is_none());
    }

    #[tokio::test]
    async fn test_case_insensitive_lookup() {
        use burncloud_common::types::Price;
        let cache = PriceCache::empty();

        let price = Price {
            id: 1,
            model: "GPT-4O".to_string(),
            currency: "USD".to_string(),
            input_price: 5000,
            output_price: 15000,
            cache_read_input_price: None,
            cache_creation_input_price: None,
            batch_input_price: None,
            batch_output_price: None,
            priority_input_price: None,
            priority_output_price: None,
            audio_input_price: None,
            audio_output_price: None,
            reasoning_price: None,
            embedding_price: None,
            image_price: None,
            video_price: None,
            music_price: None,
            source: None,
            region: None,
            context_window: None,
            max_output_tokens: None,
            supports_vision: None,
            supports_function_calling: None,
            voices_pricing: None,
            video_pricing: None,
            asr_pricing: None,
            realtime_pricing: None,
            model_type: None,
            synced_at: None,
            created_at: None,
            updated_at: None,
        };

        {
            let mut guard = cache.inner.write().await;
            guard.insert((price.model.to_lowercase(), String::new()), price);
        }

        assert!(cache.get("gpt-4o", None).await.is_some());
        assert!(cache.get("GPT-4O", None).await.is_some());
        assert!(cache.get("Gpt-4o", None).await.is_some());
    }
}
