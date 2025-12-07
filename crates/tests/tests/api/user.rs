use burncloud_tests::TestClient;
use crate::common::spawn_app;
use serde_json::json;

#[tokio::test]
async fn test_user_management_lifecycle() -> anyhow::Result<()> {
    // 1. Start Server
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url);
    
    // 2. Register User
    let username = format!("testuser-{}", uuid::Uuid::new_v4());
    let password = "password123";
    let body = json!({
        "username": username,
        "password": password,
        "email": format!("{}@example.com", username)
    });
    
    let res = client.post("/console/api/user/register", &body).await?;
    assert!(res["success"].as_bool().unwrap_or(false));
    let user_id = res["data"]["id"].as_str().unwrap();
    
    // 3. Login
    let login_body = json!({
        "username": username,
        "password": password
    });
    let login_res = client.post("/console/api/user/login", &login_body).await?;
    assert!(login_res["success"].as_bool().unwrap_or(false));
    assert_eq!(login_res["data"]["username"], username);
    assert!(!login_res["data"]["token"].is_null());
    
    // 4. List Users (Should contain the new user)
    // List users might require admin permission in future, but currently open.
    let list_res = client.get("/console/api/user").await?;
    assert!(list_res["success"].as_bool().unwrap_or(false));
    let users = list_res["data"].as_array().unwrap();
    
    let found = users.iter().any(|u| u["username"] == username);
    assert!(found, "Newly registered user not found in list");
    
    Ok(())
}
