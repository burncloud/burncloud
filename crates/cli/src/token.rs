use anyhow::Result;
use burncloud_database::Database;
use burncloud_database_models::{TokenInput, TokenModel, TokenUpdateInput};
use clap::ArgMatches;
use std::io::{self, Write};

pub async fn handle_token_command(db: &Database, matches: &ArgMatches) -> Result<()> {
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
            let user_id = sub_m.get_one::<String>("user-id").map(|s| s.as_str());

            let tokens = TokenModel::list(db, limit, offset, user_id).await?;

            if tokens.is_empty() {
                println!("No tokens found.");
                return Ok(());
            }

            println!(
                "{:<52} {:<20} {:<12} {:>12} {:<8} {:>12}",
                "Key", "Name", "User", "Quota", "Status", "Expired"
            );
            println!("{}", "-".repeat(120));

            for token in tokens {
                let name = token.name.as_deref().unwrap_or("-");
                let quota = if token.unlimited_quota {
                    "unlimited".to_string()
                } else {
                    token.remain_quota.to_string()
                };
                let status = if token.status == 1 {
                    "active"
                } else {
                    "disabled"
                };
                let expired = if token.expired_time == -1 {
                    "never".to_string()
                } else {
                    format!("{}", token.expired_time)
                };
                println!(
                    "{:<52} {:<20} {:<12} {:>12} {:<8} {:>12}",
                    token.key, name, token.user_id, quota, status, expired
                );
            }
        }
        Some(("create", sub_m)) => {
            let user_id = sub_m.get_one::<String>("user-id").unwrap().to_string();
            let name = sub_m.get_one::<String>("name").cloned();
            let quota = sub_m
                .get_one::<String>("quota")
                .and_then(|s| s.parse().ok());
            let unlimited = sub_m.get_flag("unlimited");
            let expired = sub_m
                .get_one::<String>("expired")
                .and_then(|s| s.parse().ok());

            let input = TokenInput {
                user_id: user_id.clone(),
                name,
                remain_quota: quota,
                unlimited_quota: if unlimited { Some(true) } else { None },
                expired_time: expired,
            };

            let token = TokenModel::create(db, &input).await?;
            println!("✓ Token created successfully!");
            println!("Key: {}", token.key);
        }
        Some(("update", sub_m)) => {
            let key = sub_m.get_one::<String>("key").unwrap();
            let name = sub_m.get_one::<String>("name").cloned();
            let quota = sub_m
                .get_one::<String>("quota")
                .and_then(|s| s.parse().ok());
            let status = sub_m
                .get_one::<String>("status")
                .and_then(|s| s.parse().ok());

            let input = TokenUpdateInput {
                name,
                remain_quota: quota,
                status,
                expired_time: None,
            };

            let updated = TokenModel::update(db, key, &input).await?;
            if updated {
                println!("✓ Token '{}' updated successfully!", key);
            } else {
                println!("Token '{}' not found or no changes made.", key);
            }
        }
        Some(("delete", sub_m)) => {
            let key = sub_m.get_one::<String>("key").unwrap();
            let skip_confirm = sub_m.get_flag("yes");

            if !skip_confirm {
                print!("Are you sure you want to delete token '{}'? [y/N] ", key);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim().to_lowercase();
                if input != "y" && input != "yes" {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            let deleted = TokenModel::delete(db, key).await?;
            if deleted {
                println!("✓ Token deleted successfully!");
            } else {
                println!("Token not found.");
            }
        }
        _ => {
            println!("Usage: burncloud token <list|create|update|delete>");
            println!("Run 'burncloud token --help' for more information.");
        }
    }

    Ok(())
}
