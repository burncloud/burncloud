//! Price Sync Module
//!
//! This module provides functionality for syncing model pricing data from
//! LiteLLM's model_prices_and_context_window.json file.

use std::collections::HashMap;

use burncloud_database::{sqlx, Database};
use burncloud_database_models::{PriceInput, PriceModel};
use reqwest::Client;
use serde::Deserialize;

/// URL for LiteLLM's model prices JSON file
const LITELLM_PRICES_URL: &str = "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";

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
    /// Context window size
    #[serde(default)]
    pub max_input_tokens: Option<u32>,
    /// Maximum output tokens
    #[serde(default)]
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
}

impl LiteLLMPrice {
    /// Convert per-token cost to per-1M tokens price
    pub fn to_per_million_price(&self) -> (Option<f64>, Option<f64>) {
        let input_price = self.input_cost_per_token.map(|c| c * 1_000_000.0);
        let output_price = self.output_cost_per_token.map(|c| c * 1_000_000.0);
        (input_price, output_price)
    }
}

/// Service for syncing prices from LiteLLM
pub struct PriceSyncService {
    db: Database,
    http_client: Client,
}

impl PriceSyncService {
    /// Create a new PriceSyncService
    pub fn new(db: Database) -> Self {
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

            // Get pricing info
            let (input_price, output_price) = price_data.to_per_million_price();

            // Skip if no pricing info
            let (input, output) = match (input_price, output_price) {
                (Some(i), Some(o)) => (i, o),
                (Some(i), None) => (i, i), // Use input price for output if not specified
                (None, Some(o)) => (o, o), // Use output price for input if not specified
                (None, None) => continue,  // No pricing info, skip
            };

            // Create price input
            let price_input = PriceInput {
                model: model_name.clone(),
                input_price: input,
                output_price: output,
                currency: Some("USD".to_string()),
                alias_for: price_data.pricing_model.clone(),
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

        println!("Capabilities sync complete: {} models updated", updated_count);
        Ok(updated_count)
    }
}

/// Start a background price sync task
///
/// This spawns a tokio task that syncs prices periodically
pub fn start_price_sync_task(db: Database, interval_secs: u64) -> tokio::task::JoinHandle<()> {
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
                Ok(count) => println!("Periodic capabilities sync complete: {} models updated", count),
                Err(e) => eprintln!("Periodic capabilities sync failed: {}", e),
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
        };

        let (input, output) = price.to_per_million_price();
        assert!(input.is_none());
        assert!(output.is_none());
    }
}
