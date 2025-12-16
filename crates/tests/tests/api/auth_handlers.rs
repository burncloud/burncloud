use crate::common::spawn_app;
use burncloud_tests::TestClient;
use serde_json::json;
use uuid::Uuid;

// Helper function to generate unique test usernames
fn generate_test_username(prefix: &str) -> String {
    format!("{}_{}", prefix, &Uuid::new_v4().to_string().replace("-", "")[..8])
}

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
    assert_eq!(res["success"], false, "Login should fail with wrong password");
    assert!(res["message"].as_str().unwrap().contains("Invalid credentials"));

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
    assert_eq!(res["success"], false, "Login should fail for nonexistent user");
    assert!(res["message"].as_str().unwrap().contains("not found"));

    Ok(())
}

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
