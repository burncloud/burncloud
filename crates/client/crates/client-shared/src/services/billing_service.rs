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
    /// USD balance in nanodollars (9 decimal precision)
    #[serde(default)]
    pub balance_usd: i64,
    /// CNY balance in nanodollars (9 decimal precision)
    #[serde(default)]
    pub balance_cny: i64,
}

pub struct BillingService;

impl BillingService {
    fn get_base_url() -> String {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        format!("http://127.0.0.1:{}/console/api", port)
    }

    pub async fn get_billing_summary(
        user_id: &str,
        start: Option<&str>,
        end: Option<&str>,
    ) -> Result<BillingSummary, String> {
        let mut url = format!("{}/billing/summary?user_id={}", Self::get_base_url(), user_id);
        if let Some(s) = start {
            url.push_str(&format!("&start={}", s));
        }
        if let Some(e) = end {
            url.push_str(&format!("&end={}", e));
        }

        let client = reqwest::Client::new();
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("API Error: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        if let Some(data) = json.get("data") {
            serde_json::from_value(data.clone()).map_err(|e| e.to_string())
        } else {
            Err("Missing data field in response".to_string())
        }
    }
}