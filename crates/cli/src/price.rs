use anyhow::Result;
use burncloud_database::Database;
use burncloud_database_models::{PriceInput, PriceModel};
use clap::ArgMatches;

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
                    if let Some(alias) = price.alias_for {
                        println!("Alias For: {}", alias);
                    }
                    println!("Currency: {}", price.currency);
                }
                None => {
                    println!("No price found for model '{}'", model);
                }
            }
        }
        _ => {
            println!("Usage: burncloud price <list|set|get|delete>");
            println!("Run 'burncloud price --help' for more information.");
        }
    }

    Ok(())
}
