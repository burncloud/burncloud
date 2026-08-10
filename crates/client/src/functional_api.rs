use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};

use crate::backend::{server_root, ClientState};

fn url(path: &str) -> String {
    format!("{}{}", server_root(), path)
}

fn authenticated(request: RequestBuilder) -> Result<RequestBuilder, String> {
    ClientState::load()
        .auth_token
        .map(|token| request.header("Authorization", format!("Bearer {token}")))
        .ok_or_else(|| "No authenticated BurnCloud session".to_string())
}

async fn response_json<T: serde::de::DeserializeOwned>(request: RequestBuilder) -> Result<T, String> {
    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    let text = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("API request failed ({status}): {text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("Invalid API response: {e}; body={text}"))
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
pub struct SecuritySummary {
    #[serde(default)] pub success: bool,
    #[serde(default)] pub score: u8,
    #[serde(default)] pub blocked_count: u64,
    #[serde(default)] pub threat_source_count: u64,
    #[serde(default)] pub sparkline: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SecurityFilters {
    #[serde(default)] pub success: bool,
    #[serde(default)] pub content_filter_enabled: bool,
    #[serde(default)] pub blacklist_enabled: bool,
    #[serde(default)] pub custom_rules: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
pub struct RiskEvent {
    #[serde(default)] pub id: i64,
    #[serde(default)] pub time: String,
    #[serde(default)] pub source: String,
    #[serde(default)] pub target: String,
    #[serde(default)] pub event_type: String,
    #[serde(default)] pub severity: String,
    #[serde(default)] pub status: String,
    #[serde(default)] pub detail: String,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
pub struct RiskEventPage {
    #[serde(default)] pub success: bool,
    #[serde(default)] pub data: Vec<RiskEvent>,
    #[serde(default)] pub total: i64,
    #[serde(default)] pub page: i32,
    #[serde(default)] pub page_size: i32,
}

pub async fn security_summary() -> Result<SecuritySummary, String> {
    let request = authenticated(Client::new().get(url("/console/api/monitor/security")))?;
    response_json(request).await
}

pub async fn security_filters() -> Result<SecurityFilters, String> {
    let request = authenticated(Client::new().get(url("/console/api/monitor/security/filters")))?;
    response_json(request).await
}

pub async fn save_security_filters(filters: &SecurityFilters) -> Result<SecurityFilters, String> {
    let body = serde_json::json!({
        "content_filter_enabled": filters.content_filter_enabled,
        "blacklist_enabled": filters.blacklist_enabled,
        "custom_rules": filters.custom_rules,
    });
    let request = authenticated(Client::new().put(url("/console/api/monitor/security/filters")))?.json(&body);
    response_json(request).await
}

pub async fn risk_events() -> Result<RiskEventPage, String> {
    let request = authenticated(Client::new().get(url("/console/api/monitor/security/events?page=1&page_size=100")))?;
    response_json(request).await
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EnvelopeValue {
    #[serde(default)] pub success: bool,
    #[serde(default)] pub data: serde_json::Value,
    #[serde(default)] pub message: Option<String>,
}

pub async fn circuit_breaker_status() -> Result<serde_json::Value, String> {
    let request = authenticated(Client::new().get(url("/console/api/monitor/security/circuit-breaker-status")))?;
    let response: EnvelopeValue = response_json(request).await?;
    if response.success { Ok(response.data) } else { Err(response.message.unwrap_or_else(|| "Circuit breaker status request failed".to_string())) }
}

pub async fn emergency_circuit_break(reason: &str) -> Result<serde_json::Value, String> {
    let request = authenticated(Client::new().post(url("/console/api/monitor/security/emergency-circuit-break")))?
        .json(&serde_json::json!({ "reason": reason }));
    let response: EnvelopeValue = response_json(request).await?;
    if response.success { Ok(response.data) } else { Err(response.message.unwrap_or_else(|| "Emergency circuit break failed".to_string())) }
}

pub async fn cache_stats() -> Result<serde_json::Value, String> {
    let request = authenticated(Client::new().get(url("/console/api/cache/stats")))?;
    let response: EnvelopeValue = response_json(request).await?;
    if response.success { Ok(response.data) } else { Err(response.message.unwrap_or_else(|| "Cache stats request failed".to_string())) }
}

pub async fn clear_cache() -> Result<(), String> {
    let request = authenticated(Client::new().post(url("/console/api/cache/clear")))?;
    let response: EnvelopeValue = response_json(request).await?;
    if response.success { Ok(()) } else { Err(response.message.unwrap_or_else(|| "Cache clear failed".to_string())) }
}
