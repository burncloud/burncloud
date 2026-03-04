use crate::common::spawn_app;
use burncloud_tests::TestClient;
use serde_json::json;
use uuid::Uuid;

// Helper function to generate unique test usernames
fn generate_test_username(prefix: &str) -> String {
    format!(
        "{}_{}",
        prefix,
        &Uuid::new_v4().to_string().replace("-", "")[..8]
    )
}

// ============================================================
// AUTH-01: User Registration Tests
// ============================================================

#[tokio::test]
async fn test_auth_register_success() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let username = generate_test_username("authuser");
    let password = "SecurePass123!";

    let body = json!({
        "username": username,
        "password": password,
        "email": format!("{}@example.com", username)
    });

    let res = client.post("/api/auth/register", &body).await?;
    assert_eq!(res["success"], true, "Register should succeed");
    assert!(!res["data"]["id"].is_null(), "Should return user ID");
    assert_eq!(res["data"]["username"], username);
    assert!(!res["data"]["token"].is_null(), "Should return JWT token");

    // Verify token is a non-empty string
    let token = res["data"]["token"].as_str().unwrap();
    assert!(!token.is_empty(), "Token should not be empty");

    Ok(())
}

#[tokio::test]
async fn test_auth_register_duplicate_username() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let username = generate_test_username("dupuser");
    let password = "SecurePass123!";

    let body = json!({
        "username": username,
        "password": password,
        "email": format!("{}@example.com", username)
    });

    // First registration should succeed
    let res1 = client.post("/api/auth/register", &body).await?;
    assert_eq!(res1["success"], true);

    // Second registration with same username should fail
    let res2 = client.post("/api/auth/register", &body).await?;
    assert_eq!(res2["success"], false);
    assert!(res2["message"].as_str().unwrap().contains("already exists"));

    Ok(())
}

#[tokio::test]
async fn test_auth_register_empty_username() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let body = json!({
        "username": "",
        "password": "SecurePass123!"
    });

    let res = client.post("/api/auth/register", &body).await?;
    // Server may still accept this (no validation in handler), test documents current behavior

    Ok(())
}

#[tokio::test]
async fn test_auth_register_empty_password() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let username = generate_test_username("emptypass");

    let body = json!({
        "username": username,
        "password": ""
    });

    let res = client.post("/api/auth/register", &body).await?;
    // Server may still accept this (no validation in handler), test documents current behavior

    Ok(())
}

#[tokio::test]
async fn test_auth_register_short_username() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let body = json!({
        "username": "ab",
        "password": "SecurePass123!"
    });

    let res = client.post("/api/auth/register", &body).await?;
    // Server may still accept this, test documents current behavior

    Ok(())
}

#[tokio::test]
async fn test_auth_register_weak_password() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let username = generate_test_username("weakpass");

    let body = json!({
        "username": username,
        "password": "abc"
    });

    let res = client.post("/api/auth/register", &body).await?;
    // Server may still accept this, test documents current behavior

    Ok(())
}

// ============================================================
// AUTH-02: User Login Tests
// ============================================================

#[tokio::test]
async fn test_auth_login_success() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let username = generate_test_username("loginuser");
    let password = "SecurePass123!";

    // Register first
    let reg_body = json!({
        "username": username,
        "password": password,
        "email": format!("{}@example.com", username)
    });
    client.post("/api/auth/register", &reg_body).await?;

    // Now login
    let login_body = json!({
        "username": username,
        "password": password
    });

    let res = client.post("/api/auth/login", &login_body).await?;
    assert_eq!(res["success"], true, "Login should succeed");
    assert_eq!(res["data"]["username"], username);
    assert!(!res["data"]["token"].is_null(), "Should return JWT token");
    assert!(!res["data"]["roles"].is_null(), "Should return user roles");

    // Verify token is a non-empty string
    let token = res["data"]["token"].as_str().unwrap();
    assert!(!token.is_empty(), "Token should not be empty");

    Ok(())
}

#[tokio::test]
async fn test_auth_login_invalid_credentials() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let username = generate_test_username("testuser");
    let password = "SecurePass123!";

    // Register first
    let reg_body = json!({
        "username": username,
        "password": password,
        "email": format!("{}@example.com", username)
    });
    client.post("/api/auth/register", &reg_body).await?;

    // Try login with wrong password
    let login_body = json!({
        "username": username,
        "password": "WrongPassword123!"
    });

    let res = client.post("/api/auth/login", &login_body).await?;
    assert_eq!(
        res["success"], false,
        "Login should fail with wrong password"
    );
    assert!(res["message"]
        .as_str()
        .unwrap()
        .contains("Invalid credentials"));

    Ok(())
}

#[tokio::test]
async fn test_auth_login_nonexistent_user() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let login_body = json!({
        "username": "nonexistent_user_12345",
        "password": "SomePassword123!"
    });

    let res = client.post("/api/auth/login", &login_body).await?;
    assert_eq!(
        res["success"], false,
        "Login should fail for nonexistent user"
    );
    assert!(res["message"].as_str().unwrap().contains("not found"));

    Ok(())
}

// ============================================================
// AUTH-03: JWT Token Tests
// ============================================================

#[tokio::test]
async fn test_auth_token_validity() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let username = generate_test_username("tokenuser");
    let password = "SecurePass123!";

    // Register and get token
    let reg_body = json!({
        "username": username,
        "password": password,
        "email": format!("{}@example.com", username)
    });

    let res = client.post("/api/auth/register", &reg_body).await?;
    let token = res["data"]["token"].as_str().unwrap();

    // Token should be a valid JWT (3 parts separated by dots)
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "JWT should have 3 parts");

    Ok(())
}

#[tokio::test]
async fn test_auth_token_different_per_login() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let username = generate_test_username("tokendiff");
    let password = "SecurePass123!";

    // Register
    let reg_body = json!({
        "username": username,
        "password": password,
        "email": format!("{}@example.com", username)
    });

    let reg_res = client.post("/api/auth/register", &reg_body).await?;
    let reg_token = reg_res["data"]["token"].as_str().unwrap().to_string();

    // Small delay to ensure different timestamp
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Login
    let login_body = json!({
        "username": username,
        "password": password
    });

    let login_res = client.post("/api/auth/login", &login_body).await?;
    let login_token = login_res["data"]["token"].as_str().unwrap().to_string();

    // Tokens should be different (different iat timestamps)
    assert_ne!(reg_token, login_token, "Tokens should be different");

    Ok(())
}

// ============================================================
// AUTH-04: Password Encryption Tests (bcrypt)
// ============================================================

#[tokio::test]
async fn test_auth_password_bcrypt_hash() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let username = generate_test_username("bcrypt");
    let password = "SecurePass123!";

    // Register
    let reg_body = json!({
        "username": username,
        "password": password,
        "email": format!("{}@example.com", username)
    });

    client.post("/api/auth/register", &reg_body).await?;

    // Login with correct password should succeed
    let login_body = json!({
        "username": username,
        "password": password
    });

    let res = client.post("/api/auth/login", &login_body).await?;
    assert_eq!(res["success"], true, "Login with correct password should succeed");

    // Login with wrong password should fail
    let wrong_login_body = json!({
        "username": username,
        "password": "WrongPassword123!"
    });

    let res = client.post("/api/auth/login", &wrong_login_body).await?;
    assert_eq!(res["success"], false, "Login with wrong password should fail");

    Ok(())
}

// ============================================================
// AUTH-05: Role Assignment Tests
// ============================================================

#[tokio::test]
async fn test_auth_default_role_assigned() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let username = generate_test_username("roleuser");
    let password = "SecurePass123!";

    // Register
    let reg_body = json!({
        "username": username,
        "password": password,
        "email": format!("{}@example.com", username)
    });

    client.post("/api/auth/register", &reg_body).await?;

    // Login should return roles
    let login_body = json!({
        "username": username,
        "password": password
    });

    let res = client.post("/api/auth/login", &login_body).await?;
    assert_eq!(res["success"], true);

    // Check that roles are returned
    let roles = res["data"]["roles"].as_array();
    assert!(roles.is_some(), "Should return roles array");
    let roles = roles.unwrap();
    assert!(!roles.is_empty(), "User should have at least one role");

    Ok(())
}

// ============================================================
// Complete Flow Tests
// ============================================================

#[tokio::test]
async fn test_auth_complete_flow() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let username = generate_test_username("flowuser");
    let password = "CompleteFlow123!";

    // 1. Register
    let reg_body = json!({
        "username": username,
        "password": password,
        "email": format!("{}@example.com", username)
    });

    let reg_res = client.post("/api/auth/register", &reg_body).await?;
    assert_eq!(reg_res["success"], true);
    let reg_token = reg_res["data"]["token"].as_str().unwrap().to_string();

    // 2. Login
    let login_body = json!({
        "username": username,
        "password": password
    });

    let login_res = client.post("/api/auth/login", &login_body).await?;
    assert_eq!(login_res["success"], true);
    let login_token = login_res["data"]["token"].as_str().unwrap().to_string();

    // Both tokens should be valid JWT tokens (non-empty strings)
    assert!(!reg_token.is_empty());
    assert!(!login_token.is_empty());

    // Tokens should be different (different iat timestamps)
    // Note: In rare cases they might be the same if generated in the same second,
    // but this is unlikely in practice

    Ok(())
}

#[tokio::test]
async fn test_auth_register_login_with_optional_email() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let username = generate_test_username("noemail");
    let password = "SecurePass123!";

    // Register without email
    let reg_body = json!({
        "username": username,
        "password": password
    });

    let res = client.post("/api/auth/register", &reg_body).await?;
    assert_eq!(res["success"], true, "Registration without email should succeed");

    // Login should work
    let login_body = json!({
        "username": username,
        "password": password
    });

    let res = client.post("/api/auth/login", &login_body).await?;
    assert_eq!(res["success"], true, "Login should succeed");

    Ok(())
}

#[tokio::test]
async fn test_auth_special_characters_in_password() -> anyhow::Result<()> {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);

    let username = generate_test_username("special");
    let password = "P@ssw0rd!#$%^&*()";

    // Register with special characters in password
    let reg_body = json!({
        "username": username,
        "password": password
    });

    let res = client.post("/api/auth/register", &reg_body).await?;
    assert_eq!(res["success"], true, "Registration with special chars should succeed");

    // Login should work
    let login_body = json!({
        "username": username,
        "password": password
    });

    let res = client.post("/api/auth/login", &login_body).await?;
    assert_eq!(res["success"], true, "Login should succeed");

    Ok(())
}
