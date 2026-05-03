// HTTP service — API response parsing — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

use crate::api::response::err;
use crate::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use burncloud_database::sqlx::Row;
use burncloud_service_router_log::{RouterLog, RouterLogService};
use serde::{Deserialize, Serialize};

// ── DTOs ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct SecuritySummaryResponse {
    success: bool,
    score: u8,
    blocked_count: u64,
    threat_source_count: u64,
    sparkline: Vec<u64>,
}

#[derive(Serialize)]
struct RiskEvent {
    id: i64,
    time: String,
    source: String,
    target: String,
    event_type: String,
    severity: String,
    status: String,
    detail: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FilterConfig {
    pub content_filter_enabled: bool,
    pub blacklist_enabled: bool,
    pub custom_rules: Vec<String>,
}

impl Default for FilterConfig {
    fn default() -> Self {
        FilterConfig {
            content_filter_enabled: true,
            blacklist_enabled: true,
            custom_rules: Vec::new(),
        }
    }
}

#[derive(Serialize)]
struct FilterConfigResponse {
    success: bool,
    content_filter_enabled: bool,
    blacklist_enabled: bool,
    custom_rules: Vec<String>,
}

impl From<FilterConfig> for FilterConfigResponse {
    fn from(c: FilterConfig) -> Self {
        FilterConfigResponse {
            success: true,
            content_filter_enabled: c.content_filter_enabled,
            blacklist_enabled: c.blacklist_enabled,
            custom_rules: c.custom_rules,
        }
    }
}

#[derive(Debug, Deserialize)]
struct EmergencyBreakRequest {
    reason: String,
}

#[derive(Debug, Deserialize)]
struct EventQueryParams {
    page: Option<i32>,
    page_size: Option<i32>,
}

#[derive(Serialize)]
struct EventPageResponse {
    success: bool,
    data: Vec<RiskEvent>,
    total: i64,
    page: i32,
    page_size: i32,
}

// ── Routes ────────────────────────────────────────────────────────────────

pub fn security_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/console/api/monitor/security",
            get(security_summary),
        )
        .route(
            "/console/api/monitor/security/events",
            get(security_events),
        )
        .route(
            "/console/api/monitor/security/filters",
            get(security_filters_get).put(security_filters_put),
        )
        .route(
            "/console/api/monitor/security/emergency-circuit-break",
            post(security_emergency_circuit_break),
        )
        .route(
            "/console/api/monitor/security/circuit-breaker-status",
            get(security_circuit_breaker_status),
        )
        .layer(axum::middleware::from_fn(
            crate::api::auth::auth_middleware,
        ))
}

// ── Helpers ───────────────────────────────────────────────────────────────

/// Call a router internal endpoint and return the JSON response body.
async fn call_router_internal(path: &str) -> Result<serde_json::Value, String> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);
    let url = format!("http://127.0.0.1:{port}{path}");

    let client = reqwest::Client::new();
    let mut req = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5));

    if let Ok(secret) = std::env::var("BURNCLOUD_INTERNAL_SECRET") {
        req = req.header("X-Internal-Secret", secret);
    }

    let resp = req.send().await.map_err(|e| format!("router call failed: {e}"))?;

    if resp.status().is_success() {
        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| format!("router response parse failed: {e}"))
    } else {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        Err(format!("router returned {status}: {text}"))
    }
}

/// POST to a router internal endpoint with a JSON body.
async fn post_router_internal(
    path: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);
    let url = format!("http://127.0.0.1:{port}{path}");

    let client = reqwest::Client::new();
    let mut req = client
        .post(&url)
        .json(body)
        .timeout(std::time::Duration::from_secs(10));

    if let Ok(secret) = std::env::var("BURNCLOUD_INTERNAL_SECRET") {
        req = req.header("X-Internal-Secret", secret);
    }

    let resp = req.send().await.map_err(|e| format!("router call failed: {e}"))?;

    if resp.status().is_success() {
        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| format!("router response parse failed: {e}"))
    } else {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        Err(format!("router returned {status}: {text}"))
    }
}

/// Derive a security score (0–100) from 4xx/5xx ratio in router logs.
fn compute_security_score(logs: &[RouterLog]) -> (u8, u64, u64) {
    let total = logs.len().max(1) as u64;
    let mut blocked = 0u64;
    let mut threat_sources = std::collections::HashSet::new();

    for log in logs {
        if log.status_code >= 400 {
            blocked += 1;
            if let Some(ref uid) = log.user_id {
                threat_sources.insert(uid.clone());
            }
            if let Some(ref up) = log.upstream_id {
                threat_sources.insert(up.clone());
            }
        }
    }

    let error_ratio = blocked as f64 / total as f64;
    let score = ((1.0 - error_ratio) * 100.0).round() as u8;

    (score, blocked, threat_sources.len() as u64)
}

/// Build a 7-day sparkline from router logs (one point per day).
fn compute_sparkline(logs: &[RouterLog]) -> Vec<u64> {
    let mut buckets: [u64; 7] = [0; 7];
    let now = chrono::Utc::now();

    for log in logs {
        if let Some(ref ts_str) = log.created_at {
            if let Ok(ts) = chrono::NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d %H:%M:%S") {
                let dt = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                    ts,
                    chrono::Utc,
                );
                let days_ago = (now - dt).num_days().clamp(0, 6) as usize;
                buckets[6 - days_ago] += 1;
            }
        }
    }

    buckets.to_vec()
}

/// Convert a RouterLog with status >= 400 into a RiskEvent.
fn log_to_risk_event(log: &RouterLog) -> RiskEvent {
    let code = log.status_code;
    let severity = if code >= 500 { "critical" } else { "warning" };
    let event_type = if code >= 500 {
        "server_error"
    } else {
        "client_error"
    };
    RiskEvent {
        id: log.id,
        time: log.created_at.clone().unwrap_or_default(),
        source: log.user_id.clone().unwrap_or_else(|| "-".into()),
        target: log
            .upstream_id
            .clone()
            .unwrap_or_else(|| log.path.clone()),
        event_type: event_type.into(),
        severity: severity.into(),
        status: if code >= 500 {
            "active".into()
        } else {
            "blocked".into()
        },
        detail: format!("HTTP {code}"),
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────

/// GET /console/api/monitor/security
///
/// Aggregate security summary: score based on 4xx/5xx ratio in router_logs,
/// blocked count, threat source count, and a 7-day sparkline.
#[tracing::instrument(skip_all)]
async fn security_summary(State(state): State<AppState>) -> impl IntoResponse {
    match RouterLogService::get(&state.db, 1000, 0).await {
        Ok(logs) => {
            let (score, blocked_count, threat_source_count) = compute_security_score(&logs);
            let sparkline = compute_sparkline(&logs);
            Json(SecuritySummaryResponse {
                success: true,
                score,
                blocked_count,
                threat_source_count,
                sparkline,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("security_summary: failed to query router_logs: {e}");
            err("Failed to query router logs").into_response()
        }
    }
}

/// GET /console/api/monitor/security/events
///
/// Paginated list of risk events derived from 4xx/5xx router logs.
#[tracing::instrument(skip(state))]
async fn security_events(
    State(state): State<AppState>,
    Query(params): Query<EventQueryParams>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    match RouterLogService::get(&state.db, 5000, 0).await {
        Ok(logs) => {
            let events: Vec<RiskEvent> = logs
                .iter()
                .filter(|l| l.status_code >= 400)
                .map(log_to_risk_event)
                .collect();

            let total = events.len() as i64;
            let offset = ((page - 1) * page_size) as usize;
            let page_items: Vec<RiskEvent> = events
                .into_iter()
                .skip(offset)
                .take(page_size as usize)
                .collect();

            Json(EventPageResponse {
                success: true,
                data: page_items,
                total,
                page,
                page_size,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("security_events: failed to query router_logs: {e}");
            err("Failed to query router logs").into_response()
        }
    }
}

/// GET /console/api/monitor/security/filters
///
/// Read the security filter configuration from settings KV.
async fn security_filters_get(State(state): State<AppState>) -> impl IntoResponse {
    match state
        .db
        .query_with_params(
            "SELECT * FROM sys_settings WHERE name = ?1",
            vec!["security_filters".to_string()],
        )
        .await
    {
        Ok(rows) => {
            if rows.is_empty() {
                Json(FilterConfigResponse::from(FilterConfig::default())).into_response()
            } else {
                let val: String = rows[0]
                    .try_get("value")
                    .unwrap_or_default();
                let config: FilterConfig =
                    serde_json::from_str(&val).unwrap_or_default();
                Json(FilterConfigResponse::from(config)).into_response()
            }
        }
        Err(e) => {
            tracing::error!("security_filters_get: failed to read settings: {e}");
            err("Failed to read filter config").into_response()
        }
    }
}

/// PUT /console/api/monitor/security/filters
///
/// Update the security filter configuration in settings KV.
async fn security_filters_put(
    State(state): State<AppState>,
    Json(config): Json<FilterConfig>,
) -> impl IntoResponse {
    let json = match serde_json::to_string(&config) {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("security_filters_put: serialization failed: {e}");
            return err("Failed to serialize filter config").into_response();
        }
    };

    if let Err(e) = state
        .db
        .execute_query_with_params(
            "INSERT OR REPLACE INTO sys_settings (name, value) VALUES (?1, ?2)",
            vec!["security_filters".to_string(), json],
        )
        .await
    {
        tracing::error!("security_filters_put: failed to write settings: {e}");
        return err("Failed to save filter config").into_response();
    }

    Json(FilterConfigResponse::from(config)).into_response()
}

/// POST /console/api/monitor/security/emergency-circuit-break
///
/// Proxy to router's internal trip-all endpoint. Requires a reason.
#[tracing::instrument(skip(_state), fields(reason = %body.reason))]
async fn security_emergency_circuit_break(
    State(_state): State<AppState>,
    Json(body): Json<EmergencyBreakRequest>,
) -> impl IntoResponse {
    if body.reason.trim().is_empty() {
        return err("Reason is required for emergency circuit break").into_response();
    }

    tracing::warn!(
        "SECURITY: Emergency circuit break triggered — reason: \"{}\"",
        body.reason
    );

    match post_router_internal(
        "/console/internal/circuit-breaker/trip-all",
        &serde_json::json!({}),
    )
    .await
    {
        Ok(data) => {
            let mut resp = serde_json::Map::new();
            resp.insert("success".into(), serde_json::Value::Bool(true));
            resp.insert("data".into(), data);
            Json(serde_json::Value::Object(resp)).into_response()
        }
        Err(e) => {
            tracing::error!("emergency_circuit_break: {e}");
            err("Failed to trigger emergency circuit break").into_response()
        }
    }
}

/// GET /console/api/monitor/security/circuit-breaker-status
///
/// Proxy to router's internal health endpoint to get circuit breaker states.
#[tracing::instrument(skip_all)]
async fn security_circuit_breaker_status(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    match call_router_internal("/console/internal/health").await {
        Ok(data) => {
            let mut resp = serde_json::Map::new();
            resp.insert("success".into(), serde_json::Value::Bool(true));
            resp.insert("data".into(), data);
            Json(serde_json::Value::Object(resp)).into_response()
        }
        Err(e) => {
            tracing::error!("circuit_breaker_status: {e}");
            err("Failed to get circuit breaker status").into_response()
        }
    }
}
