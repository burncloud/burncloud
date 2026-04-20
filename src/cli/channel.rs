//! Channel management CLI commands
//!
//! This module provides CLI commands for managing API channels:
//! - add: Create a new channel
//! - list: List all channels
//! - show: Show channel details
//! - update: Update a channel
//! - delete: Delete a channel

use anyhow::{anyhow, Result};
use burncloud_common::types::{Channel, ChannelType};
use burncloud_database::Database;
use burncloud_database_channel::ChannelProviderModel;
use clap::ArgMatches;
use std::io::{self, Write};

/// Parse channel type from string
///
/// Supports: openai, azure, anthropic, gemini, aws, vertexai, deepseek
pub fn parse_channel_type(s: &str) -> Result<ChannelType> {
    match s.to_lowercase().as_str() {
        "openai" => Ok(ChannelType::OpenAI),
        "azure" => Ok(ChannelType::Azure),
        "anthropic" => Ok(ChannelType::Anthropic),
        "gemini" => Ok(ChannelType::Gemini),
        "aws" => Ok(ChannelType::Aws),
        "vertexai" | "vertex" => Ok(ChannelType::VertexAi),
        "deepseek" => Ok(ChannelType::DeepSeek),
        "newapi" | "new-api" => Ok(ChannelType::NewApi),
        "volcengine" | "volc" => Ok(ChannelType::VolcEngine),
        "doubaovideo" | "doubao-video" | "seedance" => Ok(ChannelType::DoubaoVideo),
        _ => Err(anyhow!(
            "Unsupported channel type: '{}'. Supported types: openai, azure, anthropic, gemini, aws, vertexai, deepseek, newapi, volcengine, seedance",
            s
        )),
    }
}

/// Get default models for a channel type
pub fn get_default_models(channel_type: ChannelType) -> Vec<&'static str> {
    match channel_type {
        ChannelType::OpenAI => vec!["gpt-4", "gpt-4-turbo", "gpt-3.5-turbo"],
        ChannelType::Azure => vec!["gpt-4", "gpt-35-turbo"],
        ChannelType::Anthropic => vec!["claude-3-opus", "claude-3-sonnet", "claude-3-haiku"],
        ChannelType::Gemini => vec!["gemini-1.5-pro", "gemini-1.5-flash", "gemini-pro"],
        ChannelType::Aws => vec!["claude-3-sonnet", "claude-3-haiku"],
        ChannelType::VertexAi => vec!["gemini-1.5-pro"],
        ChannelType::DeepSeek => vec!["deepseek-chat", "deepseek-coder"],
        ChannelType::NewApi => vec![],
        ChannelType::VolcEngine => vec![],
        ChannelType::DoubaoVideo => vec![
            "doubao-seedance-2-0-260128",
            "doubao-seedance-2-0-fast-260128",
        ],
        _ => vec![],
    }
}

/// Get default base URL for a channel type
///
/// Returns None for types that require user-specified URLs (Azure, AWS, VertexAI)
pub fn get_default_base_url(channel_type: ChannelType) -> Option<&'static str> {
    match channel_type {
        ChannelType::OpenAI => Some("https://api.openai.com/v1"),
        ChannelType::Anthropic => Some("https://api.anthropic.com/v1"),
        ChannelType::Gemini => Some("https://generativelanguage.googleapis.com/v1beta"),
        ChannelType::DeepSeek => Some("https://api.deepseek.com/v1"),
        // Azure, AWS, VertexAI, NewApi require user-specified URLs
        ChannelType::Azure | ChannelType::Aws | ChannelType::VertexAi => None,
        ChannelType::NewApi => None,
        ChannelType::VolcEngine => Some("https://ark.cn-beijing.volces.com/api/v3"),
        ChannelType::DoubaoVideo => Some("https://ark.cn-beijing.volces.com/api/v3"),
        _ => None,
    }
}

/// Get human-readable name for channel type
pub fn get_channel_type_name(channel_type: ChannelType) -> &'static str {
    match channel_type {
        ChannelType::OpenAI => "OpenAI",
        ChannelType::Azure => "Azure",
        ChannelType::Anthropic => "Anthropic",
        ChannelType::Gemini => "Gemini",
        ChannelType::Aws => "AWS",
        ChannelType::VertexAi => "VertexAI",
        ChannelType::DeepSeek => "DeepSeek",
        ChannelType::NewApi => "NewApi",
        ChannelType::VolcEngine => "VolcEngine",
        ChannelType::DoubaoVideo => "DoubaoVideo",
        _ => "Unknown",
    }
}

/// Get default channel name based on type
pub fn get_default_channel_name(channel_type: ChannelType) -> String {
    format!("{} Channel", get_channel_type_name(channel_type))
}

/// Handle channel add command
pub async fn cmd_channel_add(db: &Database, args: &ArgMatches) -> Result<()> {
    // Parse channel type
    let type_str = args
        .get_one::<String>("type")
        .ok_or_else(|| anyhow!("Channel type is required. Use -t or --type to specify."))?;
    let channel_type = parse_channel_type(type_str)?;

    // Get API key
    let key = args
        .get_one::<String>("key")
        .ok_or_else(|| anyhow!("API key is required. Use -k or --key to specify."))?;
    if key.is_empty() {
        return Err(anyhow!("API key cannot be empty"));
    }

    // Get models (use defaults if not provided)
    let models = if let Some(models_str) = args.get_one::<String>("models") {
        models_str.clone()
    } else {
        get_default_models(channel_type).join(",")
    };

    // Get base URL (use default if not provided)
    let base_url = if let Some(url) = args.get_one::<String>("url") {
        Some(url.clone())
    } else {
        // Azure requires explicit URL
        if channel_type == ChannelType::Azure {
            return Err(anyhow!(
                "Azure channel requires a custom base URL. Use -u or --url to specify your Azure endpoint."
            ));
        }
        get_default_base_url(channel_type).map(|s| s.to_string())
    };

    // Get channel name (use default if not provided)
    let name = if let Some(n) = args.get_one::<String>("name") {
        n.clone()
    } else {
        get_default_channel_name(channel_type)
    };

    // Get pricing region (optional)
    let pricing_region = args.get_one::<String>("pricing-region").cloned();

    // Build Channel struct
    let mut channel = Channel {
        id: 0,
        type_: channel_type as i32,
        key: key.clone(),
        status: 1, // Enabled
        name,
        weight: 1,
        created_time: None,
        test_time: None,
        response_time: None,
        base_url,
        models,
        group: "default".to_string(),
        used_quota: 0,
        model_mapping: None,
        priority: 0,
        auto_ban: 1,
        other_info: None,
        tag: None,
        setting: None,
        param_override: None,
        header_override: None,
        remark: None,
        api_version: Some("default".to_string()),
        pricing_region,
    };

    // Save to database
    let id = ChannelProviderModel::create(db, &mut channel).await?;
    println!("Channel created with ID: {}", id);

    Ok(())
}

/// Handle channel list command
pub async fn cmd_channel_list(db: &Database, args: &ArgMatches) -> Result<()> {
    let format = args
        .get_one::<String>("format")
        .map(|s| s.as_str())
        .unwrap_or("table");

    let channels = ChannelProviderModel::list(db, 100, 0).await?;

    if channels.is_empty() {
        println!("No channels found");
        return Ok(());
    }

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&channels)?;
            println!("{}", json);
        }
        _ => {
            // Table format
            println!(
                "{:<5} {:<20} {:<12} {:<10} {:<40} {:<40}",
                "ID", "Name", "Type", "Status", "Models", "Base URL"
            );
            println!("{}", "-".repeat(130));
            for channel in channels {
                let type_name = get_channel_type_name(ChannelType::from(channel.type_));
                let status = if channel.status == 1 {
                    "Active"
                } else {
                    "Inactive"
                };
                let models_display = if channel.models.len() > 35 {
                    format!("{}...", &channel.models[..32])
                } else {
                    channel.models.clone()
                };
                let base_url_display = channel.base_url.as_deref().unwrap_or("N/A");
                let url_truncated = if base_url_display.len() > 35 {
                    format!("{}...", &base_url_display[..32])
                } else {
                    base_url_display.to_string()
                };
                println!(
                    "{:<5} {:<20} {:<12} {:<10} {:<40} {:<40}",
                    channel.id, channel.name, type_name, status, models_display, url_truncated
                );
            }
        }
    }

    Ok(())
}

/// Handle channel show command
pub async fn cmd_channel_show(db: &Database, args: &ArgMatches) -> Result<()> {
    let id: i32 = args
        .get_one::<String>("id")
        .ok_or_else(|| anyhow!("Channel ID is required"))?
        .parse()?;

    let channel = ChannelProviderModel::get_by_id(db, id)
        .await?
        .ok_or_else(|| anyhow!("Channel with ID {} not found", id))?;

    // Mask the API key (show only first 8 characters)
    let masked_key = if channel.key.len() > 8 {
        format!("{}{}", &channel.key[..8], "*".repeat(channel.key.len() - 8))
    } else {
        channel.key.clone()
    };

    let type_name = get_channel_type_name(ChannelType::from(channel.type_));
    let status = if channel.status == 1 {
        "Active"
    } else {
        "Inactive"
    };

    println!("Channel Details:");
    println!("  ID:          {}", channel.id);
    println!("  Name:        {}", channel.name);
    println!("  Type:        {}", type_name);
    println!("  Status:      {}", status);
    println!("  Key:         {}", masked_key);
    println!("  Models:      {}", channel.models);
    println!(
        "  Base URL:    {}",
        channel.base_url.as_deref().unwrap_or("N/A")
    );
    println!("  Group:       {}", channel.group);
    println!("  Priority:    {}", channel.priority);
    println!("  Weight:      {}", channel.weight);
    println!("  Used Quota:  {}", channel.used_quota);

    Ok(())
}

/// Handle channel update command
pub async fn cmd_channel_update(db: &Database, args: &ArgMatches) -> Result<()> {
    let id: i32 = args
        .get_one::<String>("id")
        .ok_or_else(|| anyhow!("Channel ID is required"))?
        .parse()?;

    // Get existing channel
    let mut channel = ChannelProviderModel::get_by_id(db, id)
        .await?
        .ok_or_else(|| anyhow!("Channel with ID {} not found", id))?;

    // Update fields only if provided
    if let Some(name) = args.get_one::<String>("name") {
        channel.name = name.clone();
    }

    if let Some(key) = args.get_one::<String>("key") {
        channel.key = key.clone();
    }

    if let Some(status_str) = args.get_one::<String>("status") {
        channel.status = status_str
            .parse()
            .map_err(|e| anyhow!("Invalid status value '{}': {}", status_str, e))?;
    }

    if let Some(models) = args.get_one::<String>("models") {
        channel.models = models.clone();
    }

    if let Some(priority_str) = args.get_one::<String>("priority") {
        channel.priority = priority_str
            .parse()
            .map_err(|e| anyhow!("Invalid priority value '{}': {}", priority_str, e))?;
    }

    if let Some(weight_str) = args.get_one::<String>("weight") {
        channel.weight = weight_str
            .parse()
            .map_err(|e| anyhow!("Invalid weight value '{}': {}", weight_str, e))?;
    }

    if let Some(base_url) = args.get_one::<String>("base-url") {
        channel.base_url = Some(base_url.clone());
    }

    if let Some(pricing_region) = args.get_one::<String>("pricing-region") {
        channel.pricing_region = Some(pricing_region.clone());
    }

    // Save updates
    ChannelProviderModel::update(db, &channel).await?;

    println!("Channel {} updated successfully", id);

    Ok(())
}

/// Handle channel delete command
pub async fn cmd_channel_delete(db: &Database, args: &ArgMatches) -> Result<()> {
    let id: i32 = args
        .get_one::<String>("id")
        .ok_or_else(|| anyhow!("Channel ID is required"))?
        .parse()?;

    let skip_confirm = args.get_flag("yes");

    // Check if channel exists
    let channel = ChannelProviderModel::get_by_id(db, id)
        .await?
        .ok_or_else(|| anyhow!("Channel with ID {} not found", id))?;

    // Confirm deletion
    if !skip_confirm {
        print!("Delete channel '{}' (ID: {})? [y/N] ", channel.name, id);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("Operation cancelled");
            return Ok(());
        }
    }

    // Delete channel
    ChannelProviderModel::delete(db, id).await?;
    println!("Channel {} deleted", id);

    Ok(())
}

/// Handle channel command routing
pub async fn handle_channel_command(db: &Database, matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("add", sub_m)) => cmd_channel_add(db, sub_m).await,
        Some(("list", sub_m)) => cmd_channel_list(db, sub_m).await,
        Some(("show", sub_m)) => cmd_channel_show(db, sub_m).await,
        Some(("update", sub_m)) => cmd_channel_update(db, sub_m).await,
        Some(("delete", sub_m)) => cmd_channel_delete(db, sub_m).await,
        _ => {
            println!("Channel management commands:");
            println!("  add     Add a new channel");
            println!("  list    List all channels");
            println!("  show    Show channel details");
            println!("  update  Update a channel");
            println!("  delete  Delete a channel");
            println!("\nRun 'burncloud channel <command> --help' for more information.");
            Ok(())
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_channel_type() {
        assert_eq!(
            parse_channel_type("openai").unwrap_or_else(|e| panic!("openai should parse: {e}")),
            ChannelType::OpenAI
        );
        assert_eq!(
            parse_channel_type("OpenAI").unwrap_or_else(|e| panic!("OpenAI should parse: {e}")),
            ChannelType::OpenAI
        );
        assert_eq!(
            parse_channel_type("OPENAI").unwrap_or_else(|e| panic!("OPENAI should parse: {e}")),
            ChannelType::OpenAI
        );
        assert_eq!(
            parse_channel_type("azure").unwrap_or_else(|e| panic!("azure should parse: {e}")),
            ChannelType::Azure
        );
        assert_eq!(
            parse_channel_type("anthropic")
                .unwrap_or_else(|e| panic!("anthropic should parse: {e}")),
            ChannelType::Anthropic
        );
        assert_eq!(
            parse_channel_type("gemini").unwrap_or_else(|e| panic!("gemini should parse: {e}")),
            ChannelType::Gemini
        );
        assert_eq!(
            parse_channel_type("aws").unwrap_or_else(|e| panic!("aws should parse: {e}")),
            ChannelType::Aws
        );
        assert_eq!(
            parse_channel_type("vertexai").unwrap_or_else(|e| panic!("vertexai should parse: {e}")),
            ChannelType::VertexAi
        );
        assert_eq!(
            parse_channel_type("vertex").unwrap_or_else(|e| panic!("vertex should parse: {e}")),
            ChannelType::VertexAi
        );
        assert_eq!(
            parse_channel_type("deepseek").unwrap_or_else(|e| panic!("deepseek should parse: {e}")),
            ChannelType::DeepSeek
        );
        assert!(parse_channel_type("invalid").is_err());
    }

    #[test]
    fn test_get_default_models() {
        assert_eq!(
            get_default_models(ChannelType::OpenAI),
            vec!["gpt-4", "gpt-4-turbo", "gpt-3.5-turbo"]
        );
        assert_eq!(
            get_default_models(ChannelType::Azure),
            vec!["gpt-4", "gpt-35-turbo"]
        );
        assert_eq!(
            get_default_models(ChannelType::Anthropic),
            vec!["claude-3-opus", "claude-3-sonnet", "claude-3-haiku"]
        );
        assert_eq!(
            get_default_models(ChannelType::Gemini),
            vec!["gemini-1.5-pro", "gemini-1.5-flash", "gemini-pro"]
        );
        assert_eq!(
            get_default_models(ChannelType::Aws),
            vec!["claude-3-sonnet", "claude-3-haiku"]
        );
        assert_eq!(
            get_default_models(ChannelType::VertexAi),
            vec!["gemini-1.5-pro"]
        );
        assert_eq!(
            get_default_models(ChannelType::DeepSeek),
            vec!["deepseek-chat", "deepseek-coder"]
        );
    }

    #[test]
    fn test_get_default_base_url() {
        assert_eq!(
            get_default_base_url(ChannelType::OpenAI),
            Some("https://api.openai.com/v1")
        );
        assert_eq!(get_default_base_url(ChannelType::Azure), None);
        assert_eq!(
            get_default_base_url(ChannelType::Anthropic),
            Some("https://api.anthropic.com/v1")
        );
        assert_eq!(
            get_default_base_url(ChannelType::Gemini),
            Some("https://generativelanguage.googleapis.com/v1beta")
        );
        assert_eq!(get_default_base_url(ChannelType::Aws), None);
        assert_eq!(get_default_base_url(ChannelType::VertexAi), None);
        assert_eq!(
            get_default_base_url(ChannelType::DeepSeek),
            Some("https://api.deepseek.com/v1")
        );
    }
}
