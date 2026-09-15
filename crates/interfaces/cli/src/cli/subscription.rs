//! CLI commands for subscription management (Issue #232)

use burncloud_database::Database;
use burncloud_service_billing::SubscriptionService;
use clap::ArgMatches;

pub async fn handle_subscription_command(
    db: &Database,
    matches: &ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    match matches.subcommand() {
        Some(("subscribe", sub_m)) => {
            let user_id = *sub_m.get_one::<i32>("user").unwrap();
            let plan_id = *sub_m.get_one::<i32>("plan").unwrap();
            let duration = *sub_m.get_one::<i64>("duration").unwrap();

            let sub = SubscriptionService::subscribe(db, user_id, plan_id, duration).await?;
            println!("✅ Created subscription:");
            println!("  Subscription ID: {}", sub.id);
            println!("  User ID: {}", sub.user_id);
            println!("  Plan ID: {}", sub.plan_id);
            println!("  Channel ID: {}", sub.channel_id);
            println!("  Quota Limit: {}", sub.quota_limit);
            println!(
                "  Expires: {}",
                chrono::DateTime::from_timestamp_millis(sub.expires_at)
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default()
            );
        }
        Some(("status", sub_m)) => {
            let id = sub_m.get_one::<i32>("id").copied();
            let user = sub_m.get_one::<i32>("user").copied();

            let status = if let Some(sub_id) = id {
                SubscriptionService::get_subscription_status(db, sub_id).await?
            } else if let Some(user_id) = user {
                let sub = SubscriptionService::get_active_subscription(db, user_id)
                    .await?
                    .ok_or("No active subscription found for user")?;
                SubscriptionService::get_subscription_status(db, sub.id).await?
            } else {
                return Err("Please specify --id or --user".into());
            };

            println!("Subscription Status:");
            println!("{:-<60}", "");
            println!("  Subscription ID: {}", status.subscription.id);
            println!("  User ID: {}", status.subscription.user_id);
            println!("  Plan: {} (ID: {})", status.plan.name, status.plan.id);
            println!("  Channel ID: {}", status.subscription.channel_id);
            println!("  Status: {}", status.subscription.status);
            println!(
                "  Quota Used: {} / {}",
                status.subscription.quota_used, status.subscription.quota_limit
            );
            println!("  Quota Remaining: {}", status.quota_remaining);
            println!("  Days Remaining: {}", status.days_remaining);
            println!(
                "  Expires: {}",
                chrono::DateTime::from_timestamp_millis(status.subscription.expires_at)
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default()
            );

            if status.is_expired {
                println!("  ⚠️  Subscription is EXPIRED");
            } else {
                println!("  ✅ Subscription is ACTIVE");
            }
        }
        Some(("list", sub_m)) => {
            let user_id = *sub_m.get_one::<i32>("user").unwrap();
            let subs = SubscriptionService::list_user_subscriptions(db, user_id).await?;

            if subs.is_empty() {
                println!("No subscriptions found for user {}.", user_id);
                return Ok(());
            }

            println!("Subscriptions for User {}:", user_id);
            println!("{:-<80}", "");
            println!(
                "{:<5} {:<8} {:<8} {:<8} {:<15} {:<15} {:<10}",
                "ID", "Plan", "Channel", "Status", "Used/Limit", "Expires", "Days Left"
            );
            println!("{:-<80}", "");

            let now = chrono::Utc::now().timestamp_millis();

            for sub in subs {
                let days_left = ((sub.expires_at - now) / (24 * 60 * 60 * 1000)).max(0);
                let quota = format!("{}/{}", sub.quota_used, sub.quota_limit);
                let expires = chrono::DateTime::from_timestamp_millis(sub.expires_at)
                    .map(|t| t.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
                println!(
                    "{:<5} {:<8} {:<8} {:<8} {:<15} {:<15} {:<10}",
                    sub.id, sub.plan_id, sub.channel_id, sub.status, quota, expires, days_left
                );
            }
        }
        Some(("cancel", sub_m)) => {
            let id = *sub_m.get_one::<i32>("id").unwrap();
            let sub = SubscriptionService::cancel_subscription(db, id).await?;
            println!("✅ Cancelled subscription #{}", id);
            println!("  Status: {}", sub.status);
        }
        _ => {
            println!("Usage: burncloud subscription <command>");
            println!("Commands: subscribe, status, list, cancel");
        }
    }
    Ok(())
}
