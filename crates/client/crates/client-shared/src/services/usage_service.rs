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
    pub amount: f64,
    pub description: Option<String>,
    pub created_at: Option<String>,
}

pub struct UsageService;

impl UsageService {
    pub async fn list_recharges(user_id: &str) -> Result<Vec<Recharge>, String> {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let url = format!(
            "http://127.0.0.1:{}/console/api/user/recharges?user_id={}",
            port, user_id
        );

        let client = reqwest::Client::new();
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;

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

    pub async fn get_user_usage(user_id: &str) -> Result<UsageStats, String> {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let url = format!("http://127.0.0.1:{}/console/api/usage/{}", port, user_id);

        let client = reqwest::Client::new();
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("API Error: {}", resp.status()));
        }

        resp.json::<UsageStats>().await.map_err(|e| e.to_string())
    }
}
