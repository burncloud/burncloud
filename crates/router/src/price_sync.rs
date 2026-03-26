//! Price Sync Module
//!
//! This module provides functionality for syncing model pricing data from
//! the burncloud official pricing repository.
//!
//! # PriceSyncService
//!
//! The service supports multi-source, multi-currency price synchronization with
//! the following priority order (highest to lowest):
//! 1. Local override configuration (pricing.override.json)
//! 2. Local main configuration (pricing.json)
//! 3. Remote burncloud repository (with Gitee fallback)

use std::path::PathBuf;
use std::sync::Arc;

use burncloud_common::PricingConfig;
use burncloud_database::{sqlx, Database};
use burncloud_database_models::{PriceInput, PriceModel, TieredPriceInput, TieredPriceModel};
use burncloud_service_billing::PriceCache;
use chrono::{DateTime, Utc};
use reqwest::Client;

/// URL for burncloud official pricing (GitHub)
pub const BURNSCLOUD_PRICES_URL: &str =
    "https://raw.githubusercontent.com/burncloud/burncloud/main/conf/pricing.json";

/// Gitee mirror for burncloud prices — used as fallback when GitHub times out in CN environments
pub const BURNSCLOUD_PRICES_URL_GITEE: &str =
    "https://gitee.com/burncloud/burncloud/raw/main/conf/pricing.json";

/// Result of a sync operation
#[derive(Debug, Clone, Default)]
pub struct SyncResult {
    /// Number of models synced
    pub models_synced: usize,
    /// Number of currencies synced
    pub currencies_synced: usize,
    /// Number of tiered pricing entries synced
    pub tiered_pricing_synced: usize,
    /// Number of models with errors
    pub errors: usize,
    /// Source of the sync
    pub source: String,
}

/// Configuration for PriceSyncService
#[derive(Debug, Clone)]
pub struct PriceSyncConfig {
    /// Path to local override configuration file
    pub override_config_path: PathBuf,
    /// Path to local main configuration file
    pub local_config_path: PathBuf,
    /// URL for remote price repository (primary, typically GitHub)
    pub remote_url: String,
    /// Fallback URL when primary times out (e.g. Gitee mirror for CN)
    pub remote_url_fallback: Option<String>,
    /// Enable remote price sync (default: true)
    pub remote_sync_enabled: bool,
    /// Remote sync interval in seconds (default: 86400 = 24 hours)
    pub remote_sync_interval_secs: u64,
}

impl Default for PriceSyncConfig {
    fn default() -> Self {
        Self {
            override_config_path: PathBuf::from("conf/pricing.override.json"),
            local_config_path: PathBuf::from("conf/pricing.json"),
            remote_url: BURNSCLOUD_PRICES_URL.to_string(),
            remote_url_fallback: Some(BURNSCLOUD_PRICES_URL_GITEE.to_string()),
            remote_sync_enabled: true,
            remote_sync_interval_secs: 86400, // 24 hours
        }
    }
}

/// Price Sync Service supporting multi-source, multi-currency synchronization
///
/// This service supports the following data sources in priority order:
/// 1. Local override configuration (highest priority)
/// 2. Local main configuration
/// 3. Remote burncloud repository (with Gitee fallback)
pub struct PriceSyncService {
    db: Arc<Database>,
    http_client: Client,
    config: PriceSyncConfig,
    /// Last time remote prices were synced
    last_remote_sync: Option<DateTime<Utc>>,
}

impl PriceSyncService {
    /// Create a new PriceSyncService with default configuration
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            http_client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            config: PriceSyncConfig::default(),
            last_remote_sync: None,
        }
    }

    /// Create a new PriceSyncService with custom configuration
    pub fn with_config(db: Arc<Database>, config: PriceSyncConfig) -> Self {
        Self {
            db,
            http_client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            config,
            last_remote_sync: None,
        }
    }

    /// Sync prices from all sources with priority ordering
    ///
    /// Priority order (highest to lowest):
    /// 1. Local override configuration
    /// 2. Local main configuration
    /// 3. Remote burncloud repository (if enabled and due)
    pub async fn sync_all(&mut self) -> anyhow::Result<SyncResult> {
        let mut total_result = SyncResult::default();

        // 1. Load local override configuration (highest priority)
        if let Some(config) = self.load_local_override()? {
            tracing::info!("Applying local override pricing configuration...");
            let result = self.apply_prices(&config, "local_override").await?;
            total_result.models_synced += result.models_synced;
            total_result.currencies_synced += result.currencies_synced;
            total_result.tiered_pricing_synced += result.tiered_pricing_synced;
            total_result.errors += result.errors;
            // Return early - override has highest priority
            return Ok(total_result);
        }

        // 2. Load local main configuration
        if let Some(config) = self.load_local_config()? {
            tracing::info!("Applying local pricing configuration...");
            let result = self.apply_prices(&config, "local").await?;
            total_result.models_synced += result.models_synced;
            total_result.currencies_synced += result.currencies_synced;
            total_result.tiered_pricing_synced += result.tiered_pricing_synced;
            total_result.errors += result.errors;
            // Return early - local config has high priority
            return Ok(total_result);
        }

        // 3. Sync from remote repository (if enabled and due)
        if self.config.remote_sync_enabled && self.should_sync_remote() {
            tracing::info!("Syncing from remote price repository...");
            match self.sync_remote_prices().await {
                Ok(result) => {
                    total_result.models_synced += result.models_synced;
                    total_result.currencies_synced += result.currencies_synced;
                    total_result.tiered_pricing_synced += result.tiered_pricing_synced;
                    total_result.errors += result.errors;
                    self.last_remote_sync = Some(Utc::now());
                }
                Err(e) => {
                    tracing::error!("Remote price sync failed: {}", e);
                    return Err(e);
                }
            }
        }

        Ok(total_result)
    }

    /// Check if remote sync is due
    fn should_sync_remote(&self) -> bool {
        match self.last_remote_sync {
            None => true,
            Some(last) => {
                let elapsed = Utc::now() - last;
                elapsed.num_seconds() >= self.config.remote_sync_interval_secs as i64
            }
        }
    }

    /// Load local override configuration file
    fn load_local_override(&self) -> anyhow::Result<Option<PricingConfig>> {
        let path = &self.config.override_config_path;
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(path)?;
        let config: PricingConfig = serde_json::from_str(&content)?;
        Ok(Some(config))
    }

    /// Load local main configuration file
    fn load_local_config(&self) -> anyhow::Result<Option<PricingConfig>> {
        let path = &self.config.local_config_path;
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(path)?;
        let config: PricingConfig = serde_json::from_str(&content)?;
        Ok(Some(config))
    }

    /// Apply pricing configuration to database
    async fn apply_prices(
        &self,
        config: &PricingConfig,
        source: &str,
    ) -> anyhow::Result<SyncResult> {
        let mut result = SyncResult {
            source: source.to_string(),
            ..Default::default()
        };

        for (model_name, model_pricing) in &config.models {
            // Extract model_type from metadata
            let model_type = model_pricing.metadata.as_ref().and_then(|_m| {
                // Infer model_type from model capabilities or explicit setting
                // For now, we use None as model_type is not stored in ModelMetadata
                None::<String>
            });

            // Convert extended pricing configs to JSON strings
            let voices_pricing_json = model_pricing.voices_pricing.as_ref().and_then(|vp| {
                vp.get("USD").and_then(|config| {
                    serde_json::to_string(&config.voices).ok()
                })
            });

            let video_pricing_json = model_pricing.video_pricing.as_ref().and_then(|vp| {
                vp.get("USD").and_then(|config| {
                    serde_json::to_string(&config.resolutions).ok()
                })
            });

            let asr_pricing_json = model_pricing.asr_pricing.as_ref().and_then(|ap| {
                ap.get("USD").and_then(|config| {
                    serde_json::to_string(&serde_json::json!({"per_minute": config.per_minute})).ok()
                })
            });

            let realtime_pricing_json = model_pricing.realtime_pricing.as_ref().and_then(|rp| {
                rp.get("USD").and_then(|config| {
                    let mut map = serde_json::Map::new();
                    if let Some(v) = config.audio_input {
                        map.insert("audio_input".to_string(), serde_json::json!(v));
                    }
                    if let Some(v) = config.audio_output {
                        map.insert("audio_output".to_string(), serde_json::json!(v));
                    }
                    if let Some(v) = config.image_input {
                        map.insert("image_input".to_string(), serde_json::json!(v));
                    }
                    if map.is_empty() {
                        None
                    } else {
                        serde_json::to_string(&map).ok()
                    }
                })
            });

            // Apply standard pricing for each currency
            for (currency, currency_pricing) in &model_pricing.pricing {
                // Get currency-specific extended pricing if available
                let currency_voices = model_pricing.voices_pricing.as_ref()
                    .and_then(|vp| vp.get(currency))
                    .and_then(|config| serde_json::to_string(&config.voices).ok())
                    .or_else(|| voices_pricing_json.clone());

                let currency_video = model_pricing.video_pricing.as_ref()
                    .and_then(|vp| vp.get(currency))
                    .and_then(|config| serde_json::to_string(&config.resolutions).ok())
                    .or_else(|| video_pricing_json.clone());

                let currency_asr = model_pricing.asr_pricing.as_ref()
                    .and_then(|ap| ap.get(currency))
                    .and_then(|config| serde_json::to_string(&serde_json::json!({"per_minute": config.per_minute})).ok())
                    .or_else(|| asr_pricing_json.clone());

                let currency_realtime = model_pricing.realtime_pricing.as_ref()
                    .and_then(|rp| rp.get(currency))
                    .and_then(|config| {
                        let mut map = serde_json::Map::new();
                        if let Some(v) = config.audio_input {
                            map.insert("audio_input".to_string(), serde_json::json!(v));
                        }
                        if let Some(v) = config.audio_output {
                            map.insert("audio_output".to_string(), serde_json::json!(v));
                        }
                        if let Some(v) = config.image_input {
                            map.insert("image_input".to_string(), serde_json::json!(v));
                        }
                        if map.is_empty() { None } else { serde_json::to_string(&map).ok() }
                    })
                    .or_else(|| realtime_pricing_json.clone());

                let price_input = PriceInput {
                    model: model_name.clone(),
                    currency: currency.clone(),
                    input_price: currency_pricing.input_price,
                    output_price: currency_pricing.output_price,
                    source: currency_pricing.source.clone().or(Some(source.to_string())),
                    voices_pricing: currency_voices,
                    video_pricing: currency_video,
                    asr_pricing: currency_asr,
                    realtime_pricing: currency_realtime,
                    model_type: model_type.clone(),
                    ..Default::default()
                };

                match PriceModel::upsert(&self.db, &price_input).await {
                    Ok(_) => {
                        result.models_synced += 1;
                        result.currencies_synced += 1;
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to upsert price for {} ({}): {}",
                            model_name, currency, e
                        );
                        result.errors += 1;
                    }
                }
            }

            // Apply cache pricing
            if let Some(ref cache_pricing) = model_pricing.cache_pricing {
                for (currency, cache_config) in cache_pricing {
                    // Get existing price and update with cache pricing
                    if let Ok(Some(mut existing)) =
                        PriceModel::get(&self.db, model_name, currency, None).await
                    {
                        existing.cache_read_input_price = Some(cache_config.cache_read_input_price);
                        existing.cache_creation_input_price =
                            cache_config.cache_creation_input_price;

                        let update_input = PriceInput {
                            model: existing.model.clone(),
                            currency: existing.currency.clone(),
                            input_price: existing.input_price,
                            output_price: existing.output_price,
                            cache_read_input_price: Some(cache_config.cache_read_input_price),
                            cache_creation_input_price: cache_config.cache_creation_input_price,
                            batch_input_price: existing.batch_input_price,
                            batch_output_price: existing.batch_output_price,
                            priority_input_price: existing.priority_input_price,
                            priority_output_price: existing.priority_output_price,
                            audio_input_price: existing.audio_input_price,
                            audio_output_price: existing.audio_output_price,
                            reasoning_price: existing.reasoning_price,
                            embedding_price: existing.embedding_price,
                            image_price: existing.image_price,
                            video_price: existing.video_price,
                            source: existing.source.clone(),
                            region: existing.region.clone(),
                            context_window: existing.context_window,
                            max_output_tokens: existing.max_output_tokens,
                            supports_vision: existing.supports_vision_bool(),
                            supports_function_calling: existing.supports_function_calling_bool(),
                            voices_pricing: existing.voices_pricing.clone(),
                            video_pricing: existing.video_pricing.clone(),
                            asr_pricing: existing.asr_pricing.clone(),
                            realtime_pricing: existing.realtime_pricing.clone(),
                            model_type: existing.model_type.clone(),
                        };

                        if let Err(e) = PriceModel::upsert(&self.db, &update_input).await {
                            tracing::error!("Failed to update cache pricing for {}: {}", model_name, e);
                        }
                    }
                }
            }

            // Apply batch pricing
            if let Some(ref batch_pricing) = model_pricing.batch_pricing {
                for (currency, batch_config) in batch_pricing {
                    if let Ok(Some(mut existing)) =
                        PriceModel::get(&self.db, model_name, currency, None).await
                    {
                        existing.batch_input_price = Some(batch_config.batch_input_price);
                        existing.batch_output_price = Some(batch_config.batch_output_price);

                        let update_input = PriceInput {
                            model: existing.model.clone(),
                            currency: existing.currency.clone(),
                            input_price: existing.input_price,
                            output_price: existing.output_price,
                            cache_read_input_price: existing.cache_read_input_price,
                            cache_creation_input_price: existing.cache_creation_input_price,
                            batch_input_price: Some(batch_config.batch_input_price),
                            batch_output_price: Some(batch_config.batch_output_price),
                            priority_input_price: existing.priority_input_price,
                            priority_output_price: existing.priority_output_price,
                            audio_input_price: existing.audio_input_price,
                            audio_output_price: existing.audio_output_price,
                            reasoning_price: existing.reasoning_price,
                            embedding_price: existing.embedding_price,
                            image_price: existing.image_price,
                            video_price: existing.video_price,
                            source: existing.source.clone(),
                            region: existing.region.clone(),
                            context_window: existing.context_window,
                            max_output_tokens: existing.max_output_tokens,
                            supports_vision: existing.supports_vision_bool(),
                            supports_function_calling: existing.supports_function_calling_bool(),
                            voices_pricing: existing.voices_pricing.clone(),
                            video_pricing: existing.video_pricing.clone(),
                            asr_pricing: existing.asr_pricing.clone(),
                            realtime_pricing: existing.realtime_pricing.clone(),
                            model_type: existing.model_type.clone(),
                        };

                        if let Err(e) = PriceModel::upsert(&self.db, &update_input).await {
                            tracing::error!("Failed to update batch pricing for {}: {}", model_name, e);
                        }
                    }
                }
            }

            // Apply tiered pricing
            if let Some(ref tiered_pricing) = model_pricing.tiered_pricing {
                for (currency, tiers) in tiered_pricing {
                    for tier in tiers {
                        let tier_input = TieredPriceInput {
                            model: model_name.clone(),
                            region: Some(currency.clone()), // Use currency as region identifier
                            currency: Some(currency.clone()),
                            tier_type: Some("context_length".to_string()),
                            tier_start: tier.tier_start,
                            tier_end: tier.tier_end,
                            input_price: tier.input_price,
                            output_price: tier.output_price,
                        };

                        match TieredPriceModel::upsert_tier(&self.db, &tier_input).await {
                            Ok(_) => {
                                result.tiered_pricing_synced += 1;
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to upsert tiered pricing for {} ({}): {}",
                                    model_name, currency, e
                                );
                                result.errors += 1;
                            }
                        }
                    }
                }
            }
        }

        tracing::info!(
            "Applied {} models, {} currencies, {} tiers from {}",
            result.models_synced, result.currencies_synced, result.tiered_pricing_synced, source
        );

        Ok(result)
    }

    /// Sync prices from remote repository (with Gitee fallback)
    async fn sync_remote_prices(&self) -> anyhow::Result<SyncResult> {
        let response = self.fetch_remote_config().await?;
        let config: PricingConfig = serde_json::from_str(&response)?;
        self.apply_prices(&config, "remote").await
    }

    /// Fetch remote pricing config, with automatic fallback to Gitee mirror on timeout/error.
    async fn fetch_remote_config(&self) -> anyhow::Result<String> {
        match self
            .http_client
            .get(&self.config.remote_url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
        {
            Ok(response) => {
                let text = response.text().await?;
                Ok(text)
            }
            Err(e) => {
                if let Some(fallback_url) = &self.config.remote_url_fallback {
                    tracing::warn!(
                        primary_url = %self.config.remote_url,
                        error = %e,
                        "Primary URL failed, trying fallback mirror"
                    );
                    let response = self
                        .http_client
                        .get(fallback_url)
                        .send()
                        .await?
                        .error_for_status()?;
                    let text = response.text().await?;
                    Ok(text)
                } else {
                    Err(e.into())
                }
            }
        }
    }

    /// Import tiered pricing from a JSON structure
    ///
    /// This is used for models like Qwen that have tiered pricing based on context length.
    pub async fn import_tiered_pricing(&self, tiers: &[TieredPriceInput]) -> anyhow::Result<usize> {
        let mut imported_count = 0;

        for tier in tiers {
            // Validate tier data (prices are now i64 nanodollars, so compare with 0)
            if tier.input_price < 0 || tier.output_price < 0 {
                tracing::error!(
                    "Skipping tier with invalid price for model {}: prices must be >= 0",
                    tier.model
                );
                continue;
            }

            if let Some(tier_end) = tier.tier_end {
                if tier.tier_start >= tier_end {
                    tracing::error!(
                        "Skipping tier with invalid range for model {}: tier_start ({}) must be < tier_end ({})",
                        tier.model, tier.tier_start, tier_end
                    );
                    continue;
                }
            }

            match TieredPriceModel::upsert_tier(&self.db, tier).await {
                Ok(_) => {
                    imported_count += 1;
                    tracing::info!(
                        "Imported tier for model {} ({}-{} tokens): ${:.4}/${:.4} per 1M",
                        tier.model,
                        tier.tier_start,
                        tier.tier_end.map_or("∞".to_string(), |e| format!("{}", e)),
                        tier.input_price,
                        tier.output_price
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to import tier for {}: {}", tier.model, e);
                }
            }
        }

        tracing::info!(
            "Tiered pricing import complete: {} tiers imported",
            imported_count
        );
        Ok(imported_count)
    }

    /// Sync model capabilities to the local database
    ///
    /// Returns the number of capabilities updated/inserted
    pub async fn sync_capabilities(&self) -> anyhow::Result<usize> {
        let text = self.fetch_remote_config().await?;
        let config: PricingConfig = serde_json::from_str(&text)?;
        let mut updated_count = 0;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let conn = self.db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = self.db.kind() == "postgres";

        for (model_name, model_pricing) in &config.models {
            // Get pricing info for capabilities table
            let (input_price, output_price) = model_pricing.pricing.get("USD").map(|p| {
                (Some(p.input_price as f64 / 1_000_000_000.0), Some(p.output_price as f64 / 1_000_000_000.0))
            }).unwrap_or((None, None));

            // Get metadata
            let (context_window, max_output_tokens, supports_vision, supports_function_calling) =
                model_pricing.metadata.as_ref()
                    .map(|m| {
                        (m.context_window, m.max_output_tokens, m.supports_vision, m.supports_function_calling)
                    })
                    .unwrap_or((None, None, false, false));

            // Build the SQL
            let sql = if is_postgres {
                r#"
                INSERT INTO model_capabilities (model, context_window, max_output_tokens, supports_vision, supports_function_calling, input_price, output_price, synced_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT(model) DO UPDATE SET
                    context_window = EXCLUDED.context_window,
                    max_output_tokens = EXCLUDED.max_output_tokens,
                    supports_vision = EXCLUDED.supports_vision,
                    supports_function_calling = EXCLUDED.supports_function_calling,
                    input_price = EXCLUDED.input_price,
                    output_price = EXCLUDED.output_price,
                    synced_at = EXCLUDED.synced_at
                "#
            } else {
                r#"
                INSERT INTO model_capabilities (model, context_window, max_output_tokens, supports_vision, supports_function_calling, input_price, output_price, synced_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(model) DO UPDATE SET
                    context_window = excluded.context_window,
                    max_output_tokens = excluded.max_output_tokens,
                    supports_vision = excluded.supports_vision,
                    supports_function_calling = excluded.supports_function_calling,
                    input_price = excluded.input_price,
                    output_price = excluded.output_price,
                    synced_at = excluded.synced_at
                "#
            };

            let result = sqlx::query(sql)
                .bind(model_name)
                .bind(context_window)
                .bind(max_output_tokens)
                .bind(supports_vision)
                .bind(supports_function_calling)
                .bind(input_price)
                .bind(output_price)
                .bind(now)
                .execute(pool)
                .await;

            match result {
                Ok(_) => {
                    updated_count += 1;
                }
                Err(e) => {
                    tracing::error!("Failed to sync capabilities for {}: {}", model_name, e);
                }
            }
        }

        tracing::info!(
            "Capabilities sync complete: {} models updated",
            updated_count
        );
        Ok(updated_count)
    }
}

/// Start a background price sync task
///
/// This spawns a tokio task that syncs prices periodically from multiple sources
pub fn start_price_sync_task(
    db: Arc<Database>,
    interval_secs: u64,
    config: Option<PriceSyncConfig>,
    price_cache: PriceCache,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut service = match config {
            Some(cfg) => PriceSyncService::with_config(db.clone(), cfg),
            None => PriceSyncService::new(db.clone()),
        };
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));

        // Initial sync
        tracing::info!("Starting initial multi-source price sync...");
        match service.sync_all().await {
            Ok(result) => {
                tracing::info!(
                    models = result.models_synced,
                    currencies = result.currencies_synced,
                    tiers = result.tiered_pricing_synced,
                    "Initial price sync complete"
                );
                if let Err(e) = price_cache.refresh(&db).await {
                    tracing::error!("Failed to refresh price cache after initial sync: {e}");
                }
            }
            Err(e) => tracing::error!("Initial price sync failed: {e}"),
        }

        // Periodic sync
        loop {
            interval.tick().await;
            tracing::info!("Starting periodic multi-source price sync...");
            match service.sync_all().await {
                Ok(result) => {
                    tracing::info!(
                        models = result.models_synced,
                        currencies = result.currencies_synced,
                        tiers = result.tiered_pricing_synced,
                        "Periodic price sync complete"
                    );
                    if let Err(e) = price_cache.refresh(&db).await {
                        tracing::error!("Failed to refresh price cache after periodic sync: {e}");
                    }
                }
                Err(e) => tracing::error!("Periodic price sync failed: {e}"),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PriceSyncConfig::default();
        assert_eq!(config.remote_url, BURNSCLOUD_PRICES_URL);
        assert_eq!(config.remote_url_fallback, Some(BURNSCLOUD_PRICES_URL_GITEE.to_string()));
        assert!(config.remote_sync_enabled);
        assert_eq!(config.remote_sync_interval_secs, 86400);
    }

    #[test]
    fn test_sync_result_default() {
        let result = SyncResult::default();
        assert_eq!(result.models_synced, 0);
        assert_eq!(result.currencies_synced, 0);
        assert_eq!(result.tiered_pricing_synced, 0);
        assert_eq!(result.errors, 0);
        assert!(result.source.is_empty());
    }
}
