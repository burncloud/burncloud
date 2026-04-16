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
//! 2. Remote pricing_data repository (GitHub, with Gitee fallback)
//!    - Startup fast path: if DB already has prices, skip remote fetch
//!    - Periodic sync (forced=true): always fetches remote

use std::path::PathBuf;
use std::sync::Arc;

use burncloud_common::PricingConfig;
use burncloud_database::{sqlx, Database};
use burncloud_database_models::{
    BillingPriceModel, BillingTieredPriceModel, PriceInput, TieredPriceInput,
};
use burncloud_service_billing::PriceCache;
use chrono::{DateTime, Utc};
use reqwest::Client;

/// URL for burncloud official pricing (GitHub)
pub const BURNSCLOUD_PRICES_URL: &str =
    "https://raw.githubusercontent.com/burncloud/pricing_data/main/pricing.json";

/// Gitee mirror for burncloud prices — used as fallback when GitHub times out in CN environments
pub const BURNSCLOUD_PRICES_URL_GITEE: &str =
    "https://gitee.com/burncloud/pricing_data/raw/main/pricing.json";

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
/// 2. Remote pricing_data repository (GitHub, with Gitee fallback)
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

    /// Sync prices from all sources with priority ordering.
    ///
    /// When `forced` is false (startup): if DB already has prices, skip remote fetch.
    /// When `forced` is true (periodic/force-sync): always pull from remote.
    ///
    /// On remote failure:
    /// - If DB has prices → warn and return Ok (graceful degradation)
    /// - If DB is empty → retry up to 3 times (5s, 15s, 30s), then return Err (fatal)
    pub async fn sync_all(&mut self, forced: bool) -> anyhow::Result<SyncResult> {
        // 1. Local override (highest priority, always checked)
        if let Some(config) = self.load_local_override()? {
            tracing::info!("Applying local override pricing configuration...");
            return self.apply_prices(&config, "local_override").await;
        }

        // 2. Startup fast path: DB has prices and sync not forced → skip remote
        if !forced {
            let db_count = self.count_db_models().await.unwrap_or(0);
            if db_count > 0 {
                tracing::info!(
                    models = db_count,
                    "DB already has prices, skipping remote sync (startup fast path)"
                );
                return Ok(SyncResult {
                    source: "db_cache".to_string(),
                    ..Default::default()
                });
            }
        }

        // 3. Pull from remote with retry on cold start
        const RETRY_DELAYS_SECS: &[u64] = &[5, 15, 30];
        let mut last_err: Option<anyhow::Error> = None;
        for (attempt, &delay) in RETRY_DELAYS_SECS.iter().enumerate() {
            match self.sync_remote_prices().await {
                Ok(result) => {
                    self.last_remote_sync = Some(Utc::now());
                    return Ok(result);
                }
                Err(e) => {
                    if attempt < RETRY_DELAYS_SECS.len() - 1 {
                        tracing::warn!(
                            attempt = attempt + 1,
                            delay_secs = delay,
                            error = %e,
                            "Remote price sync failed, retrying..."
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                    }
                    last_err = Some(e);
                }
            }
        }

        // All retries exhausted
        let err = last_err
            .ok_or_else(|| anyhow::anyhow!("last_err is set after at least one retry attempt"))?;
        let db_count = self.count_db_models().await.unwrap_or(0);
        if db_count > 0 {
            tracing::warn!(
                error = %err,
                models = db_count,
                "Remote sync failed after all retries, using existing DB prices"
            );
            return Ok(SyncResult {
                source: "db_fallback".to_string(),
                ..Default::default()
            });
        }

        tracing::error!(
            error = %err,
            "FATAL: pricing_data unreachable and DB has no prices. \
             Check network connectivity or pre-seed DB."
        );
        Err(err)
    }

    /// Count distinct model names in the prices table.
    async fn count_db_models(&self) -> anyhow::Result<usize> {
        let conn = self.db.get_connection()?;
        let row: (i64,) = sqlx::query_as("SELECT COUNT(DISTINCT model) FROM billing_prices")
            .fetch_one(conn.pool())
            .await?;
        Ok(row.0 as usize)
    }

    /// Load local override configuration file
    fn load_local_override(&self) -> anyhow::Result<Option<PricingConfig>> {
        let path = &self.config.override_config_path;
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(path)?;
        let config = PricingConfig::from_json(&content)?;
        Ok(Some(config))
    }

    /// Apply pricing configuration to database
    pub async fn apply_prices(
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
            let model_type: Option<String> = model_pricing.metadata.as_ref().and(None);

            // Convert extended pricing configs to JSON strings
            let voices_pricing_json = model_pricing.voices_pricing.as_ref().and_then(|vp| {
                vp.get("USD")
                    .and_then(|config| serde_json::to_string(&config.voices).ok())
            });

            let video_pricing_json = model_pricing.video_pricing.as_ref().and_then(|vp| {
                vp.get("USD")
                    .and_then(|config| serde_json::to_string(&config.resolutions).ok())
            });

            let asr_pricing_json = model_pricing.asr_pricing.as_ref().and_then(|ap| {
                ap.get("USD").and_then(|config| {
                    serde_json::to_string(&serde_json::json!({"per_minute": config.per_minute}))
                        .ok()
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
                let currency_voices = model_pricing
                    .voices_pricing
                    .as_ref()
                    .and_then(|vp| vp.get(currency))
                    .and_then(|config| serde_json::to_string(&config.voices).ok())
                    .or_else(|| voices_pricing_json.clone());

                let currency_video = model_pricing
                    .video_pricing
                    .as_ref()
                    .and_then(|vp| vp.get(currency))
                    .and_then(|config| serde_json::to_string(&config.resolutions).ok())
                    .or_else(|| video_pricing_json.clone());

                // Derive video_price from video_pricing["720p"] so the billing formula
                // cost = video_tokens × video_price / 1_000_000 gives the correct per-second cost.
                // video_tokens = duration × resolution_weight (720p=2, 480p=1), so:
                //   video_price (nanodollars/MTok) = price_720p_per_sec (nanodollars) × 500_000
                // This means 480p requests naturally cost half of 720p via resolution_weight.
                let video_price_derived: Option<i64> = model_pricing
                    .video_pricing
                    .as_ref()
                    .and_then(|vp| vp.get(currency))
                    .and_then(|config| config.resolutions.get("720p").copied())
                    .map(|price_per_sec_nanos: i64| {
                        (price_per_sec_nanos as i128 * 1_000_000 / 2) as i64
                    });

                let currency_asr = model_pricing
                    .asr_pricing
                    .as_ref()
                    .and_then(|ap| ap.get(currency))
                    .and_then(|config| {
                        serde_json::to_string(&serde_json::json!({"per_minute": config.per_minute}))
                            .ok()
                    })
                    .or_else(|| asr_pricing_json.clone());

                let currency_realtime = model_pricing
                    .realtime_pricing
                    .as_ref()
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
                        if map.is_empty() {
                            None
                        } else {
                            serde_json::to_string(&map).ok()
                        }
                    })
                    .or_else(|| realtime_pricing_json.clone());

                let price_input = PriceInput {
                    model: model_name.clone(),
                    currency: currency.clone(),
                    input_price: currency_pricing.input_price,
                    output_price: currency_pricing.output_price,
                    image_price: currency_pricing.image_output_price,
                    audio_output_price: currency_pricing.audio_output_price,
                    music_price: currency_pricing.music_price,
                    video_price: video_price_derived,
                    source: currency_pricing.source.clone().or(Some(source.to_string())),
                    voices_pricing: currency_voices,
                    video_pricing: currency_video,
                    asr_pricing: currency_asr,
                    realtime_pricing: currency_realtime,
                    model_type: model_type.clone(),
                    ..Default::default()
                };

                // Audit: read existing price before upsert so we can log changes
                let old_price = if source == "remote" {
                    BillingPriceModel::get(&self.db, model_name, currency, None)
                        .await
                        .ok()
                        .flatten()
                } else {
                    None
                };

                match BillingPriceModel::upsert(&self.db, &price_input).await {
                    Ok(_) => {
                        result.models_synced += 1;
                        result.currencies_synced += 1;
                        // Emit structured audit log when price changes on remote sync
                        if let Some(old) = old_price {
                            let new_in = currency_pricing.input_price;
                            let new_out = currency_pricing.output_price;
                            if old.input_price != new_in || old.output_price != new_out {
                                tracing::info!(
                                    model = model_name,
                                    currency = currency,
                                    old_input_price = old.input_price,
                                    new_input_price = new_in,
                                    old_output_price = old.output_price,
                                    new_output_price = new_out,
                                    changed_at = %Utc::now(),
                                    "price_changed"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to upsert price for {} ({}): {}",
                            model_name,
                            currency,
                            e
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
                        BillingPriceModel::get(&self.db, model_name, currency, None).await
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
                            music_price: existing.music_price,
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

                        if let Err(e) = BillingPriceModel::upsert(&self.db, &update_input).await {
                            tracing::error!(
                                "Failed to update cache pricing for {}: {}",
                                model_name,
                                e
                            );
                        }
                    }
                }
            }

            // Apply batch pricing
            if let Some(ref batch_pricing) = model_pricing.batch_pricing {
                for (currency, batch_config) in batch_pricing {
                    if let Ok(Some(mut existing)) =
                        BillingPriceModel::get(&self.db, model_name, currency, None).await
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
                            music_price: existing.music_price,
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

                        if let Err(e) = BillingPriceModel::upsert(&self.db, &update_input).await {
                            tracing::error!(
                                "Failed to update batch pricing for {}: {}",
                                model_name,
                                e
                            );
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

                        match BillingTieredPriceModel::upsert_tier(&self.db, &tier_input).await {
                            Ok(_) => {
                                result.tiered_pricing_synced += 1;
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to upsert tiered pricing for {} ({}): {}",
                                    model_name,
                                    currency,
                                    e
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
            result.models_synced,
            result.currencies_synced,
            result.tiered_pricing_synced,
            source
        );

        Ok(result)
    }

    /// Sync prices from remote repository (with Gitee fallback).
    /// Includes model count drop protection and price change audit logging.
    async fn sync_remote_prices(&self) -> anyhow::Result<SyncResult> {
        let response = self.fetch_remote_config().await?;
        let config = PricingConfig::from_json(&response)?;

        // Model count drop protection: warn if new data has >50% fewer models
        let prev_count = self.count_db_models().await.unwrap_or(0);
        let new_count = config.models.len();
        if prev_count > 0 && new_count * 2 < prev_count {
            tracing::warn!(
                prev_models = prev_count,
                new_models = new_count,
                "Remote pricing data has >50% fewer models than current DB — possible data issue"
            );
        }

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

            match BillingTieredPriceModel::upsert_tier(&self.db, tier).await {
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
        let config = PricingConfig::from_json(&text)?;
        let mut updated_count = 0;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let conn = self.db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = self.db.kind() == "postgres";

        for (model_name, model_pricing) in &config.models {
            // Get pricing info for capabilities table
            let (input_price, output_price) = model_pricing
                .pricing
                .get("USD")
                .map(|p| {
                    (
                        Some(p.input_price as f64 / 1_000_000_000.0),
                        Some(p.output_price as f64 / 1_000_000_000.0),
                    )
                })
                .unwrap_or((None, None));

            // Get metadata
            let (context_window, max_output_tokens, supports_vision, supports_function_calling) =
                model_pricing
                    .metadata
                    .as_ref()
                    .map(|m| {
                        (
                            m.context_window,
                            m.max_output_tokens,
                            m.supports_vision,
                            m.supports_function_calling,
                        )
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

/// Start a background price sync task.
///
/// `force_sync_rx`: receives one-shot reply channels from the HTTP force-sync endpoint.
/// Each message triggers an immediate forced sync; the result is sent back via the oneshot.
pub fn start_price_sync_task(
    db: Arc<Database>,
    interval_secs: u64,
    config: Option<PriceSyncConfig>,
    price_cache: PriceCache,
    mut force_sync_rx: tokio::sync::mpsc::Receiver<tokio::sync::oneshot::Sender<SyncResult>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut service = match config {
            Some(cfg) => PriceSyncService::with_config(db.clone(), cfg),
            None => PriceSyncService::new(db.clone()),
        };
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        // Don't fire a tick immediately on first poll (we do the initial sync manually below)
        interval.reset();

        // Initial sync (not forced — use DB fast path if available)
        // Allow skipping for test environments
        if std::env::var("SKIP_INITIAL_PRICE_SYNC").is_ok() {
            tracing::info!("Skipping initial price sync (SKIP_INITIAL_PRICE_SYNC is set)");
        } else {
            match service.sync_all(false).await {
                Ok(result) => {
                    tracing::info!(
                        models = result.models_synced,
                        source = result.source,
                        "Initial price sync complete"
                    );
                    if let Err(e) = price_cache.refresh(&db).await {
                        tracing::error!("Failed to refresh price cache after initial sync: {e}");
                    }
                }
                Err(e) => {
                    tracing::warn!("Initial price sync failed: {e}");
                    // Non-fatal: server can start without prices (e.g. test environments)
                }
            }
        } // end SKIP_INITIAL_PRICE_SYNC else

        // Event loop: respond to periodic ticks and force-sync requests
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    tracing::info!("Starting periodic price sync...");
                    match service.sync_all(true).await {
                        Ok(result) => {
                            tracing::info!(
                                models = result.models_synced,
                                source = result.source,
                                "Periodic price sync complete"
                            );
                            if let Err(e) = price_cache.refresh(&db).await {
                                tracing::error!("Failed to refresh price cache after periodic sync: {e}");
                            }
                        }
                        Err(e) => tracing::error!("Periodic price sync failed: {e}"),
                    }
                }
                Some(reply_tx) = force_sync_rx.recv() => {
                    tracing::info!("Force price sync requested via HTTP endpoint...");
                    match service.sync_all(true).await {
                        Ok(result) => {
                            tracing::info!(
                                models = result.models_synced,
                                source = result.source,
                                "Force price sync complete"
                            );
                            if let Err(e) = price_cache.refresh(&db).await {
                                tracing::error!("Failed to refresh price cache after force sync: {e}");
                            }
                            let _ = reply_tx.send(result);
                        }
                        Err(e) => {
                            tracing::error!("Force price sync failed: {e}");
                            let _ = reply_tx.send(SyncResult {
                                source: format!("error: {e}"),
                                ..Default::default()
                            });
                        }
                    }
                }
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
        assert_eq!(
            config.remote_url_fallback,
            Some(BURNSCLOUD_PRICES_URL_GITEE.to_string())
        );
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
