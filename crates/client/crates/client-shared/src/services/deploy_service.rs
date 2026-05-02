use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployRequest {
    #[serde(rename = "type")]
    pub type_: i32,
    pub key: String,
    pub name: String,
    pub models: String,
    #[serde(default = "default_group")]
    pub group: String,
    #[serde(default = "default_weight")]
    pub weight: i32,
    #[serde(default = "default_priority")]
    pub priority: i64,
}

fn default_group() -> String {
    "default".to_string()
}
fn default_weight() -> i32 {
    1
}
fn default_priority() -> i64 {
    1
}

pub struct DeployService;

impl DeployService {
    fn get_base_url() -> String {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        format!("http://127.0.0.1:{}/console/api/channel", port)
    }

    pub async fn deploy(req: &DeployRequest) -> Result<i32, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        let resp = client
            .post(Self::get_base_url())
            .json(req)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Deploy failed: {}", text));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let id = json
            .get("data")
            .and_then(|d| d.get("id"))
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "Invalid response: missing channel id".to_string())?;

        Ok(id as i32)
    }
}
