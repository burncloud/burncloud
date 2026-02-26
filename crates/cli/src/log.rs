use anyhow::Result;
use burncloud_common::scaled_to_rate;
use burncloud_database::sqlx;
use burncloud_database::Database;
use burncloud_database_router::{get_usage_stats, DbRouterLog, RouterDatabase};
use clap::ArgMatches;
use serde::Serialize;

/// Log list item for JSON output
#[derive(Debug, Clone, Serialize)]
pub struct LogListItem {
    pub id: i64,
    pub request_id: String,
    pub user_id: Option<String>,
    pub channel_id: Option<String>,
    pub model: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    /// Cost in dollars (converted from nanodollars)
    pub cost: f64,
    pub timestamp: Option<String>,
}

impl From<DbRouterLog> for LogListItem {
    fn from(log: DbRouterLog) -> Self {
        // Extract model from path if possible (e.g., "/v1/chat/completions" -> "N/A")
        // In most cases, model is in request body, not path
        let model = extract_model_from_path(&log.path);

        // Convert nanodollars to dollars
        let cost = log.cost as f64 / 1_000_000_000.0;

        LogListItem {
            id: log.id,
            request_id: log.request_id,
            user_id: log.user_id,
            channel_id: log.upstream_id,
            model,
            prompt_tokens: log.prompt_tokens,
            completion_tokens: log.completion_tokens,
            cost,
            timestamp: log.created_at,
        }
    }
}

/// Try to extract model name from API path
fn extract_model_from_path(path: &str) -> String {
    // Common patterns:
    // /v1/chat/completions -> N/A (model in body)
    // /v1/models/{model} -> {model}
    // /v1/engines/{model}/completions -> {model}

    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // Look for model after /v1/models/ or /v1/engines/
    for i in 0..parts.len().saturating_sub(1) {
        if parts[i] == "models" || parts[i] == "engines" {
            if i + 1 < parts.len() {
                return parts[i + 1].to_string();
            }
        }
    }

    // If no model found in path, return N/A
    "N/A".to_string()
}

/// Handle log list command
pub async fn cmd_log_list(db: &Database, matches: &ArgMatches) -> Result<()> {
    let user_id = matches.get_one::<String>("user-id").map(|s| s.as_str());
    let channel_id = matches.get_one::<String>("channel-id").map(|s| s.as_str());
    let model = matches.get_one::<String>("model").map(|s| s.as_str());
    let limit: i32 = matches
        .get_one::<String>("limit")
        .unwrap()
        .parse()
        .unwrap_or(100);
    let offset: i32 = matches
        .get_one::<String>("offset")
        .unwrap()
        .parse()
        .unwrap_or(0);
    let format = matches
        .get_one::<String>("format")
        .map(|s| s.as_str())
        .unwrap_or("table");

    // Fetch logs with optional filtering
    let logs = RouterDatabase::get_logs_filtered(db, user_id, channel_id, model, limit, offset)
        .await?;

    if logs.is_empty() {
        println!("No logs found");
        return Ok(());
    }

    // Convert to list items
    let list_items: Vec<LogListItem> = logs.into_iter().map(LogListItem::from).collect();

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&list_items)?;
            println!("{}", json);
        }
        _ => {
            // Table format
            println!(
                "{:<8} {:<36} {:<36} {:<20} {:<15} {:<15} {:<12} {:<20}",
                "ID", "UserID", "ChannelID", "Model", "PromptTokens", "CompletionTokens", "Cost", "Timestamp"
            );
            println!("{}", "-".repeat(170));
            for item in &list_items {
                let user_id = item.user_id.as_deref().unwrap_or("N/A");
                let channel_id = item.channel_id.as_deref().unwrap_or("N/A");
                let timestamp = item.timestamp.as_deref().unwrap_or("N/A");
                println!(
                    "{:<8} {:<36} {:<36} {:<20} {:<15} {:<15} ${:<11.6} {:<20}",
                    item.id,
                    user_id,
                    channel_id,
                    item.model,
                    item.prompt_tokens,
                    item.completion_tokens,
                    item.cost,
                    timestamp
                );
            }
        }
    }

    Ok(())
}

/// Route log commands
pub async fn handle_log_command(db: &Database, matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", sub_m)) => {
            cmd_log_list(db, sub_m).await?;
        }
        Some(("usage", sub_m)) => {
            cmd_log_usage(db, sub_m).await?;
        }
        _ => {
            println!("Usage: burncloud log <list|usage>");
            println!("Run 'burncloud log --help' for more information.");
        }
    }

    Ok(())
}

/// Handle log usage command
pub async fn cmd_log_usage(db: &Database, matches: &ArgMatches) -> Result<()> {
    let user_id = matches
        .get_one::<String>("user-id")
        .map(|s| s.as_str())
        .ok_or_else(|| anyhow::anyhow!("--user-id is required"))?;

    let period = matches
        .get_one::<String>("period")
        .map(|s| s.as_str())
        .unwrap_or("month");

    // Validate period
    if !matches!(period, "day" | "week" | "month") {
        return Err(anyhow::anyhow!(
            "Invalid period '{}'. Must be: day, week, or month",
            period
        ));
    }

    let format = matches
        .get_one::<String>("format")
        .map(|s| s.as_str())
        .unwrap_or("table");

    // Get usage stats from database
    let stats = get_usage_stats(db, user_id, period).await?;

    // Get USD to CNY exchange rate for cost conversion
    let cny_rate = get_usd_to_cny_rate(db).await?;

    // Convert cost from nanodollars to dollars
    let cost_usd = stats.total_cost_nano as f64 / 1_000_000_000.0;
    let cost_cny = cost_usd * cny_rate;

    match format {
        "json" => {
            let output = UsageOutput {
                user_id: user_id.to_string(),
                period: period.to_string(),
                total_requests: stats.total_requests,
                total_prompt_tokens: stats.total_prompt_tokens,
                total_completion_tokens: stats.total_completion_tokens,
                cost_usd,
                cost_cny,
            };
            let json = serde_json::to_string_pretty(&output)?;
            println!("{}", json);
        }
        _ => {
            // Table format
            let period_label = match period {
                "day" => "Last 24 Hours",
                "week" => "Last 7 Days",
                "month" => "Last 30 Days",
                _ => "Last 30 Days",
            };

            println!("Usage Statistics for User: {}", user_id);
            println!("Period: {}", period_label);
            println!("{}", "=".repeat(50));
            println!();
            println!("  Total Requests:        {}", stats.total_requests);
            println!("  Total Prompt Tokens:   {}", stats.total_prompt_tokens);
            println!(
                "  Total Completion Tokens: {}",
                stats.total_completion_tokens
            );
            println!("  Total Tokens:           {}",
                stats.total_prompt_tokens + stats.total_completion_tokens);
            println!();
            println!("  Cost (USD):            ${:.6}", cost_usd);
            println!("  Cost (CNY):            ¥{:.6}", cost_cny);
            println!();
        }
    }

    Ok(())
}

/// Output structure for usage command (JSON format)
#[derive(Debug, Clone, Serialize)]
pub struct UsageOutput {
    pub user_id: String,
    pub period: String,
    pub total_requests: i64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub cost_usd: f64,
    pub cost_cny: f64,
}

/// Get USD to CNY exchange rate from database
async fn get_usd_to_cny_rate(db: &Database) -> Result<f64> {
    let conn = db.get_connection()?;

    let sql = "SELECT rate FROM exchange_rates WHERE from_currency = ? AND to_currency = ?";

    let rate_nano: Option<i64> = sqlx::query_scalar(sql)
        .bind("USD")
        .bind("CNY")
        .fetch_optional(conn.pool())
        .await?;

    if let Some(rate) = rate_nano {
        return Ok(scaled_to_rate(rate));
    }

    // Try reverse rate
    let reverse_rate_nano: Option<i64> = sqlx::query_scalar(sql)
        .bind("CNY")
        .bind("USD")
        .fetch_optional(conn.pool())
        .await?;

    if let Some(reverse_rate) = reverse_rate_nano {
        let reverse = scaled_to_rate(reverse_rate);
        if reverse > 0.0 {
            return Ok(1.0 / reverse);
        }
    }

    // Default rate if not found
    Ok(7.24)
}
