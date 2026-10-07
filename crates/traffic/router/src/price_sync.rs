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

use burncloud_commerce_contracts::pricing::{CurrencyPricing, ModelPricing, PricingConfig};
use burncloud_database::{sqlx, Database};
use burncloud_database_billing::{
    BillingPriceModel, BillingTieredPriceModel, DatabaseError, Price, PriceInput, TieredPriceInput,
};
use burncloud_database_model::{ModelCapabilityInput, ModelCapabilityModel};
use burncloud_service_billing::PriceCache;

/// HTTP client timeout for price sync API calls (seconds).
const HTTP_CLIENT_TIMEOUT_SECS: u64 = 30;
/// Default remote price sync interval (seconds).
const DEFAULT_REMOTE_SYNC_INTERVAL_SECS: u64 = 86400;

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
            remote_sync_interval_secs: DEFAULT_REMOTE_SYNC_INTERVAL_SECS,
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

/// Extended (non-token) pricing serialized for the USD entry of a model.
///
/// Used as the fallback when a currency has no extended pricing of its own.
struct UsdExtendedPricing {
    /// TTS voices pricing JSON
    voices: Option<String>,
    /// Video resolutions pricing JSON
    video: Option<String>,
    /// ASR per-minute pricing JSON
    asr: Option<String>,
    /// Realtime audio/image pricing JSON
    realtime: Option<String>,
}

/// Extended pricing fields of one currency, falling back to the USD entry.
struct CurrencyExtendedPricing {
    /// TTS voices pricing JSON
    voices: Option<String>,
    /// Video resolutions pricing JSON
    video: Option<String>,
    /// ASR per-minute pricing JSON
    asr: Option<String>,
    /// Realtime audio/image pricing JSON
    realtime: Option<String>,
}

/// Serialize one currency's extended pricing sections, falling back to the USD
/// entries when the currency has none.
fn currency_extended_pricing(
    model_pricing: &ModelPricing,
    currency: &str,
    usd_pricing: &UsdExtendedPricing,
) -> CurrencyExtendedPricing {
    let voices = model_pricing
        .voices_pricing
        .as_ref()
        .and_then(|vp| vp.get(currency))
        .and_then(|config| serde_json::to_string(&config.voices).ok())
        .or_else(|| usd_pricing.voices.clone());

    let video = model_pricing
        .video_pricing
        .as_ref()
        .and_then(|vp| vp.get(currency))
        .and_then(|config| serde_json::to_string(&config.resolutions).ok())
        .or_else(|| usd_pricing.video.clone());

    let asr = model_pricing
        .asr_pricing
        .as_ref()
        .and_then(|ap| ap.get(currency))
        .and_then(|config| {
            serde_json::to_string(&serde_json::json!({"per_minute": config.per_minute})).ok()
        })
        .or_else(|| usd_pricing.asr.clone());

    let realtime = model_pricing
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
        .or_else(|| usd_pricing.realtime.clone());

    CurrencyExtendedPricing {
        voices,
        video,
        asr,
        realtime,
    }
}

/// Derive `video_price` from `video_pricing["720p"]` so the billing formula
/// cost = video_tokens × video_price / 1_000_000 gives the correct per-second cost.
/// video_tokens = duration × resolution_weight (720p=2, 480p=1), so:
///   video_price (nanodollars/MTok) = price_720p_per_sec (nanodollars) × 500_000
/// This means 480p requests naturally cost half of 720p via resolution_weight.
fn video_price_720p(model_pricing: &ModelPricing, currency: &str) -> Option<i64> {
    model_pricing
        .video_pricing
        .as_ref()
        .and_then(|vp| vp.get(currency))
        .and_then(|config| config.resolutions.get("720p").copied())
        .map(|price_per_sec_nanos: i64| (price_per_sec_nanos as i128 * 1_000_000 / 2) as i64)
}

/// Serialize a model's USD-keyed extended pricing sections.
fn usd_extended_pricing(model_pricing: &ModelPricing) -> UsdExtendedPricing {
    let voices = model_pricing.voices_pricing.as_ref().and_then(|vp| {
        vp.get("USD")
            .and_then(|config| serde_json::to_string(&config.voices).ok())
    });

    let video = model_pricing.video_pricing.as_ref().and_then(|vp| {
        vp.get("USD")
            .and_then(|config| serde_json::to_string(&config.resolutions).ok())
    });

    let asr = model_pricing.asr_pricing.as_ref().and_then(|ap| {
        ap.get("USD").and_then(|config| {
            serde_json::to_string(&serde_json::json!({"per_minute": config.per_minute})).ok()
        })
    });

    let realtime = model_pricing.realtime_pricing.as_ref().and_then(|rp| {
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

    UsdExtendedPricing {
        voices,
        video,
        asr,
        realtime,
    }
}

impl PriceSyncService {
    /// Create a new PriceSyncService with default configuration
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            http_client: Client::builder()
                .timeout(std::time::Duration::from_secs(HTTP_CLIENT_TIMEOUT_SECS))
                .build()
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to build HTTP client with timeout, using default: {e}");
                    Client::new()
                }),
            config: PriceSyncConfig::default(),
            last_remote_sync: None,
        }
    }

    /// Create a new PriceSyncService with custom configuration
    pub fn with_config(db: Arc<Database>, config: PriceSyncConfig) -> Self {
        Self {
            db,
            http_client: Client::builder()
                .timeout(std::time::Duration::from_secs(HTTP_CLIENT_TIMEOUT_SECS))
                .build()
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to build HTTP client with timeout, using default: {e}");
                    Client::new()
                }),
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
            if let Some(cached) = self.db_cache_result().await {
                return Ok(cached);
            }
        }

        // 3. Pull from remote with retry on cold start
        let (remote, last_err) = self.sync_remote_with_retries().await;
        let sync_err = match remote {
            Some(result) => return Ok(result),
            // At least one attempt always runs, so a failure leaves an error behind.
            None => match last_err {
                Some(err) => err,
                None => unreachable!("sync_remote_with_retries ran no attempts"),
            },
        };

        self.remote_sync_failure(sync_err).await
    }

    /// Handle a fully-failed remote sync: fall back to cached DB prices, or fail
    /// fatally when the DB has none.
    async fn remote_sync_failure(&self, sync_err: anyhow::Error) -> anyhow::Result<SyncResult> {
        let db_count = self.count_db_models().await.unwrap_or(0);
        if db_count == 0 {
            tracing::error!(
                error = %sync_err,
                "FATAL: pricing_data unreachable and DB has no prices. \
                 Check network connectivity or pre-seed DB."
            );
            return Err(sync_err);
        }

        tracing::warn!(
            error = %sync_err,
            models = db_count,
            "Remote sync failed after all retries, using existing DB prices"
        );
        Ok(SyncResult {
            source: "db_fallback".to_string(),
            ..Default::default()
        })
    }

    /// Startup fast path: when the DB already holds prices, skip the remote fetch.
    ///
    /// Returns `None` when the DB has no prices and the remote sync should run.
    async fn db_cache_result(&self) -> Option<SyncResult> {
        let db_count = self.count_db_models().await.unwrap_or(0);
        if db_count == 0 {
            return None;
        }

        tracing::info!(
            models = db_count,
            "DB already has prices, skipping remote sync (startup fast path)"
        );
        Some(SyncResult {
            source: "db_cache".to_string(),
            ..Default::default()
        })
    }

    /// Pull remote prices, retrying on the cold-start delays (5s, 15s, 30s).
    ///
    /// Returns the successful result, or the last error if every attempt failed.
    async fn sync_remote_with_retries(&mut self) -> (Option<SyncResult>, Option<anyhow::Error>) {
        const RETRY_DELAYS_SECS: &[u64] = &[5, 15, 30];
        let mut last_err: Option<anyhow::Error> = None;
        for (attempt, &delay) in RETRY_DELAYS_SECS.iter().enumerate() {
            match self.sync_remote_prices().await {
                Ok(result) => {
                    self.last_remote_sync = Some(Utc::now());
                    return (Some(result), None);
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

        (None, last_err)
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
            let model_type: Option<String> = model_pricing
                .metadata
                .as_ref()
                .and_then(|m| m.provider.clone());

            self.apply_model_pricing(model_name, model_pricing, model_type, source, &mut result)
                .await;
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

    /// Apply one model's standard per-currency prices, then its cache, batch and
    /// tiered pricing rows.
    ///
    /// `result` accumulates per-currency/tier counters and error counts.
    async fn apply_model_pricing(
        &self,
        model_name: &str,
        model_pricing: &ModelPricing,
        model_type: Option<String>,
        source: &str,
        result: &mut SyncResult,
    ) {
        let usd_pricing = usd_extended_pricing(model_pricing);

        for (currency, currency_pricing) in &model_pricing.pricing {
            self.apply_currency_pricing(
                model_name,
                model_pricing,
                currency,
                currency_pricing,
                &usd_pricing,
                model_type.clone(),
                source,
                result,
            )
            .await;
        }

        self.apply_cache_and_batch_pricing(model_name, model_pricing, result)
            .await;
        self.apply_tiered_pricing(model_name, model_pricing, result)
            .await;
    }

    /// Upsert one model/currency price row, logging price changes on remote sync.
    ///
    /// # Arguments
    /// * `model_name` - Model whose price is being written
    /// * `model_pricing` - Full per-model pricing config (for extended fields)
    /// * `currency` - Currency code of `currency_pricing`
    /// * `currency_pricing` - Standard prices for this currency
    /// * `usd_pricing` - USD extended pricing, used as the fallback
    /// * `model_type` - Provider label stored on the row
    /// * `source` - Sync source ("remote", "local_override", ...)
    /// * `result` - Accumulator for sync counters and error counts
    #[allow(
        clippy::too_many_arguments,
        reason = "private helper carrying one price row's context; grouping it into a struct would add a type with a single caller"
    )]
    async fn apply_currency_pricing(
        &self,
        model_name: &str,
        model_pricing: &ModelPricing,
        currency: &str,
        currency_pricing: &CurrencyPricing,
        usd_pricing: &UsdExtendedPricing,
        model_type: Option<String>,
        source: &str,
        result: &mut SyncResult,
    ) {
        let extended = currency_extended_pricing(model_pricing, currency, usd_pricing);

        let price_input = PriceInput {
            model: model_name.to_string(),
            currency: currency.to_string(),
            input_price: currency_pricing.input_price,
            output_price: currency_pricing.output_price,
            image_price: currency_pricing.image_output_price,
            audio_output_price: currency_pricing.audio_output_price,
            music_price: currency_pricing.music_price,
            video_price: video_price_720p(model_pricing, currency),
            source: currency_pricing.source.clone().or(Some(source.to_string())),
            voices_pricing: extended.voices,
            video_pricing: extended.video,
            asr_pricing: extended.asr,
            realtime_pricing: extended.realtime,
            model_type,
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

        let outcome = BillingPriceModel::upsert(&self.db, &price_input).await;
        self.record_price_upsert(
            outcome,
            old_price,
            model_name,
            currency,
            currency_pricing,
            result,
        );
    }

    /// Update the sync counters and log a price change for one upsert outcome.
    ///
    /// # Arguments
    /// * `outcome` - Result of the price upsert
    /// * `old_price` - Price row read before the upsert, used for change auditing
    /// * `model_name` - Model whose price was written
    /// * `currency` - Currency of the written row
    /// * `currency_pricing` - Prices that were written for this currency
    /// * `result` - Accumulator for sync counters and error counts
    #[allow(
        clippy::too_many_arguments,
        reason = "private helper carrying one upsert's audit context; grouping it into a struct would add a type with a single caller"
    )]
    fn record_price_upsert(
        &self,
        outcome: Result<(), DatabaseError>,
        old_price: Option<Price>,
        model_name: &str,
        currency: &str,
        currency_pricing: &CurrencyPricing,
        result: &mut SyncResult,
    ) {
        match outcome {
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

    /// Apply one model's cache and batch pricing column updates.
    async fn apply_cache_and_batch_pricing(
        &self,
        model_name: &str,
        model_pricing: &ModelPricing,
        _result: &mut SyncResult,
    ) {
        self.apply_cache_pricing(model_name, model_pricing).await;
        self.apply_batch_pricing(model_name, model_pricing).await;
    }

    /// Apply one model's cache pricing column updates.
    async fn apply_cache_pricing(&self, model_name: &str, model_pricing: &ModelPricing) {
        // Apply cache pricing — atomic column update, no read-then-write race
        if let Some(ref cache_pricing) = model_pricing.cache_pricing {
            for (currency, cache_config) in cache_pricing {
                match BillingPriceModel::update_cache_pricing(
                    &self.db,
                    model_name,
                    None,
                    Some(cache_config.cache_read_input_price),
                    cache_config.cache_creation_input_price,
                )
                .await
                {
                    Ok(true) => {}
                    Ok(false) => {
                        tracing::warn!(
                            model = model_name,
                            currency = currency,
                            "No existing price row for cache pricing update — base price must be synced first"
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to update cache pricing for {}: {}", model_name, e);
                    }
                }
            }
        }
    }

    /// Apply one model's batch pricing column updates.
    async fn apply_batch_pricing(&self, model_name: &str, model_pricing: &ModelPricing) {
        // Apply batch pricing — atomic column update, no read-then-write race
        if let Some(ref batch_pricing) = model_pricing.batch_pricing {
            for (currency, batch_config) in batch_pricing {
                match BillingPriceModel::update_batch_pricing(
                    &self.db,
                    model_name,
                    None,
                    Some(batch_config.batch_input_price),
                    Some(batch_config.batch_output_price),
                )
                .await
                {
                    Ok(true) => {}
                    Ok(false) => {
                        tracing::warn!(
                            model = model_name,
                            currency = currency,
                            "No existing price row for batch pricing update — base price must be synced first"
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to update batch pricing for {}: {}", model_name, e);
                    }
                }
            }
        }
    }

    /// Apply one model's tiered pricing rows.
    async fn apply_tiered_pricing(
        &self,
        model_name: &str,
        model_pricing: &ModelPricing,
        result: &mut SyncResult,
    ) {
        if let Some(ref tiered_pricing) = model_pricing.tiered_pricing {
            for (currency, tiers) in tiered_pricing {
                for tier in tiers {
                    let tier_input = TieredPriceInput {
                        model: model_name.to_string(),
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
            if !valid_tier_or_logged(tier) {
                continue;
            }

            self.import_one_tier(tier, &mut imported_count).await;
        }

        tracing::info!(
            "Tiered pricing import complete: {} tiers imported",
            imported_count
        );
        Ok(imported_count)
    }

    /// Upsert a single validated tier and report the outcome.
    async fn import_one_tier(&self, tier: &TieredPriceInput, imported_count: &mut usize) {
        match BillingTieredPriceModel::upsert_tier(&self.db, tier).await {
            Ok(_) => {
                *imported_count += 1;
                tracing::info!(
                    "Imported tier for model {} ({}-{} tokens): ${:.4}/${:.4} per 1M",
                    tier.model,
                    tier.tier_start,
                    tier.tier_end.map_or("∞".to_string(), |e| format!("{e}")),
                    tier.input_price,
                    tier.output_price
                );
            }
            Err(e) => {
                tracing::error!("Failed to import tier for {}: {}", tier.model, e);
            }
        }
    }

    /// Sync model capabilities to the local database
    ///
    /// Returns the number of capabilities updated/inserted.
    ///
    /// Traffic owns the projection from the pricing document; Supply owns persistence and SQL.
    pub async fn sync_capabilities(&self) -> anyhow::Result<usize> {
        let text = self.fetch_remote_config().await?;
        let config = PricingConfig::from_json(&text)?;
        let mut updated_count = 0;

        for (model_name, model_pricing) in &config.models {
            // The historical mixed table keeps a USD price snapshot for compatibility. Commerce's
            // billing tables remain the canonical pricing truth.
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

            let input = ModelCapabilityInput {
                model: model_name.clone(),
                context_window,
                max_output_tokens,
                supports_vision,
                supports_function_calling,
                input_price,
                output_price,
            };

            match ModelCapabilityModel::upsert(self.db.as_ref(), &input).await {
                Ok(()) => updated_count += 1,
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
                            if reply_tx.send(result).is_err() {
                                tracing::debug!("force-sync reply dropped (receiver gone)");
                            }
                        }
                        Err(e) => {
                            tracing::error!("Force price sync failed: {e}");
                            if reply_tx
                                .send(SyncResult {
                                    source: format!("error: {e}"),
                                    ..Default::default()
                                })
                                .is_err()
                            {
                                tracing::debug!("force-sync reply dropped (receiver gone)");
                            }
                        }
                    }
                }
            }
        }
    })
}

/// Validate a tier before import: prices must be non-negative and the range
/// must be non-empty. Invalid tiers are logged and skipped.
fn valid_tier_or_logged(tier: &TieredPriceInput) -> bool {
    // Prices are i64 nanodollars, so compare with 0
    if tier.input_price < 0 || tier.output_price < 0 {
        tracing::error!(
            "Skipping tier with invalid price for model {}: prices must be >= 0",
            tier.model
        );
        return false;
    }

    if let Some(tier_end) = tier.tier_end {
        if tier.tier_start >= tier_end {
            tracing::error!(
                "Skipping tier with invalid range for model {}: tier_start ({}) must be < tier_end ({})",
                tier.model,
                tier.tier_start,
                tier_end
            );
            return false;
        }
    }

    true
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
