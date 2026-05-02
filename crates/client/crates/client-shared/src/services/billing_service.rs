// HTTP service — API response parsing — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone, PartialEq, Default, Serialize)]
pub struct BillingModelSummary {
    pub model: String,
    #[serde(default)]
    pub requests: i64,
    #[serde(default)]
    pub prompt_tokens: i64,
    #[serde(default)]
    pub cache_read_tokens: i64,
    #[serde(default)]
    pub completion_tokens: i64,
    #[serde(default)]
    pub reasoning_tokens: i64,
    #[serde(default)]
    pub cost_usd: f64,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default, Serialize)]
pub struct BillingSummary {
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    #[serde(default)]
    pub pre_migration_requests: i64,
    #[serde(default)]
    pub models: Vec<BillingModelSummary>,
    #[serde(default)]
    pub total_cost_usd: f64,
}

pub struct BillingService;

impl BillingService {
    /// Fetch billing summary from the public `/api/billing/summary` endpoint.
    /// `token` is the JWT Bearer token for authentication.
    pub async fn get_billing_summary(token: &str) -> Result<BillingSummary, String> {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let url = format!("http://127.0.0.1:{}/api/billing/summary", port);

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
            Ok(BillingSummary::default())
        }
    }
}
