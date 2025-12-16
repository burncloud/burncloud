//! Example usage of the UserService
//!
//! This example demonstrates how to use the service-user crate
//! for user registration, login, and token management.

use burncloud_database::create_default_database;
use burncloud_database_user::UserDatabase;
use burncloud_service_user::UserService;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== BurnCloud User Service Example ===\n");

    // Initialize database
    let db = create_default_database().await?;
    UserDatabase::init(&db).await?;
    println!("✓ Database initialized\n");

    // Create service
    let service = UserService::new();
    println!("✓ UserService created\n");

    // Use a unique username for this run
    let username = format!("user_{}", Uuid::new_v4().to_string().split('-').next().unwrap());
    let email = format!("{}@example.com", username);

    // Register a new user
    println!("Registering user '{}'...", username);
    let user_id = service
        .register_user(&db, &username, "secure_password", Some(email))
        .await?;
    println!("✓ User registered with ID: {}\n", user_id);

    // Login
    println!("Logging in as '{}'...", username);
    let auth_token = service.login_user(&db, &username, "secure_password").await?;
    println!("✓ Login successful!");
    println!("  Token: {}", auth_token.token);
    println!("  User ID: {}", auth_token.user_id);
    println!("  Username: {}", auth_token.username);
    println!("  Expires at: {}\n", auth_token.expires_at);

    // Validate token
    println!("Validating token...");
    let (validated_user_id, validated_username) = service.validate_token(&auth_token.token)?;
    println!("✓ Token is valid!");
    println!("  User ID: {}", validated_user_id);
    println!("  Username: {}\n", validated_username);

    // Demonstrate failed login
    println!("Attempting login with wrong password...");
    match service.login_user(&db, &username, "wrong_password").await {
        Ok(_) => println!("✗ Should have failed!"),
        Err(e) => println!("✓ Login failed as expected: {}\n", e),
    }

    // Demonstrate duplicate registration
    println!("Attempting to register '{}' again...", username);
    match service
        .register_user(&db, &username, "password", None)
        .await
    {
        Ok(_) => println!("✗ Should have failed!"),
        Err(e) => println!("✓ Registration failed as expected: {}\n", e),
    }

    println!("=== Example completed successfully! ===");
    Ok(())
}
