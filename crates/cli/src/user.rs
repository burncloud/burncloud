use anyhow::Result;
use bcrypt::{hash, DEFAULT_COST};
use burncloud_common::utils::verify_password;
use burncloud_database::Database;
use burncloud_database_user::{DbUser, UserDatabase};
use clap::ArgMatches;
use uuid::Uuid;

/// Handle user login command
pub async fn cmd_user_login(db: &Database, matches: &ArgMatches) -> Result<()> {
    let username = matches.get_one::<String>("username").unwrap();
    let password = matches.get_one::<String>("password").unwrap();

    // Get user by username
    let user = match UserDatabase::get_user_by_username(db, username).await? {
        Some(u) => u,
        None => {
            return Err(anyhow::anyhow!("Invalid username or password"));
        }
    };

    // Check if user is active
    if user.status != 1 {
        return Err(anyhow::anyhow!("User account is disabled"));
    }

    // Verify password
    let password_hash = match &user.password_hash {
        Some(hash) => hash,
        None => {
            return Err(anyhow::anyhow!(
                "User has no password set. Please use OAuth login."
            ));
        }
    };

    let is_valid = verify_password(password, password_hash)?;
    if !is_valid {
        return Err(anyhow::anyhow!("Invalid username or password"));
    }

    // Generate a simple login token (UUID)
    let login_token = Uuid::new_v4().to_string();

    // Output user info and token
    println!("Login successful!");
    println!();
    println!("User Information:");
    println!("  ID: {}", user.id);
    println!("  Username: {}", user.username);
    if let Some(email) = &user.email {
        println!("  Email: {}", email);
    }
    println!("  Status: {}", if user.status == 1 { "Active" } else { "Disabled" });

    // Convert nanodollar balances to display format
    let balance_usd = user.balance_usd as f64 / 1_000_000_000.0;
    let balance_cny = user.balance_cny as f64 / 1_000_000_000.0;
    println!("  Balance (USD): ${:.2}", balance_usd);
    println!("  Balance (CNY): ¥{:.2}", balance_cny);

    // Get user roles
    let roles = UserDatabase::get_user_roles(db, &user.id).await?;
    if !roles.is_empty() {
        println!("  Roles: {}", roles.join(", "));
    }

    println!();
    println!("Login Token: {}", login_token);

    Ok(())
}

/// Handle user register command
pub async fn cmd_user_register(db: &Database, matches: &ArgMatches) -> Result<()> {
    let username = matches.get_one::<String>("username").unwrap();
    let password = matches.get_one::<String>("password").unwrap();
    let email = matches.get_one::<String>("email").cloned();

    // Check if user already exists
    if UserDatabase::get_user_by_username(db, username).await?.is_some() {
        return Err(anyhow::anyhow!(
            "User '{}' already exists",
            username
        ));
    }

    // Hash password
    let password_hash = hash(password, DEFAULT_COST)?;

    // Generate user ID
    let user_id = Uuid::new_v4().to_string();

    // Create user struct
    let user = DbUser {
        id: user_id.clone(),
        username: username.clone(),
        email,
        password_hash: Some(password_hash),
        github_id: None,
        status: 1,
        balance_usd: 0,
        balance_cny: 0,
        preferred_currency: Some("USD".to_string()),
    };

    // Create user in database
    UserDatabase::create_user(db, &user).await?;

    // Assign default 'user' role
    UserDatabase::assign_role(db, &user_id, "user").await?;

    println!("User '{}' registered successfully!", username);
    println!("User ID: {}", user_id);

    Ok(())
}

/// Handle user list command
pub async fn cmd_user_list(db: &Database, matches: &ArgMatches) -> Result<()> {
    let limit: i64 = matches
        .get_one::<String>("limit")
        .unwrap()
        .parse()
        .unwrap_or(100);
    let offset: i64 = matches
        .get_one::<String>("offset")
        .unwrap()
        .parse()
        .unwrap_or(0);
    let format = matches
        .get_one::<String>("format")
        .map(|s| s.as_str())
        .unwrap_or("table");

    // Get all users (list_users doesn't support pagination, we'll apply it here)
    let mut users = UserDatabase::list_users(db).await?;

    // Apply offset and limit
    let start = offset as usize;
    let end = std::cmp::min(start + limit as usize, users.len());
    let users: Vec<_> = if start < users.len() {
        users.drain(start..end).collect()
    } else {
        vec![]
    };

    if users.is_empty() {
        println!("No users found");
        return Ok(());
    }

    match format {
        "json" => {
            // Create a simplified user view for JSON output
            let users_json: Vec<serde_json::Value> = users
                .iter()
                .map(|u| {
                    serde_json::json!({
                        "id": u.id,
                        "username": u.username,
                        "email": u.email,
                        "balance_usd": u.balance_usd as f64 / 1_000_000_000.0,
                        "balance_cny": u.balance_cny as f64 / 1_000_000_000.0,
                        "status": if u.status == 1 { "Active" } else { "Disabled" }
                    })
                })
                .collect();
            let json = serde_json::to_string_pretty(&users_json)?;
            println!("{}", json);
        }
        _ => {
            // Table format
            println!(
                "{:<40} {:<20} {:<30} {:<15} {:<15} {:<10}",
                "ID", "Username", "Email", "Balance_USD", "Balance_CNY", "Status"
            );
            println!("{}", "-".repeat(135));
            for user in users {
                let balance_usd = user.balance_usd as f64 / 1_000_000_000.0;
                let balance_cny = user.balance_cny as f64 / 1_000_000_000.0;
                let email = user.email.as_deref().unwrap_or("N/A");
                let status = if user.status == 1 { "Active" } else { "Disabled" };
                println!(
                    "{:<40} {:<20} {:<30} ${:<14.2} ¥{:<14.2} {:<10}",
                    user.id, user.username, email, balance_usd, balance_cny, status
                );
            }
        }
    }

    Ok(())
}

/// Handle user recharges command
pub async fn cmd_user_recharges(db: &Database, matches: &ArgMatches) -> Result<()> {
    let user_id = matches.get_one::<String>("user-id").unwrap();
    let limit: i64 = matches
        .get_one::<String>("limit")
        .unwrap()
        .parse()
        .unwrap_or(100);

    // Get recharges for user
    let mut recharges = UserDatabase::list_recharges(db, user_id).await?;

    // Apply limit
    let recharges: Vec<_> = recharges.drain(..std::cmp::min(limit as usize, recharges.len())).collect();

    if recharges.is_empty() {
        println!("No recharges found for user: {}", user_id);
        return Ok(());
    }

    // Table format output
    println!(
        "{:<10} {:<15} {:<10} {:<40} {:<20}",
        "ID", "Amount", "Currency", "Description", "CreatedAt"
    );
    println!("{}", "-".repeat(100));
    for recharge in recharges {
        // Convert nanodollars to display format
        let amount = recharge.amount as f64 / 1_000_000_000.0;
        let currency = recharge.currency.as_deref().unwrap_or("USD");
        let description = recharge.description.as_deref().unwrap_or("N/A");
        let created_at = recharge.created_at.as_deref().unwrap_or("N/A");
        println!(
            "{:<10} ${:<14.2} {:<10} {:<40} {:<20}",
            recharge.id, amount, currency, description, created_at
        );
    }

    Ok(())
}

/// Handle user check-username command
pub async fn cmd_user_check_username(db: &Database, matches: &ArgMatches) -> Result<()> {
    let username = matches.get_one::<String>("username").unwrap();

    // Check if username exists
    let existing_user = UserDatabase::get_user_by_username(db, username).await?;

    if existing_user.is_some() {
        println!("Already taken");
    } else {
        println!("Available");
    }

    Ok(())
}

/// Handle user topup command
pub async fn cmd_user_topup(db: &Database, matches: &ArgMatches) -> Result<()> {
    let user_id = matches.get_one::<String>("user-id").unwrap();
    let amount_str = matches.get_one::<String>("amount").unwrap();
    let currency = matches.get_one::<String>("currency").unwrap().to_uppercase();

    // Validate currency
    if currency != "USD" && currency != "CNY" {
        return Err(anyhow::anyhow!(
            "Invalid currency '{}'. Must be USD or CNY",
            currency
        ));
    }

    // Parse amount (in dollars) and convert to nanodollars
    let amount_dollar: f64 = amount_str
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid amount '{}': {}", amount_str, e))?;

    if amount_dollar <= 0.0 {
        return Err(anyhow::anyhow!("Amount must be greater than 0"));
    }

    let amount_nano = (amount_dollar * 1_000_000_000.0) as i64;

    // Create recharge record (this also updates the balance)
    let recharge = burncloud_database_user::DbRecharge {
        id: 0, // Auto-generated
        user_id: user_id.clone(),
        amount: amount_nano,
        currency: Some(currency.clone()),
        description: Some(format!("CLI topup: {} {}", amount_dollar, currency)),
        created_at: None,
    };

    let recharge_id = UserDatabase::create_recharge(db, &recharge).await?;

    // Get new balance
    let new_balance_nano = UserDatabase::update_balance(db, user_id, 0, Some(&currency)).await?;

    // Convert nanodollars to display format
    let new_balance = new_balance_nano as f64 / 1_000_000_000.0;

    // Output success message
    println!("Topup successful!");
    println!();
    println!("Recharge Details:");
    println!("  Recharge ID: {}", recharge_id);
    println!("  User ID: {}", user_id);
    println!("  Amount: {} {}", amount_dollar, currency);
    println!("  New Balance ({}): {:.2}", currency, new_balance);

    Ok(())
}

/// Route user commands
pub async fn handle_user_command(db: &Database, matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("register", sub_m)) => {
            cmd_user_register(db, sub_m).await?;
        }
        Some(("login", sub_m)) => {
            cmd_user_login(db, sub_m).await?;
        }
        Some(("list", sub_m)) => {
            cmd_user_list(db, sub_m).await?;
        }
        Some(("topup", sub_m)) => {
            cmd_user_topup(db, sub_m).await?;
        }
        Some(("recharges", sub_m)) => {
            cmd_user_recharges(db, sub_m).await?;
        }
        Some(("check-username", sub_m)) => {
            cmd_user_check_username(db, sub_m).await?;
        }
        _ => {
            println!("Usage: burncloud user <register|login|list|topup|recharges|check-username>");
            println!("Run 'burncloud user --help' for more information.");
        }
    }

    Ok(())
}
