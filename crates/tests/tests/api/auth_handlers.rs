#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::let_unit_value,
    clippy::redundant_pattern,
    clippy::manual_is_multiple_of,
    clippy::let_and_return,
    clippy::to_string_trait_impl,
    clippy::to_string_in_format_args,
    clippy::redundant_pattern_matching
)]
use crate::common::spawn_app;
use burncloud_tests::TestClient;
use serde_json::json;
use uuid::Uuid;

const TEST_BOOTSTRAP_TOKEN: &str = "burncloud-e2e-bootstrap-token-2026";

fn generate_test_username(prefix: &str) -> String {
    format!("{}_{}", prefix, &Uuid::new_v4().to_string().replace('-', "")[..8])
}

#[tokio::test]
async fn test_auth_register_success() -> anyhow::Result<()> {
    let client = TestClient::new(&spawn_app().await);
    let username = generate_test_username("authuser");
    let body = json!({"username": username, "password": "SecurePass123!", "email": format!("{}@example.com", username)});
    let res = client.post("/api/auth/register", &body).await?;
    assert_eq!(res["success"], true);
    assert_eq!(res["data"]["roles"][0], "user");
    assert!(!res["data"]["token"].as_str().unwrap_or_default().is_empty());
    Ok(())
}

#[tokio::test]
async fn test_auth_bootstrap_token_cannot_be_replayed() -> anyhow::Result<()> {
    let client = TestClient::new(&spawn_app().await);
    let body = json!({
        "username": generate_test_username("replay"),
        "password": "SecurePass123!",
        "email": "replay@example.invalid",
        "bootstrap_token": TEST_BOOTSTRAP_TOKEN
    });
    let res = client.post("/api/auth/register", &body).await?;
    assert_eq!(res["success"], false);
    assert!(res["message"].as_str().unwrap_or_default().contains("already been completed"));
    Ok(())
}

#[tokio::test]
async fn test_auth_register_duplicate_username() -> anyhow::Result<()> {
    let client = TestClient::new(&spawn_app().await);
    let username = generate_test_username("dupuser");
    let body = json!({"username": username, "password": "SecurePass123!", "email": format!("{}@example.com", username)});
    assert_eq!(client.post("/api/auth/register", &body).await?["success"], true);
    let second = client.post("/api/auth/register", &body).await?;
    assert_eq!(second["success"], false);
    assert!(second["message"].as_str().unwrap_or_default().contains("already exists"));
    Ok(())
}

#[tokio::test]
async fn test_auth_login_success() -> anyhow::Result<()> {
    let client = TestClient::new(&spawn_app().await);
    let username = generate_test_username("loginuser");
    let password = "SecurePass123!";
    client.post("/api/auth/register", &json!({"username": username, "password": password, "email": format!("{}@example.com", username)})).await?;
    let res = client.post("/api/auth/login", &json!({"username": username, "password": password})).await?;
    assert_eq!(res["success"], true);
    assert_eq!(res["data"]["roles"][0], "user");
    Ok(())
}

#[tokio::test]
async fn test_auth_login_invalid_credentials() -> anyhow::Result<()> {
    let client = TestClient::new(&spawn_app().await);
    let username = generate_test_username("badlogin");
    client.post("/api/auth/register", &json!({"username": username, "password": "SecurePass123!", "email": format!("{}@example.com", username)})).await?;
    let res = client.post("/api/auth/login", &json!({"username": username, "password": "WrongPassword123!"})).await?;
    assert_eq!(res["success"], false);
    assert!(res["message"].as_str().unwrap_or_default().contains("Invalid credentials"));
    Ok(())
}

#[tokio::test]
async fn test_auth_login_nonexistent_user() -> anyhow::Result<()> {
    let client = TestClient::new(&spawn_app().await);
    let res = client.post("/api/auth/login", &json!({"username": "nonexistent_user_12345", "password": "SomePassword123!"})).await?;
    assert_eq!(res["success"], false);
    assert!(res["message"].as_str().unwrap_or_default().contains("not found"));
    Ok(())
}

#[tokio::test]
async fn test_auth_complete_flow() -> anyhow::Result<()> {
    let client = TestClient::new(&spawn_app().await);
    let username = generate_test_username("flowuser");
    let password = "CompleteFlow123!";
    let reg = client.post("/api/auth/register", &json!({"username": username, "password": password, "email": format!("{}@example.com", username)})).await?;
    assert_eq!(reg["success"], true);
    assert_eq!(reg["data"]["roles"][0], "user");
    let login = client.post("/api/auth/login", &json!({"username": username, "password": password})).await?;
    assert_eq!(login["success"], true);
    assert_eq!(login["data"]["roles"][0], "user");
    Ok(())
}