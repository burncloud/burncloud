//! Quota cache for caching user quota balance
//!
//! Reduces database load for quota check queries.

use crate::{CacheResult, CacheService};
use burncloud_database::Database;
use burncloud_database_router::{RouterToken, RouterTokenModel};

/// Quota cache key prefix
const QUOTA_KEY_PREFIX: &str = "quota:";

/// Quota info for caching
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuotaInfo {
    /// Token string
    pub token: String,
    /// Quota limit (-1 for unlimited)
    pub quota_limit: i64,
    /// Used quota
    pub used_quota: i64,
    /// Remaining quota
    pub remaining: i64,
}

impl From<&RouterToken> for QuotaInfo {
    fn from(token: &RouterToken) -> Self {
        let remaining = if token.quota_limit < 0 {
            i64::MAX // unlimited
        } else {
            (token.quota_limit - token.used_quota).max(0)
        };
        Self {
            token: token.token.clone(),
            quota_limit: token.quota_limit,
            used_quota: token.used_quota,
            remaining,
        }
    }
}

/// Quota cache wrapper
pub struct QuotaCache {
    cache: CacheService,
}

impl QuotaCache {
    /// Create a new quota cache
    pub fn new(cache: CacheService) -> Self {
        Self { cache }
    }

    /// Get quota from cache or database
    pub async fn get_or_fetch(&self, db: &Database, token: &str) -> CacheResult<Option<QuotaInfo>> {
        let cache_key = format!("{}{}", QUOTA_KEY_PREFIX, token);

        // Try cache first
        if let Some(cached) = self.cache.get::<QuotaInfo>(&cache_key).await? {
            tracing::trace!("Quota cache hit for token: {}", token);
            return Ok(Some(cached));
        }

        // Cache miss - fetch from database
        tracing::trace!("Quota cache miss for token: {}", token);
        let token_data = RouterTokenModel::validate(db, token).await?;

        // Convert to quota info and cache
        let quota_info = token_data.as_ref().map(QuotaInfo::from);

        if let Some(ref info) = quota_info {
            let ttl = self.cache.config().quota_ttl_secs;
            if let Err(e) = self.cache.set(&cache_key, info, ttl).await {
                tracing::warn!("Failed to cache quota for token {}: {}", token, e);
            }
        }

        Ok(quota_info)
    }

    /// Check if quota is sufficient (cached)
    pub async fn check_quota(&self, db: &Database, token: &str, cost: i64) -> CacheResult<bool> {
        if cost <= 0 {
            return Ok(true);
        }

        let quota_info = self.get_or_fetch(db, token).await?;
        match quota_info {
            Some(info) => {
                // unlimited quota
                if info.quota_limit < 0 {
                    Ok(true)
                } else {
                    Ok(info.remaining >= cost)
                }
            }
            None => Ok(false), // token not found
        }
    }

    /// Invalidate quota cache for a token
    pub async fn invalidate(&self, token: &str) -> CacheResult<()> {
        self.cache.invalidate_quota(token).await
    }

    /// Update quota in cache after deduction (write-through)
    pub async fn update_after_deduction(
        &self,
        token: &str,
        quota_limit: i64,
        used_quota: i64,
    ) -> CacheResult<()> {
        let cache_key = format!("{}{}", QUOTA_KEY_PREFIX, token);
        let remaining = if quota_limit < 0 {
            i64::MAX
        } else {
            (quota_limit - used_quota).max(0)
        };

        let info = QuotaInfo {
            token: token.to_string(),
            quota_limit,
            used_quota,
            remaining,
        };

        let ttl = self.cache.config().quota_ttl_secs;
        self.cache.set(&cache_key, &info, ttl).await
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
    fn test_quota_info_from_token() {
        let token = RouterToken {
            token: "test-token".to_string(),
            user_id: "user-1".to_string(),
            status: "active".to_string(),
            quota_limit: 1000,
            used_quota: 300,
            expired_time: -1,
            accessed_time: 0,
        };

        let info = QuotaInfo::from(&token);
        assert_eq!(info.remaining, 700);
        assert_eq!(info.quota_limit, 1000);
        assert_eq!(info.used_quota, 300);
    }

    #[test]
    fn test_quota_info_unlimited() {
        let token = RouterToken {
            token: "test-token".to_string(),
            user_id: "user-1".to_string(),
            status: "active".to_string(),
            quota_limit: -1,
            used_quota: 1000,
            expired_time: -1,
            accessed_time: 0,
        };

        let info = QuotaInfo::from(&token);
        assert_eq!(info.remaining, i64::MAX);
    }
}
