use anyhow::Result;
use burncloud_database::Database;
use burncloud_database_models::{
    PriceInput, PriceModel, TieredPriceInput, TieredPriceModel,
};
use clap::ArgMatches;
use std::path::Path;

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

            let prices = PriceModel::list(db, limit, offset).await?;

            if prices.is_empty() {
                println!("No prices found.");
                return Ok(());
            }

            println!(
                "{:<30} {:>15} {:>15} {:>10}",
                "Model", "Input ($/1M)", "Output ($/1M)", "Alias"
            );
            println!("{}", "-".repeat(72));

            for price in prices {
                let alias = price.alias_for.as_deref().unwrap_or("-");
                println!(
                    "{:<30} {:>15.4} {:>15.4} {:>10}",
                    price.model, price.input_price, price.output_price, alias
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
            let alias_for = sub_m.get_one::<String>("alias").cloned();

            let input = PriceInput {
                model: model.clone(),
                input_price,
                output_price,
                currency: Some("USD".to_string()),
                alias_for,
                // Advanced pricing fields - set to None for manual CLI entry
                cache_read_price: None,
                cache_creation_price: None,
                batch_input_price: None,
                batch_output_price: None,
                priority_input_price: None,
                priority_output_price: None,
                audio_input_price: None,
                full_pricing: None,
            };

            PriceModel::upsert(db, &input).await?;
            println!(
                "✓ Price set for '{}': input=${:.4}/1M, output=${:.4}/1M",
                model, input_price, output_price
            );
        }
        Some(("delete", sub_m)) => {
            let model = sub_m.get_one::<String>("model").unwrap();

            PriceModel::delete(db, model).await?;
            println!("✓ Price deleted for '{}'", model);
        }
        Some(("get", sub_m)) => {
            let model = sub_m.get_one::<String>("model").unwrap();

            match PriceModel::get(db, model).await? {
                Some(price) => {
                    println!("Model: {}", price.model);
                    println!("Input Price: ${:.4}/1M tokens", price.input_price);
                    println!("Output Price: ${:.4}/1M tokens", price.output_price);
                    if let Some(alias) = &price.alias_for {
                        println!("Alias For: {}", alias);
                    }
                    println!("Currency: {}", price.currency);
                    // Display advanced pricing if available
                    if let Some(cache_read) = price.cache_read_price {
                        println!("Cache Read Price: ${:.4}/1M tokens", cache_read);
                    }
                    if let Some(cache_creation) = price.cache_creation_price {
                        println!("Cache Creation Price: ${:.4}/1M tokens", cache_creation);
                    }
                    if let Some(batch_input) = price.batch_input_price {
                        println!("Batch Input Price: ${:.4}/1M tokens", batch_input);
                    }
                    if let Some(batch_output) = price.batch_output_price {
                        println!("Batch Output Price: ${:.4}/1M tokens", batch_output);
                    }
                    if let Some(priority_input) = price.priority_input_price {
                        println!("Priority Input Price: ${:.4}/1M tokens", priority_input);
                    }
                    if let Some(priority_output) = price.priority_output_price {
                        println!("Priority Output Price: ${:.4}/1M tokens", priority_output);
                    }
                    if let Some(audio_input) = price.audio_input_price {
                        println!("Audio Input Price: ${:.4}/1M tokens", audio_input);
                    }
                }
                None => {
                    println!("No price found for model '{}'", model);
                }
            }
        }
        _ => {
            println!("Usage: burncloud price <list|set|get|delete|list-tiers|add-tier|import-tiered>");
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
                println!(
                    "{:<15} {:<15} {:>15} {:>15.4} {:>15.4}",
                    region, tier.tier_start, tier_end, tier.input_price, tier.output_price
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

            let input = TieredPriceInput {
                model: model.clone(),
                region,
                tier_start,
                tier_end,
                input_price,
                output_price,
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
                        println!(
                            "  [{}] {}-{} tokens: ${:.4}/${:.4} per 1M (input/output)",
                            region, tier.tier_start, tier_end, tier.input_price, tier.output_price
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
