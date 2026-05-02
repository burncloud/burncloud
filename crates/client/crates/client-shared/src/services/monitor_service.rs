// HTTP service — API response parsing — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

use serde::{Deserialize, Serialize};

// ── Existing DTOs ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub usage_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub usage_percent: f32,
    pub core_count: usize,
    pub frequency: u64,
    pub brand: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub usage_percent: f32,
    pub mount_point: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
    pub timestamp: u64,
}

// ── Security DTOs ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecuritySummary {
    pub score: u8,
    pub blocked_count: u64,
    pub threat_source_count: u64,
    pub sparkline: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RiskEvent {
    pub id: i64,
    pub time: String,
    pub source: String,
    pub target: String,
    pub event_type: String,
    pub severity: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RiskEventPage {
    pub data: Vec<RiskEvent>,
    pub total: i64,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilterConfig {
    pub content_filter_enabled: bool,
    pub blacklist_enabled: bool,
    #[serde(default)]
    pub custom_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CircuitBreakerStatus {
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyBreakRequest {
    pub reason: String,
}

// ── Helper ────────────────────────────────────────────────────────────────

fn get_port() -> String {
    std::env::var("PORT").unwrap_or_else(|_| "3000".to_string())
}

fn extract_data<T: serde::de::DeserializeOwned>(json: &serde_json::Value) -> Result<T, String> {
    if let Some(data) = json.get("data") {
        serde_json::from_value(data.clone()).map_err(|e| e.to_string())
    } else {
        Err("No data field in response".to_string())
    }
}

/// Extract data from a response that may use either `{ data: T }` or flat `T` format.
fn extract_data_or_flat<T: serde::de::DeserializeOwned>(json: &serde_json::Value) -> Result<T, String> {
    if let Some(data) = json.get("data") {
        serde_json::from_value(data.clone()).map_err(|e| e.to_string())
    } else {
        serde_json::from_value(json.clone()).map_err(|e| e.to_string())
    }
}

// ── MonitorService ────────────────────────────────────────────────────────

pub struct MonitorService;

impl MonitorService {
    // ── Existing methods ──────────────────────────────────────────────

    pub async fn get_system_metrics() -> Result<SystemMetrics, String> {
        let port = get_port();
        let url = format!("http://127.0.0.1:{}/console/api/monitor", port);

        let client = reqwest::Client::new();
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("API Error: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        extract_data(&json)
    }

    // ── Security API methods ──────────────────────────────────────────

    pub async fn get_security_summary() -> Result<SecuritySummary, String> {
        let port = get_port();
        let url = format!("http://127.0.0.1:{}/console/api/monitor/security", port);

        let client = reqwest::Client::new();
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("API Error: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        extract_data_or_flat(&json)
    }

    pub async fn list_risk_events(page: i32, page_size: i32) -> Result<RiskEventPage, String> {
        let port = get_port();
        let url = format!(
            "http://127.0.0.1:{}/console/api/monitor/security/events?page={page}&page_size={page_size}",
            port
        );

        let client = reqwest::Client::new();
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("API Error: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        extract_data(&json)
    }

    pub async fn get_filter_config() -> Result<FilterConfig, String> {
        let port = get_port();
        let url = format!("http://127.0.0.1:{}/console/api/monitor/security/filters", port);

        let client = reqwest::Client::new();
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("API Error: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        extract_data_or_flat(&json)
    }

    pub async fn update_filter_config(config: &FilterConfig) -> Result<FilterConfig, String> {
        let port = get_port();
        let url = format!("http://127.0.0.1:{}/console/api/monitor/security/filters", port);

        let client = reqwest::Client::new();
        let resp = client
            .put(&url)
            .json(config)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("API Error: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        extract_data_or_flat(&json)
    }

    pub async fn emergency_circuit_break(reason: String) -> Result<serde_json::Value, String> {
        let port = get_port();
        let url = format!(
            "http://127.0.0.1:{}/console/api/monitor/security/emergency-circuit-break",
            port
        );

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .json(&EmergencyBreakRequest { reason })
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("API Error: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        extract_data_or_flat(&json)
    }

    pub async fn get_circuit_breaker_status() -> Result<CircuitBreakerStatus, String> {
        let port = get_port();
        let url = format!(
            "http://127.0.0.1:{}/console/api/monitor/security/circuit-breaker-status",
            port
        );

        let client = reqwest::Client::new();
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("API Error: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        extract_data(&json)
    }
}