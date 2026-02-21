//! Pricing configuration module for advanced pricing dimensions.
//!
//! This module defines the schema and data structures for pricing.json files,
//! supporting multi-currency, tiered pricing, cache pricing, batch pricing,
//! and other advanced pricing features.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root structure for pricing.json configuration file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingConfig {
    /// Schema version (e.g., "1.0")
    pub version: String,
    /// When this configuration was last updated
    pub updated_at: DateTime<Utc>,
    /// Source of the pricing data (e.g., "local", "litellm", "community")
    pub source: String,
    /// Model pricing configurations keyed by model name
    pub models: HashMap<String, ModelPricing>,
}

/// Pricing configuration for a single model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Standard pricing per currency
    #[serde(default)]
    pub pricing: HashMap<String, CurrencyPricing>,
    /// Tiered pricing per currency (for usage-based tiers like Qwen)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiered_pricing: Option<HashMap<String, Vec<TieredPriceConfig>>>,
    /// Cache pricing per currency (for Prompt Caching)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_pricing: Option<HashMap<String, CachePricingConfig>>,
    /// Batch pricing per currency (for Batch API)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_pricing: Option<HashMap<String, BatchPricingConfig>>,
    /// Model metadata (context window, capabilities, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ModelMetadata>,
}

/// Pricing for a specific currency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyPricing {
    /// Input price per 1M tokens
    pub input_price: f64,
    /// Output price per 1M tokens
    pub output_price: f64,
    /// Source of this pricing (e.g., "openai", "anthropic", "converted")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Tiered pricing configuration for usage-based pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredPriceConfig {
    /// Starting token count for this tier (inclusive)
    pub tier_start: i64,
    /// Ending token count for this tier (exclusive, None = no limit)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier_end: Option<i64>,
    /// Input price per 1M tokens for this tier
    pub input_price: f64,
    /// Output price per 1M tokens for this tier
    pub output_price: f64,
}

/// Cache pricing for Prompt Caching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePricingConfig {
    /// Cache read price per 1M tokens (usually 10% of standard)
    pub cache_read_input_price: f64,
    /// Cache creation price per 1M tokens
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_price: Option<f64>,
}

/// Batch pricing for Batch API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPricingConfig {
    /// Batch input price per 1M tokens (usually 50% of standard)
    pub batch_input_price: f64,
    /// Batch output price per 1M tokens
    pub batch_output_price: f64,
}

/// Model metadata for capabilities and limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Maximum context window in tokens
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<i64>,
    /// Maximum output tokens
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i64>,
    /// Whether the model supports vision/image input
    #[serde(default)]
    pub supports_vision: bool,
    /// Whether the model supports function calling
    #[serde(default)]
    pub supports_function_calling: bool,
    /// Whether the model supports streaming
    #[serde(default = "default_true")]
    pub supports_streaming: bool,
    /// Provider name (e.g., "openai", "anthropic")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Model family (e.g., "gpt-4", "claude-3")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    /// Release date
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
}

fn default_true() -> bool {
    true
}

impl PricingConfig {
    /// Create a new empty pricing configuration.
    pub fn new(source: &str) -> Self {
        Self {
            version: "1.0".to_string(),
            updated_at: Utc::now(),
            source: source.to_string(),
            models: HashMap::new(),
        }
    }

    /// Parse pricing configuration from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Validate the pricing configuration.
    /// Returns a list of warnings for non-critical issues.
    pub fn validate(&self) -> Result<Vec<ValidationWarning>, ValidationError> {
        let mut warnings = Vec::new();

        // Validate version format
        if !self.version.contains('.') {
            warnings.push(ValidationWarning {
                field: "version".to_string(),
                message: format!("Version '{}' should be in format 'X.Y'", self.version),
                suggestion: "Use semantic versioning like '1.0'".to_string(),
            });
        }

        // Validate each model's pricing
        for (model_name, model_pricing) in &self.models {
            // Check for negative prices
            for (currency, pricing) in &model_pricing.pricing {
                if pricing.input_price < 0.0 {
                    return Err(ValidationError::NegativePrice {
                        model: model_name.clone(),
                        field: format!("{}.input_price", currency),
                        value: pricing.input_price,
                    });
                }
                if pricing.output_price < 0.0 {
                    return Err(ValidationError::NegativePrice {
                        model: model_name.clone(),
                        field: format!("{}.output_price", currency),
                        value: pricing.output_price,
                    });
                }

                // Warn if price seems unreasonably high (> $1000/1M tokens)
                if pricing.input_price > 1000.0 || pricing.output_price > 1000.0 {
                    warnings.push(ValidationWarning {
                        field: format!("models.{}.pricing.{}", model_name, currency),
                        message: format!(
                            "Price ${:.2}/1M seems unusually high",
                            pricing.input_price.max(pricing.output_price)
                        ),
                        suggestion: "Verify the pricing is correct".to_string(),
                    });
                }
            }

            // Validate tiered pricing
            if let Some(ref tiered) = model_pricing.tiered_pricing {
                for (currency, tiers) in tiered {
                    // Check tier ordering
                    for (i, tier) in tiers.iter().enumerate() {
                        if tier.input_price < 0.0 || tier.output_price < 0.0 {
                            return Err(ValidationError::NegativePrice {
                                model: model_name.clone(),
                                field: format!("tiered_pricing.{}.tier[{}]", currency, i),
                                value: tier.input_price.min(tier.output_price),
                            });
                        }

                        // Check tier boundaries
                        if let Some(tier_end) = tier.tier_end {
                            if tier_end <= tier.tier_start {
                                return Err(ValidationError::InvalidTier {
                                    model: model_name.clone(),
                                    tier_index: i,
                                    message: format!(
                                        "tier_end ({}) must be greater than tier_start ({})",
                                        tier_end, tier.tier_start
                                    ),
                                });
                            }
                        }

                        // Check for gaps between tiers
                        if i > 0 {
                            let prev_tier = &tiers[i - 1];
                            if let Some(prev_end) = prev_tier.tier_end {
                                if prev_end != tier.tier_start {
                                    warnings.push(ValidationWarning {
                                        field: format!(
                                            "models.{}.tiered_pricing.{}.tier[{}]",
                                            model_name, currency, i
                                        ),
                                        message: format!(
                                            "Gap between tiers: previous ends at {}, current starts at {}",
                                            prev_end, tier.tier_start
                                        ),
                                        suggestion: "Tiers should be contiguous for accurate billing".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }

            // Check if model has any pricing
            if model_pricing.pricing.is_empty() {
                warnings.push(ValidationWarning {
                    field: format!("models.{}", model_name),
                    message: "Model has no pricing configured".to_string(),
                    suggestion: "Add pricing for at least one currency (USD recommended)".to_string(),
                });
            }
        }

        Ok(warnings)
    }

    /// Get pricing for a specific model and currency.
    pub fn get_pricing(&self, model: &str, currency: &str) -> Option<&CurrencyPricing> {
        self.models.get(model)?.pricing.get(currency)
    }

    /// Get tiered pricing for a specific model and currency.
    pub fn get_tiered_pricing(
        &self,
        model: &str,
        currency: &str,
    ) -> Option<&Vec<TieredPriceConfig>> {
        self.models
            .get(model)?
            .tiered_pricing
            .as_ref()?
            .get(currency)
    }

    /// Get cache pricing for a specific model and currency.
    pub fn get_cache_pricing(&self, model: &str, currency: &str) -> Option<&CachePricingConfig> {
        self.models
            .get(model)?
            .cache_pricing
            .as_ref()?
            .get(currency)
    }

    /// List all models in this configuration.
    pub fn list_models(&self) -> Vec<&String> {
        self.models.keys().collect()
    }
}

/// Validation warning for non-critical issues.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Field path that triggered the warning
    pub field: String,
    /// Warning message
    pub message: String,
    /// Suggested fix
    pub suggestion: String,
}

/// Validation error for critical issues.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Negative price in {model}.{field}: {value}")]
    NegativePrice {
        model: String,
        field: String,
        value: f64,
    },
    #[error("Invalid tier configuration in {model} at tier {tier_index}: {message}")]
    InvalidTier {
        model: String,
        tier_index: usize,
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricing_config_creation() {
        let config = PricingConfig::new("test");
        assert_eq!(config.version, "1.0");
        assert_eq!(config.source, "test");
        assert!(config.models.is_empty());
    }

    #[test]
    fn test_pricing_config_json_roundtrip() {
        let mut config = PricingConfig::new("test");
        let mut pricing = HashMap::new();
        pricing.insert(
            "USD".to_string(),
            CurrencyPricing {
                input_price: 10.0,
                output_price: 30.0,
                source: Some("openai".to_string()),
            },
        );

        let model_pricing = ModelPricing {
            pricing,
            tiered_pricing: None,
            cache_pricing: None,
            batch_pricing: None,
            metadata: Some(ModelMetadata {
                context_window: Some(128000),
                max_output_tokens: Some(4096),
                supports_vision: true,
                supports_function_calling: true,
                supports_streaming: true,
                provider: Some("openai".to_string()),
                family: Some("gpt-4".to_string()),
                release_date: None,
            }),
        };

        config.models.insert("gpt-4-turbo".to_string(), model_pricing);

        let json = config.to_json().unwrap();
        let parsed = PricingConfig::from_json(&json).unwrap();

        assert_eq!(parsed.models.len(), 1);
        assert!(parsed.models.contains_key("gpt-4-turbo"));
    }

    #[test]
    fn test_pricing_config_validation() {
        let mut config = PricingConfig::new("test");

        // Add valid pricing
        let mut pricing = HashMap::new();
        pricing.insert(
            "USD".to_string(),
            CurrencyPricing {
                input_price: 10.0,
                output_price: 30.0,
                source: None,
            },
        );

        config.models.insert(
            "test-model".to_string(),
            ModelPricing {
                pricing,
                tiered_pricing: None,
                cache_pricing: None,
                batch_pricing: None,
                metadata: None,
            },
        );

        let warnings = config.validate().unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_negative_price_validation() {
        let mut config = PricingConfig::new("test");

        let mut pricing = HashMap::new();
        pricing.insert(
            "USD".to_string(),
            CurrencyPricing {
                input_price: -10.0,
                output_price: 30.0,
                source: None,
            },
        );

        config.models.insert(
            "test-model".to_string(),
            ModelPricing {
                pricing,
                tiered_pricing: None,
                cache_pricing: None,
                batch_pricing: None,
                metadata: None,
            },
        );

        let result = config.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::NegativePrice { model, field, .. } => {
                assert_eq!(model, "test-model");
                assert!(field.contains("input_price"));
            }
            _ => panic!("Expected NegativePrice error"),
        }
    }

    #[test]
    fn test_tiered_pricing_validation() {
        let mut config = PricingConfig::new("test");

        let mut pricing = HashMap::new();
        pricing.insert(
            "USD".to_string(),
            CurrencyPricing {
                input_price: 10.0,
                output_price: 30.0,
                source: None,
            },
        );

        let tiered = vec![
            TieredPriceConfig {
                tier_start: 0,
                tier_end: Some(32000),
                input_price: 1.2,
                output_price: 6.0,
            },
            TieredPriceConfig {
                tier_start: 32000,
                tier_end: Some(128000),
                input_price: 2.4,
                output_price: 12.0,
            },
        ];

        let mut tiered_map = HashMap::new();
        tiered_map.insert("USD".to_string(), tiered);

        config.models.insert(
            "qwen-max".to_string(),
            ModelPricing {
                pricing,
                tiered_pricing: Some(tiered_map),
                cache_pricing: None,
                batch_pricing: None,
                metadata: None,
            },
        );

        let warnings = config.validate().unwrap();
        // Should have no warnings for valid tier configuration
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_invalid_tier_boundaries() {
        let mut config = PricingConfig::new("test");

        let mut pricing = HashMap::new();
        pricing.insert(
            "USD".to_string(),
            CurrencyPricing {
                input_price: 10.0,
                output_price: 30.0,
                source: None,
            },
        );

        // Invalid tier: tier_end <= tier_start
        let tiered = vec![TieredPriceConfig {
            tier_start: 100,
            tier_end: Some(50),
            input_price: 1.2,
            output_price: 6.0,
        }];

        let mut tiered_map = HashMap::new();
        tiered_map.insert("USD".to_string(), tiered);

        config.models.insert(
            "invalid-model".to_string(),
            ModelPricing {
                pricing,
                tiered_pricing: Some(tiered_map),
                cache_pricing: None,
                batch_pricing: None,
                metadata: None,
            },
        );

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_pricing_methods() {
        let mut config = PricingConfig::new("test");

        let mut pricing = HashMap::new();
        pricing.insert(
            "USD".to_string(),
            CurrencyPricing {
                input_price: 10.0,
                output_price: 30.0,
                source: None,
            },
        );

        let cache_pricing = CachePricingConfig {
            cache_read_input_price: 1.0,
            cache_creation_input_price: Some(1.25),
        };
        let mut cache_map = HashMap::new();
        cache_map.insert("USD".to_string(), cache_pricing);

        config.models.insert(
            "claude-3".to_string(),
            ModelPricing {
                pricing,
                tiered_pricing: None,
                cache_pricing: Some(cache_map),
                batch_pricing: None,
                metadata: None,
            },
        );

        // Test get_pricing
        let p = config.get_pricing("claude-3", "USD");
        assert!(p.is_some());
        assert_eq!(p.unwrap().input_price, 10.0);

        // Test get_cache_pricing
        let c = config.get_cache_pricing("claude-3", "USD");
        assert!(c.is_some());
        assert_eq!(c.unwrap().cache_read_input_price, 1.0);

        // Test non-existent model
        assert!(config.get_pricing("nonexistent", "USD").is_none());
    }
}
