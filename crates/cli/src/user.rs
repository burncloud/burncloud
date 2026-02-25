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

/// Route user commands
pub async fn handle_user_command(db: &Database, matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("register", sub_m)) => {
            cmd_user_register(db, sub_m).await?;
        }
        Some(("login", sub_m)) => {
            cmd_user_login(db, sub_m).await?;
        }
        _ => {
            println!("Usage: burncloud user <register|login>");
            println!("Run 'burncloud user --help' for more information.");
        }
    }

    Ok(())
}
