use anyhow::Result;
use bcrypt::{hash, DEFAULT_COST};
use burncloud_database::Database;
use burncloud_database_user::{DbUser, UserDatabase};
use clap::ArgMatches;
use uuid::Uuid;

/// Handle user register command
pub async fn cmd_user_register(db: &Database, matches: &ArgMatches) -> Result<()> {
    let username = matches.get_one::<String>("username").unwrap();
    let password = matches.get_one::<String>("password").unwrap();
    let email = matches.get_one::<String>("email").cloned();

    // Check if user already exists
    if let Some(_) = UserDatabase::get_user_by_username(db, username).await? {
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
        _ => {
            println!("Usage: burncloud user <register>");
            println!("Run 'burncloud user --help' for more information.");
        }
    }

    Ok(())
}
