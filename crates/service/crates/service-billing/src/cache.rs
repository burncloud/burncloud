use burncloud_common::types::Price;
use burncloud_database::Database;
use burncloud_database_models::PriceModel;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory price cache.
///
/// Loaded at server startup and refreshed atomically whenever `POST /api/v1/prices`
/// is called — no background polling needed.
///
/// Cache key: lowercase model name.
/// When a model appears with multiple regions/currencies, the first loaded entry wins.
/// Consumers that need region-specific pricing should call `PriceModel::get()` directly.
#[derive(Clone)]
pub struct PriceCache {
    pub(crate) inner: Arc<RwLock<HashMap<String, Price>>>,
}

impl PriceCache {
    /// Create an empty cache (useful for testing).
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load all prices from the database.
    pub async fn load(db: &Database) -> Result<Self, burncloud_database::DatabaseError> {
        let cache = Self::empty();
        cache.refresh(db).await?;
        Ok(cache)
    }

    /// Reload all prices from the database, replacing the current cache atomically.
    pub async fn refresh(&self, db: &Database) -> Result<(), burncloud_database::DatabaseError> {
        let prices = PriceModel::list(db, 10_000, 0, None, None).await?;

        let mut map = HashMap::with_capacity(prices.len());
        for price in prices {
            // Normalise key to lowercase for case-insensitive lookup
            map.entry(price.model.to_lowercase())
                .or_insert(price);
        }

        let mut guard = self.inner.write().await;
        *guard = map;
        tracing::info!(count = guard.len(), "PriceCache refreshed");
        Ok(())
    }

    /// Look up a price by model name (case-insensitive).
    /// Returns `None` when the model is not in the cache.
    pub async fn get(&self, model: &str) -> Option<Price> {
        let guard = self.inner.read().await;
        guard.get(&model.to_lowercase()).cloned()
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
        assert!(cache.get("gpt-4o").await.is_none());
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
            source: None,
            region: None,
            context_window: None,
            max_output_tokens: None,
            supports_vision: None,
            supports_function_calling: None,
            synced_at: None,
            created_at: None,
            updated_at: None,
        };

        {
            let mut guard = cache.inner.write().await;
            guard.insert(price.model.to_lowercase(), price);
        }

        assert!(cache.get("gpt-4o").await.is_some());
        assert!(cache.get("GPT-4O").await.is_some());
        assert!(cache.get("Gpt-4o").await.is_some());
    }
}
