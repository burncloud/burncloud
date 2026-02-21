//! Currency and exchange rate management CLI commands
//!
//! This module provides CLI commands for managing exchange rates and currency conversion.

use anyhow::Result;
use burncloud_common::Currency;
use burncloud_database::sqlx;
use burncloud_database::Database;
use clap::ArgMatches;
use std::str::FromStr;

/// Handle currency subcommands
pub async fn handle_currency_command(db: &Database, matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list-rates", _)) => cmd_list_rates(db).await,
        Some(("set-rate", sub_m)) => {
            let from = sub_m.get_one::<String>("from").unwrap();
            let to = sub_m.get_one::<String>("to").unwrap();
            let rate: f64 = sub_m
                .get_one::<String>("rate")
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| anyhow::anyhow!("Invalid rate value"))?;

            cmd_set_rate(db, from, to, rate).await
        }
        Some(("refresh", _)) => cmd_refresh_rates(db).await,
        Some(("convert", sub_m)) => {
            let amount: f64 = sub_m
                .get_one::<String>("amount")
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| anyhow::anyhow!("Invalid amount value"))?;
            let from = sub_m.get_one::<String>("from").unwrap();
            let to = sub_m.get_one::<String>("to").unwrap();

            cmd_convert(db, amount, from, to).await
        }
        _ => {
            println!("Unknown currency subcommand. Use --help for usage.");
            Ok(())
        }
    }
}

/// List all exchange rates
async fn cmd_list_rates(db: &Database) -> Result<()> {
    use burncloud_database::sqlx;

    let conn = db.get_connection()?;
    let sql = "SELECT from_currency, to_currency, rate, updated_at FROM exchange_rates ORDER BY from_currency, to_currency";

    let rows = sqlx::query_as::<_, (String, String, f64, Option<i64>)>(sql)
        .fetch_all(conn.pool())
        .await?;

    if rows.is_empty() {
        println!("No exchange rates configured.");
        println!();
        println!("Use 'burncloud currency set-rate' to add exchange rates.");
        println!("Example: burncloud currency set-rate --from USD --to CNY --rate 7.2");
        return Ok(());
    }

    println!("{:<15} {:<15} {:>15} {:>20}", "From", "To", "Rate", "Updated");
    println!("{}", "-".repeat(70));

    for (from, to, rate, updated_at) in rows {
        let updated = updated_at
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
            })
            .unwrap_or_else(|| "-".to_string());

        println!("{:<15} {:<15} {:>15.6} {:>20}", from, to, rate, updated);
    }

    Ok(())
}

/// Set an exchange rate
async fn cmd_set_rate(db: &Database, from: &str, to: &str, rate: f64) -> Result<()> {
    // Validate currencies
    let from_currency = Currency::from_str(from)
        .map_err(|e| anyhow::anyhow!("Invalid 'from' currency: {}", e))?;
    let to_currency = Currency::from_str(to)
        .map_err(|e| anyhow::anyhow!("Invalid 'to' currency: {}", e))?;

    if rate <= 0.0 {
        return Err(anyhow::anyhow!("Rate must be positive"));
    }

    let conn = db.get_connection()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("Time error: {}", e))?
        .as_secs() as i64;

    let sql = match db.kind().as_str() {
        "postgres" => r#"
            INSERT INTO exchange_rates (from_currency, to_currency, rate, updated_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT(from_currency, to_currency) DO UPDATE SET
                rate = EXCLUDED.rate,
                updated_at = EXCLUDED.updated_at
        "#,
        _ => r#"
            INSERT INTO exchange_rates (from_currency, to_currency, rate, updated_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(from_currency, to_currency) DO UPDATE SET
                rate = excluded.rate,
                updated_at = excluded.updated_at
        "#,
    };

    sqlx::query(sql)
        .bind(from_currency.code())
        .bind(to_currency.code())
        .bind(rate)
        .bind(now)
        .execute(conn.pool())
        .await?;

    println!(
        "✓ Exchange rate set: {} → {} = {:.6}",
        from_currency, to_currency, rate
    );
    println!();
    println!("Note: Remember to also set the reverse rate if needed.");
    println!("  Example: burncloud currency set-rate --from {} --to {} --rate {:.6}",
        to_currency, from_currency, 1.0 / rate);

    Ok(())
}

/// Refresh exchange rates (placeholder for external API integration)
async fn cmd_refresh_rates(_db: &Database) -> Result<()> {
    println!("Exchange rate refresh is not yet implemented.");
    println!();
    println!("Future versions will support automatic rate updates from:");
    println!("  - ExchangeRate-API (exchangerate-api.com)");
    println!("  - Fixer.io");
    println!();
    println!("For now, manually set rates using:");
    println!("  burncloud currency set-rate --from USD --to CNY --rate <value>");

    Ok(())
}

/// Convert an amount between currencies
async fn cmd_convert(db: &Database, amount: f64, from: &str, to: &str) -> Result<()> {
    let from_currency = Currency::from_str(from)
        .map_err(|e| anyhow::anyhow!("Invalid 'from' currency: {}", e))?;
    let to_currency = Currency::from_str(to)
        .map_err(|e| anyhow::anyhow!("Invalid 'to' currency: {}", e))?;

    // Simple direct lookup for conversion
    let conn = db.get_connection()?;
    let sql = "SELECT rate FROM exchange_rates WHERE from_currency = ? AND to_currency = ?";

    let rate: Option<f64> = sqlx::query_scalar(sql)
        .bind(from_currency.code())
        .bind(to_currency.code())
        .fetch_optional(conn.pool())
        .await?;

    let converted = if let Some(r) = rate {
        amount * r
    } else {
        // Try reverse rate
        let sql = "SELECT rate FROM exchange_rates WHERE from_currency = ? AND to_currency = ?";
        let reverse_rate: Option<f64> = sqlx::query_scalar(sql)
            .bind(to_currency.code())
            .bind(from_currency.code())
            .fetch_optional(conn.pool())
            .await?;

        if let Some(rr) = reverse_rate {
            if rr > 0.0 {
                amount / rr
            } else {
                amount
            }
        } else if from_currency == to_currency {
            amount
        } else {
            println!("Warning: No exchange rate found for {} → {}", from_currency, to_currency);
            amount
        }
    };

    // Get symbol for display
    let from_symbol = from_currency.symbol();
    let to_symbol = to_currency.symbol();

    println!(
        "{} {:.2} = {} {:.6}",
        from_symbol, amount, to_symbol, converted
    );

    // Show rate if available
    if let Some(r) = rate {
        println!("(Rate: {} → {} = {:.6})", from_currency, to_currency, r);
    } else if from_currency == to_currency {
        println!("(Same currency, no conversion needed)");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_currency_parsing() {
        use burncloud_common::Currency;
        use std::str::FromStr;

        assert_eq!(Currency::from_str("USD").unwrap(), Currency::USD);
        assert_eq!(Currency::from_str("CNY").unwrap(), Currency::CNY);
        assert_eq!(Currency::from_str("EUR").unwrap(), Currency::EUR);
        assert!(Currency::from_str("GBP").is_err());
    }
}
