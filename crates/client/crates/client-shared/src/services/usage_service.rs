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

pub struct UsageService;

impl UsageService {
    pub async fn get_user_usage(user_id: &str) -> Result<UsageStats, String> {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let url = format!("http://127.0.0.1:{}/console/api/usage/{}", port, user_id);

        let client = reqwest::Client::new();
        let resp = client.get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("API Error: {}", resp.status()));
        }

        resp.json::<UsageStats>().await.map_err(|e| e.to_string())
    }
}
