use anyhow::Result;
use burncloud_common::{dollars_to_nano, nano_to_dollars, pricing_config::{
    BatchPricingConfig, CachePricingConfig, CurrencyPricing, ModelMetadata, ModelPricing,
    PricingConfig, TieredPriceConfig,
}};
use burncloud_database::Database;
use burncloud_database_models::{
    PriceV2Input, PriceV2Model, TieredPriceInput, TieredPriceModel,
};
use chrono::Utc;
use clap::ArgMatches;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

/// Helper to convert f64 dollars to i64 nanodollars
fn to_nano(price: f64) -> i64 {
    dollars_to_nano(price) as i64
}

/// Helper to convert i64 nanodollars to f64 dollars
fn from_nano(price: i64) -> f64 {
    nano_to_dollars(price as u64)
}

pub async fn handle_price_command(db: &Database, matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", sub_m)) => {
            let limit: i32 = sub_m
                .get_one::<String>("limit")
                .and_then(|s| s.parse().ok())
                .unwrap_or(100);
            let offset: i32 = sub_m
                .get_one::<String>("offset")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let currency = sub_m.get_one::<String>("currency").map(|s| s.as_str());

            // Use PriceV2Model for multi-currency support
            let prices = PriceV2Model::list(db, limit, offset, currency).await?;

            if prices.is_empty() {
                println!("No prices found.");
                return Ok(());
            }

            println!(
                "{:<30} {:>8} {:>15} {:>15} {:>10}",
                "Model", "Currency", "Input ($/1M)", "Output ($/1M)", "Region"
            );
            println!("{}", "-".repeat(80));

            for price in prices {
                let region = price.region.as_deref().unwrap_or("-");
                println!(
                    "{:<30} {:>8} {:>15.4} {:>15.4} {:>10}",
                    price.model, price.currency, from_nano(price.input_price), from_nano(price.output_price), region
                );
            }
        }
        Some(("set", sub_m)) => {
            let model = sub_m.get_one::<String>("model").unwrap().to_string();
            let input_price: f64 = sub_m
                .get_one::<String>("input")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let output_price: f64 = sub_m
                .get_one::<String>("output")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let currency = sub_m
                .get_one::<String>("currency")
                .map(|s| s.to_string())
                .unwrap_or_else(|| "USD".to_string());
            let region = sub_m.get_one::<String>("region").cloned();

            // Optional advanced pricing fields
            let cache_read_input_price: Option<f64> = sub_m
                .get_one::<String>("cache-read")
                .and_then(|s| s.parse().ok());
            let cache_creation_input_price: Option<f64> = sub_m
                .get_one::<String>("cache-creation")
                .and_then(|s| s.parse().ok());
            let batch_input_price: Option<f64> = sub_m
                .get_one::<String>("batch-input")
                .and_then(|s| s.parse().ok());
            let batch_output_price: Option<f64> = sub_m
                .get_one::<String>("batch-output")
                .and_then(|s| s.parse().ok());

            // Use PriceV2Model for multi-currency support
            // Convert f64 dollar input to i64 nanodollars
            let input = PriceV2Input {
                model: model.clone(),
                currency: currency.clone(),
                input_price: to_nano(input_price),
                output_price: to_nano(output_price),
                cache_read_input_price: cache_read_input_price.map(to_nano),
                cache_creation_input_price: cache_creation_input_price.map(to_nano),
                batch_input_price: batch_input_price.map(to_nano),
                batch_output_price: batch_output_price.map(to_nano),
                priority_input_price: None,
                priority_output_price: None,
                audio_input_price: None,
                source: Some("cli".to_string()),
                region,
                context_window: None,
                max_output_tokens: None,
                supports_vision: None,
                supports_function_calling: None,
            };

            PriceV2Model::upsert(db, &input).await?;

            let region_str = input.region.as_deref().unwrap_or("");
            let region_display = if region_str.is_empty() { "".to_string() } else { format!(" [{}]", region_str) };
            println!(
                "✓ Price set for '{}': {} input={:.4}/1M, output={:.4}/1M{}",
                model, currency, input_price, output_price, region_display
            );

            if let Some(cr) = cache_read_input_price {
                println!("  Cache read: {:.4}/1M", cr);
            }
            if let Some(bi) = batch_input_price {
                println!("  Batch input: {:.4}/1M", bi);
            }
        }
        Some(("delete", sub_m)) => {
            let model = sub_m.get_one::<String>("model").unwrap();

            PriceV2Model::delete_all_for_model(db, model).await?;
            println!("✓ All prices deleted for '{}'", model);
        }
        Some(("get", sub_m)) => {
            let model = sub_m.get_one::<String>("model").unwrap();
            let currency = sub_m.get_one::<String>("currency").map(|s| s.as_str());
            let verbose = sub_m.get_flag("verbose");

            // Use PriceV2Model for multi-currency support
            if let Some(curr) = currency {
                // Get specific currency
                match PriceV2Model::get(db, model, curr, None).await? {
                    Some(price) => {
                        println!("Model: {}", price.model);
                        println!("Currency: {}", price.currency);
                        println!("Input Price: {:.4}/1M tokens", from_nano(price.input_price));
                        println!("Output Price: {:.4}/1M tokens", from_nano(price.output_price));
                        if let Some(region) = &price.region {
                            println!("Region: {}", region);
                        }
                        // Display advanced pricing if available
                        if let Some(cache_read) = price.cache_read_input_price {
                            println!("Cache Read Price: {:.4}/1M tokens", from_nano(cache_read));
                        }
                        if let Some(cache_creation) = price.cache_creation_input_price {
                            println!("Cache Creation Price: {:.4}/1M tokens", from_nano(cache_creation));
                        }
                        if let Some(batch_input) = price.batch_input_price {
                            println!("Batch Input Price: {:.4}/1M tokens", from_nano(batch_input));
                        }
                        if let Some(batch_output) = price.batch_output_price {
                            println!("Batch Output Price: {:.4}/1M tokens", from_nano(batch_output));
                        }
                    }
                    None => {
                        println!("No {} price found for model '{}'", curr, model);
                    }
                }
            } else {
                // Get all currencies
                let prices = PriceV2Model::get_all_currencies(db, model, None).await?;
                if prices.is_empty() {
                    println!("No prices found for model '{}'", model);
                } else {
                    println!("Model: {}", model);
                    println!("{}", "-".repeat(50));
                    for price in prices {
                        println!("\nCurrency: {}", price.currency);
                        if let Some(region) = &price.region {
                            println!("Region: {}", region);
                        }
                        println!("Input Price: {:.4}/1M tokens", from_nano(price.input_price));
                        println!("Output Price: {:.4}/1M tokens", from_nano(price.output_price));
                        if let Some(cache_read) = price.cache_read_input_price {
                            println!("Cache Read Price: {:.4}/1M tokens", from_nano(cache_read));
                        }
                        if let Some(batch_input) = price.batch_input_price {
                            println!("Batch Input Price: {:.4}/1M tokens", from_nano(batch_input));
                        }
                    }
                }
            }

            // Show tiered pricing in verbose mode
            if verbose {
                println!("\n--- Tiered Pricing ---");
                let has_tiered = TieredPriceModel::has_tiered_pricing(db, model).await?;
                if has_tiered {
                    let tiers = TieredPriceModel::get_tiers(db, model, None).await?;
                    for tier in tiers {
                        let region = tier.region.as_deref().unwrap_or("universal");
                        let tier_end = tier.tier_end.map_or("∞".to_string(), |e| format!("{}", e));
                        println!(
                            "  [{}] {}-{} tokens: {:.4}/{:.4} per 1M (in/out)",
                            region, tier.tier_start, tier_end, from_nano(tier.input_price), from_nano(tier.output_price)
                        );
                    }
                } else {
                    println!("  No tiered pricing configured for this model.");
                }
            }
        }
        Some(("show", sub_m)) => {
            let model = sub_m.get_one::<String>("model").unwrap();
            let currency = sub_m.get_one::<String>("currency").map(|s| s.as_str());
            let region = sub_m.get_one::<String>("region").map(|s| s.as_str());

            println!("=== Pricing Details for '{}' ===\n", model);

            // Get all prices for this model
            let prices = if let Some(curr) = currency {
                PriceV2Model::get(db, model, curr, region).await?
                    .map(|p| vec![p])
                    .unwrap_or_default()
            } else {
                PriceV2Model::get_all_currencies(db, model, region).await?
            };

            if prices.is_empty() {
                println!("No prices found for model '{}'", model);
                return Ok(());
            }

            // Display standard pricing
            println!("Standard Pricing:");
            println!("{:<10} {:>15} {:>15} {:>10}", "Currency", "Input ($/1M)", "Output ($/1M)", "Region");
            println!("{}", "-".repeat(55));
            for price in &prices {
                let region_str = price.region.as_deref().unwrap_or("-");
                println!(
                    "{:<10} {:>15.4} {:>15.4} {:>10}",
                    price.currency, from_nano(price.input_price), from_nano(price.output_price), region_str
                );
            }

            // Check for advanced pricing
            let has_cache = prices.iter().any(|p| p.cache_read_input_price.is_some());
            let has_batch = prices.iter().any(|p| p.batch_input_price.is_some());

            if has_cache {
                println!("\nCache Pricing:");
                println!("{:<10} {:>20} {:>20}", "Currency", "Cache Read ($/1M)", "Cache Creation ($/1M)");
                println!("{}", "-".repeat(55));
                for price in &prices {
                    if price.cache_read_input_price.is_some() || price.cache_creation_input_price.is_some() {
                        let cache_read = price.cache_read_input_price.map(from_nano).unwrap_or(0.0);
                        let cache_creation = price.cache_creation_input_price
                            .map(|v| format!("{:.4}", from_nano(v)))
                            .unwrap_or("-".to_string());
                        println!(
                            "{:<10} {:>20.4} {:>20}",
                            price.currency,
                            cache_read,
                            cache_creation
                        );
                    }
                }
            }

            if has_batch {
                println!("\nBatch Pricing:");
                println!("{:<10} {:>20} {:>20}", "Currency", "Batch Input ($/1M)", "Batch Output ($/1M)");
                println!("{}", "-".repeat(55));
                for price in &prices {
                    if price.batch_input_price.is_some() || price.batch_output_price.is_some() {
                        let batch_input = price.batch_input_price.map(from_nano).unwrap_or(0.0);
                        let batch_output = price.batch_output_price.map(from_nano).unwrap_or(0.0);
                        println!(
                            "{:<10} {:>20.4} {:>20.4}",
                            price.currency,
                            batch_input,
                            batch_output
                        );
                    }
                }
            }

            // Show tiered pricing
            let has_tiered = TieredPriceModel::has_tiered_pricing(db, model).await?;
            if has_tiered {
                let tiers = TieredPriceModel::get_tiers(db, model, region).await?;
                println!("\nTiered Pricing:");
                println!("{:<12} {:>12} {:>12} {:>15} {:>15}", "Region", "Tier Start", "Tier End", "Input ($/1M)", "Output ($/1M)");
                println!("{}", "-".repeat(70));
                for tier in tiers {
                    let region = tier.region.as_deref().unwrap_or("universal");
                    let tier_end = tier.tier_end.map_or("∞".to_string(), |e| format!("{}", e));
                    println!(
                        "{:<12} {:>12} {:>12} {:>15.4} {:>15.4}",
                        region, tier.tier_start, tier_end, from_nano(tier.input_price), from_nano(tier.output_price)
                    );
                }
            }

            // Show metadata
            if let Some(first_price) = prices.first() {
                if first_price.context_window.is_some() || first_price.supports_vision_bool().unwrap_or(false) {
                    println!("\nModel Metadata:");
                    if let Some(cw) = first_price.context_window {
                        println!("  Context Window: {} tokens", cw);
                    }
                    if let Some(mo) = first_price.max_output_tokens {
                        println!("  Max Output: {} tokens", mo);
                    }
                    if first_price.supports_vision_bool().unwrap_or(false) {
                        println!("  Vision: Yes");
                    }
                    if first_price.supports_function_calling_bool().unwrap_or(false) {
                        println!("  Function Calling: Yes");
                    }
                }
            }
        }
        Some(("sync-status", _)) => {
            // Show sync status by counting models with advanced pricing
            let prices = PriceV2Model::list(db, 10000, 0, None).await?;

            let mut total = 0;
            let mut with_cache = 0;
            let mut with_batch = 0;
            let mut with_priority = 0;
            let mut with_audio = 0;

            for price in &prices {
                total += 1;
                if price.cache_read_input_price.is_some() || price.cache_creation_input_price.is_some() {
                    with_cache += 1;
                }
                if price.batch_input_price.is_some() || price.batch_output_price.is_some() {
                    with_batch += 1;
                }
                if price.priority_input_price.is_some() || price.priority_output_price.is_some() {
                    with_priority += 1;
                }
                if price.audio_input_price.is_some() {
                    with_audio += 1;
                }
            }

            // Check tiered pricing count
            let tiered_prices = TieredPriceModel::list_all(db).await?;
            let mut tiered_models = std::collections::HashSet::new();
            for tier in &tiered_prices {
                tiered_models.insert(tier.model.clone());
            }
            let with_tiered = tiered_models.len() as i32;

            println!("Pricing Sync Status");
            println!("{}", "=".repeat(50));
            println!("Total price entries: {}", total);
            println!("Models with cache pricing: {}", with_cache);
            println!("Models with batch pricing: {}", with_batch);
            println!("Models with priority pricing: {}", with_priority);
            println!("Models with audio pricing: {}", with_audio);
            println!("Models with tiered pricing: {}", with_tiered);
            println!();
            println!("Tiered pricing entries: {}", tiered_prices.len());
            println!();
            println!("Note: Prices are synced hourly from LiteLLM by default.");
            println!("Use 'burncloud price import <file>' to import pricing configuration.");
        }
        Some(("import", sub_m)) => {
            let file_path = sub_m.get_one::<String>("file").unwrap();
            let override_mode = sub_m.get_flag("override");

            let content = std::fs::read_to_string(Path::new(file_path))?;
            let config = PricingConfig::from_json(&content)?;

            // Validate configuration
            match config.validate() {
                Ok(warnings) => {
                    if !warnings.is_empty() {
                        println!("Validation warnings:");
                        for warning in &warnings {
                            println!("  - {}: {}", warning.field, warning.message);
                            println!("    Suggestion: {}", warning.suggestion);
                        }
                        println!();
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Validation error: {}", e));
                }
            }

            let model_count = config.models.len();
            if !override_mode && model_count > 0 {
                println!("This will import pricing for {} model(s).", model_count);
                print!("Continue? [y/N] ");
                std::io::stdout().flush()?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Import cancelled.");
                    return Ok(());
                }
            }

            let mut prices_imported = 0;
            let mut tiers_imported = 0;
            let mut errors = Vec::new();

            for (model_name, model_pricing) in &config.models {
                // Import standard pricing for each currency
                for (currency, pricing) in &model_pricing.pricing {
                    // Get metadata if available
                    let metadata = &model_pricing.metadata;

                    // Determine region from source or default to None
                    let region = pricing.source.as_ref().and_then(|s| {
                        if s == "cn" || s == "international" {
                            Some(s.clone())
                        } else {
                            None
                        }
                    });

                    // Build PriceV2Input
                    let input = PriceV2Input {
                        model: model_name.clone(),
                        currency: currency.clone(),
                        input_price: pricing.input_price,
                        output_price: pricing.output_price,
                        cache_read_input_price: None,
                        cache_creation_input_price: None,
                        batch_input_price: None,
                        batch_output_price: None,
                        priority_input_price: None,
                        priority_output_price: None,
                        audio_input_price: None,
                        source: pricing.source.clone(),
                        region,
                        context_window: metadata.as_ref().and_then(|m| m.context_window),
                        max_output_tokens: metadata.as_ref().and_then(|m| m.max_output_tokens),
                        supports_vision: metadata.as_ref().map(|m| m.supports_vision),
                        supports_function_calling: metadata.as_ref().map(|m| m.supports_function_calling),
                    };

                    match PriceV2Model::upsert(db, &input).await {
                        Ok(_) => prices_imported += 1,
                        Err(e) => errors.push(format!("Failed to import {} ({}): {}", model_name, currency, e)),
                    }
                }

                // Import cache pricing if available
                if let Some(ref cache_pricing) = model_pricing.cache_pricing {
                    for (currency, cache) in cache_pricing {
                        // Update existing price with cache pricing
                        if let Some(existing) = PriceV2Model::get(db, model_name, currency, None).await? {
                            let supports_vision = existing.supports_vision_bool();
                            let supports_function_calling = existing.supports_function_calling_bool();
                            let input = PriceV2Input {
                                model: model_name.clone(),
                                currency: currency.clone(),
                                input_price: existing.input_price,
                                output_price: existing.output_price,
                                cache_read_input_price: Some(cache.cache_read_input_price),
                                cache_creation_input_price: cache.cache_creation_input_price,
                                batch_input_price: existing.batch_input_price,
                                batch_output_price: existing.batch_output_price,
                                priority_input_price: existing.priority_input_price,
                                priority_output_price: existing.priority_output_price,
                                audio_input_price: existing.audio_input_price,
                                source: existing.source.clone(),
                                region: existing.region.clone(),
                                context_window: existing.context_window,
                                max_output_tokens: existing.max_output_tokens,
                                supports_vision,
                                supports_function_calling,
                            };
                            match PriceV2Model::upsert(db, &input).await {
                                Ok(_) => {}
                                Err(e) => errors.push(format!("Failed to update cache pricing for {} ({}): {}", model_name, currency, e)),
                            }
                        }
                    }
                }

                // Import batch pricing if available
                if let Some(ref batch_pricing) = model_pricing.batch_pricing {
                    for (currency, batch) in batch_pricing {
                        if let Some(existing) = PriceV2Model::get(db, model_name, currency, None).await? {
                            let supports_vision = existing.supports_vision_bool();
                            let supports_function_calling = existing.supports_function_calling_bool();
                            let input = PriceV2Input {
                                model: model_name.clone(),
                                currency: currency.clone(),
                                input_price: existing.input_price,
                                output_price: existing.output_price,
                                cache_read_input_price: existing.cache_read_input_price,
                                cache_creation_input_price: existing.cache_creation_input_price,
                                batch_input_price: Some(batch.batch_input_price),
                                batch_output_price: Some(batch.batch_output_price),
                                priority_input_price: existing.priority_input_price,
                                priority_output_price: existing.priority_output_price,
                                audio_input_price: existing.audio_input_price,
                                source: existing.source.clone(),
                                region: existing.region.clone(),
                                context_window: existing.context_window,
                                max_output_tokens: existing.max_output_tokens,
                                supports_vision,
                                supports_function_calling,
                            };
                            match PriceV2Model::upsert(db, &input).await {
                                Ok(_) => {}
                                Err(e) => errors.push(format!("Failed to update batch pricing for {} ({}): {}", model_name, currency, e)),
                            }
                        }
                    }
                }

                // Import tiered pricing if available
                if let Some(ref tiered_pricing) = model_pricing.tiered_pricing {
                    for (currency, tiers) in tiered_pricing {
                        for tier in tiers {
                            // Determine region from currency or default
                            let region = if currency == "CNY" {
                                Some("cn".to_string())
                            } else if currency == "USD" {
                                Some("international".to_string())
                            } else {
                                None
                            };

                            let tier_input = TieredPriceInput {
                                model: model_name.clone(),
                                region: region.clone(),
                                tier_start: tier.tier_start,
                                tier_end: tier.tier_end,
                                input_price: tier.input_price,
                                output_price: tier.output_price,
                            };

                            match TieredPriceModel::upsert_tier(db, &tier_input).await {
                                Ok(_) => tiers_imported += 1,
                                Err(e) => errors.push(format!(
                                    "Failed to import tier for {} ({}): {}",
                                    model_name, currency, e
                                )),
                            }
                        }
                    }
                }
            }

            println!("✓ Import complete:");
            println!("  Prices imported: {}", prices_imported);
            println!("  Tiered entries imported: {}", tiers_imported);
            if !errors.is_empty() {
                println!("\nErrors:");
                for error in &errors {
                    println!("  - {}", error);
                }
            }
        }
        Some(("export", sub_m)) => {
            let file_path = sub_m.get_one::<String>("file").unwrap();
            let format = sub_m.get_one::<String>("format").unwrap();

            // Fetch all prices from prices table
            let prices = PriceV2Model::list(db, 100000, 0, None).await?;
            let tiered_prices = TieredPriceModel::list_all(db).await?;

            if prices.is_empty() {
                println!("No prices to export.");
                return Ok(());
            }

            // Group prices by model
            let mut models: HashMap<String, ModelPricing> = HashMap::new();

            for price in &prices {
                let entry = models.entry(price.model.clone()).or_insert_with(|| {
                    ModelPricing {
                        pricing: HashMap::new(),
                        tiered_pricing: None,
                        cache_pricing: None,
                        batch_pricing: None,
                        metadata: None,
                    }
                });

                // Add standard pricing
                entry.pricing.insert(
                    price.currency.clone(),
                    CurrencyPricing {
                        input_price: price.input_price,
                        output_price: price.output_price,
                        source: price.source.clone(),
                    },
                );

                // Add cache pricing if available
                if price.cache_read_input_price.is_some() || price.cache_creation_input_price.is_some() {
                    let cache_map = entry.cache_pricing.get_or_insert_with(HashMap::new);
                    cache_map.insert(
                        price.currency.clone(),
                        CachePricingConfig {
                            cache_read_input_price: price.cache_read_input_price.unwrap_or(0),
                            cache_creation_input_price: price.cache_creation_input_price,
                        },
                    );
                }

                // Add batch pricing if available
                if price.batch_input_price.is_some() || price.batch_output_price.is_some() {
                    let batch_map = entry.batch_pricing.get_or_insert_with(HashMap::new);
                    batch_map.insert(
                        price.currency.clone(),
                        BatchPricingConfig {
                            batch_input_price: price.batch_input_price.unwrap_or(0),
                            batch_output_price: price.batch_output_price.unwrap_or(0),
                        },
                    );
                }

                // Add metadata if available
                if price.context_window.is_some() || price.supports_vision_bool().unwrap_or(false) {
                    entry.metadata = Some(ModelMetadata {
                        context_window: price.context_window,
                        max_output_tokens: price.max_output_tokens,
                        supports_vision: price.supports_vision_bool().unwrap_or(false),
                        supports_function_calling: price.supports_function_calling_bool().unwrap_or(false),
                        supports_streaming: true,
                        provider: None,
                        family: None,
                        release_date: None,
                    });
                }
            }

            // Add tiered pricing
            for tier in &tiered_prices {
                let entry = models.entry(tier.model.clone()).or_insert_with(|| {
                    ModelPricing {
                        pricing: HashMap::new(),
                        tiered_pricing: None,
                        cache_pricing: None,
                        batch_pricing: None,
                        metadata: None,
                    }
                });

                let tiered_map = entry.tiered_pricing.get_or_insert_with(HashMap::new);
                let currency = if tier.region.as_deref() == Some("cn") {
                    "CNY".to_string()
                } else {
                    "USD".to_string()
                };

                let tiers = tiered_map.entry(currency).or_insert_with(Vec::new);
                tiers.push(TieredPriceConfig {
                    tier_start: tier.tier_start,
                    tier_end: tier.tier_end,
                    input_price: tier.input_price,
                    output_price: tier.output_price,
                });
            }

            // Sort tiers by tier_start
            for model_pricing in models.values_mut() {
                if let Some(ref mut tiered_map) = model_pricing.tiered_pricing {
                    for tiers in tiered_map.values_mut() {
                        tiers.sort_by_key(|t| t.tier_start);
                    }
                }
            }

            let config = PricingConfig {
                version: "1.0".to_string(),
                updated_at: Utc::now(),
                source: "export".to_string(),
                models,
            };

            let output = if format == "json" {
                config.to_json()?
            } else {
                // CSV format
                let mut csv = String::from("model,currency,input_price,output_price,cache_read_price,batch_input_price,region\n");
                for (model, pricing) in &config.models {
                    for (currency, price) in &pricing.pricing {
                        csv.push_str(&format!(
                            "{},{},{},{},{},{},\n",
                            model,
                            currency,
                            price.input_price,
                            price.output_price,
                            pricing.cache_pricing
                                .as_ref()
                                .and_then(|c| c.get(currency))
                                .map(|c| c.cache_read_input_price.to_string())
                                .unwrap_or_default(),
                            pricing.batch_pricing
                                .as_ref()
                                .and_then(|b| b.get(currency))
                                .map(|b| b.batch_input_price.to_string())
                                .unwrap_or_default(),
                        ));
                    }
                }
                csv
            };

            std::fs::write(Path::new(file_path), &output)?;
            println!("✓ Exported {} models to '{}'", config.models.len(), file_path);
        }
        Some(("validate", sub_m)) => {
            let file_path = sub_m.get_one::<String>("file").unwrap();

            let content = std::fs::read_to_string(Path::new(file_path))?;
            let config = match PricingConfig::from_json(&content) {
                Ok(c) => c,
                Err(e) => {
                    println!("❌ JSON parsing error: {}", e);
                    return Ok(());
                }
            };

            println!("Validating pricing configuration: {}", file_path);
            println!("{}", "=".repeat(50));
            println!("Version: {}", config.version);
            println!("Source: {}", config.source);
            println!("Models: {}", config.models.len());
            println!();

            match config.validate() {
                Ok(warnings) => {
                    if warnings.is_empty() {
                        println!("✅ Configuration is valid with no warnings.");
                    } else {
                        println!("✅ Configuration is valid with {} warning(s):", warnings.len());
                        println!();
                        for warning in &warnings {
                            println!("  Field: {}", warning.field);
                            println!("  Message: {}", warning.message);
                            println!("  Suggestion: {}", warning.suggestion);
                            println!();
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Configuration is invalid:");
                    println!("  {}", e);
                }
            }
        }
        _ => {
            println!("Usage: burncloud price <list|set|get|delete|import|export|validate|sync-status>");
            println!("Run 'burncloud price --help' for more information.");
        }
    }

    Ok(())
}

/// Handle tiered pricing commands
pub async fn handle_tiered_command(db: &Database, matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list-tiers", sub_m)) => {
            let model = sub_m.get_one::<String>("model").unwrap();
            let region = sub_m.get_one::<String>("region").map(|s| s.as_str());

            let tiers = TieredPriceModel::get_tiers(db, model, region).await?;

            if tiers.is_empty() {
                println!("No tiered pricing found for model '{}'", model);
                return Ok(());
            }

            println!("Tiered pricing for '{}':", model);
            println!(
                "{:<15} {:<15} {:>15} {:>15} {:>10}",
                "Region", "Tier Start", "Tier End", "Input ($/1M)", "Output ($/1M)"
            );
            println!("{}", "-".repeat(72));

            for tier in tiers {
                let region = tier.region.as_deref().unwrap_or("-");
                let tier_end = tier.tier_end.map_or("∞".to_string(), |e| format!("{}", e));
                // Convert nanodollars to dollars for display
                let input_dollars = from_nano(tier.input_price);
                let output_dollars = from_nano(tier.output_price);
                println!(
                    "{:<15} {:<15} {:>15} {:>15.4} {:>15.4}",
                    region, tier.tier_start, tier_end, input_dollars, output_dollars
                );
            }
        }
        Some(("add-tier", sub_m)) => {
            let model = sub_m.get_one::<String>("model").unwrap().to_string();
            let region = sub_m.get_one::<String>("region").cloned();
            let tier_start: i64 = sub_m
                .get_one::<String>("tier-start")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let tier_end: Option<i64> = sub_m
                .get_one::<String>("tier-end")
                .and_then(|s| s.parse().ok());
            let input_price: f64 = sub_m
                .get_one::<String>("input-price")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let output_price: f64 = sub_m
                .get_one::<String>("output-price")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);

            // Convert to i64 nanodollars for storage
            let input = TieredPriceInput {
                model: model.clone(),
                region,
                tier_start,
                tier_end,
                input_price: to_nano(input_price),
                output_price: to_nano(output_price),
            };

            TieredPriceModel::upsert_tier(db, &input).await?;
            println!(
                "✓ Tier added for '{}': {}-{} tokens at ${:.4}/${:.4} per 1M",
                model,
                tier_start,
                tier_end.map_or("∞".to_string(), |e| format!("{}", e)),
                input_price,
                output_price
            );
        }
        Some(("import-tiered", sub_m)) => {
            let file_path = sub_m.get_one::<String>("file").unwrap();

            let content = std::fs::read_to_string(Path::new(file_path))?;
            let tiers: Vec<TieredPriceInput> = serde_json::from_str(&content)?;

            let mut count = 0;
            for tier in &tiers {
                match TieredPriceModel::upsert_tier(db, tier).await {
                    Ok(_) => count += 1,
                    Err(e) => eprintln!("Failed to import tier for {}: {}", tier.model, e),
                }
            }

            println!("✓ Imported {} tiered pricing entries", count);
        }
        Some(("delete-tiers", sub_m)) => {
            let model = sub_m.get_one::<String>("model").unwrap();
            let region = sub_m.get_one::<String>("region").map(|s| s.as_str());

            TieredPriceModel::delete_tiers(db, model, region).await?;
            println!("✓ Deleted tiered pricing for '{}'", model);
        }
        Some(("check-tiered", sub_m)) => {
            let model = sub_m.get_one::<String>("model").unwrap();

            let has_tiered = TieredPriceModel::has_tiered_pricing(db, model).await?;

            if has_tiered {
                println!("✓ Model '{}' has tiered pricing configured", model);

                // Show the tiers
                let tiers = TieredPriceModel::get_tiers(db, model, None).await?;
                if !tiers.is_empty() {
                    println!("\nTier configuration:");
                    for tier in tiers {
                        let region = tier.region.as_deref().unwrap_or("universal");
                        let tier_end = tier.tier_end.map_or("∞".to_string(), |e| format!("{}", e));
                        // Convert nanodollars to dollars for display
                        let input_dollars = from_nano(tier.input_price);
                        let output_dollars = from_nano(tier.output_price);
                        println!(
                            "  [{}] {}-{} tokens: ${:.4}/${:.4} per 1M (input/output)",
                            region, tier.tier_start, tier_end, input_dollars, output_dollars
                        );
                    }
                }
            } else {
                println!("✗ Model '{}' does not have tiered pricing configured", model);
                println!("  This model will use standard per-token pricing.");
            }
        }
        _ => {
            println!("Usage: burncloud tiered <list-tiers|add-tier|import-tiered|delete-tiers|check-tiered>");
            println!("Run 'burncloud tiered --help' for more information.");
        }
    }

    Ok(())
}
