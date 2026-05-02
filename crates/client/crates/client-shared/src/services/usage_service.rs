// HTTP service — API response parsing — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone, PartialEq, Default, Serialize)]
pub struct UsageStats {
    #[serde(default)]
    pub prompt_tokens: i64,
    #[serde(default)]
    pub completion_tokens: i64,
    #[serde(default)]
    pub total_tokens: i64,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default, Serialize)]
pub struct Recharge {
    pub id: i32,
    pub user_id: String,
    /// Amount in nanodollars (9 decimal precision)
    pub amount: i64,
    pub description: Option<String>,
    pub created_at: Option<String>,
}

pub struct UsageService;

impl UsageService {
    /// List recharge records for the authenticated user.
    /// `token` is the JWT Bearer token for authentication.
    /// The server extracts user_id from the JWT claims, so no user_id parameter is needed.
    pub async fn list_recharges(token: &str) -> Result<Vec<Recharge>, String> {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let url = format!("http://127.0.0.1:{}/console/api/user/recharges", port);

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("API Error: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        if let Some(data) = json.get("data") {
            serde_json::from_value(data.clone()).map_err(|e| e.to_string())
        } else {
            Ok(vec![])
        }
    }

    /// Get usage statistics for a specific user.
    /// `token` is the JWT Bearer token for authentication.
    pub async fn get_user_usage(user_id: &str, token: &str) -> Result<UsageStats, String> {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let url = format!("http://127.0.0.1:{}/console/api/usage/{}", port, user_id);

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("API Error: {}", resp.status()));
        }

        resp.json::<UsageStats>().await.map_err(|e| e.to_string())
    }
}
