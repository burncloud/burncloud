//! CLI commands for billing plan management (Issue #232)

use burncloud_common::types::{BillingMode, BillingPlanInput};
use burncloud_database::Database;
use burncloud_service_billing::SubscriptionService;
use clap::ArgMatches;

pub async fn handle_plan_command(
    db: &Database,
    matches: &ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    match matches.subcommand() {
        Some(("create", sub_m)) => {
            let name = sub_m.get_one::<String>("name")
                .ok_or("Missing required argument: name")?.to_string();
            let monthly_fee = *sub_m.get_one::<i64>("monthly-fee")
                .ok_or("Missing required argument: monthly-fee")?;
            let billing_mode_str = sub_m.get_one::<String>("billing-mode")
                .ok_or("Missing required argument: billing-mode")?;
            let billing_mode: BillingMode = billing_mode_str.parse()?;
            let request_limit = sub_m.get_one::<i64>("request-limit").copied();
            let token_limit = sub_m.get_one::<i64>("token-limit").copied();
            let channel_id = *sub_m.get_one::<i32>("channel-id")
                .ok_or("Missing required argument: channel-id")?;

            let input = BillingPlanInput {
                name,
                monthly_fee_cny: monthly_fee,
                billing_mode,
                request_limit,
                token_limit,
                channel_id,
            };

            let plan = SubscriptionService::create_plan(db, input).await?;
            println!("✅ Created billing plan:");
            println!("  ID: {}", plan.id);
            println!("  Name: {}", plan.name);
            println!("  Monthly Fee: {} CNY", plan.monthly_fee / 1_000_000_000);
            println!("  Billing Mode: {}", plan.billing_mode);
            println!("  Channel ID: {}", plan.channel_id);
            if let Some(limit) = plan.request_limit {
                println!("  Request Limit: {}", limit);
            }
            if let Some(limit) = plan.token_limit {
                println!("  Token Limit: {}", limit);
            }
        }
        Some(("list", sub_m)) => {
            let channel = sub_m.get_one::<i32>("channel").copied();

            let plans = if let Some(channel_id) = channel {
                SubscriptionService::list_plans_by_channel(db, channel_id).await?
            } else {
                SubscriptionService::list_plans(db).await?
            };

            if plans.is_empty() {
                println!("No billing plans found.");
                return Ok(());
            }

            println!("Billing Plans:");
            println!("{:-<80}", "");
            println!(
                "{:<5} {:<20} {:<15} {:<12} {:<10} {:<10}",
                "ID", "Name", "Monthly Fee", "Mode", "Channel", "Limit"
            );
            println!("{:-<80}", "");

            for plan in plans {
                let fee = plan.monthly_fee / 1_000_000_000;
                let limit = plan
                    .request_limit
                    .map(|l| format!("{} req", l))
                    .or(plan.token_limit.map(|l| format!("{} tok", l)))
                    .unwrap_or_default();
                println!(
                    "{:<5} {:<20} {:<15} {:<12} {:<10} {:<10}",
                    plan.id,
                    plan.name,
                    format!("{} CNY", fee),
                    plan.billing_mode,
                    plan.channel_id,
                    limit
                );
            }
        }
        Some(("show", sub_m)) => {
            let id = *sub_m.get_one::<i32>("id")
                .ok_or("Missing required argument: id")?;
            let plan = SubscriptionService::get_plan(db, id).await?;
            println!("Billing Plan #{}:", plan.id);
            println!("  Name: {}", plan.name);
            println!("  Monthly Fee: {} CNY", plan.monthly_fee / 1_000_000_000);
            println!("  Billing Mode: {}", plan.billing_mode);
            println!("  Channel ID: {}", plan.channel_id);
            if let Some(limit) = plan.request_limit {
                println!("  Request Limit: {}", limit);
            }
            if let Some(limit) = plan.token_limit {
                println!("  Token Limit: {}", limit);
            }
            println!(
                "  Created: {}",
                chrono::DateTime::from_timestamp_millis(plan.created_at)
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default()
            );
        }
        Some(("delete", sub_m)) => {
            let id = *sub_m.get_one::<i32>("id")
                .ok_or("Missing required argument: id")?;
            let deleted = SubscriptionService::delete_plan(db, id).await?;
            if deleted {
                println!("✅ Deleted billing plan #{}", id);
            } else {
                println!("❌ Plan #{} not found", id);
            }
        }
        _ => {
            println!("Usage: burncloud plan <command>");
            println!("Commands: create, list, show, delete");
        }
    }
    Ok(())
}
