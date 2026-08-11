use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};

use crate::backend::{server_root, Channel, ClientState};

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

/// Update a provider without erasing L2 shaper reservation thresholds that are
/// present in the current server ChannelDto but not editable in this UI yet.
///
/// The current server update handler also forces `status = 1`. To avoid silently
/// reactivating a down/inactive provider, edits are rejected unless the current
/// channel is active. This can be relaxed once the server supports preserve-status
/// update semantics.
pub async fn update_channel_preserving_reservations(channel: &Channel) -> Result<(), String> {
    if channel.id <= 0 {
        return Err("Channel id is required for update".to_string());
    }

    let get_request = authenticated(Client::new().get(url(&format!("/console/api/channel/{}", channel.id))))?;
    let current: EnvelopeValue = response_json(get_request).await?;
    if !current.success {
        return Err(current.message.unwrap_or_else(|| "Unable to load current channel before update".to_string()));
    }

    let current_status = current.data.get("status").and_then(|value| value.as_i64()).unwrap_or(1);
    if current_status != 1 {
        return Err(
            "This provider is inactive/down. The current BurnCloud PUT /console/api/channel handler would implicitly reactivate it, so this client refuses the edit to preserve routing state."
                .to_string(),
        );
    }

    let reservation_green = current.data.get("reservation_green").cloned().unwrap_or(serde_json::Value::Null);
    let reservation_yellow = current.data.get("reservation_yellow").cloned().unwrap_or(serde_json::Value::Null);
    let reservation_red = current.data.get("reservation_red").cloned().unwrap_or(serde_json::Value::Null);

    let payload = serde_json::json!({
        "id": channel.id,
        "type": channel.type_,
        "key": channel.key,
        "name": channel.name,
        "base_url": channel.base_url,
        "models": channel.models,
        "group": channel.group,
        "weight": channel.weight,
        "priority": channel.priority,
        "param_override": channel.param_override,
        "header_override": channel.header_override,
        "api_version": channel.api_version,
        "model_mapping": channel.model_mapping,
        "rpm_cap": channel.rpm_cap,
        "tpm_cap": channel.tpm_cap,
        "reservation_green": reservation_green,
        "reservation_yellow": reservation_yellow,
        "reservation_red": reservation_red,
    });

    let put_request = authenticated(Client::new().put(url("/console/api/channel")))?.json(&payload);
    let updated: EnvelopeValue = response_json(put_request).await?;
    if updated.success {
        Ok(())
    } else {
        Err(updated.message.unwrap_or_else(|| "Provider update failed".to_string()))
    }
}
