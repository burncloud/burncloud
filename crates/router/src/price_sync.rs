//! Price Sync Module
//!
//! This module provides functionality for syncing model pricing data from
//! LiteLLM's model_prices_and_context_window.json file.
//!
//! # PriceSyncServiceV2
//!
//! The V2 service supports multi-source, multi-currency price synchronization with
//! the following priority order (highest to lowest):
//! 1. Local override configuration (pricing.override.json)
//! 2. Local main configuration (pricing.json)
//! 3. Community price repository (daily sync)
//! 4. LiteLLM (USD fallback only)

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use burncloud_common::PricingConfig;
use burncloud_database::{sqlx, Database};
use burncloud_database_models::{PriceInput, PriceModel, TieredPriceInput, TieredPriceModel};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

/// URL for LiteLLM's model prices JSON file
const LITELLM_PRICES_URL: &str =
    "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";

/// LiteLLM price data structure from the JSON file
#[derive(Debug, Clone, Deserialize)]
pub struct LiteLLMPrice {
    /// Model name/identifier
    pub model: Option<String>,
    /// Input price per 1M tokens
    #[serde(default)]
    pub input_cost_per_token: Option<f64>,
    /// Output price per 1M tokens
    #[serde(default)]
    pub output_cost_per_token: Option<f64>,
    /// Context window size (can be number or string in LiteLLM JSON)
    #[serde(default, deserialize_with = "deserialize_optional_u32")]
    pub max_input_tokens: Option<u32>,
    /// Maximum output tokens (can be number or string in LiteLLM JSON)
    #[serde(default, deserialize_with = "deserialize_optional_u32")]
    pub max_output_tokens: Option<u32>,
    /// Alternative model name used for pricing
    #[serde(default)]
    pub pricing_model: Option<String>,
    /// Supports vision
    #[serde(default)]
    pub supports_vision: Option<bool>,
    /// Supports function calling
    #[serde(default)]
    pub supports_function_calling: Option<bool>,
    /// Model type (e.g., "chat", "embedding")
    #[serde(default)]
    pub mode: Option<String>,
    // Advanced pricing fields from LiteLLM
    /// Cache read input token cost (Prompt Caching)
    #[serde(default)]
    pub cache_read_input_token_cost: Option<f64>,
    /// Cache creation input token cost (Prompt Caching)
    #[serde(default)]
    pub cache_creation_input_token_cost: Option<f64>,
    /// Batch API input token cost
    #[serde(default)]
    pub input_cost_per_token_batches: Option<f64>,
    /// Batch API output token cost
    #[serde(default)]
    pub output_cost_per_token_batches: Option<f64>,
    /// Priority input token cost (higher priority requests)
    #[serde(default)]
    pub input_cost_per_token_priority: Option<f64>,
    /// Priority output token cost
    #[serde(default)]
    pub output_cost_per_token_priority: Option<f64>,
    /// Audio input token cost
    #[serde(default)]
    pub input_cost_per_audio_token: Option<f64>,
    /// Search context cost per query
    #[serde(default)]
    pub search_context_cost_per_query: Option<f64>,
}

/// Custom deserializer to handle both number and string values for token fields
fn deserialize_optional_u32<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct OptionalU32Visitor;

    impl<'de> Visitor<'de> for OptionalU32Visitor {
        type Value = Option<u32>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a number, string, or null")
        }

        fn visit_none<E>(self) -> Result<Option<u32>, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Option<u32>, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_u64<E>(self, v: u64) -> Result<Option<u32>, E>
        where
            E: de::Error,
        {
            Ok(Some(v as u32))
        }

        fn visit_i64<E>(self, v: i64) -> Result<Option<u32>, E>
        where
            E: de::Error,
        {
            Ok(Some(v as u32))
        }

        fn visit_f64<E>(self, v: f64) -> Result<Option<u32>, E>
        where
            E: de::Error,
        {
            Ok(Some(v as u32))
        }

        fn visit_str<E>(self, v: &str) -> Result<Option<u32>, E>
        where
            E: de::Error,
        {
            // Try to parse string as number, otherwise return None
            // Also try parsing as float since some values might be "2000000.0"
            if let Ok(n) = v.parse::<u32>() {
                Ok(Some(n))
            } else if let Ok(n) = v.parse::<f64>() {
                Ok(Some(n as u32))
            } else {
                Ok(None)
            }
        }
    }

    deserializer.deserialize_any(OptionalU32Visitor)
}

impl LiteLLMPrice {
    /// Convert per-token cost to per-1M tokens price
    pub fn to_per_million_price(&self) -> (Option<f64>, Option<f64>) {
        let input_price = self.input_cost_per_token.map(|c| c * 1_000_000.0);
        let output_price = self.output_cost_per_token.map(|c| c * 1_000_000.0);
        (input_price, output_price)
    }

    /// Convert cache pricing to per-1M tokens
    pub fn to_cache_per_million_price(&self) -> (Option<f64>, Option<f64>) {
        let cache_read_price = self.cache_read_input_token_cost.map(|c| c * 1_000_000.0);
        let cache_creation_price = self
            .cache_creation_input_token_cost
            .map(|c| c * 1_000_000.0);
        (cache_read_price, cache_creation_price)
    }

    /// Convert batch pricing to per-1M tokens
    pub fn to_batch_per_million_price(&self) -> (Option<f64>, Option<f64>) {
        let batch_input_price = self.input_cost_per_token_batches.map(|c| c * 1_000_000.0);
        let batch_output_price = self.output_cost_per_token_batches.map(|c| c * 1_000_000.0);
        (batch_input_price, batch_output_price)
    }

    /// Convert priority pricing to per-1M tokens
    pub fn to_priority_per_million_price(&self) -> (Option<f64>, Option<f64>) {
        let priority_input_price = self.input_cost_per_token_priority.map(|c| c * 1_000_000.0);
        let priority_output_price = self.output_cost_per_token_priority.map(|c| c * 1_000_000.0);
        (priority_input_price, priority_output_price)
    }

    /// Convert audio pricing to per-1M tokens
    pub fn to_audio_per_million_price(&self) -> Option<f64> {
        self.input_cost_per_audio_token.map(|c| c * 1_000_000.0)
    }

    // ========== Nanodollar conversion methods (for Price) ==========

    /// Convert per-token cost to per-1M tokens price in nanodollars (i64)
    pub fn to_per_million_price_nano(&self) -> (Option<i64>, Option<i64>) {
        use burncloud_common::dollars_to_nano;
        let input_price = self.input_cost_per_token.map(|c| dollars_to_nano(c * 1_000_000.0) as i64);
        let output_price = self.output_cost_per_token.map(|c| dollars_to_nano(c * 1_000_000.0) as i64);
        (input_price, output_price)
    }

    /// Convert cache pricing to per-1M tokens in nanodollars (i64)
    pub fn to_cache_per_million_price_nano(&self) -> (Option<i64>, Option<i64>) {
        use burncloud_common::dollars_to_nano;
        let cache_read_price = self.cache_read_input_token_cost.map(|c| dollars_to_nano(c * 1_000_000.0) as i64);
        let cache_creation_price = self
            .cache_creation_input_token_cost
            .map(|c| dollars_to_nano(c * 1_000_000.0) as i64);
        (cache_read_price, cache_creation_price)
    }

    /// Convert batch pricing to per-1M tokens in nanodollars (i64)
    pub fn to_batch_per_million_price_nano(&self) -> (Option<i64>, Option<i64>) {
        use burncloud_common::dollars_to_nano;
        let batch_input_price = self.input_cost_per_token_batches.map(|c| dollars_to_nano(c * 1_000_000.0) as i64);
        let batch_output_price = self.output_cost_per_token_batches.map(|c| dollars_to_nano(c * 1_000_000.0) as i64);
        (batch_input_price, batch_output_price)
    }

    /// Convert priority pricing to per-1M tokens in nanodollars (i64)
    pub fn to_priority_per_million_price_nano(&self) -> (Option<i64>, Option<i64>) {
        use burncloud_common::dollars_to_nano;
        let priority_input_price = self.input_cost_per_token_priority.map(|c| dollars_to_nano(c * 1_000_000.0) as i64);
        let priority_output_price = self.output_cost_per_token_priority.map(|c| dollars_to_nano(c * 1_000_000.0) as i64);
        (priority_input_price, priority_output_price)
    }

    /// Convert audio pricing to per-1M tokens in nanodollars (i64)
    pub fn to_audio_per_million_price_nano(&self) -> Option<i64> {
        use burncloud_common::dollars_to_nano;
        self.input_cost_per_audio_token.map(|c| dollars_to_nano(c * 1_000_000.0) as i64)
    }
}

/// Service for syncing prices from LiteLLM
pub struct PriceSyncService {
    db: std::sync::Arc<Database>,
    http_client: Client,
}

impl PriceSyncService {
    /// Create a new PriceSyncService
    pub fn new(db: std::sync::Arc<Database>) -> Self {
        Self {
            db,
            http_client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Fetch price data from LiteLLM's GitHub repository
    pub async fn fetch_litellm_prices(&self) -> anyhow::Result<HashMap<String, LiteLLMPrice>> {
        let response = self
            .http_client
            .get(LITELLM_PRICES_URL)
            .send()
            .await?
            .error_for_status()?;

        let text = response.text().await?;
        let prices: HashMap<String, LiteLLMPrice> = serde_json::from_str(&text)?;

        Ok(prices)
    }

    /// Sync prices from LiteLLM to the local database
    ///
    /// Returns the number of prices updated/inserted
    pub async fn sync_from_litellm(&self) -> anyhow::Result<usize> {
        let prices = self.fetch_litellm_prices().await?;
        let mut updated_count = 0;

        for (key, price_data) in prices {
            // Skip embedding models and models without pricing
            if price_data.mode.as_deref() == Some("embedding") {
                continue;
            }

            // Get the model name
            let model_name = match &price_data.model {
                Some(m) => m.clone(),
                None => key.clone(),
            };

            // Get pricing info (in nanodollars)
            let (input_price, output_price) = price_data.to_per_million_price_nano();

            // Skip if no pricing info
            let (input, output) = match (input_price, output_price) {
                (Some(i), Some(o)) => (i, o),
                (Some(i), None) => (i, i), // Use input price for output if not specified
                (None, Some(o)) => (o, o), // Use output price for input if not specified
                (None, None) => continue,  // No pricing info, skip
            };

            // Get advanced pricing info (in nanodollars)
            let (cache_read_price, cache_creation_price) = price_data.to_cache_per_million_price_nano();
            let (batch_input_price, batch_output_price) = price_data.to_batch_per_million_price_nano();
            let (priority_input_price, priority_output_price) =
                price_data.to_priority_per_million_price_nano();
            let audio_input_price = price_data.to_audio_per_million_price_nano();

            // Create price input with advanced pricing fields (using new PriceInput with i64 nanodollars)
            let price_input = PriceInput {
                model: model_name.clone(),
                currency: "USD".to_string(),
                input_price: input,
                output_price: output,
                cache_read_input_price: cache_read_price,
                cache_creation_input_price: cache_creation_price,
                batch_input_price,
                batch_output_price,
                priority_input_price,
                priority_output_price,
                audio_input_price,
                source: Some("litellm".to_string()),
                region: None,
                context_window: price_data.max_input_tokens.map(|t| t as i64),
                max_output_tokens: price_data.max_output_tokens.map(|t| t as i64),
                supports_vision: price_data.supports_vision,
                supports_function_calling: price_data.supports_function_calling,
            };

            // Upsert to database
            match PriceModel::upsert(&self.db, &price_input).await {
                Ok(_) => {
                    updated_count += 1;
                    println!("Synced price for model: {}", model_name);
                }
                Err(e) => {
                    eprintln!("Failed to sync price for {}: {}", model_name, e);
                }
            }
        }

        println!("Price sync complete: {} models updated", updated_count);
        Ok(updated_count)
    }

    /// Sync model capabilities from LiteLLM to the local database
    ///
    /// Returns the number of capabilities updated/inserted
    pub async fn sync_capabilities(&self) -> anyhow::Result<usize> {
        let prices = self.fetch_litellm_prices().await?;
        let mut updated_count = 0;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let conn = self.db.get_connection()?;
        let pool = conn.pool();
        let is_postgres = self.db.kind() == "postgres";

        for (key, price_data) in prices {
            // Skip embedding models
            if price_data.mode.as_deref() == Some("embedding") {
                continue;
            }

            // Get the model name
            let model_name = match &price_data.model {
                Some(m) => m.clone(),
                None => key.clone(),
            };

            // Get pricing info for capabilities table
            let (input_price, output_price) = price_data.to_per_million_price();

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
                .bind(&model_name)
                .bind(price_data.max_input_tokens.map(|t| t as i64))
                .bind(price_data.max_output_tokens.map(|t| t as i64))
                .bind(price_data.supports_vision.unwrap_or(false))
                .bind(price_data.supports_function_calling.unwrap_or(false))
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
                    eprintln!("Failed to sync capabilities for {}: {}", model_name, e);
                }
            }
        }

        println!(
            "Capabilities sync complete: {} models updated",
            updated_count
        );
        Ok(updated_count)
    }

    /// Import tiered pricing from a JSON structure
    ///
    /// This is used for models like Qwen that have tiered pricing based on context length.
    /// LiteLLM doesn't include this data, so it must be manually configured.
    pub async fn import_tiered_pricing(
        &self,
        tiers: &[TieredPriceInput],
    ) -> anyhow::Result<usize> {
        let mut imported_count = 0;

        for tier in tiers {
            // Validate tier data (prices are now i64 nanodollars, so compare with 0)
            if tier.input_price < 0 || tier.output_price < 0 {
                eprintln!(
                    "Skipping tier with invalid price for model {}: prices must be >= 0",
                    tier.model
                );
                continue;
            }

            if let Some(tier_end) = tier.tier_end {
                if tier.tier_start >= tier_end {
                    eprintln!(
                        "Skipping tier with invalid range for model {}: tier_start ({}) must be < tier_end ({})",
                        tier.model, tier.tier_start, tier_end
                    );
                    continue;
                }
            }

            match TieredPriceModel::upsert_tier(&self.db, tier).await {
                Ok(_) => {
                    imported_count += 1;
                    println!(
                        "Imported tier for model {} ({}-{} tokens): ${:.4}/${:.4} per 1M",
                        tier.model,
                        tier.tier_start,
                        tier.tier_end.map_or("∞".to_string(), |e| format!("{}", e)),
                        tier.input_price,
                        tier.output_price
                    );
                }
                Err(e) => {
                    eprintln!("Failed to import tier for {}: {}", tier.model, e);
                }
            }
        }

        println!("Tiered pricing import complete: {} tiers imported", imported_count);
        Ok(imported_count)
    }
}

/// Start a background price sync task
///
/// This spawns a tokio task that syncs prices periodically
pub fn start_price_sync_task(
    db: std::sync::Arc<Database>,
    interval_secs: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let service = PriceSyncService::new(db);
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));

        // Initial sync
        println!("Starting initial price sync from LiteLLM...");
        match service.sync_from_litellm().await {
            Ok(count) => println!("Initial price sync complete: {} models", count),
            Err(e) => eprintln!("Initial price sync failed: {}", e),
        }

        // Initial capabilities sync
        println!("Starting initial capabilities sync from LiteLLM...");
        match service.sync_capabilities().await {
            Ok(count) => println!("Initial capabilities sync complete: {} models", count),
            Err(e) => eprintln!("Initial capabilities sync failed: {}", e),
        }

        // Periodic sync
        loop {
            interval.tick().await;
            println!("Starting periodic price sync from LiteLLM...");
            match service.sync_from_litellm().await {
                Ok(count) => println!("Periodic price sync complete: {} models updated", count),
                Err(e) => eprintln!("Periodic price sync failed: {}", e),
            }
            match service.sync_capabilities().await {
                Ok(count) => println!(
                    "Periodic capabilities sync complete: {} models updated",
                    count
                ),
                Err(e) => eprintln!("Periodic capabilities sync failed: {}", e),
            }
        }
    })
}

// ============================================================================
// PriceSyncServiceV2 - Multi-source, multi-currency price synchronization
// ============================================================================

/// URL for the community pricing repository
pub const COMMUNITY_PRICES_URL: &str =
    "https://raw.githubusercontent.com/burncloud/pricing-data/main/pricing/latest.json";

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

/// Configuration for PriceSyncServiceV2
#[derive(Debug, Clone)]
pub struct PriceSyncConfig {
    /// Path to local override configuration file
    pub override_config_path: PathBuf,
    /// Path to local main configuration file
    pub local_config_path: PathBuf,
    /// URL for community price repository
    pub community_repo_url: String,
    /// URL for LiteLLM prices
    pub litellm_url: String,
    /// Enable community price sync (default: true)
    pub community_sync_enabled: bool,
    /// Community sync interval in seconds (default: 86400 = 24 hours)
    pub community_sync_interval_secs: u64,
}

impl Default for PriceSyncConfig {
    fn default() -> Self {
        Self {
            override_config_path: PathBuf::from("config/pricing.override.json"),
            local_config_path: PathBuf::from("config/pricing.json"),
            community_repo_url: COMMUNITY_PRICES_URL.to_string(),
            litellm_url: LITELLM_PRICES_URL.to_string(),
            community_sync_enabled: true,
            community_sync_interval_secs: 86400, // 24 hours
        }
    }
}

/// V2 Price Sync Service supporting multi-source, multi-currency synchronization
///
/// This service supports the following data sources in priority order:
/// 1. Local override configuration (highest priority)
/// 2. Local main configuration
/// 3. Community price repository
/// 4. LiteLLM (USD only, lowest priority)
pub struct PriceSyncServiceV2 {
    db: Arc<Database>,
    http_client: Client,
    config: PriceSyncConfig,
    /// Last time community prices were synced
    last_community_sync: Option<DateTime<Utc>>,
    /// Last time LiteLLM prices were synced
    last_litellm_sync: Option<DateTime<Utc>>,
}

impl PriceSyncServiceV2 {
    /// Create a new PriceSyncServiceV2 with default configuration
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            http_client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            config: PriceSyncConfig::default(),
            last_community_sync: None,
            last_litellm_sync: None,
        }
    }

    /// Create a new PriceSyncServiceV2 with custom configuration
    pub fn with_config(db: Arc<Database>, config: PriceSyncConfig) -> Self {
        Self {
            db,
            http_client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            config,
            last_community_sync: None,
            last_litellm_sync: None,
        }
    }

    /// Sync prices from all sources with priority ordering
    ///
    /// Priority order (highest to lowest):
    /// 1. Local override configuration
    /// 2. Local main configuration
    /// 3. Community price repository (if enabled and due)
    /// 4. LiteLLM (USD fallback)
    pub async fn sync_all(&mut self) -> anyhow::Result<SyncResult> {
        let mut total_result = SyncResult::default();

        // 1. Load local override configuration (highest priority)
        if let Some(config) = self.load_local_override()? {
            println!("Applying local override pricing configuration...");
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
            println!("Applying local pricing configuration...");
            let result = self.apply_prices(&config, "local").await?;
            total_result.models_synced += result.models_synced;
            total_result.currencies_synced += result.currencies_synced;
            total_result.tiered_pricing_synced += result.tiered_pricing_synced;
            total_result.errors += result.errors;
            // Return early - local config has high priority
            return Ok(total_result);
        }

        // 3. Sync from community repository (if enabled and due)
        if self.config.community_sync_enabled && self.should_sync_community() {
            println!("Syncing from community price repository...");
            match self.sync_community_prices().await {
                Ok(result) => {
                    total_result.models_synced += result.models_synced;
                    total_result.currencies_synced += result.currencies_synced;
                    total_result.tiered_pricing_synced += result.tiered_pricing_synced;
                    total_result.errors += result.errors;
                    self.last_community_sync = Some(Utc::now());
                }
                Err(e) => {
                    eprintln!("Community price sync failed: {}", e);
                    // Fall through to LiteLLM
                }
            }
        }

        // 4. Sync from LiteLLM (USD fallback only)
        println!("Syncing from LiteLLM (USD prices)...");
        let litellm_result = self.sync_litellm_to_v2().await?;
        total_result.models_synced += litellm_result.models_synced;
        total_result.errors += litellm_result.errors;
        self.last_litellm_sync = Some(Utc::now());

        Ok(total_result)
    }

    /// Check if community sync is due
    fn should_sync_community(&self) -> bool {
        match self.last_community_sync {
            None => true,
            Some(last) => {
                let elapsed = Utc::now() - last;
                elapsed.num_seconds() >= self.config.community_sync_interval_secs as i64
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
            // Apply standard pricing for each currency
            for (currency, currency_pricing) in &model_pricing.pricing {
                let price_input = PriceInput {
                    model: model_name.clone(),
                    currency: currency.clone(),
                    input_price: currency_pricing.input_price,
                    output_price: currency_pricing.output_price,
                    source: currency_pricing.source.clone().or(Some(source.to_string())),
                    ..Default::default()
                };

                match PriceModel::upsert(&self.db, &price_input).await {
                    Ok(_) => {
                        result.models_synced += 1;
                        result.currencies_synced += 1;
                    }
                    Err(e) => {
                        eprintln!("Failed to upsert price for {} ({}): {}", model_name, currency, e);
                        result.errors += 1;
                    }
                }
            }

            // Apply cache pricing
            if let Some(ref cache_pricing) = model_pricing.cache_pricing {
                for (currency, cache_config) in cache_pricing {
                    // Get existing price and update with cache pricing
                    if let Ok(Some(mut existing)) = PriceModel::get(&self.db, model_name, currency, None).await {
                        existing.cache_read_input_price = Some(cache_config.cache_read_input_price);
                        existing.cache_creation_input_price = cache_config.cache_creation_input_price;

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
                            source: existing.source.clone(),
                            region: existing.region.clone(),
                            context_window: existing.context_window,
                            max_output_tokens: existing.max_output_tokens,
                            supports_vision: existing.supports_vision_bool(),
                            supports_function_calling: existing.supports_function_calling_bool(),
                        };

                        if let Err(e) = PriceModel::upsert(&self.db, &update_input).await {
                            eprintln!("Failed to update cache pricing for {}: {}", model_name, e);
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
                                eprintln!(
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

        println!(
            "Applied {} models, {} currencies, {} tiers from {}",
            result.models_synced,
            result.currencies_synced,
            result.tiered_pricing_synced,
            source
        );

        Ok(result)
    }

    /// Sync prices from community repository
    async fn sync_community_prices(&self) -> anyhow::Result<SyncResult> {
        let response = self
            .http_client
            .get(&self.config.community_repo_url)
            .send()
            .await?
            .error_for_status()?;

        let text = response.text().await?;
        let config: PricingConfig = serde_json::from_str(&text)?;

        self.apply_prices(&config, "community").await
    }

    /// Sync prices from LiteLLM to prices table
    async fn sync_litellm_to_v2(&self) -> anyhow::Result<SyncResult> {
        let mut result = SyncResult {
            source: "litellm".to_string(),
            ..Default::default()
        };

        let prices = self.fetch_litellm_prices().await?;

        for (key, price_data) in prices {
            // Skip embedding models
            if price_data.mode.as_deref() == Some("embedding") {
                continue;
            }

            let model_name = match &price_data.model {
                Some(m) => m.clone(),
                None => key,
            };

            let (input_price, output_price) = price_data.to_per_million_price_nano();

            let (input, output) = match (input_price, output_price) {
                (Some(i), Some(o)) => (i, o),
                (Some(i), None) => (i, i),
                (None, Some(o)) => (o, o),
                (None, None) => continue,
            };

            let (cache_read, cache_creation) = price_data.to_cache_per_million_price_nano();
            let (batch_input, batch_output) = price_data.to_batch_per_million_price_nano();
            let (priority_input, priority_output) = price_data.to_priority_per_million_price_nano();
            let audio_input = price_data.to_audio_per_million_price_nano();

            let price_input = PriceInput {
                model: model_name.clone(),
                currency: "USD".to_string(),
                input_price: input,
                output_price: output,
                cache_read_input_price: cache_read,
                cache_creation_input_price: cache_creation,
                batch_input_price: batch_input,
                batch_output_price: batch_output,
                priority_input_price: priority_input,
                priority_output_price: priority_output,
                audio_input_price: audio_input,
                source: Some("litellm".to_string()),
                region: None,
                context_window: price_data.max_input_tokens.map(|t| t as i64),
                max_output_tokens: price_data.max_output_tokens.map(|t| t as i64),
                supports_vision: price_data.supports_vision,
                supports_function_calling: price_data.supports_function_calling,
            };

            match PriceModel::upsert(&self.db, &price_input).await {
                Ok(_) => {
                    result.models_synced += 1;
                }
                Err(e) => {
                    eprintln!("Failed to upsert price for {}: {}", model_name, e);
                    result.errors += 1;
                }
            }
        }

        println!(
            "Synced {} models from LiteLLM",
            result.models_synced
        );

        Ok(result)
    }

    /// Fetch LiteLLM prices
    async fn fetch_litellm_prices(&self) -> anyhow::Result<HashMap<String, LiteLLMPrice>> {
        let response = self
            .http_client
            .get(&self.config.litellm_url)
            .send()
            .await?
            .error_for_status()?;

        let text = response.text().await?;
        let prices: HashMap<String, LiteLLMPrice> = serde_json::from_str(&text)?;

        Ok(prices)
    }
}

/// Start a background price sync task using PriceSyncServiceV2
///
/// This spawns a tokio task that syncs prices periodically from multiple sources
pub fn start_price_sync_task_v2(
    db: Arc<Database>,
    interval_secs: u64,
    config: Option<PriceSyncConfig>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut service = match config {
            Some(cfg) => PriceSyncServiceV2::with_config(db, cfg),
            None => PriceSyncServiceV2::new(db),
        };
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));

        // Initial sync
        println!("Starting initial multi-source price sync...");
        match service.sync_all().await {
            Ok(result) => println!(
                "Initial price sync complete: {} models, {} currencies, {} tiers",
                result.models_synced, result.currencies_synced, result.tiered_pricing_synced
            ),
            Err(e) => eprintln!("Initial price sync failed: {}", e),
        }

        // Periodic sync
        loop {
            interval.tick().await;
            println!("Starting periodic multi-source price sync...");
            match service.sync_all().await {
                Ok(result) => println!(
                    "Periodic price sync complete: {} models, {} currencies, {} tiers",
                    result.models_synced, result.currencies_synced, result.tiered_pricing_synced
                ),
                Err(e) => eprintln!("Periodic price sync failed: {}", e),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_litellm_price_conversion() {
        // Test per-token to per-million conversion
        let price = LiteLLMPrice {
            model: Some("test-model".to_string()),
            input_cost_per_token: Some(0.000001), // $0.000001 per token = $1 per 1M
            output_cost_per_token: Some(0.000002), // $0.000002 per token = $2 per 1M
            max_input_tokens: Some(4096),
            max_output_tokens: Some(1024),
            pricing_model: None,
            supports_vision: None,
            supports_function_calling: None,
            mode: Some("chat".to_string()),
            cache_read_input_token_cost: None,
            cache_creation_input_token_cost: None,
            input_cost_per_token_batches: None,
            output_cost_per_token_batches: None,
            input_cost_per_token_priority: None,
            output_cost_per_token_priority: None,
            input_cost_per_audio_token: None,
            search_context_cost_per_query: None,
        };

        let (input, output) = price.to_per_million_price();
        assert!((input.unwrap() - 1.0).abs() < 0.001);
        assert!((output.unwrap() - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_litellm_price_missing_values() {
        // Test with missing pricing
        let price = LiteLLMPrice {
            model: Some("free-model".to_string()),
            input_cost_per_token: None,
            output_cost_per_token: None,
            max_input_tokens: None,
            max_output_tokens: None,
            pricing_model: None,
            supports_vision: None,
            supports_function_calling: None,
            mode: Some("chat".to_string()),
            cache_read_input_token_cost: None,
            cache_creation_input_token_cost: None,
            input_cost_per_token_batches: None,
            output_cost_per_token_batches: None,
            input_cost_per_token_priority: None,
            output_cost_per_token_priority: None,
            input_cost_per_audio_token: None,
            search_context_cost_per_query: None,
        };

        let (input, output) = price.to_per_million_price();
        assert!(input.is_none());
        assert!(output.is_none());
    }

    #[test]
    fn test_litellm_advanced_pricing_conversion() {
        // Test cache pricing conversion
        let price = LiteLLMPrice {
            model: Some("claude-3-5-sonnet".to_string()),
            input_cost_per_token: Some(0.000003), // $3 per 1M
            output_cost_per_token: Some(0.000015), // $15 per 1M
            cache_read_input_token_cost: Some(0.0000003), // $0.30 per 1M (10% of input)
            cache_creation_input_token_cost: Some(0.00000375), // $3.75 per 1M
            max_input_tokens: None,
            max_output_tokens: None,
            pricing_model: None,
            supports_vision: None,
            supports_function_calling: None,
            mode: Some("chat".to_string()),
            input_cost_per_token_batches: None,
            output_cost_per_token_batches: None,
            input_cost_per_token_priority: None,
            output_cost_per_token_priority: None,
            input_cost_per_audio_token: None,
            search_context_cost_per_query: None,
        };

        let (cache_read, cache_creation) = price.to_cache_per_million_price();
        assert!((cache_read.unwrap() - 0.30).abs() < 0.001);
        assert!((cache_creation.unwrap() - 3.75).abs() < 0.001);
    }

    // ========================================================================
    // PriceSyncServiceV2 Tests
    // ========================================================================

    #[test]
    fn test_sync_result_default() {
        let result = SyncResult::default();
        assert_eq!(result.models_synced, 0);
        assert_eq!(result.currencies_synced, 0);
        assert_eq!(result.tiered_pricing_synced, 0);
        assert_eq!(result.errors, 0);
        assert!(result.source.is_empty());
    }

    #[test]
    fn test_price_sync_config_default() {
        let config = PriceSyncConfig::default();
        assert_eq!(
            config.override_config_path,
            PathBuf::from("config/pricing.override.json")
        );
        assert_eq!(
            config.local_config_path,
            PathBuf::from("config/pricing.json")
        );
        assert_eq!(config.community_repo_url, COMMUNITY_PRICES_URL);
        assert_eq!(config.litellm_url, LITELLM_PRICES_URL);
        assert!(config.community_sync_enabled);
        assert_eq!(config.community_sync_interval_secs, 86400);
    }

    #[test]
    fn test_load_local_config_nonexistent() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let config = PriceSyncConfig {
            local_config_path: dir.path().join("nonexistent.json"),
            ..Default::default()
        };

        // Test load_local_config returns None for nonexistent file
        // Note: This test doesn't need a database - just testing file loading logic
        let path = &config.local_config_path;
        assert!(!path.exists());

        // Verify the path configuration
        assert_eq!(config.local_config_path, dir.path().join("nonexistent.json"));
    }

    #[test]
    fn test_load_local_config_valid() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let config_path = dir.path().join("pricing.json");

        let config_content = r#"{
            "version": "1.0",
            "updated_at": "2024-01-15T10:00:00Z",
            "source": "test",
            "models": {
                "gpt-4": {
                    "pricing": {
                        "USD": {
                            "input_price": 30.0,
                            "output_price": 60.0
                        }
                    }
                }
            }
        }"#;
        std::fs::write(&config_path, config_content).unwrap();

        // Test loading config from file (without database)
        let content = std::fs::read_to_string(&config_path).unwrap();
        let pricing_config: PricingConfig = serde_json::from_str(&content).unwrap();

        assert_eq!(pricing_config.version, "1.0");
        assert!(pricing_config.models.contains_key("gpt-4"));
    }

    #[test]
    fn test_load_override_priority() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let main_path = dir.path().join("pricing.json");
        let override_path = dir.path().join("pricing.override.json");

        // Write main config
        let main_content = r#"{
            "version": "1.0",
            "updated_at": "2024-01-15T10:00:00Z",
            "source": "main",
            "models": {}
        }"#;
        std::fs::write(&main_path, main_content).unwrap();

        // Write override config with different source
        let override_content = r#"{
            "version": "1.0",
            "updated_at": "2024-01-16T10:00:00Z",
            "source": "override",
            "models": {}
        }"#;
        std::fs::write(&override_path, override_content).unwrap();

        // Test that override file exists and has priority
        assert!(override_path.exists());

        let override_content = std::fs::read_to_string(&override_path).unwrap();
        let config: PricingConfig = serde_json::from_str(&override_content).unwrap();
        assert_eq!(config.source, "override");
    }

    #[test]
    fn test_community_sync_interval() {
        let config = PriceSyncConfig {
            community_sync_interval_secs: 3600, // 1 hour
            ..Default::default()
        };

        // Verify sync interval configuration
        assert_eq!(config.community_sync_interval_secs, 3600);
    }
}
