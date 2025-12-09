use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct User {
    #[serde(default)]
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub role: String, // "admin", "user", etc.
    #[serde(default)]
    pub balance: f64, // Remaining quota
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub status: i32, // 1: Active, 0: Disabled
    #[serde(default)]
    pub created_at: String,
}

pub struct UserService;

impl UserService {
    fn get_base_url() -> String {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        format!("http://127.0.0.1:{}/console/api", port)
    }

    pub async fn list() -> Result<Vec<User>, String> {
        let url = format!("{}/list_users", Self::get_base_url());
        println!("UserService: Fetching users from {}", url);
        let client = reqwest::Client::new();
        // In a real app, we need to attach the Bearer token here
        // For now, we assume the backend might be open or we use a root token from a global state
        // But since this is client-side code running in LiveView (server-side),
        // we might need a way to pass the current user's context.
        // For MVP, we'll try a simple GET.

        let resp = client.get(&url).send().await.map_err(|e| {
            let err = e.to_string();
            println!("UserService: Network error: {}", err);
            err
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            println!("UserService: API Error status: {}", status);
            return Err(format!("API Error: {}", status));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

        if let Some(data) = json.get("data") {
            serde_json::from_value(data.clone()).map_err(|e| e.to_string())
        } else {
            Ok(vec![])
        }
    }

    pub async fn create(username: &str, password: &str) -> Result<(), String> {
        let url = format!("{}/user/register", Self::get_base_url()); // Corrected path
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "username": username,
            "password": password
        });

        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("Create failed: {}", resp.status()));
        }
        Ok(())
    }

    pub async fn topup(user_id: &str, amount: f64) -> Result<f64, String> {
        let url = format!("{}/user/topup", Self::get_base_url());
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "user_id": user_id,
            "amount": amount
        });

        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("Topup failed: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let new_balance = json
            .get("data")
            .and_then(|d| d.get("balance"))
            .and_then(|b| b.as_f64())
            .unwrap_or(0.0);

        Ok(new_balance)
    }

    // Add update/delete/quota methods as needed
}
