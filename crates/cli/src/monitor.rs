//! Monitor status CLI commands
//!
//! This module provides CLI commands for system monitoring:
//! - status: Show system status and metrics

use anyhow::Result;
use burncloud_database::{Database, sqlx};
use burncloud_database_models::ChannelModel;
use clap::ArgMatches;
use serde::Serialize;

/// System status output for JSON format
#[derive(Debug, Clone, Serialize)]
pub struct SystemStatus {
    pub total_channels: usize,
    pub active_channels: usize,
    pub today_requests: i64,
    pub today_tokens: i64,
    /// Revenue in dollars (converted from nanodollars)
    pub today_revenue_usd: f64,
}

/// Handle monitor status command
pub async fn cmd_monitor_status(db: &Database, matches: &ArgMatches) -> Result<()> {
    let format = matches
        .get_one::<String>("format")
        .map(|s| s.as_str())
        .unwrap_or("table");

    // Get channel counts
    let channels = ChannelModel::list(db, 10000, 0).await?;
    let total_channels = channels.len();
    let active_channels = channels.iter().filter(|c| c.status == 1).count();

    // Get today's statistics from router_logs
    let (today_requests, today_tokens, today_revenue_nano) =
        get_today_stats(db).await?;

    // Convert nanodollars to dollars
    let today_revenue_usd = today_revenue_nano as f64 / 1_000_000_000.0;

    let status = SystemStatus {
        total_channels,
        active_channels,
        today_requests,
        today_tokens,
        today_revenue_usd,
    };

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&status)?;
            println!("{}", json);
        }
        _ => {
            // Table format
            println!("System Status");
            println!("{}", "=".repeat(50));
            println!();
            println!("  Channels:");
            println!("    Total:      {}", status.total_channels);
            println!("    Active:     {}", status.active_channels);
            println!("    Inactive:   {}", status.total_channels - status.active_channels);
            println!();
            println!("  Today's Statistics:");
            println!("    Requests:   {}", status.today_requests);
            println!("    Tokens:     {}", status.today_tokens);
            println!("    Revenue:    ${:.6}", status.today_revenue_usd);
            println!();
        }
    }

    Ok(())
}

/// Get today's statistics from router_logs
///
/// Returns (requests, tokens, revenue_nano)
async fn get_today_stats(db: &Database) -> Result<(i64, i64, i64)> {
    let conn = db.get_connection()?;
    let is_postgres = db.kind() == "postgres";

    // Calculate today's start timestamp (midnight UTC)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("Time error: {}", e))?
        .as_secs() as i64;

    // Get start of today (midnight UTC)
    let today_start = now - (now % 86400);

    let sql = if is_postgres {
        r#"
        SELECT
            COUNT(*) as requests,
            COALESCE(SUM(prompt_tokens + completion_tokens), 0) as tokens,
            COALESCE(SUM(cost), 0) as revenue
        FROM router_logs
        WHERE created_at IS NOT NULL AND CAST(created_at AS BIGINT) >= $1
        "#
    } else {
        r#"
        SELECT
            COUNT(*) as requests,
            COALESCE(SUM(prompt_tokens + completion_tokens), 0) as tokens,
            COALESCE(SUM(cost), 0) as revenue
        FROM router_logs
        WHERE created_at IS NOT NULL AND CAST(created_at AS INTEGER) >= ?
        "#
    };

    let row: (Option<i64>, Option<i64>, Option<i64>) = sqlx::query_as(sql)
        .bind(today_start.to_string())
        .fetch_one(conn.pool())
        .await?;

    Ok((
        row.0.unwrap_or(0),
        row.1.unwrap_or(0),
        row.2.unwrap_or(0),
    ))
}

/// Route monitor commands
pub async fn handle_monitor_command(db: &Database, matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("status", sub_m)) => {
            cmd_monitor_status(db, sub_m).await?;
        }
        _ => {
            println!("Usage: burncloud monitor <status>");
            println!("Run 'burncloud monitor --help' for more information.");
        }
    }

    Ok(())
}
