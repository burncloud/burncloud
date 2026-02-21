use anyhow::Result;
use burncloud_database::Database;
use burncloud_database_models::{ProtocolConfigInput, ProtocolConfigModel};
use clap::ArgMatches;

pub async fn handle_protocol_command(db: &Database, matches: &ArgMatches) -> Result<()> {
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

            let configs = ProtocolConfigModel::list(db, limit, offset).await?;

            if configs.is_empty() {
                println!("No protocol configs found.");
                return Ok(());
            }

            println!(
                "{:<5} {:<12} {:<18} {:<8} {:<35}",
                "ID", "ChannelType", "API Version", "Default", "Chat Endpoint"
            );
            println!("{}", "-".repeat(80));

            for config in configs {
                let channel_type_name = channel_type_to_name(config.channel_type);
                let is_default = if config.is_default { "Yes" } else { "No" };
                let endpoint = config.chat_endpoint.as_deref().unwrap_or("-");
                println!(
                    "{:<5} {:<12} {:<18} {:<8} {:<35}",
                    config.id, channel_type_name, config.api_version, is_default, endpoint
                );
            }
        }
        Some(("add", sub_m)) => {
            let channel_type: i32 = sub_m
                .get_one::<String>("channel-type")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let api_version = sub_m.get_one::<String>("api-version").unwrap().to_string();
            let is_default = sub_m.get_flag("default");
            let chat_endpoint = sub_m.get_one::<String>("chat-endpoint").cloned();
            let embed_endpoint = sub_m.get_one::<String>("embed-endpoint").cloned();
            let models_endpoint = sub_m.get_one::<String>("models-endpoint").cloned();
            let request_mapping = sub_m.get_one::<String>("request-mapping").cloned();
            let response_mapping = sub_m.get_one::<String>("response-mapping").cloned();
            let detection_rules = sub_m.get_one::<String>("detection-rules").cloned();

            let input = ProtocolConfigInput {
                channel_type,
                api_version: api_version.clone(),
                is_default: Some(is_default),
                chat_endpoint,
                embed_endpoint,
                models_endpoint,
                request_mapping,
                response_mapping,
                detection_rules,
            };

            ProtocolConfigModel::upsert(db, &input).await?;
            println!(
                "✓ Protocol config added: {} ({})",
                channel_type_to_name(channel_type),
                api_version
            );
        }
        Some(("delete", sub_m)) => {
            let id: i32 = sub_m
                .get_one::<String>("id")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            let deleted = ProtocolConfigModel::delete(db, id).await?;
            if deleted {
                println!("✓ Protocol config {} deleted", id);
            } else {
                println!("Protocol config {} not found", id);
            }
        }
        Some(("show", sub_m)) => {
            let id: i32 = sub_m
                .get_one::<String>("id")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            // For show, we need to get by ID which isn't implemented, so use list and filter
            let configs = ProtocolConfigModel::list(db, 1000, 0).await?;
            let config = configs.iter().find(|c| c.id == id);

            match config {
                Some(c) => {
                    println!("ID: {}", c.id);
                    println!(
                        "Channel Type: {} ({})",
                        channel_type_to_name(c.channel_type),
                        c.channel_type
                    );
                    println!("API Version: {}", c.api_version);
                    println!("Is Default: {}", c.is_default);
                    println!(
                        "Chat Endpoint: {}",
                        c.chat_endpoint.as_deref().unwrap_or("-")
                    );
                    println!(
                        "Embed Endpoint: {}",
                        c.embed_endpoint.as_deref().unwrap_or("-")
                    );
                    println!(
                        "Models Endpoint: {}",
                        c.models_endpoint.as_deref().unwrap_or("-")
                    );
                    if let Some(ref req_map) = c.request_mapping {
                        println!("Request Mapping: {}", req_map);
                    }
                    if let Some(ref resp_map) = c.response_mapping {
                        println!("Response Mapping: {}", resp_map);
                    }
                    if let Some(ref rules) = c.detection_rules {
                        println!("Detection Rules: {}", rules);
                    }
                }
                None => {
                    println!("Protocol config {} not found", id);
                }
            }
        }
        Some(("test", sub_m)) => {
            let channel_id: i32 = sub_m
                .get_one::<String>("channel-id")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let model = sub_m.get_one::<String>("model").map(|s| s.as_str());

            // For testing, we would need to actually make a request
            // This is a placeholder for now
            println!("Testing protocol for channel {}...", channel_id);
            if let Some(m) = model {
                println!("Model: {}", m);
            }
            println!("Protocol test not yet implemented - requires live channel connection");
        }
        _ => {
            println!("Usage: burncloud protocol <list|add|delete|show|test>");
            println!("Run 'burncloud protocol --help' for more information.");
        }
    }

    Ok(())
}

/// Convert channel type ID to human-readable name
fn channel_type_to_name(channel_type: i32) -> &'static str {
    match channel_type {
        0 => "OpenAI",
        1 => "Anthropic",
        2 => "Azure",
        3 => "AWS",
        4 => "Gemini",
        5 => "VertexAI",
        6 => "DeepSeek",
        7 => "Moonshot",
        _ => "Unknown",
    }
}
