use serde::{Deserialize, Serialize};
use crate::api_client::ApiClient; // Assuming we have a generic client or use reqwest directly for now

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub id: String,
    pub username: String,
    pub roles: Vec<String>,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResult {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<LoginResponse>,
}

pub struct AuthService;

impl AuthService {
    pub async fn login(username: &str, password: &str) -> Result<LoginResponse, String> {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let url = format!("http://127.0.0.1:{}/console/api/user/login", port);
        
        let body = serde_json::json!({
            "username": username,
            "password": password
        });

        let client = reqwest::Client::new();
        let resp = client.post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let json: AuthResult = resp.json().await.map_err(|e| e.to_string())?;

        if json.success {
            if let Some(data) = json.data {
                // TODO: Save token to LocalStorage (if web) or Memory (if desktop)
                // For now just return it
                Ok(data)
            } else {
                Err("No data received".to_string())
            }
        } else {
            Err(json.message.unwrap_or_else(|| "Login failed".to_string()))
        }
    }

    pub async fn register(username: &str, password: &str, email: Option<&str>) -> Result<String, String> {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let url = format!("http://127.0.0.1:{}/console/api/user/register", port);
        
        let body = serde_json::json!({
            "username": username,
            "password": password,
            "email": email
        });

        let client = reqwest::Client::new();
        let resp = client.post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

        if json["success"].as_bool().unwrap_or(false) {
            Ok("Registration successful".to_string())
        } else {
            Err(json["message"].as_str().unwrap_or("Registration failed").to_string())
        }
    }
}
