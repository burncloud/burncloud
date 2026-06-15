// Router core — LLM request/response handling — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

mod adaptor;
pub mod affinity;
mod aimd_limiter;
mod balancer;
pub mod channel_state;
mod circuit_breaker;
mod config;
pub mod exchange_rate;
mod limiter;
pub mod metrics;
pub mod model_router;
pub mod order_type;
pub mod passthrough;
pub mod price_sync;
pub mod rate_budget;
pub mod response_parser;
pub mod response_quality;
mod scheduler;
mod stream_peek;
mod state;
pub mod stream_parser;
pub mod token_counter;

/// Peek first chunk timeout (seconds). Used to detect immediate errors (auth, rate limit).
/// Set to same as request timeout - rely on TCP keepalive for server crash detection.
const PEEK_FIRST_CHUNK_TIMEOUT_SECS: u64 = 36000;

// ============================================================
// Empty Response Counter - Track consecutive empty responses
// ============================================================
use std::sync::RwLock as StdRwLock;
use std::collections::HashMap;

/// Maximum consecutive empty responses before marking as failure
const EMPTY_RESPONSE_THRESHOLD: u32 = 3;

/// Counter for tracking consecutive empty responses per channel.
/// Uses sliding window: only penalizes after threshold consecutive failures.
pub struct EmptyResponseCounter {
    counters: StdRwLock<HashMap<String, u32>>,
    threshold: u32,
}

impl EmptyResponseCounter {
    pub fn new() -> Self {
        Self {
            counters: StdRwLock::new(HashMap::new()),
            threshold: EMPTY_RESPONSE_THRESHOLD,
        }
    }

    /// Record an empty response. Returns true if threshold exceeded (should penalize).
    pub fn record_empty(&self, channel_id: &str) -> bool {
        let mut counters = self.counters.write().unwrap();
        let count = counters.entry(channel_id.to_string()).or_insert(0);
        *count += 1;
        let exceeded = *count >= self.threshold;
        
        // Warning when approaching threshold (threshold - 1)
        if *count == self.threshold - 1 {
            tracing::warn!(
                channel_id = channel_id,
                count = *count,
                threshold = self.threshold,
                "Empty response counter approaching threshold - consider monitoring channel"
            );
        } else if exceeded {
            tracing::warn!(
                channel_id = channel_id,
                count = *count,
                threshold = self.threshold,
                "Consecutive empty responses exceeded threshold, marking as failure"
            );
        } else {
            tracing::debug!(
                channel_id = channel_id,
                count = *count,
                threshold = self.threshold,
                "Empty response recorded, not yet at threshold"
            );
        }
        exceeded
    }

    /// Reset counter on successful response (non-empty).
    pub fn reset(&self, channel_id: &str) {
        let mut counters = self.counters.write().unwrap();
        if let Some(count) = counters.get_mut(channel_id) {
            if *count > 0 {
                tracing::debug!(
                    channel_id = channel_id,
                    previous_count = *count,
                    "Resetting empty response counter after successful response"
                );
                *count = 0;
            }
        }
    }

    /// Get current counter value for a channel (for monitoring/admin purposes).
    pub fn get_count(&self, channel_id: &str) -> u32 {
        let counters = self.counters.read().unwrap();
        counters.get(channel_id).copied().unwrap_or(0)
    }

    /// Force reset counter for a channel (admin override).
    /// Returns the previous count value.
    pub fn force_reset(&self, channel_id: &str) -> u32 {
        let mut counters = self.counters.write().unwrap();
        let previous_count = counters.remove(channel_id).unwrap_or(0);
        if previous_count > 0 {
            tracing::info!(
                channel_id = channel_id,
                previous_count = previous_count,
                "Admin forced reset of empty response counter"
            );
        }
        previous_count
    }

    /// Get all channels with non-zero counters (for monitoring).
    pub fn get_all_counts(&self) -> Vec<(String, u32)> {
        let counters = self.counters.read().unwrap();
        counters
            .iter()
            .filter(|(_, &count)| count > 0)
            .map(|(k, &v)| (k.clone(), v))
            .collect()
    }
}

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderValue, Method, StatusCode, Uri},
    response::Response,
    routing::post,
    Router,
};
use balancer::RoundRobinBalancer;
use burncloud_common::types::OpenAIChatRequest;
use burncloud_common::TrafficColor;
use burncloud_database::Database;
use burncloud_database_channel::ChannelProviderModel;
use burncloud_database_router::{
    CandidateInfo, FailoverAttempt, RouterDatabase, RouterLog, RouterRequestLog,
    RouterTokenValidationResult, RouterVideoTask, RouterVideoTaskModel, StoragePolicy,
};
use burncloud_service_billing::{
    get_parser, parse_chunk_or_default, parse_response_or_default, UnifiedTokenCounter,
};
use burncloud_service_user::UserService;
use channel_state::ChannelStateTracker;
use circuit_breaker::CircuitBreaker;
use config::{AuthType, Upstream};
use futures::stream::StreamExt;
use http_body_util::BodyExt;
use limiter::RateLimiter;
use model_router::ModelRouter;
use order_type::OrderType;
use reqwest::Client;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, RwLock};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

/// Video billing: resolution multiplier for 720p Seedance videos.
const SEEDANCE_RESOLUTION_WEIGHT_HD: i64 = 2;
const SEEDANCE_RESOLUTION_WEIGHT_SD: i64 = 1;
/// Default price sync interval (24 hours).
const DEFAULT_PRICE_SYNC_INTERVAL_SECS: u64 = 86400;
/// Video task polling timeout (10 minutes).
const VIDEO_TASK_TIMEOUT_SECS: u64 = 600;
/// Router log channel buffer capacity.
const LOG_CHANNEL_BUFFER: usize = 1000;
/// Circuit breaker failure threshold before opening.
const CIRCUIT_BREAKER_FAILURE_THRESHOLD: u32 = 5;
/// Circuit breaker cooldown duration after opening (seconds).
const CIRCUIT_BREAKER_COOLDOWN_SECS: u64 = 30;
/// Timeout for price sync trigger requests (seconds).
const PRICE_SYNC_TRIGGER_TIMEOUT_SECS: u64 = 60;
/// Default video duration for Veo billing when not specified in request (seconds).
const VEO_DEFAULT_DURATION_SECS: i64 = 8;
/// Default video sample count for Veo billing when not specified in request.
const VEO_DEFAULT_SAMPLE_COUNT: i64 = 1;
/// Default video duration for Seedance billing when not specified in request (seconds).
const SEEDANCE_DEFAULT_DURATION_SECS: i64 = 5;
/// Default resolution for Seedance billing when not specified in request.
const SEEDANCE_DEFAULT_RESOLUTION: &str = "720p";
/// Protocol identifier constants for upstream channels.
const PROTOCOL_OPENAI: &str = "openai";
const PROTOCOL_CLAUDE: &str = "claude";
const PROTOCOL_GEMINI: &str = "gemini";
const PROTOCOL_ZAI: &str = "zai";
/// SSE stream termination marker sent to clients.
const SSE_DONE_MARKER: &str = "data: [DONE]\n\n";
/// HTTP connection timeout (seconds). Time allowed for establishing TCP connection.
const HTTP_CONNECT_TIMEOUT_SECS: u64 = 30;
/// HTTP request timeout (seconds). Total time for request completion (600 minutes).
const HTTP_REQUEST_TIMEOUT_SECS: u64 = 36000;
/// HTTP pool idle timeout (seconds). Time before idle connections are closed.
const HTTP_POOL_IDLE_TIMEOUT_SECS: u64 = 90;
/// HTTP TCP keepalive interval (seconds).
const HTTP_TCP_KEEPALIVE_SECS: u64 = 30;

pub use scheduler::SchedulingRequest;
pub use state::AppState;

/// Data collected during request processing for router_request_logs table.
/// Populated by proxy_logic and sent asynchronously to the database.
#[derive(Debug, Clone, Default)]
struct RequestLogData {
    /// Request body (may be truncated/sanitized)
    request_body: Option<String>,
    request_body_truncated: bool,
    /// Sanitized request headers
    request_headers: Option<String>,
    /// Response body (may be truncated)
    response_body: Option<String>,
    response_body_truncated: bool,
    /// Candidate channels considered
    candidates: Vec<CandidateInfo>,
    /// Number of candidates (for quick queries)
    candidates_count: i32,
    /// Affinity cache key (session_id + model)
    affinity_key: Option<String>,
    /// Channel ID from affinity cache hit (if any)
    affinity_hit_channel_id: Option<i32>,
    /// Failover attempts with errors
    failover_history: Vec<FailoverAttempt>,
    /// Streaming response summary
    stream_chunk_count: u32,
    stream_first_chunk_latency_ms: Option<u64>,
    stream_last_chunk_latency_ms: Option<u64>,
}

/// Record a failover attempt to the request log data.
/// Call this before `continue` in the failover loop.
fn record_failover_attempt(
    log_data: &mut Option<RequestLogData>,
    attempt: u32,
    upstream: &Upstream,
    error: Option<&str>,
    latency_ms: u64,
) {
    if let Some(data) = log_data {
        data.failover_history.push(FailoverAttempt {
            attempt,
            channel_id: upstream.id.clone(),
            channel_name: upstream.name.clone(),
            error: error.map(|s| s.to_string()),
            latency_ms,
        });
    }
}

/// Named return type for [`proxy_logic`] — replaces the previous 8-tuple.
///
/// Each field is self-documenting; adding a new field only requires updating
/// this struct + the `RouterLog` construction site, not every return point.
struct ProxyResult {
    response: Response,
    upstream_id: Option<String>,
    final_status: StatusCode,
    pricing_region: Option<String>,
    video_task_id: Option<String>,
    /// L2 Shaper outcome: `(layer_decision, traffic_color)` for RouterLog.
    /// `None` when the request never reached the shaper (early routing error).
    shaper_outcome: Option<(String, String)>,
    /// L6 Observability: routing decision from `route_with_scheduler`.
    /// Used by the priority chain to compute `layer_decision` for RouterLog.
    routing_decision: Option<model_router::RoutingDecision>,
    /// L6 Observability: `SchedulingRequest.color` for `traffic_color` fallback.
    /// When `shaper_outcome` is `None` (no Shaper processing), `traffic_color`
    /// falls back to this value per the checklist requirement.
    sched_request_color: TrafficColor,
    /// Error classification for RouterLog: "upstream_error", "timeout",
    /// "auth_failed", "rate_limit", "router_reject", or None for success.
    error_type: Option<String>,
    /// Detailed request log data for router_request_logs table.
    /// Only populated when storage_policy != StoragePolicy::None.
    request_log_data: Option<RequestLogData>,
}

#[derive(serde::Deserialize)]
struct JwtClaims {
    sub: String,
    #[allow(dead_code)]
    exp: usize,
    #[allow(dead_code)]
    iat: usize,
}

/// Build a JSON error response body: `{"error": "<message>"}`.
fn json_error_body(message: impl std::fmt::Display) -> Body {
    Body::from(serde_json::json!({"error": message.to_string()}).to_string())
}

/// Record a channel error in both circuit breaker and channel state tracker.
///
/// Affinity eviction policy (P0-1): ServerError, Timeout, and ConnectionError
/// evict the affinity entry immediately so the next request re-picks via HRW
/// instead of pinning to a sick channel. AuthFailed, PaymentRequired,
/// RateLimited, and ModelNotFound do NOT evict — they are not upstream
/// health problems and the affined channel may still be the best choice.
fn record_upstream_failure(
    state: &AppState,
    upstream: &Upstream,
    model_name: Option<&str>,
    failure_type: FailureType,
    error_msg: &str,
    session_id: &str,
) {
    let channel_id: i32 = upstream.id.parse().unwrap_or(0);
    state
        .circuit_breaker
        .record_failure_with_type(&upstream.id, failure_type.clone());
    state
        .channel_state_tracker
        .record_error(channel_id, model_name, &failure_type, error_msg);
    // Immediate evict on upstream health failures (P0-1). These indicate the
    // channel is unhealthy right now; waiting for CB trip (5 failures) would
    // pin the user to a sick channel for too long.
    let should_evict = matches!(
        &failure_type,
        FailureType::ServerError
            | FailureType::Timeout
            | FailureType::ConnectionError
            | FailureType::EmptyResponse
    );
    if should_evict {
        if let Some(model) = model_name {
            state.affinity_cache.evict(session_id, model);
            tracing::debug!(
                session_id,
                model,
                channel_id,
                "Affinity evicted — upstream failure {:?} for {}",
                failure_type,
                upstream.name
            );
        }
    }
}

/// Record a successful upstream response in circuit breaker and affinity cache.
///
/// Extracted from the two success paths (passthrough + converted) to avoid
/// duplicating the `record_success` + `affinity_cache.insert` pattern (P0-1).
fn record_upstream_success(
    state: &AppState,
    upstream: &Upstream,
    model_name: Option<&str>,
    session_id: &str,
) {
    state.circuit_breaker.record_success(&upstream.id);
    let channel_id: i32 = upstream.id.parse().unwrap_or(0);
    if let Some(model) = model_name {
        state.affinity_cache.insert(session_id, model, channel_id);
    }
}

/// Check for empty response (zero tokens) and handle as failure if detected.
///
/// Returns `true` if empty response was detected (caller should continue to next candidate),
/// `false` if response has valid tokens (caller should proceed normally).
fn check_empty_response(
    state: &AppState,
    upstream: &Upstream,
    model_name: Option<&str>,
    session_id: &str,
    token_counter: &UnifiedTokenCounter,
) -> bool {
    let upstream_id_str = upstream.id.clone();
    let usage = token_counter.get_usage();
    if usage.is_empty() {
        tracing::warn!(
            channel_id = %upstream.id,
            model = ?model_name,
            "Empty response (zero tokens) detected, treating as failure"
        );

        // Record to sliding window counter for non-streaming responses
        let should_penalize = state.empty_response_counter.record_empty(&upstream_id_str);

        // Only record failure if threshold exceeded (sliding window logic)
        if should_penalize {
            record_upstream_failure(
                state,
                upstream,
                model_name,
                FailureType::EmptyResponse,
                "Empty response with zero tokens",
                session_id,
            );
        }

        // Note: We already called record_upstream_success earlier, which updated
        // affinity to this channel. The record_upstream_failure will evict affinity,
        // so the next request will not be stuck on this bad channel.
        true // Empty response detected
    } else {
        // Successful response - reset the counter
        state.empty_response_counter.reset(&upstream_id_str);
        false // Response has tokens
    }
}

/// Classify an upstream HTTP error into a [`FailureType`] for circuit breaker
/// and channel state tracking. Shared by passthrough and converted paths
/// (audit decision D12 — DRY).
fn classify_upstream_error(
    status: StatusCode,
    headers: &HeaderMap,
    error_info: &response_parser::ErrorInfo,
) -> FailureType {
    match status {
        StatusCode::UNAUTHORIZED => FailureType::AuthFailed,
        StatusCode::PAYMENT_REQUIRED => FailureType::PaymentRequired,
        StatusCode::TOO_MANY_REQUESTS => {
            let retry_after = headers
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
            let scope = error_info
                .scope
                .clone()
                .unwrap_or(circuit_breaker::RateLimitScope::Unknown);
            FailureType::RateLimited { scope, retry_after }
        }
        StatusCode::NOT_FOUND => FailureType::ModelNotFound,
        _ if status.is_server_error() => FailureType::ServerError,
        _ => FailureType::ServerError,
    }
}

// ============== Request Log Sanitization (Issue #334) ==============

/// Maximum size for request/response body logging (64KB).
/// Bodies larger than this are truncated.
const MAX_LOG_BODY_SIZE: usize = 64 * 1024;

/// Sensitive header names that should be redacted in logs.
const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "api-key",
    "x-api-key",
    "x-auth-token",
    "cookie",
    "set-cookie",
];

/// Sensitive JSON fields that should be redacted in request bodies.
const SENSITIVE_FIELDS: &[&str] = &[
    "api_key",
    "apiKey",
    "api-key",
    "key",
    "token",
    "password",
    "secret",
    "authorization",
];
/// Safely cut a string at a UTF-8 character boundary.
/// Returns a string slice that is at most max_bytes long.
fn safe_cut(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    
    // Find the character boundary at or before max_bytes
    let mut boundary = max_bytes;
    while boundary > 0 && !s.is_char_boundary(boundary) {
        boundary -= 1;
    }
    
    &s[..boundary]
}

/// Sanitize request body for logging: remove sensitive fields, truncate if too large.
/// Returns (sanitized_body, was_truncated).
fn sanitize_request_body(body_bytes: &[u8]) -> (Option<String>, bool) {
    if body_bytes.is_empty() {
        return (None, false);
    }

    // Try to parse as JSON for field redaction
    let body_str = String::from_utf8_lossy(body_bytes);

    // Check if body is too large
    let truncated = body_bytes.len() > MAX_LOG_BODY_SIZE;

    if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&body_str) {
        // Redact sensitive fields
        redact_sensitive_fields(&mut json);

        let sanitized = if truncated {
            // Truncate the JSON string representation
            let json_str = json.to_string();
            format!("{}... [TRUNCATED: {} bytes total]",
                safe_cut(&json_str, MAX_LOG_BODY_SIZE),
                body_bytes.len())
        } else {
            json.to_string()
        };
        (Some(sanitized), truncated)
    } else {
        // Not valid JSON, treat as plain text
        let sanitized = if truncated {
            format!("{}... [TRUNCATED: {} bytes total]",
                safe_cut(&body_str, MAX_LOG_BODY_SIZE),
                body_bytes.len())
        } else {
            body_str.to_string()
        };
        (Some(sanitized), truncated)
    }
}

/// Recursively redact sensitive fields in a JSON value.
fn redact_sensitive_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for key in map.keys().cloned().collect::<Vec<_>>() {
                let key_lower = key.to_lowercase();
                if SENSITIVE_FIELDS.iter().any(|f| key_lower.contains(&f.to_lowercase())) {
                    map.insert(key, serde_json::Value::String("***REDACTED***".to_string()));
                } else if let Some(nested) = map.get_mut(&key) {
                    redact_sensitive_fields(nested);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                redact_sensitive_fields(item);
            }
        }
        _ => {}
    }
}

/// Sanitize request headers for logging: remove sensitive headers.
fn sanitize_request_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    if headers.is_empty() {
        return None;
    }

    let mut sanitized_map = serde_json::Map::new();
    for (name, value) in headers {
        let name_str = name.as_str().to_lowercase();
        if SENSITIVE_HEADERS.iter().any(|h| name_str.contains(h)) {
            sanitized_map.insert(name.to_string(), serde_json::Value::String("***REDACTED***".to_string()));
        } else if let Ok(v) = value.to_str() {
            sanitized_map.insert(name.to_string(), serde_json::Value::String(v.to_string()));
        }
    }

    if sanitized_map.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(sanitized_map).to_string())
    }
}

/// Sanitize response body for logging: truncate if too large.
fn sanitize_response_body(body: &[u8]) -> (Option<String>, bool) {
    if body.is_empty() {
        return (None, false);
    }

    let truncated = body.len() > MAX_LOG_BODY_SIZE;
    let body_str = String::from_utf8_lossy(body);

    // Try to parse as JSON for prettier output
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body_str) {
        let json_str = json.to_string();
        if truncated {
            (Some(format!("{}... [TRUNCATED: {} bytes total]",
                safe_cut(&json_str, MAX_LOG_BODY_SIZE),
                body.len())), true)
        } else {
            (Some(json_str), false)
        }
    } else {
        if truncated {
            (Some(format!("{}... [TRUNCATED: {} bytes total]",
                safe_cut(&body_str, MAX_LOG_BODY_SIZE),
                body.len())), true)
        } else {
            (Some(body_str.to_string()), false)
        }
    }
}

/// Helper function to build a response safely without panicking.
/// Falls back to an empty body with the same status if body construction fails.
fn build_response(status: StatusCode, body: Body) -> Response {
    Response::builder()
        .status(status)
        .body(body)
        .unwrap_or_else(|_| {
            // Fallback: return a minimal response with the same status
            Response::builder()
                .status(status)
                .body(Body::empty())
                .unwrap_or_else(|_| Response::new(Body::empty()))
        })
}

/// Apply header_override JSON to a request builder.
fn apply_header_override(
    req_builder: reqwest::RequestBuilder,
    override_str: Option<&str>,
) -> reqwest::RequestBuilder {
    let Some(override_str) = override_str else {
        return req_builder;
    };
    if let Ok(header_map) =
        serde_json::from_str::<std::collections::HashMap<String, String>>(override_str)
    {
        let mut req_builder = req_builder;
        for (k, v) in header_map {
            req_builder = req_builder.header(k, v);
        }
        req_builder
    } else {
        req_builder
    }
}

/// Helper function to build a response with a header safely.
fn build_response_with_header(
    status: StatusCode,
    header_name: &str,
    header_value: &str,
    body: Body,
) -> Response {
    Response::builder()
        .status(status)
        .header(header_name, header_value)
        .body(body)
        .unwrap_or_else(|_| {
            Response::builder()
                .status(status)
                .body(Body::empty())
                .unwrap_or_else(|_| Response::new(Body::empty()))
        })
}

/// Startup helper: load every channel's L2 Shaper config (rpm_cap / tpm_cap /
/// reservation triple) from `channel_providers` and feed it into
/// [`rate_budget::InMemoryBudget`]. Channels with `rpm_cap = NULL` (or zero)
/// stay unconfigured — they'll fail-open at request time, and that count is
/// surfaced via `tracing::warn!` here and `fail_open_count` at runtime.
///
/// **Fail-open on error.** A DB outage or query failure is logged and ignored:
/// the router must boot even if the shaper config can't be loaded. Subsequent
/// `try_consume` calls all hit unconfigured-channel fail-open until the next
/// reload.
async fn configure_rate_budget_from_db(db: &Database, budget: &rate_budget::InMemoryBudget) {
    let conn = match db.get_connection() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(
                error = %e,
                "rate_budget: failed to acquire DB connection at startup — all channels will fail-open"
            );
            return;
        }
    };
    let pool = conn.pool();

    type ChannelCapRow = (
        i32,
        Option<i32>,
        Option<i64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
    );
    let rows: Vec<ChannelCapRow> = match burncloud_database::sqlx::query_as(
        "SELECT id, rpm_cap, tpm_cap, reservation_green, reservation_yellow, reservation_red \
             FROM channel_providers",
    )
    .fetch_all(pool)
    .await
    {
        Ok(rs) => rs,
        Err(e) => {
            tracing::error!(
                error = %e,
                "rate_budget: failed to query channel_providers at startup — all channels will fail-open"
            );
            return;
        }
    };

    let mut configured_count: usize = 0;
    let mut unconfigured_count: usize = 0;
    let mut sample_unconfigured_ids: Vec<i32> = Vec::new();

    for (id, rpm_cap, tpm_cap, res_g, res_y, res_r) in rows {
        match (rpm_cap, tpm_cap) {
            (Some(rpm), Some(tpm)) if rpm > 0 && tpm > 0 => {
                // Per-color shares: NULL → migration default (0.4/0.4/0.2).
                // `configure` itself validates sum-to-1.0 and falls back to
                // default if invalid (FM8 fix in subtask 4).
                let reservation = rate_budget::ChannelReservation {
                    green: res_g.unwrap_or(0.4),
                    yellow: res_y.unwrap_or(0.4),
                    red: res_r.unwrap_or(0.2),
                };
                budget.configure(id, rpm as u32, tpm as u64, reservation);
                configured_count += 1;
            }
            _ => {
                unconfigured_count += 1;
                if sample_unconfigured_ids.len() < 5 {
                    sample_unconfigured_ids.push(id);
                }
            }
        }
    }

    tracing::info!(
        configured_count,
        "rate_budget: loaded channel cap configs from channel_providers"
    );
    if unconfigured_count > 0 {
        tracing::warn!(
            unconfigured_count,
            sample_ids = ?sample_unconfigured_ids,
            "rate_budget: N channels lack rpm_cap, fail-open"
        );
    }
}

pub async fn create_router_app(
    db: Arc<Database>,
) -> anyhow::Result<(
    Router,
    Router,
    mpsc::Sender<tokio::sync::oneshot::Sender<price_sync::SyncResult>>,
)> {
    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(HTTP_CONNECT_TIMEOUT_SECS))
        .timeout(std::time::Duration::from_secs(HTTP_REQUEST_TIMEOUT_SECS))
        .pool_idle_timeout(std::time::Duration::from_secs(HTTP_POOL_IDLE_TIMEOUT_SECS))
        .tcp_keepalive(std::time::Duration::from_secs(HTTP_TCP_KEEPALIVE_SECS))
        .build()?;
    let balancer = Arc::new(RoundRobinBalancer::new());
    // Rate limiter: 100 burst, 10 requests/second
    let limiter = Arc::new(RateLimiter::new(100.0, 10.0));
    // Circuit breaker: 5 failure threshold, 30s cooldown
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        CIRCUIT_BREAKER_FAILURE_THRESHOLD,
        CIRCUIT_BREAKER_COOLDOWN_SECS,
    ));
    let model_router = Arc::new(ModelRouter::new(db.clone()));
    // Channel State Tracker for health monitoring
    let channel_state_tracker = Arc::new(ChannelStateTracker::new());
    // Dynamic Adaptor Factory for protocol adaptation
    let adaptor_factory = Arc::new(adaptor::factory::DynamicAdaptorFactory::new(db.clone()));
    // API Version Detector for handling deprecated versions
    let api_version_detector = Arc::new(adaptor::detector::ApiVersionDetector::new(db.clone()));
    // Price cache + cost calculator (loaded at startup; refreshed on POST /api/v1/prices)
    let price_cache = burncloud_service_billing::PriceCache::load(&db)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to load price cache at startup: {e} — using empty cache");
            burncloud_service_billing::PriceCache::empty()
        });
    let cost_calculator = burncloud_service_billing::CostCalculator::new(price_cache.clone());

    // Exchange Rate Service for multi-currency cost calculations
    let exchange_rate_service = Arc::new(exchange_rate::ExchangeRateService::new(db.clone()));
    if let Err(e) = exchange_rate_service.load_rates_from_db().await {
        tracing::warn!("Failed to load exchange rates at startup: {e}");
    }
    let exch_clone = exchange_rate_service.clone();
    tokio::spawn(async move {
        exch_clone.start_sync_task();
    });

    // Scheduler policies for multi-channel routing
    let scheduler_policies = Arc::new(RwLock::new(scheduler::load_scheduler_config()));

    // L3 Affinity flow cache (HRW + dual TTL: 5min sticky / 30min hard).
    let affinity_cache = Arc::new(affinity::AffinityCache::default());

    // L2 Shaper budget. MVP: in-memory, single-instance. Channels are
    // configured lazily via channel_providers reload (rpm_cap / tpm_cap /
    // reservation columns added in migration 0011).
    let rate_budget = Arc::new(rate_budget::InMemoryBudget::new());
    // Eager-load every channel's cap from DB at startup. DB failures here
    // are logged and ignored — router must boot even if shaper config is
    // unavailable. Channels with rpm_cap = NULL stay unconfigured (fail-open).
    configure_rate_budget_from_db(&db, rate_budget.as_ref()).await;
    // Counter for fail-open admissions (unconfigured channels). Surfaced
    // via /router/status so admins notice silently-permissive channels.
    let fail_open_count = Arc::new(AtomicU64::new(0));

    // Billing strict mode: read BILLING_STRICT_MODE env var (default: true).
    // When strict, preflight PriceNotFound returns 400; when not strict, only warns.
    let billing_strict = std::env::var("BILLING_STRICT_MODE")
        .map(|v| v != "false" && v != "0")
        .unwrap_or(true);
    let billing_preflight_rejected_count = Arc::new(AtomicU64::new(0));
    let billing_post_settle_price_missing_count = Arc::new(AtomicU64::new(0));

    // Request log storage policy: read REQUEST_LOG_STORAGE_POLICY env var (default: summary).
    // Values: "full" (complete request/response), "summary" (metadata only), "none" (disabled).
    let request_log_storage_policy = match std::env::var("REQUEST_LOG_STORAGE_POLICY")
        .unwrap_or_else(|_| "summary".to_string())
        .to_lowercase()
        .as_str()
    {
        "full" => StoragePolicy::Full,
        "none" => StoragePolicy::None,
        _ => StoragePolicy::Summary, // Default to summary for unknown values
    };
    tracing::info!(
        policy = ?request_log_storage_policy,
        "Request log storage policy configured"
    );

    // AIMD → InMemoryBudget feedback channel (capacity=1, latest-wins debounce).
    // When the adaptive limiter learns a new RPM limit, it sends an update here;
    // a background task reconfigures the budget bucket (audit decision D6/D10).
    let (budget_update_tx, mut budget_update_rx) = mpsc::channel::<state::BudgetUpdate>(1);
    {
        let rate_budget = rate_budget.clone();
        tokio::spawn(async move {
            tracing::info!("AIMD budget-update task started");
            while let Some(update) = budget_update_rx.recv().await {
                // Reconfigure the channel's RPM cap to the learned limit.
                // TPM cap stays unchanged (AIMD only learns RPM).
                if let Some(snapshot) = rate_budget.snapshot(update.channel_id) {
                    rate_budget.configure(
                        update.channel_id,
                        update.learned_limit,
                        snapshot.tpm_cap,
                        rate_budget::ChannelReservation::default(),
                    );
                    tracing::debug!(
                        channel_id = update.channel_id,
                        learned_rpm = update.learned_limit,
                        "AIMD feedback: reconfigured budget RPM cap"
                    );
                }
            }
        });
    }

    // Start background price sync task (every 24 hours)
    // Prices pulled from pricing_data repo (GitHub/Gitee fallback).
    // Price cache is refreshed after each successful sync.
    let interval_secs = std::env::var("PRICE_SYNC_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_PRICE_SYNC_INTERVAL_SECS);
    let (force_sync_tx, force_sync_rx) =
        tokio::sync::mpsc::channel::<tokio::sync::oneshot::Sender<price_sync::SyncResult>>(4);
    price_sync::start_price_sync_task(
        db.clone(),
        interval_secs,
        None,
        price_cache.clone(),
        force_sync_rx,
    );

    // Setup Async Logging Channel
    let (log_tx, mut log_rx) = mpsc::channel::<RouterLog>(LOG_CHANNEL_BUFFER);
    let db_for_logger = db.clone(); // Clone Arc

    // Spawn Logging Task
    tokio::spawn(async move {
        tracing::info!("Logging task started");
        while let Some(log) = log_rx.recv().await {
            // Need to create a new default database or use the shared one?
            // Since Database struct isn't thread-safe or Clone by default, we rely on Arc<Database>.
            // But RouterDatabase::insert_log takes &Database.
            if let Err(e) = RouterDatabase::insert_log(&db_for_logger, &log).await {
                tracing::error!("Failed to insert log: {}", e);
            }
        }
    });

    // Setup Async Request Log Channel (detailed request/response logging)
    let (request_log_tx, mut request_log_rx) = mpsc::channel::<RouterRequestLog>(LOG_CHANNEL_BUFFER);
    let db_for_request_logger = db.clone();

    // Spawn Request Logging Task
    tokio::spawn(async move {
        tracing::info!("Request logging task started");
        while let Some(log) = request_log_rx.recv().await {
            if let Err(e) = RouterDatabase::insert_request_log(&db_for_request_logger, &log).await {
                tracing::error!("Failed to insert request log: {}", e);
            }
        }
    });

    let state = AppState {
        client,
        db, // Arc<Database>
        balancer,
        limiter,
        circuit_breaker,
        log_tx,
        request_log_tx,
        model_router,
        channel_state_tracker,
        adaptor_factory,
        api_version_detector,
        price_cache,
        cost_calculator,
        force_sync_tx: force_sync_tx.clone(),
        exchange_rate_service,
        scheduler_policies,
        affinity_cache,
        rate_budget,
        fail_open_count,
        billing_strict,
        billing_preflight_rejected_count,
        billing_post_settle_price_missing_count,
        budget_update_tx,
        request_log_storage_policy,
        empty_response_counter: Arc::new(EmptyResponseCounter::new()),
        channel_health_manager: Arc::new(crate::channel_health_manager::ChannelHealthManager::new())
    };

    use burncloud_common::constants::INTERNAL_PREFIX;

    let health_path = format!("{}/health", INTERNAL_PREFIX);
    let price_sync_path = format!("{}/prices/sync", INTERNAL_PREFIX);
    let trip_all_path = format!("{}/circuit-breaker/trip-all", INTERNAL_PREFIX);
    let metrics_path = format!("{}/metrics", INTERNAL_PREFIX);

    // Internal routes that must be registered BEFORE LiveView's catch-all
    // `/console/{*path}` in the server layer, otherwise LiveView intercepts
    // them and returns HTML instead of JSON.
    let internal_app = Router::new()
        .route(&health_path, axum::routing::get(health_status_handler))
        .route(&price_sync_path, post(price_sync_handler))
        .route(&trip_all_path, post(circuit_breaker_trip_all_handler))
        .route(&metrics_path, axum::routing::get(metrics_handler))
        .with_state(state.clone());

    let app = Router::new()
        .route("/v1/models", axum::routing::get(models_handler))
        .route("/api/v1/usage", axum::routing::get(usage_handler))
        .route(
            "/api/v1/usage/models",
            axum::routing::get(usage_models_handler),
        )
        .fallback(proxy_handler)
        .layer(CorsLayer::permissive())
        .with_state(state);

    Ok((app, internal_app, force_sync_tx))
}

/// POST /console/internal/prices/sync
///
/// Triggers an immediate forced price sync. Waits up to 60 seconds for completion.
/// Internal-only; no auth required (server is assumed behind firewall).
async fn price_sync_handler(State(state): State<AppState>) -> Response {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    if state.force_sync_tx.send(reply_tx).await.is_err() {
        return build_response_with_header(
            StatusCode::SERVICE_UNAVAILABLE,
            "content-type",
            "application/json",
            Body::from(r#"{"error":{"message":"Price sync task is not running","type":"server_error"}}"#),
        );
    }
    match tokio::time::timeout(
        std::time::Duration::from_secs(PRICE_SYNC_TRIGGER_TIMEOUT_SECS),
        reply_rx,
    )
    .await
    {
        Ok(Ok(result)) => {
            let body = format!(
                r#"{{"models_synced":{},"currencies_synced":{},"tiers_synced":{},"errors":{},"source":"{}"}}"#,
                result.models_synced,
                result.currencies_synced,
                result.tiered_pricing_synced,
                result.errors,
                result.source,
            );
            build_response_with_header(
                StatusCode::OK,
                "content-type",
                "application/json",
                Body::from(body),
            )
        }
        Ok(Err(_)) => build_response_with_header(
            StatusCode::INTERNAL_SERVER_ERROR,
            "content-type",
            "application/json",
            Body::from(r#"{"error":{"message":"Price sync task dropped the reply channel","type":"server_error"}}"#),
        ),
        Err(_) => build_response_with_header(
            StatusCode::GATEWAY_TIMEOUT,
            "content-type",
            "application/json",
            Body::from(r#"{"error":{"message":"Price sync timed out after 60 seconds","type":"timeout_error"}}"#),
        ),
    }
}

/// POST /console/internal/circuit-breaker/trip-all
///
/// Emergency operation: forces all known upstream circuits into Open state.
/// Each tripped upstream will remain Open for the full cooldown duration
/// before transitioning to Half-Open. Returns the list of tripped upstream IDs.
async fn circuit_breaker_trip_all_handler(State(state): State<AppState>) -> Response {
    let tripped = state.circuit_breaker.trip_all();
    let body = serde_json::json!({
        "status": "all_circuits_tripped",
        "tripped_upstreams": tripped,
        "count": tripped.len(),
    });
    let json = serde_json::to_string(&body)
        .unwrap_or_else(|_| r#"{"status":"all_circuits_tripped"}"#.to_string());
    build_response_with_header(
        StatusCode::OK,
        "content-type",
        "application/json",
        Body::from(json),
    )
}

async fn models_handler(State(state): State<AppState>) -> Response {
    // Fetch all distinct models from channel_abilities
    // This shows models that have at least one enabled channel
    use burncloud_database_channel::ChannelAbilityModel;

    let mut model_entries = Vec::new();
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Ok(models) = ChannelAbilityModel::list_distinct_models(&state.db).await {
        for model in models {
            model_entries.push(serde_json::json!({
                "id": model,
                "object": "model",
                "created": current_time,
                "owned_by": "burncloud",
                "permission": [],
                "root": model,
                "parent": null,
            }));
        }
    }

    let response_json = serde_json::json!({
        "object": "list",
        "data": model_entries
    });

    build_response_with_header(
        StatusCode::OK,
        "content-type",
        "application/json",
        Body::from(
            serde_json::to_string(&response_json)
                .unwrap_or_else(|_| r#"{"object":"list","data":[]}"#.to_string()),
        ),
    )
}

/// Helper: extract and validate the Bearer token from an Authorization header.
/// Returns (user_id, user_group) on success or an error Response.
async fn extract_token_user(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Result<String, Response> {
    let token = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or_else(|| {
            build_response(
                StatusCode::UNAUTHORIZED,
                Body::from(r#"{"error":"Missing Bearer token"}"#),
            )
        })?;

    match RouterDatabase::validate_token_and_get_info(&state.db, &token).await {
        Ok(Some(info)) => Ok(info.user_id),
        Ok(None) => {
            // Fall back to legacy token table
            match RouterDatabase::validate_token_detailed(&state.db, &token).await {
                Ok(RouterTokenValidationResult::Valid(t)) => Ok(t.user_id),
                _ => {
                    // Fall back to JWT: decode and extract sub (user_id)
                    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
                        "burncloud-default-secret-change-in-production".to_string()
                    });
                    let decoded = jsonwebtoken::decode::<JwtClaims>(
                        &token,
                        &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
                        &jsonwebtoken::Validation::default(),
                    );
                    match decoded {
                        Ok(data) => Ok(data.claims.sub),
                        _ => Err(build_response(
                            StatusCode::UNAUTHORIZED,
                            Body::from(
                                r#"{"error":{"message":"Invalid Token","type":"invalid_request_error","code":"invalid_token"}}"#,
                            ),
                        )),
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Token validation DB error: {e}");
            Err(build_response(
                StatusCode::SERVICE_UNAVAILABLE,
                Body::from(r#"{"error":"Service temporarily unavailable"}"#),
            ))
        }
    }
}

/// GET /api/v1/usage — overall usage for the authenticated token holder.
async fn usage_handler(State(state): State<AppState>, headers: axum::http::HeaderMap) -> Response {
    let user_id = match extract_token_user(&state, &headers).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match burncloud_database_router::get_usage_stats(&state.db, &user_id, "month").await {
        Ok(stats) => build_response_with_header(
            StatusCode::OK,
            "content-type",
            "application/json",
            Body::from(
                serde_json::to_string(&serde_json::json!({
                    "user_id": user_id,
                    "period": "month",
                    "total_requests": stats.total_requests,
                    "prompt_tokens": stats.total_prompt_tokens,
                    "completion_tokens": stats.total_completion_tokens,
                    "total_cost_nano": stats.total_cost_nano,
                    "total_cost_usd": stats.total_cost_nano as f64 / 1_000_000_000.0,
                }))
                .unwrap_or_else(|_| "{}".to_string()),
            ),
        ),
        Err(e) => build_response(StatusCode::INTERNAL_SERVER_ERROR, json_error_body(&e)),
    }
}

/// GET /api/v1/usage/models — usage broken down by model for the authenticated token holder.
async fn usage_models_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    let user_id = match extract_token_user(&state, &headers).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match burncloud_database_router::get_usage_stats_by_model(&state.db, &user_id, "month").await {
        Ok(rows) => build_response_with_header(
            StatusCode::OK,
            "content-type",
            "application/json",
            Body::from(
                serde_json::to_string(&serde_json::json!({
                    "user_id": user_id,
                    "period": "month",
                    "models": rows.iter().map(|r| serde_json::json!({
                        "model": r.model,
                        "requests": r.requests,
                        "prompt_tokens": r.prompt_tokens,
                        "completion_tokens": r.completion_tokens,
                        "cache_read_tokens": r.cache_read_tokens,
                        "reasoning_tokens": r.reasoning_tokens,
                        "cost_nano": r.cost_nano,
                        "cost_usd": r.cost_nano as f64 / 1_000_000_000.0,
                    })).collect::<Vec<_>>(),
                }))
                .unwrap_or_else(|_| "{}".to_string()),
            ),
        ),
        Err(e) => build_response(StatusCode::INTERNAL_SERVER_ERROR, json_error_body(&e)),
    }
}

/// Normalize doubled path prefixes caused by client SDKs that include the
/// endpoint path in their base_url. For example, when a client sets
/// base_url = "https://gateway/v1/messages" and the SDK appends "/v1/messages",
/// the resulting path is "/v1/messages/v1/messages" — this function collapses
/// it back to "/v1/messages".
fn normalize_doubled_path(path: &str) -> String {
    // Known endpoint prefixes that clients may double
    const PREFIXES: &[&str] = &["/v1/messages", "/v1/chat/completions", "/v1/embeddings"];

    // Handle /v1/v1/* -> /v1/* (client base_url includes /v1 prefix)
    // Example: client sets base_url="http://host:port/v1" and SDK appends "/v1/messages"
    // Result: "/v1/v1/messages" -> "/v1/messages"
    if path.starts_with("/v1/v1/") {
        let rest = &path[3..]; // Skip "/v1" -> "/v1" // Skip first "/v1", keep "/v1/messages"
        tracing::debug!(original = %path, normalized = %rest, "Normalized /v1/v1 path prefix");
        return rest.to_string();
    } else if path == "/v1/v1" {
        return "/v1".to_string();
    }

    // Handle doubled endpoint paths: /v1/messages/v1/messages -> /v1/messages
    // This happens when client base_url already includes the endpoint path (e.g., "https://gateway/v1/messages")
    // and SDK appends the same endpoint again.
    for prefix in PREFIXES {
        let doubled = format!("{prefix}{prefix}");
        if path == doubled {
            // Exact match: /v1/messages/v1/messages -> /v1/messages
            tracing::debug!(original = %path, normalized = %prefix, "Normalized doubled endpoint path");
            return prefix.to_string();
        }
    }

    path.to_string()
}

/// Inject video_tokens into usage for video models whose responses contain no usage field.
///
/// Used for request-side billing when the upstream response has no token/usage data.
/// Injects `video_tokens` into usage only when the request succeeded and usage is currently empty.
/// `source` is used for tracing (e.g. "veo", "seedance").
///
/// Exposed as `pub(crate)` so unit tests can call it without a running server.
pub(crate) fn inject_video_tokens_if_empty(
    status: axum::http::StatusCode,
    mut usage: burncloud_service_billing::UnifiedUsage,
    video_tokens: i64,
    source: &str,
) -> burncloud_service_billing::UnifiedUsage {
    if status.is_success() && usage.is_empty() && video_tokens > 0 {
        usage.video_tokens = video_tokens;
        tracing::info!(
            video_tokens,
            source,
            "Request-side billing: injected video_tokens"
        );
    }
    usage
}

async fn health_status_handler(State(state): State<AppState>) -> Response {
    let circuit_breaker_status = state.circuit_breaker.get_status_map();
    let channel_states = state.channel_state_tracker.get_all_states();

    // Collect scheduler policies for observability
    let scheduler_info = {
        let policies = state.scheduler_policies.read().await;
        let mut map = std::collections::HashMap::new();
        for (group, kind) in policies.iter() {
            map.insert(
                group.clone(),
                match kind {
                    scheduler::SchedulerKind::Passthrough => "passthrough".to_string(),
                    scheduler::SchedulerKind::Combined { config } => format!(
                        "combined(h={:.1},c={:.1},r={:.1})",
                        config.health_weight, config.cost_weight, config.rpm_weight
                    ),
                },
            );
        }
        map
    };

    // L2 Shaper observability (issue #151): per-channel budget snapshots +
    // fail-open counter so admins can spot silently-permissive channels and
    // verify the three-color buckets are draining as expected.
    // Iterates `channel_states` (the known-channel set tracked by the
    // channel_state_tracker) and calls the public `BudgetBackend::snapshot`
    // API. Channels with `rpm_cap = NULL` (unconfigured) return `None` and
    // are filtered out — the `fail_open_count` field surfaces their volume.
    let budget_snapshots: Vec<serde_json::Value> = channel_states
        .iter()
        .filter_map(|(ch_id, _)| {
            state.rate_budget.snapshot(*ch_id).map(|snap| {
                serde_json::json!({
                    "channel_id": ch_id,
                    "rpm_cap": snap.rpm_cap,
                    "rpm_remaining_green": snap.rpm_remaining_green,
                    "rpm_remaining_yellow": snap.rpm_remaining_yellow,
                    "rpm_remaining_red": snap.rpm_remaining_red,
                    "tpm_cap": snap.tpm_cap,
                    "tpm_remaining_green": snap.tpm_remaining_green,
                    "tpm_remaining_yellow": snap.tpm_remaining_yellow,
                    "tpm_remaining_red": snap.tpm_remaining_red,
                })
            })
        })
        .collect();
    let fail_open_count = state
        .fail_open_count
        .load(std::sync::atomic::Ordering::Relaxed);

    // Build comprehensive health report
    let health_report = serde_json::json!({
        "scheduler_policies": scheduler_info,
        "circuit_breaker": circuit_breaker_status,
        "channels": channel_states.iter().map(|(ch_id, ch_state)| {
            let models: Vec<_> = ch_state.models.values().map(|model_state| {
                serde_json::json!({
                    "model": model_state.model,
                    "status": format!("{:?}", model_state.status),
                    "success_count": model_state.success_count,
                    "failure_count": model_state.failure_count,
                    "avg_latency_ms": model_state.avg_latency_ms,
                    "adaptive_limit": {
                        "current_limit": model_state.adaptive_limit.get_current_limit(),
                        "learned_limit": model_state.adaptive_limit.get_learned_limit(),
                        "state": format!("{:?}", model_state.adaptive_limit.get_state()),
                    },
                    "last_error": model_state.last_error,
                })
            }).collect();

            (ch_id.to_string(), serde_json::json!({
                "auth_ok": ch_state.auth_ok,
                "balance_status": format!("{:?}", ch_state.balance_status),
                "models": models,
            }))
        }).collect::<std::collections::HashMap<_, _>>(),
        "budget_snapshots": budget_snapshots,
        "fail_open_count": fail_open_count,
        "billing_strict": state.billing_strict,
        "billing_preflight_rejected_count": state.billing_preflight_rejected_count.load(std::sync::atomic::Ordering::Relaxed),
        "billing_post_settle_price_missing_count": state.billing_post_settle_price_missing_count.load(std::sync::atomic::Ordering::Relaxed),
    });

    let json = serde_json::to_string(&health_report).unwrap_or_else(|_| "{}".to_string());

    build_response_with_header(
        StatusCode::OK,
        "content-type",
        "application/json",
        Body::from(json),
    )
}

async fn proxy_handler(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let raw_path = uri.path().to_string();

    // Normalize doubled path prefixes caused by client SDKs that include
    // the endpoint path in their base_url (e.g. /v1/messages/v1/messages → /v1/messages).
    let path = normalize_doubled_path(&raw_path);

    // 0. Authenticate User
    // Support "Authorization: Bearer sk-xxx", "x-api-key: sk-xxx" (Anthropic native), and "x-goog-api-key: sk-xxx" (Gemini native)
    let user_auth = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .or_else(|| headers.get("x-api-key").and_then(|h| h.to_str().ok()))
        .or_else(|| headers.get("x-goog-api-key").and_then(|h| h.to_str().ok()));

    let user_token = match user_auth {
        Some(token) => token.to_string(),
        None => {
            return build_response_with_header(
                StatusCode::UNAUTHORIZED,
                "content-type",
                "application/json",
                Body::from(r#"{"error":{"message":"Unauthorized: Missing Bearer Token","type":"authentication_error","code":"missing_token"}}"#),
            );
        }
    };

    // Check against DB
    let (user_id, user_group, quota_limit, used_quota, order_type_str, price_cap) =
        match RouterDatabase::validate_token_and_get_info(&state.db, &user_token).await {
            Ok(Some(info)) => {
                // Update accessed_time non-blocking
                let db = state.db.clone();
                let token = user_token.clone();
                tokio::spawn(async move {
                    let _ = RouterDatabase::update_token_accessed_time(&db, &token).await;
                });
                (
                    info.user_id,
                    info.group,
                    info.remain_quota,
                    info.used_quota,
                    info.order_type,
                    info.price_cap,
                )
            }
            Ok(None) => {
                // Fallback to old token table logic with detailed validation
                match RouterDatabase::validate_token_detailed(&state.db, &user_token).await {
                    Ok(RouterTokenValidationResult::Valid(t)) => {
                        // Update accessed_time non-blocking
                        let db = state.db.clone();
                        let token = user_token.clone();
                        tokio::spawn(async move {
                            let _ = RouterDatabase::update_token_accessed_time(&db, &token).await;
                        });
                        (
                            t.user_id,
                            "default".to_string(),
                            t.quota_limit,
                            t.used_quota,
                            None,
                            None,
                        )
                    }
                    Ok(RouterTokenValidationResult::Expired) => {
                        return build_response_with_header(
                            StatusCode::UNAUTHORIZED,
                            "content-type",
                            "application/json",
                            Body::from(
                                r#"{"error":{"message":"Token has expired","type":"invalid_request_error","code":"token_expired"}}"#,
                            ),
                        )
                    }
                    Ok(RouterTokenValidationResult::Invalid) => {
                        // Fall back to JWT: decode and extract sub (user_id)
                        let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
                            "burncloud-default-secret-change-in-production".to_string()
                        });
                        let decoded = jsonwebtoken::decode::<JwtClaims>(
                            &user_token,
                            &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
                            &jsonwebtoken::Validation::default(),
                        );
                        match decoded {
                            Ok(data) => (
                                data.claims.sub,
                                "default".to_string(),
                                -1_i64,
                                0_i64,
                                None,
                                None,
                            ),
                            _ => {
                                return build_response_with_header(
                                    StatusCode::UNAUTHORIZED,
                                    "content-type",
                                    "application/json",
                                    Body::from(
                                        r#"{"error":{"message":"Invalid Token","type":"invalid_request_error","code":"invalid_token"}}"#,
                                    ),
                                )
                            }
                        }
                    }
                    Err(e) => {
                        return build_response_with_header(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "content-type",
                            "application/json",
                            Body::from(format!(
                                r#"{{"error":{{"message":"Internal Auth Error: {}","type":"server_error"}}}}"#,
                                e
                            )),
                        )
                    }
                }
            }
            Err(e) => {
                return build_response_with_header(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "content-type",
                    "application/json",
                    Body::from(format!(
                        r#"{{"error":{{"message":"Internal Auth Error: {}","type":"server_error"}}}}"#,
                        e
                    )),
                )
            }
        };

    if quota_limit >= 0 && used_quota >= quota_limit {
        return build_response_with_header(
            StatusCode::PAYMENT_REQUIRED,
            "content-type",
            "application/json",
            Body::from(
                r#"{"error":{"message":"Insufficient quota","type":"insufficient_quota_error","code":"insufficient_quota"}}"#,
            ),
        );
    }

    // Rate Limiting Check
    if !state.limiter.check(&user_id, 1.0) {
        return build_response_with_header(
            StatusCode::TOO_MANY_REQUESTS,
            "content-type",
            "application/json",
            Body::from(r#"{"error":{"message":"Too Many Requests","type":"rate_limit_error","code":"rate_limit_exceeded"}}"#),
        );
    }

    // Buffer body for token counting and retries
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return build_response_with_header(
                StatusCode::BAD_REQUEST,
                "content-type",
                "application/json",
                Body::from(format!(
                    r#"{{"error":{{"message":"Body Read Error: {}","type":"invalid_request_error","code":"body_read_error"}}}}"#,
                    e
                )),
            )
        }
    };

    // Estimate Prompt Tokens (Simple approximation: 1 token ~= 4 bytes)
    // TODO(issue): Integrate tiktoken-rs for precise counting
    //   - Current approximation is inaccurate for non-ASCII text
    //   - tiktoken-rs provides accurate token counting for OpenAI models
    //   - Consider: model-specific tokenizers (cl100k_base, o200k_base, etc.)

    // Extract model name for pricing before proxy_logic consumes body_bytes.
    // For Gemini native paths (e.g. /v1beta/models/gemini-2.5-flash:generateContent),
    // the model name is in the URL path, not the body.
    let model_name = serde_json::from_slice::<serde_json::Value>(&body_bytes)
        .ok()
        .and_then(|v| {
            v.get("model")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
        })
        .or_else(|| crate::passthrough::extract_model_from_gemini_path(&path))
        // Strip Gemini method suffixes like ":generateContent", ":streamGenerateContent"
        .map(|s| {
            s.split_once(':')
                .map(|(base, _)| base.to_string())
                .unwrap_or(s)
        });

    // Extract Veo-specific fields for request-side billing.
    // Veo's predictLongRunning response has no usageMetadata; duration is in the request body.
    let (video_duration_secs, video_sample_count) = {
        let v = serde_json::from_slice::<serde_json::Value>(&body_bytes).ok();
        let dur = v
            .as_ref()
            .and_then(|v| v.get("durationSeconds").and_then(|d| d.as_i64()))
            .unwrap_or(VEO_DEFAULT_DURATION_SECS);
        let count = v
            .as_ref()
            .and_then(|v| v.get("sampleCount").and_then(|d| d.as_i64()))
            .unwrap_or(VEO_DEFAULT_SAMPLE_COUNT);
        (dur, count)
    };

    // Extract Seedance / NewApi video generation fields for request-side billing.
    // Seedance's response has no usage field; duration and resolution are in the request body.
    // Only compute seedance fields for video generation paths — zero them out otherwise
    // to prevent phantom video_tokens injection on non-video requests.
    let (seedance_duration_secs, seedance_resolution) = if path == "/v1/video/generations" {
        let v = serde_json::from_slice::<serde_json::Value>(&body_bytes).ok();
        let dur = v
            .as_ref()
            .and_then(|v| v.get("duration").and_then(|d| d.as_i64()))
            .filter(|&d| d > 0) // duration=-1 means model-decided; fall back to default
            .unwrap_or(SEEDANCE_DEFAULT_DURATION_SECS);
        let res = v
            .as_ref()
            .and_then(|v| v.get("resolution").and_then(|r| r.as_str()))
            .unwrap_or(SEEDANCE_DEFAULT_RESOLUTION)
            .to_string();
        (dur, res)
    } else {
        (0i64, "480p".to_string())
    };

    // Detect request type for advanced billing
    // 1. Batch API: detected via headers or metadata.batch_id
    // 2. Priority: detected via metadata.priority or x-priority header
    let (is_batch_request, is_priority_request) = {
        let body_json = serde_json::from_slice::<serde_json::Value>(&body_bytes).ok();

        let batch = headers
            .get("x-batch-request")
            .or_else(|| headers.get("openai-batch"))
            .is_some()
            || body_json
                .as_ref()
                .and_then(|v| v.get("metadata").and_then(|m| m.get("batch_id")))
                .is_some();

        let priority = headers
            .get("x-priority")
            .map(|v| v.to_str().unwrap_or("") == "high")
            .unwrap_or(false)
            || body_json
                .as_ref()
                .and_then(|v| v.get("metadata").and_then(|m| m.get("priority")))
                .and_then(|p| p.as_str())
                .map(|s| s == "high" || s == "urgent")
                .unwrap_or(false);

        (batch, priority)
    };

    // Video task polling: GET /v1/videos/{task_id}
    // Must be handled before proxy_logic because GET requests have no model field for routing.
    // Look up the task_id → channel_id mapping saved during the original POST.
    if method == Method::GET && path.starts_with("/v1/videos/") {
        let task_id = path.trim_start_matches("/v1/videos/");

        let task = match RouterVideoTaskModel::get_by_task_id(&state.db, task_id).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                return build_response_with_header(
                    StatusCode::NOT_FOUND,
                    "content-type",
                    "application/json",
                    Body::from(format!(
                        r#"{{"error":{{"message":"Task {} not found","code":"task_not_found"}}}}"#,
                        task_id
                    )),
                );
            }
            Err(e) => {
                tracing::error!(error = ?e, task_id, "DB error looking up video task");
                return build_response_with_header(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "content-type",
                    "application/json",
                    Body::from(r#"{"error":{"message":"Database error","code":"internal_error"}}"#),
                );
            }
        };

        let channel = match ChannelProviderModel::get_by_id(&state.db, task.channel_id).await {
            Ok(Some(c)) => c,
            Ok(None) | Err(_) => {
                return build_response_with_header(
                    StatusCode::BAD_GATEWAY,
                    "content-type",
                    "application/json",
                    Body::from(
                        r#"{"error":{"message":"Upstream channel not available","code":"channel_unavailable"}}"#,
                    ),
                );
            }
        };

        let base_url = channel.base_url.unwrap_or_default();
        let upstream_url = format!("{}/v1/videos/{task_id}", base_url.trim_end_matches('/'));

        let upstream_resp = state
            .client
            .get(&upstream_url)
            .header("Authorization", format!("Bearer {}", channel.key))
            .timeout(std::time::Duration::from_secs(VIDEO_TASK_TIMEOUT_SECS))
            .send()
            .await;

        return match upstream_resp {
            Ok(resp) => {
                let status = resp.status();
                let resp_bytes = resp.bytes().await.unwrap_or_else(|e| {
                    tracing::warn!("Failed to read response bytes: {}", e);
                    axum::body::Bytes::new()
                });
                build_response_with_header(
                    status,
                    "content-type",
                    "application/json",
                    Body::from(resp_bytes),
                )
            }
            Err(e) => {
                tracing::error!(error = ?e, upstream_url, "GET video task upstream error");
                build_response_with_header(
                    StatusCode::BAD_GATEWAY,
                    "content-type",
                    "application/json",
                    Body::from(
                        r#"{"error":{"message":"Upstream request failed","code":"bad_gateway"}}"#,
                    ),
                )
            }
        };
    }

    // Create unified token counter for streaming response parsing
    let token_counter = Arc::new(UnifiedTokenCounter::new());

    // Perform Proxy Logic
    let result = proxy_logic(
        &state,
        method,
        uri,
        headers,
        body_bytes,
        &path,
        &user_id,
        &user_group,
        order_type_str.as_deref(),
        price_cap,
        token_counter.clone(),
        model_name.as_deref(),
        start_time,
    )
    .await;

    // Save video task mapping asynchronously (fire-and-forget)
    if let Some(task_id) = result.video_task_id {
        if let Some(ch_id) = result
            .upstream_id
            .as_ref()
            .and_then(|s| s.parse::<i32>().ok())
        {
            let db = state.db.clone();
            let task = RouterVideoTask {
                task_id,
                channel_id: ch_id,
                user_id: Some(user_id.clone()),
                model: model_name.clone(),
                duration: seedance_duration_secs,
                resolution: seedance_resolution.clone(),
            };
            tokio::spawn(async move {
                if let Err(e) = RouterVideoTaskModel::save(&db, &task).await {
                    tracing::warn!(error = ?e, "Failed to save video task mapping");
                }
            });
        }
    }

    // Get final unified token usage
    let usage = token_counter.get_usage();

    // Veo request-side billing: inject video_tokens when response has no usageMetadata
    let veo_tokens = if model_name
        .as_deref()
        .is_some_and(|m| m.to_lowercase().contains("veo"))
    {
        video_duration_secs * video_sample_count
    } else {
        0
    };
    let usage = inject_video_tokens_if_empty(result.final_status, usage, veo_tokens, "veo");

    // Seedance request-side billing: inject video_tokens from duration × resolution_weight
    let resolution_weight: i64 = if seedance_resolution == "720p" {
        SEEDANCE_RESOLUTION_WEIGHT_HD
    } else {
        SEEDANCE_RESOLUTION_WEIGHT_SD
    };
    let seedance_tokens = seedance_duration_secs * resolution_weight;
    let usage =
        inject_video_tokens_if_empty(result.final_status, usage, seedance_tokens, "seedance");

    // Calculate cost using CostCalculator (nanodollars)
    let (cost, cost_breakdown, cost_status) = if !usage.is_empty() {
        if let Some(model) = &model_name {
            match state
                .cost_calculator
                .calculate(
                    model,
                    &usage,
                    &request_id,
                    is_batch_request,
                    is_priority_request,
                    result.pricing_region.as_deref(),
                )
                .await
            {
                Ok(result) => {
                    let total = result.usd_amount_nano;
                    (total, result.breakdown, Some("ok".to_string()))
                }
                Err(burncloud_service_billing::BillingError::PriceNotFound(m)) => {
                    tracing::warn!(model = %m, "PriceNotFound — no price configured for this model");
                    state
                        .billing_post_settle_price_missing_count
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    (0, Default::default(), Some("price_missing".to_string()))
                }
                Err(e) => {
                    tracing::warn!("Cost calculation error for {model}: {e}");
                    (0, Default::default(), Some("calc_error".to_string()))
                }
            }
        } else {
            (0, Default::default(), Some("no_model".to_string()))
        }
    } else if result.error_type.is_some() {
        (0, Default::default(), Some("upstream_error".to_string()))
    } else {
        (0, Default::default(), None)
    };

    // Warn if cost is non-zero but model is unknown — reconciliation data will be degraded
    if model_name.is_none() && cost > 0 {
        tracing::warn!(cost, %request_id, "cost > 0 but model unknown — reconciliation data degraded");
    }

    // L6 Observability: compute layer_decision via priority chain (issue #152).
    // Priority: failover_N > affinity_hit/scorer_picked > shaper_own/shaper_borrow/
    // shaper_unconfigured > shaper_reject. Decision D9: do NOT change
    // ShaperContext.outcome semantics; override at RouterLog construction point.
    // Priority chain: scheduler decision (failover_N / affinity_hit /
    // scorer_picked) overrides shaper outcome when both are present.
    let layer_decision = {
        let sched_label = result.routing_decision.as_ref().map(|d| d.to_label());
        let shaper_label = result.shaper_outcome.as_ref().map(|(lbl, _)| lbl.as_str());
        match (sched_label, shaper_label) {
            (Some(s), _) => Some(s), // Scheduler decision wins (highest priority)
            (None, Some(sh)) => Some(sh.to_string()), // Shaper outcome only
            (None, None) => None,    // No decision (pre-routing error)
        }
    };
    // traffic_color: prefer shaper_outcome color (final color after Shaper
    // processing). When Shaper is inactive (no shaper_outcome), fall back
    // to SchedulingRequest.color per the L6 Observability checklist.
    let traffic_color = result
        .shaper_outcome
        .as_ref()
        .map(|(_, c)| c.clone())
        .or_else(|| Some(result.sched_request_color.as_char().to_string()));

    // Async Log
    // Clone upstream_id before it's moved into RouterLog, for response header injection.
    let upstream_id_for_header = result.upstream_id.clone();

    tracing::info!(
        request_id = %request_id,
        path = %path,
        model = ?model_name,
        channel = ?upstream_id_for_header,
        status = result.final_status.as_u16(),
        latency_ms = start_time.elapsed().as_millis() as i64,
        prompt_tokens = usage.input_tokens,
        completion_tokens = usage.output_tokens,
        cost_nano = cost,
        layer = ?layer_decision,
        color = ?traffic_color,
        "request completed"
    );

    let log = RouterLog {
        id: 0, // Auto-generated by database
        request_id: request_id.clone(),
        user_id: Some(user_id.clone()),
        path,
        upstream_id: result.upstream_id,
        status_code: result.final_status.as_u16() as i32,
        latency_ms: start_time.elapsed().as_millis() as i64,
        prompt_tokens: usage.input_tokens as i32,
        completion_tokens: usage.output_tokens as i32,
        cost,
        model: model_name.clone(),
        cache_read_tokens: usage.cache_read_tokens as i32,
        reasoning_tokens: usage.reasoning_tokens as i32,
        pricing_region: result.pricing_region,
        video_tokens: usage.video_tokens as i32,
        cache_write_tokens: usage.cache_write_tokens as i32,
        audio_input_tokens: usage.audio_input_tokens as i32,
        audio_output_tokens: usage.audio_output_tokens as i32,
        image_tokens: usage.image_tokens as i32,
        embedding_tokens: usage.embedding_tokens as i32,
        input_cost: cost_breakdown.input_cost,
        output_cost: cost_breakdown.output_cost,
        // TODO: promote cache_read_cost/cache_write_cost to separate CostBreakdown fields
        // (calculator.rs already computes them separately before merging; see compute_breakdown)
        cache_read_cost: cost_breakdown.cache_cost, // currently stores read+write merged
        cache_write_cost: 0,
        audio_cost: cost_breakdown.audio_cost,
        image_cost: cost_breakdown.image_cost,
        video_cost: cost_breakdown.video_cost,
        reasoning_cost: cost_breakdown.reasoning_cost,
        embedding_cost: cost_breakdown.embedding_cost,
        layer_decision,
        traffic_color,
        cost_status,
        error_type: result.error_type,
        created_at: None, // Auto-generated by database
    };

    if state.log_tx.send(log).await.is_err() {
        tracing::error!(
            cost,
            "billing log channel full or closed — request cost NOT recorded"
        );
    }

    // Send detailed request log (Issue #334)
    // Only send if we have data and storage policy is not 'none'
    if let Some(log_data) = result.request_log_data {
        let request_log = RouterRequestLog {
            id: 0,
            request_id: request_id.clone(),
            request_body: log_data.request_body,
            request_body_truncated: log_data.request_body_truncated,
            request_headers: log_data.request_headers,
            response_body: log_data.response_body,
            response_body_truncated: log_data.response_body_truncated,
            response_status: Some(result.final_status.as_u16() as i32),
            stream_chunk_count: log_data.stream_chunk_count as i32,
            stream_first_chunk_latency_ms: log_data.stream_first_chunk_latency_ms.map(|v| v as i64),
            stream_last_chunk_latency_ms: log_data.stream_last_chunk_latency_ms.map(|v| v as i64),
            candidates: if log_data.candidates.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&log_data.candidates).unwrap_or_else(|_| "[]".to_string()))
            },
            candidates_count: log_data.candidates_count,
            affinity_key: log_data.affinity_key,
            affinity_hit_channel_id: log_data.affinity_hit_channel_id,
            failover_history: if log_data.failover_history.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&log_data.failover_history).unwrap_or_else(|_| "[]".to_string()))
            },
            storage_policy: state.request_log_storage_policy.as_str().to_string(),
            created_at: None,
        };

        // Use try_send to avoid blocking if channel is full
        let _ = state.request_log_tx.try_send(request_log);
    }

    // Deduct quota (non-blocking)
    // Use cost > 0 (not total_tokens > 0) so video/audio/music requests are also deducted.
    // total_tokens only counts text tokens; multi-modal costs flow through cost (nanodollars).
    if cost > 0 {
        let db = state.db.clone();
        let token_for_quota = user_token.to_string();
        let user_id_for_quota = user_id.clone();
        tokio::spawn(async move {
            let _ =
                RouterDatabase::deduct_quota(&db, &user_id_for_quota, &token_for_quota, cost).await;
        });
    }

    // Inject route-tracing headers for client-side observability.
    let response = if let Some(ref ch_id) = upstream_id_for_header {
        let mut r = result.response;
        r.headers_mut().insert(
            "X-Channel-Id",
            ch_id
                .parse()
                .unwrap_or_else(|_| HeaderValue::from_static("0")),
        );
        if let Some(ref m) = model_name {
            r.headers_mut().insert(
                "X-Model-Id",
                m.parse()
                    .unwrap_or_else(|_| HeaderValue::from_static("unknown")),
            );
        }
        r
    } else {
        result.response
    };

    response
}

use burncloud_common::types::ChannelType;
use circuit_breaker::FailureType;
use passthrough::{should_passthrough, PassthroughDecision};
use rate_budget::{BudgetBackend, BudgetGuard, ConsumeOutcome};
use response_parser::{parse_error_response, parse_rate_limit_info};

/// Per-failover-loop bookkeeping for the L2 Shaper integration (issue #151).
///
/// Bundling `color`, `est_tpm`, `outcome`, and `rejected_count` into one
/// struct keeps the loop body free of orphan `mut` locals (audit Eng E-4).
struct ShaperContext {
    /// Traffic color resolved by the L1 Classifier (or `Yellow` for path-routed).
    color: TrafficColor,
    /// Estimated TPM for this request: `body.max_tokens` or 4096 fallback.
    est_tpm: u64,
    /// Most recent iteration's shaper outcome label
    /// (`shaper_own` / `shaper_borrow` / `shaper_unconfigured`). Set on admit;
    /// used by the success-return path for `RouterLog.layer_decision`.
    outcome: Option<&'static str>,
    /// Count of candidates the L2 Shaper actively `Rejected`. When this equals
    /// `candidates.len()` after the loop, the function returns 503 +
    /// `X-Rejected-By: shaper` (audit decision D12).
    rejected_count: u32,
}

#[allow(clippy::too_many_arguments)]
async fn proxy_logic(
    state: &AppState,
    method: Method,
    uri: Uri,
    _headers: HeaderMap,
    body_bytes: axum::body::Bytes,
    path: &str,
    user_id: &str,
    user_group: &str,
    order_type_str: Option<&str>,
    price_cap: Option<i64>,
    token_counter: Arc<UnifiedTokenCounter>,
    model_name: Option<&str>,
    request_start_time: Instant,
) -> ProxyResult {
    // Initialize request log data collection (Issue #334)
    // Only collect detailed data when storage policy is not 'none'
    let should_collect_detailed_log = state.request_log_storage_policy != StoragePolicy::None;
    let mut request_log_data = if should_collect_detailed_log {
        // Sanitize request body: remove sensitive fields, truncate if too large
        let (request_body, request_body_truncated) = sanitize_request_body(&body_bytes);
        // Sanitize request headers: remove authorization, api-key, etc.
        let request_headers = sanitize_request_headers(&_headers);
        Some(RequestLogData {
            request_body,
            request_body_truncated,
            request_headers,
            ..Default::default()
        })
    } else {
        None
    };

    // Model Routing
    let mut candidates: Vec<Upstream> = Vec::new();
    // L6 Observability: routing decision from route_with_scheduler.
    // Used by the priority chain to compute layer_decision for RouterLog.
    let mut sched_routing_decision: Option<model_router::RoutingDecision> = None;
    // Track pricing_region from selected channel for billing.
    // Assigned inside the retry loop (candidates guaranteed non-empty at this point).
    // Uninitialized: assigned inside the retry loop before any use
    let mut selected_pricing_region: Option<String>;

    // L2 Shaper: pre-compute est_tpm from `max_tokens` in the request body.
    // TODO(phase 2.5 #151): per-adaptor est_tpm refinement — 4096 may grossly
    //   underestimate Anthropic (200k+ caps) or overestimate small embedding
    //   requests. Refine using adaptor-specific defaults + historical regression.
    let est_tpm: u64 = serde_json::from_slice::<serde_json::Value>(&body_bytes)
        .ok()
        .and_then(|v| v.get("max_tokens").and_then(|m| m.as_u64()))
        .unwrap_or(4096);
    // Default color for path-routed (non-model) requests; overwritten when
    // the L1 Classifier (issue #150) resolves a real color from the user.
    let mut shaper_color: TrafficColor = TrafficColor::Yellow;

    // Extract session_id from conversation_id in request body (P0 affinity wiring).
    // Falls back to user_id so affinity still works when no conversation_id is set.
    let session_id: String = serde_json::from_slice::<serde_json::Value>(&body_bytes)
        .ok()
        .and_then(|v| {
            v.get("conversation_id")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| user_id.to_string());

    // Try to extract model from Gemini native path first
    let gemini_path_model = passthrough::extract_model_from_gemini_path(path);

    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
        // Prefer model from body, fall back to path-extracted model for Gemini native paths
        let model_ref = json.get("model").and_then(|v| v.as_str());
        let model_opt = model_ref.or(gemini_path_model.as_deref());

        if let Some(model) = model_opt {
            // Use scheduler-based routing for multi-channel failover
            // Clone the policy for this group, then release the lock immediately.
            // This prevents the lock from being held during SQL queries and async
            // price lookups in route_with_scheduler, which could starve reload_handler.
            let scheduler_kind = {
                let policies = state.scheduler_policies.read().await;
                policies.get(&user_group.to_lowercase()).cloned()
            };
            // Lock released here — before SQL queries in route_with_scheduler

            // L1 Classifier: resolve color via service-user (audit decision
            // E-D3 — router stays color-agnostic), build OrderType from the
            // token's router_tokens columns, and carry user_id into the
            // request so L3 Affinity (HRW) gets a real stickiness key.
            // session_id stays None until conversation tracking lands.
            let color = UserService::resolve_traffic_class(&state.db, user_id)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!(
                        "L1 Classifier: resolve_traffic_class failed for user_id={}: {} — falling back to TrafficColor::Yellow",
                        user_id, e
                    );
                    TrafficColor::Yellow
                });
            let order_type = OrderType::from_db_row(order_type_str, price_cap);
            // Capture for L2 Shaper before sched_request consumes the value.
            shaper_color = color;
            let sched_request = scheduler::SchedulingRequest {
                user_id: Some(user_id.to_string()),
                color,
                order_type,
                session_id: Some(session_id.clone()),
            };

            tracing::debug!(
                "ProxyLogic: Attempting to route model '{}' for group '{}' (color={:?}, order_type={})",
                model,
                user_group,
                sched_request.color,
                sched_request.order_type.as_label()
            );

            match state
                .model_router
                .route_with_scheduler(model_router::RouteInputs {
                    group: user_group,
                    model,
                    state_tracker: &state.channel_state_tracker,
                    price_cache: &state.price_cache,
                    exchange_rate: &state.exchange_rate_service,
                    scheduler_kind: scheduler_kind.as_ref(),
                    request: &sched_request,
                    affinity_cache: Some(state.affinity_cache.as_ref()),
                })
                .await
            {
                Ok((channels, routing_decision)) if !channels.is_empty() => {
                    tracing::debug!(
                        "ModelRouter: Got {} candidates for {}",
                        channels.len(),
                        model
                    );

                    // Store routing_decision for L6 Observability priority chain.
                    // Will be overridden by Failover{attempt} if failover loop
                    // advances past attempt 0.
                    sched_routing_decision = routing_decision;

                    for channel in channels {
                        let channel_type = ChannelType::from(channel.type_);
                        // Path-based channel filtering (Issue #263)
                        // OpenAI format requests should only go to OpenAI-type channels
                        // Anthropic format requests should only go to Anthropic-type channels
                        let is_openai_path = path.starts_with("/v1/chat/completions")
                            || path.starts_with("/v1/completions")
                            || path.starts_with("/v1/embeddings");
                        let is_anthropic_path = path.starts_with("/v1/messages");

                        // Skip channel if path format does not match channel type
                        if is_openai_path
                            && !matches!(channel_type, ChannelType::OpenAI | ChannelType::Zai)
                        {
                            tracing::debug!(
                                "Skipping {:?} channel for OpenAI format path: {}",
                                channel_type,
                                path
                            );
                            continue;
                        }
                        if is_anthropic_path && !matches!(channel_type, ChannelType::Anthropic) {
                            tracing::debug!(
                                "Skipping {:?} channel for Anthropic format path: {}",
                                channel_type,
                                path
                            );
                            continue;
                        }

                        let (auth_type, protocol) = match channel_type {
                            ChannelType::OpenAI => (AuthType::Bearer, PROTOCOL_OPENAI.to_string()),
                            ChannelType::Anthropic => {
                                (AuthType::Claude, PROTOCOL_CLAUDE.to_string())
                            }
                            ChannelType::Gemini | ChannelType::VertexAi => {
                                (AuthType::GoogleAI, PROTOCOL_GEMINI.to_string())
                            }
                            ChannelType::Zai => (AuthType::Bearer, PROTOCOL_ZAI.to_string()),
                            _ => (AuthType::Bearer, PROTOCOL_OPENAI.to_string()),
                        };
                        let ch_id = channel.id.to_string();
                        candidates.push(Upstream {
                            id: ch_id,
                            name: channel.name,
                            base_url: channel.base_url.unwrap_or_default(),
                            api_key: channel.key,
                            match_path: String::new(),
                            auth_type,
                            priority: channel.priority as i32,
                            protocol,
                            param_override: channel.param_override.clone(),
                            header_override: channel.header_override.clone(),
                            api_version: channel.api_version.clone(),
                            pricing_region: channel.pricing_region.clone(),
                        });
                    }
                }
                Ok(_) => {
                    tracing::debug!(
                        "ModelRouter: No candidates for {} (Group: {})",
                        model,
                        user_group
                    );
                }
                Err(e) => {
                    // NoAvailableChannelsError - all channels are unavailable
                    tracing::warn!("ModelRouter: No available channels for {}: {}", model, e);
                    // 503 response contract (audit decision D12): clients must
                    // be able to distinguish a local Shaper / OrderType reject
                    // from an upstream 5xx. We attach two headers:
                    //   - X-Rejected-By: which router layer dropped the request
                    //   - Retry-After: seconds the client should back off
                    // Future Shaper rejections add `X-Rejected-By: shaper`
                    // from inside the Shaper path; here we emit `order_type`
                    // or `scheduler`.
                    let rejected_by = if e.reason.contains("OrderType") {
                        "order_type"
                    } else {
                        "scheduler"
                    };
                    let body = Body::from(format!(
                        r#"{{"error":{{"message":"{}","type":"service_unavailable","code":"no_available_channels","rejected_by":"{}"}}}}"#,
                        e, rejected_by
                    ));
                    let response = Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header("content-type", "application/json")
                        .header("X-Rejected-By", rejected_by)
                        .header("Retry-After", "60")
                        .body(body)
                        .unwrap_or_else(|_| {
                            Response::builder()
                                .status(StatusCode::SERVICE_UNAVAILABLE)
                                .body(Body::empty())
                                .unwrap_or_else(|_| Response::new(Body::empty()))
                        });
                    return ProxyResult {
                        response,
                        upstream_id: None,
                        final_status: StatusCode::SERVICE_UNAVAILABLE,
                        pricing_region: None,
                        video_task_id: None,
                        shaper_outcome: None,
                        routing_decision: None,
                        sched_request_color: shaper_color,
                        error_type: Some("router_reject".to_string()),
                        request_log_data: None,
                    };
                }
            }
        } else {
            tracing::debug!("ProxyLogic: No 'model' field in JSON body");
        }
    } else {
        tracing::debug!("ProxyLogic: Failed to parse body as JSON");
    }

    if candidates.is_empty() {
        // Return proper Anthropic-style error for Claude Code compatibility
        let error_body = serde_json::json!({
            "error": {
                "type": "invalid_request_error",
                "message": format!("No matching channel found for path: {}", path),
                "code": "no_available_channel"
            }
        });
        return ProxyResult {
            response: build_response_with_header(
                StatusCode::NOT_FOUND,
                "content-type",
                "application/json",
                Body::from(error_body.to_string()),
            ),
            upstream_id: None,
            final_status: StatusCode::NOT_FOUND,
            pricing_region: None,
            video_task_id: None,
            shaper_outcome: None,
            routing_decision: None,
            sched_request_color: shaper_color,
            error_type: Some("router_reject".to_string()),
                        request_log_data: None,
        };
    }

    // Preflight billing check: reject requests for models with no price configured.
    // In strict mode (default), returns 400 to prevent unbilled usage.
    // In non-strict mode, only warns and allows the request through.
    if let Some(model) = model_name {
        if let Err(e) = state.cost_calculator.preflight(model, None).await {
            if state.billing_strict {
                tracing::warn!(model = %model, "Preflight billing check failed — rejecting request: {e}");
                state
                    .billing_preflight_rejected_count
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return ProxyResult {
                    response: build_response_with_header(
                        StatusCode::BAD_REQUEST,
                        "content-type",
                        "application/json",
                        Body::from(format!(
                            r#"{{"error":{{"message":"Model '{}' is not supported or has no price configured","type":"invalid_request_error","code":"model_not_found"}}}}"#,
                            model
                        )),
                    ),
                    upstream_id: None,
                    final_status: StatusCode::BAD_REQUEST,
                    pricing_region: None,
                    video_task_id: None,
                    shaper_outcome: None,
                    routing_decision: None,
                    sched_request_color: shaper_color,
                    error_type: Some("router_reject".to_string()),
                        request_log_data: None,
                };
            } else {
                tracing::warn!(model = %model, "Preflight billing check failed — non-strict mode, allowing request: {e}");
            }
        }
    }

    let mut last_error = String::new();
    #[allow(unused_assignments)]
    let mut last_upstream_id = None;

    // L2 Shaper bookkeeping (issue #151) shared across the failover loop.
    let mut shaper_ctx = ShaperContext {
        color: shaper_color,
        est_tpm,
        outcome: None,
        rejected_count: 0,
    };
    let total_candidates = candidates.len();

    // Record candidates for request log (Issue #334)
    if let Some(ref mut log_data) = request_log_data {
        log_data.candidates = candidates.iter().map(|u| CandidateInfo {
            id: u.id.clone(),
            name: u.name.clone(),
            protocol: u.protocol.clone(),
            priority: u.priority,
        }).collect();
        log_data.candidates_count = candidates.len() as i32;

        // Record affinity key and hit
        log_data.affinity_key = Some(session_id.clone());
        // Check if we have an affinity hit (first candidate was from affinity cache)
        if let Some(ref decision) = sched_routing_decision {
            if matches!(decision, model_router::RoutingDecision::AffinityHit) {
                log_data.affinity_hit_channel_id = candidates.first().and_then(|c| c.id.parse().ok());
            }
        }
    }

    for (attempt, upstream) in candidates.iter().enumerate() {
        // L5 Failover: override routing decision when attempt > 0.
        if attempt > 0 {
            sched_routing_decision = Some(model_router::RoutingDecision::Failover {
                attempt: attempt as u32,
            });
        }
        last_upstream_id = Some(upstream.id.clone());

        // Update pricing_region for billing to match the actual upstream serving
        selected_pricing_region = upstream.pricing_region.clone();

        // L2 Shaper Check (issue #151) — runs BEFORE the circuit breaker so
        // a locally-overloaded channel is rejected without consuming a CB slot.
        // Unconfigured channels (rpm_cap = NULL) bypass the bucket via
        // `is_configured` (audit Eng E-N1), counted via fail_open_count for
        // /router/status visibility (FM2). On admit, a `BudgetGuard` is held
        // for the rest of the iteration: success → `commit(est_tpm)`; any
        // continue / panic / cancel → `Drop` refunds full est_tpm (audit FM4).
        let channel_id_i32: i32 = upstream.id.parse().unwrap_or(0);
        let mut budget_guard: Option<BudgetGuard<'_>> = None;
        let iter_label: &'static str = if !state.rate_budget.is_configured(channel_id_i32) {
            state
                .fail_open_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            "shaper_unconfigured"
        } else {
            let outcome =
                state
                    .rate_budget
                    .try_consume(channel_id_i32, shaper_ctx.color, shaper_ctx.est_tpm);
            if outcome == ConsumeOutcome::Rejected {
                shaper_ctx.rejected_count += 1;
                tracing::debug!(
                    channel_id = channel_id_i32,
                    color = ?shaper_ctx.color,
                    est_tpm = shaper_ctx.est_tpm,
                    "L2 Shaper rejected candidate {}, trying next", upstream.name
                );
                // Record failover attempt (Issue #334)
                record_failover_attempt(
                    &mut request_log_data,
                    attempt as u32,
                    upstream,
                    Some("L2 Shaper rejected"),
                    0,
                );
                continue;
            }
            budget_guard = Some(BudgetGuard::new(
                state.rate_budget.as_ref(),
                channel_id_i32,
                shaper_ctx.color,
                shaper_ctx.est_tpm,
            ));
            outcome.as_label()
        };
        shaper_ctx.outcome = Some(iter_label);

        // Circuit Breaker Check
        if !state.circuit_breaker.allow_request(&upstream.id) {
            tracing::debug!("Skipping upstream {} (Circuit Open)", upstream.name);
            last_error = format!("Circuit Breaker Open for {}", upstream.name);
            // Record failover attempt (Issue #334)
            record_failover_attempt(
                &mut request_log_data,
                attempt as u32,
                upstream,
                Some(&last_error),
                0, // No latency - never sent to upstream
            );
            // budget_guard drops here → full est_tpm refund (request never
            // reached upstream, so the reservation is returned to the bucket).
            continue;
        }

        // 2. Construct Target URL
        // Note: Some adaptors might override URL, but we set base here.
        let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
        let target_url = format!("{}{path}{query}", upstream.base_url);

        tracing::debug!(
            "Proxying {} -> {} (via {}) [Attempt {}] Protocol: {}",
            path,
            target_url,
            upstream.name,
            attempt + 1,
            upstream.protocol
        );

        // Determine Adaptor using DynamicAdaptorFactory
        // Currently Upstream struct stores protocol string. We should map it to ChannelType.
        // Simple heuristic map for now.
        let channel_type = match upstream.protocol.as_str() {
            "claude" => ChannelType::Anthropic,
            "gemini" => ChannelType::Gemini,
            "vertex" => ChannelType::VertexAi,
            "zai" => ChannelType::Zai,
            _ => ChannelType::OpenAI,
        };

        // 3. Parse Request Body early for passthrough detection
        let mut body_json: serde_json::Value = match serde_json::from_slice(&body_bytes) {
            Ok(v) => v,
            Err(_) => {
                last_error = "Invalid JSON body".to_string();
                continue;
            }
        };

        // 4. Check if we should use passthrough mode (Gemini native format)
        let passthrough_decision = should_passthrough(path, &body_json, channel_type);

        // Handle passthrough mode (Gemini native or Anthropic native format)
        if passthrough_decision == PassthroughDecision::Passthrough {
            tracing::debug!(
                "Using passthrough mode: path={}, channel_type={:?}",
                path,
                channel_type
            );

            // Build target URL and auth for passthrough
            let (req_builder, is_stream, is_gemini) = if channel_type == ChannelType::Anthropic {
                // Anthropic passthrough: forward to base_url + /v1/messages
                let base = upstream.base_url.trim_end_matches('/');
                let url = format!("{base}/v1/messages");
                let is_stream = body_json
                    .get("stream")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let req = state
                    .client
                    .request(method.clone(), &url)
                    .header("x-api-key", &upstream.api_key)
                    .header("anthropic-version", "2023-06-01");
                (req, is_stream, false)
            } else if channel_type == ChannelType::OpenAI {
                // OpenAI passthrough: forward to base_url + /chat/completions
                let base = upstream.base_url.trim_end_matches('/');
                let url = format!("{base}/chat/completions");
                let is_stream = body_json
                    .get("stream")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let req = state
                    .client
                    .request(method.clone(), &url)
                    .header("Authorization", format!("Bearer {}", &upstream.api_key));
                (req, is_stream, false)
            } else {
                // Gemini passthrough
                let passthrough_url =
                    passthrough::build_gemini_passthrough_url(&upstream.base_url, path, &body_json);
                let is_stream = body_json
                    .get("stream")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let final_url = if is_stream && !passthrough_url.contains("alt=") {
                    let separator = if passthrough_url.contains('?') {
                        "&"
                    } else {
                        "?"
                    };
                    format!("{passthrough_url}{separator}alt=sse")
                } else {
                    passthrough_url.clone()
                };
                tracing::debug!("Passthrough URL: {}", final_url);
                let req = state
                    .client
                    .request(method.clone(), &final_url)
                    .header("x-goog-api-key", &upstream.api_key);
                (req, is_stream, true)
            };

            // Prepare request body
            // Safe to take ownership: passthrough path always returns or continues,
            // so body_json is never used after this point in the current iteration.
            let mut passthrough_body = std::mem::take(&mut body_json);
            // Gemini native API does not accept 'stream' in body
            if is_gemini && passthrough_body.get("stream").is_some() {
                if let Some(obj) = passthrough_body.as_object_mut() {
                    obj.remove("stream");
                }
            }

            // Apply header_override
            let req_builder =
                apply_header_override(req_builder, upstream.header_override.as_deref());

            let req_builder = req_builder.json(&passthrough_body);

            // Execute passthrough request
            match req_builder.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    let resp_headers = resp.headers().clone();

                    if status.is_server_error() {
                        last_error = format!("Upstream returned {status}");
                        record_upstream_failure(
                            state,
                            upstream,
                            model_name,
                            FailureType::ServerError,
                            &last_error,
                            &session_id,
                        );
                        continue;
                    }

                    if status.is_success() {
                        // For streaming responses, delay recording success until we confirm non-empty response
                        if !is_stream {
                            record_upstream_success(state, upstream, model_name, &session_id);
                        }

                        let channel_id: i32 = upstream.id.parse().unwrap_or(0);
                        let latency_ms = request_start_time.elapsed().as_millis() as u64;
                        // Parse rate limit info from response headers for adaptive limiter
                        let rate_limit_info =
                            parse_rate_limit_info(resp.headers(), None, &upstream.protocol);
                        state
                            .channel_state_tracker
                            .record_success(
                                channel_id,
                                model_name,
                                latency_ms,
                                rate_limit_info.request_limit,
                            )
                            .inspect(|&learned| {
                                let _ = state.budget_update_tx.try_send(state::BudgetUpdate {
                                    channel_id,
                                    learned_limit: learned,
                                });
                            });

                        // Handle streaming vs non-streaming passthrough
                        if is_stream {
                            // Peek first chunk to detect errors before sending HTTP response
                            let peek_timeout = std::time::Duration::from_secs(PEEK_FIRST_CHUNK_TIMEOUT_SECS);
                            let mut peek_error_handled = false;
                            let body_stream = resp.bytes_stream();
                            
                            let body_stream = {
                                let body_stream_for_peek = body_stream;
                                match crate::stream_peek::peek_first_chunk(body_stream_for_peek, peek_timeout).await {
                                    crate::stream_peek::PeekResult::HasFirstChunk { first_chunk, remaining_stream } => {
                                        if let Some((error_code, error_msg, is_auth)) = 
                                            crate::stream_peek::check_sse_error_in_chunk(&first_chunk) {
                                            let failure_type = if is_auth { FailureType::AuthFailed } else { FailureType::ServerError };
                                            tracing::warn!(channel_id = %upstream.id, error_code = error_code, error_msg = error_msg, ?failure_type, "SSE error in first chunk (Passthrough) - retrying");
                                            state.circuit_breaker.record_failure_with_type(&upstream.id, failure_type.clone());
                                            state.channel_state_tracker.record_error(upstream.id.parse().unwrap_or(0), model_name.as_deref(), &failure_type, &format!("SSE error: {}", error_msg));
                                            if let Some(model) = model_name { state.affinity_cache.evict(&session_id.to_string(), model); }
                                            last_error = format!("SSE error {}: {}", error_code, error_msg);
                                            peek_error_handled = true;
                                            futures::stream::empty::<Result<axum::body::Bytes, reqwest::Error>>().boxed()
                                        } else {
                                            let first_chunk_stream = futures::stream::once(async move { Ok(first_chunk) });
                                            first_chunk_stream.chain(remaining_stream).boxed()
                                        }
                                    }
                                    crate::stream_peek::PeekResult::Empty => {
                                        tracing::warn!(channel_id = %upstream.id, "Empty first chunk (Passthrough) - retrying");
                                        state.circuit_breaker.record_failure_with_type(&upstream.id, FailureType::EmptyResponse);
                                        last_error = "Empty response".to_string();
                                        peek_error_handled = true;
                                        futures::stream::empty().boxed()
                                    }
                                    crate::stream_peek::PeekResult::Error(e) => {
                                        tracing::error!(channel_id = %upstream.id, error = ?e, "Peek error (Passthrough)");
                                        state.circuit_breaker.record_failure_with_type(&upstream.id, FailureType::ServerError);
                                        last_error = format!("Network error: {}", e);
                                        peek_error_handled = true;
                                        futures::stream::empty().boxed()
                                    }
                                    crate::stream_peek::PeekResult::Timeout { stream } => {
                                        tracing::info!(channel_id = %upstream.id, "Peek timeout (Passthrough) - proceeding");
                                        stream
                                    }
                                }
                            };
                            
                            if peek_error_handled { continue; }
                            
                            let counter_clone = Arc::clone(&token_counter);
                            let counter_clone = Arc::clone(&token_counter);

                            // Clone state and upstream info for post-stream empty response check
                            let state_clone = state.clone();
                            let upstream_id_str = upstream.id.clone();
                            let _upstream_name = upstream.name.clone();
                            let model_name_clone = model_name.map(|s| s.to_string());
                            let session_id_clone = session_id.to_string();
                            let channel_id: i32 = upstream.id.parse().unwrap_or(0);

                            let parser = get_parser(channel_type);

                            // Track if we've seen any tokens during streaming
                            let seen_tokens = Arc::new(std::sync::atomic::AtomicBool::new(false));
                            let seen_tokens_clone = Arc::clone(&seen_tokens);

                            let stream = body_stream
                                .map(move |chunk_result| {
                                    match chunk_result {
                                        Ok(bytes) => {
                                            let text = String::from_utf8_lossy(&bytes);

                                            // Parse token usage from streaming response
                                            if let Some(u) = parse_chunk_or_default(
                                                parser.as_ref(),
                                                &text,
                                                "passthrough",
                                            ) {
                                                if !u.is_empty() {
                                                    seen_tokens_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                                                }
                                                counter_clone.set_from_usage(&u);
                                            }
                                            
                                            // Also check for actual content in the stream (not just usage)
                                            // This handles cases where usage is sent separately at the end
                                            // or not sent at all (some providers do not send usage in streams)
                                            if !seen_tokens_clone.load(std::sync::atomic::Ordering::Relaxed) {
                                                // Check for content in SSE format: data: {"choices":[{"delta":{"content":"..."}}]}
                                                for line in text.lines() {
                                                    let line = line.trim();
                                                    if !line.starts_with("data: ") {
                                                        continue;
                                                    }
                                                    let data = &line[6..];
                                                    if data.trim() == "[DONE]" {
                                                        continue;
                                                    }
                                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                                        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                                                            for choice in choices {
                                                                if let Some(delta) = choice.get("delta") {
                                                                    if delta.get("content").and_then(|c| c.as_str()).is_some() {
                                                                        seen_tokens_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                                                                        break;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            // Pass through raw bytes
                                            Ok(bytes)
                                        }
                                        Err(e) => Err(std::io::Error::other(e)),
                                    }
                                })
                                .chain(futures::stream::once(async move {
                                    // Check for empty response after stream ends
                                    if !seen_tokens.load(std::sync::atomic::Ordering::Relaxed) {
                                        // Use sliding window counter: only penalize after consecutive empty responses
                                        let should_penalize = state_clone.empty_response_counter.record_empty(&upstream_id_str);
                                        
                                        if should_penalize {
                                            // Threshold exceeded - record failure
                                            state_clone.circuit_breaker.record_failure_with_type(
                                                &upstream_id_str,
                                                FailureType::EmptyResponse,
                                            );
                                            state_clone.channel_state_tracker.record_error(
                                                channel_id,
                                                model_name_clone.as_deref(),
                                                &FailureType::EmptyResponse,
                                                "Consecutive empty streaming responses exceeded threshold",
                                            );

                                            // Evict affinity so next request tries different channel
                                            if let Some(model) = &model_name_clone {
                                                state_clone.affinity_cache.evict(&session_id_clone, model);
                                            }
                                        }
                                    } else {
                                        // Successful response - reset the counter and record success
                                        state_clone.empty_response_counter.reset(&upstream_id_str);
                                        // Now we can safely record success for the streaming response
                                        state_clone.circuit_breaker.record_success(&upstream_id_str);
                                        if let Some(model) = &model_name_clone {
                                            state_clone.affinity_cache.insert(&session_id_clone, model, channel_id);
                                        }
                                    }
                                    // Return empty bytes to not affect the stream
                                    Ok(axum::body::Bytes::new())
                                }));

                            // L2 Shaper success: upstream accepted the request.
                            // commit(est_tpm) refunds 0 and marks committed so
                            // Drop is a no-op. The bucket retains est_tpm of
                            // consumption (pessimistic — phase 2.5 may refine).
                            if let Some(g) = budget_guard.take() {
                                g.commit(shaper_ctx.est_tpm);
                            }
                            return ProxyResult {
                                response: Response::builder()
                                    .status(status)
                                    .header("content-type", "text/event-stream")
                                    .header("cache-control", "no-cache")
                                    .header("connection", "keep-alive")
                                    .body(Body::from_stream(stream))
                                    .unwrap_or_else(|_| {
                                        build_response(
                                            StatusCode::INTERNAL_SERVER_ERROR,
                                            Body::from(r#"{"error":{"message":"Failed to build streaming response","type":"internal_error","code":"stream_build_failed"}}"#),
                                        )
                                    }),
                                upstream_id: last_upstream_id,
                                final_status: status,
                                pricing_region: selected_pricing_region.clone(),
                                video_task_id: None,
                                shaper_outcome: Some((
                                    iter_label.to_string(),
                                    shaper_ctx.color.as_char().to_string(),
                                )),
                                routing_decision: sched_routing_decision.clone(),
                                sched_request_color: shaper_color,
                                error_type: None,
                            request_log_data: None,
                            };
                        } else {
                            // Non-streaming passthrough
                            let resp_bytes = match resp.bytes().await {
                                Ok(b) => b,
                                Err(e) => {
                                    last_error = format!("Failed to read response: {e}");
                                    tracing::warn!(
                                        "Passthrough: {} response read failed: {}",
                                        upstream.name,
                                        e
                                    );
                                    continue;
                                }
                            };

                            // Parse response and check for embedded errors
                            // Some providers (e.g., Xunfei) return HTTP 200 with error in body
                            if let Ok(resp_json) =
                                serde_json::from_slice::<serde_json::Value>(&resp_bytes)
                            {
                                // Check for embedded error in HTTP 200 response
                                // Format: {"error": {...}, "type": "error"} or {"type": "error", ...}
                                let is_error_response = resp_json
                                    .get("type")
                                    .map(|t| t.as_str() == Some("error"))
                                    .unwrap_or(false)
                                    || resp_json.get("error").is_some();

                                if is_error_response {
                                    let body_str = String::from_utf8_lossy(&resp_bytes);
                                    let error_info =
                                        parse_error_response(&body_str, &upstream.protocol);
                                    let error_message =
                                        error_info.message.as_deref().unwrap_or("Unknown error");

                                    tracing::warn!(
                                        "Passthrough: {} returned HTTP 200 with embedded error: {}",
                                        upstream.name,
                                        error_message
                                    );

                                    // Record as auth failure if it looks like an auth error
                                    let failure_type = if error_message.contains("AppIdNoAuth")
                                        || error_message.contains("NoAuth")
                                        || error_message.contains("Invalid")
                                        || error_message.contains("Expired")
                                        || error_message.contains("expired")
                                    {
                                        FailureType::AuthFailed
                                    } else {
                                        FailureType::ServerError
                                    };

                                    record_upstream_failure(
                                        state,
                                        upstream,
                                        model_name,
                                        failure_type.clone(),
                                        error_message,
                                        &session_id,
                                    );

                                    // Evict from affinity cache so next request tries a different channel
                                    if let Some(model) = model_name {
                                        state.affinity_cache.evict(&session_id, model);
                                    }
                                    tracing::info!(
                                        "Affinity evicted — embedded error in HTTP 200 for {}",
                                        upstream.name
                                    );

                                    last_error = error_message.to_string();
                                    continue; // Try next candidate
                                }

                                let resp_usage = parse_response_or_default(
                                    get_parser(channel_type).as_ref(),
                                    &resp_json,
                                    "passthrough",
                                );
                                token_counter.set_from_usage(&resp_usage);
                            }
                            
                            // Check response quality using the new quality detection system
                            // This replaces simple empty check with comprehensive quality analysis
                            let body_str = String::from_utf8_lossy(&resp_bytes);
                            let (quality, is_failure) = check_response_quality(
                                state,
                                upstream,
                                model_name,
                                &session_id,
                                &body_str,
                                status,
                                &resp_headers,
                            );
                            
                            if is_failure {
                                tracing::warn!(
                                    channel_id = %upstream.id,
                                    model = ?model_name,
                                    quality = ?quality,
                                    "Response quality check failed for passthrough"
                                );
                                continue; // Try next candidate
                            }

                            // L2 Shaper success — non-streaming passthrough.
                            // Use actual_tpm from parsed usage (audit decision D9 —
                            // non-streaming paths have complete usage data).
                            // Fall back to est_tpm when usage parsing yields 0.
                            let actual_tpm = token_counter.get_usage().total_tokens() as u64;
                            let commit_tpm = if actual_tpm > 0 {
                                actual_tpm
                            } else {
                                shaper_ctx.est_tpm
                            };
                            if let Some(g) = budget_guard.take() {
                                g.commit(commit_tpm);
                            }
                            return ProxyResult {
                                response: build_response_with_header(
                                    status,
                                    "content-type",
                                    "application/json",
                                    Body::from(resp_bytes),
                                ),
                                upstream_id: last_upstream_id,
                                final_status: status,
                                pricing_region: selected_pricing_region.clone(),
                                video_task_id: None,
                                shaper_outcome: Some((
                                    iter_label.to_string(),
                                    shaper_ctx.color.as_char().to_string(),
                                )),
                                routing_decision: sched_routing_decision.clone(),
                                sched_request_color: shaper_color,
                                error_type: None,
                            request_log_data: None,
                            };
                        }
                    } else {
                        // Non-success status (4xx) — capture headers before consuming body.
                        let resp_headers = resp.headers().clone();
                        let body_bytes = match resp.bytes().await {
                            Ok(b) => b,
                            Err(e) => {
                                last_error = format!("Failed to read error response: {e}");
                                // 4xx body-read failure: request DID reach
                                // upstream but actual usage = 0. Let
                                // budget_guard drop → full est_tpm refund.
                                return ProxyResult {
                                    response: build_response_with_header(
                                        status,
                                        "content-type",
                                        "application/json",
                                        Body::from(format!(
                                            r#"{{"error":{{"message":"{}","type":"upstream_error","code":"read_error"}}}}"#,
                                            last_error
                                        )),
                                    ),
                                    upstream_id: last_upstream_id,
                                    final_status: status,
                                    pricing_region: selected_pricing_region.clone(),
                                    video_task_id: None,
                                    shaper_outcome: Some((
                                        iter_label.to_string(),
                                        shaper_ctx.color.as_char().to_string(),
                                    )),
                                    routing_decision: sched_routing_decision.clone(),
                                    sched_request_color: shaper_color,
                                    error_type: Some("upstream_error".to_string()),
                        request_log_data: None,
                                };
                            }
                        };

                        // Classify all 4xx errors (not just 429) for circuit breaker
                        // and channel state tracking (P1 — passthrough error mapping).
                        let body_str = String::from_utf8_lossy(&body_bytes);
                        let error_info = parse_error_response(&body_str, &upstream.protocol);
                        let error_message =
                            error_info.message.as_deref().unwrap_or("Unknown error");
                        let failure_type =
                            classify_upstream_error(status, &resp_headers, &error_info);
                        // Auth/payment failures affect entire channel (model_name=None)
                        let error_model = match status {
                            StatusCode::UNAUTHORIZED | StatusCode::PAYMENT_REQUIRED => None,
                            _ => model_name,
                        };
                        record_upstream_failure(
                            state,
                            upstream,
                            error_model,
                            failure_type.clone(),
                            error_message,
                            &session_id,
                        );
                        // 429 (rate limit), 401 (auth failed), 402 (payment required):
                        // try next ranked candidate — other channels may have valid credentials.
                        if status == StatusCode::TOO_MANY_REQUESTS
                            || status == StatusCode::UNAUTHORIZED
                            || status == StatusCode::PAYMENT_REQUIRED
                        {
                            tracing::warn!(
                                "Passthrough: {} returned {}, trying next candidate",
                                upstream.name,
                                status.as_u16()
                            );
                            last_error = error_message.to_string();
                            continue;
                        }

                        // Other 4xx response: actual usage = 0. budget_guard drops
                        // on return → full est_tpm refund (no commit).
                        let et = match failure_type {
                            FailureType::AuthFailed => "auth_failed",
                            FailureType::RateLimited { .. } => "rate_limit",
                            FailureType::ModelNotFound => "upstream_error",
                            FailureType::PaymentRequired => "upstream_error",
                            FailureType::ServerError => "upstream_error",
                            FailureType::Timeout => "timeout",
                            FailureType::ConnectionError => "upstream_error",
                            FailureType::EmptyResponse => "empty_response",
                        };
                        return ProxyResult {
                            response: build_response_with_header(
                                status,
                                "content-type",
                                "application/json",
                                Body::from(body_bytes),
                            ),
                            upstream_id: last_upstream_id,
                            final_status: status,
                            pricing_region: selected_pricing_region.clone(),
                            video_task_id: None,
                            shaper_outcome: Some((
                                iter_label.to_string(),
                                shaper_ctx.color.as_char().to_string(),
                            )),
                            routing_decision: sched_routing_decision.clone(),
                            sched_request_color: shaper_color,
                            error_type: Some(et.to_string()),
                            request_log_data: None,
                        };
                    }
                }
                Err(e) => {
                    last_error = format!("Network Error: {e}");
                    let failure_type = if e.is_timeout() {
                        FailureType::Timeout
                    } else {
                        FailureType::ConnectionError
                    };
                    record_upstream_failure(
                        state,
                        upstream,
                        model_name,
                        failure_type,
                        &last_error,
                        &session_id,
                    );
                    tracing::warn!(
                        "Failover: {} network error: {}, trying next...",
                        upstream.name,
                        e
                    );
                    continue;
                }
            }
        }

        // 5. Use DynamicAdaptorFactory to get adaptor (supports both static and dynamic configs)
        let adaptor = state
            .adaptor_factory
            .get_adaptor(channel_type, upstream.api_version.as_deref())
            .await;

        // 6. Prepare Request Body (for conversion mode)
        // Preserve stream flag from original request before conversion
        let original_stream = body_json
            .get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Extract model name from original request before conversion
        let original_model = body_json
            .get("model")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());

        let request_body_json: Option<serde_json::Value> =
            if let Ok(req) = serde_json::from_slice::<OpenAIChatRequest>(&body_bytes) {
                let mut converted = adaptor
                    .convert_request(&req)
                    .or_else(|| Some(serde_json::json!(req))); // Use converted or original

                // Preserve stream flag and model in converted body for adaptor's build_request
                #[allow(clippy::collapsible_match)]
                if let Some(ref mut body) = converted {
                    if let serde_json::Value::Object(ref mut map) = body {
                        if original_stream {
                            map.insert("stream".to_string(), serde_json::Value::Bool(true));
                        }
                        // Preserve model name for adaptors that need it (e.g., Gemini)
                        if let Some(ref model) = original_model {
                            map.insert(
                                "model".to_string(),
                                serde_json::Value::String(model.clone()),
                            );
                        }
                    }
                }
                converted
            } else {
                // Use the already parsed JSON
                Some(body_json)
            };

        // SAFETY: The two branches above both return Some
        let mut request_body_json = match request_body_json {
            Some(json) => json,
            None => {
                last_error = "Failed to prepare request body".to_string();
                continue;
            }
        };

        // Check if streaming is requested
        let is_stream = request_body_json
            .get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Apply param_override
        if let Some(ref override_str) = upstream.param_override {
            if let Ok(serde_json::Value::Object(override_map)) =
                serde_json::from_str::<serde_json::Value>(override_str)
            {
                if let serde_json::Value::Object(ref mut body_map) = request_body_json {
                    for (k, v) in override_map {
                        body_map.insert(k, v);
                    }
                    tracing::debug!("Applied param_override for {}", upstream.name);
                }
            }
        }

        // 4. Build Request via Adaptor
        let req_builder = state.client.request(method.clone(), &target_url);

        // Apply header_override
        let req_builder = apply_header_override(req_builder, upstream.header_override.as_deref());

        let req_builder = adaptor
            .build_request(
                &state.client,
                req_builder,
                &upstream.api_key,
                &request_body_json,
            )
            .await;

        // 5. Execute
        match req_builder.send().await {
            Ok(resp) => {
                let status = resp.status();
                let resp_headers = resp.headers().clone();

                // Handle different response status codes
                if status.is_server_error() {
                    // 5xx Server Error
                    last_error = format!("Upstream returned {status}");
                    record_upstream_failure(
                        state,
                        upstream,
                        model_name,
                        FailureType::ServerError,
                        &last_error,
                        &session_id,
                    );
                    continue;
                }

                if resp.status().is_success() {
                    // For streaming responses, delay recording success until we confirm non-empty response
                    if !is_stream {
                        record_upstream_success(state, upstream, model_name, &session_id);
                    }
                    let status = resp.status();

                    // Parse rate limit info from response headers
                    let rate_limit_info = parse_rate_limit_info(
                        &resp_headers,
                        None, // No body for success
                        &upstream.protocol,
                    );

                    // Record success in channel state tracker with learned upstream limit
                    let channel_id: i32 = upstream.id.parse().unwrap_or(0);
                    let latency_ms = request_start_time.elapsed().as_millis() as u64;
                    state
                        .channel_state_tracker
                        .record_success(
                            channel_id,
                            model_name,
                            latency_ms,
                            rate_limit_info.request_limit,
                        )
                        .inspect(|&learned| {
                            let _ = state.budget_update_tx.try_send(state::BudgetUpdate {
                                channel_id,
                                learned_limit: learned,
                            });
                        });

                    // Log rate limit info for debugging/monitoring
                    if rate_limit_info.request_limit.is_some()
                        || rate_limit_info.token_limit.is_some()
                    {
                        tracing::debug!(
                            "Rate limit info for {}: requests={:?}, tokens={:?}, remaining={:?}, retry_after={:?}",
                            upstream.name,
                            rate_limit_info.request_limit,
                            rate_limit_info.token_limit,
                            rate_limit_info.remaining,
                            rate_limit_info.retry_after
                        );
                    }

                    // Optimization: If protocol is OpenAI, we can stream directly without parsing/buffering
                    // This satisfies the "Passthrough Principle" and enables streaming.
                    if upstream.protocol == "openai" {
                        // For Seedance video generation, buffer response to extract task_id.
                        // The response is small JSON (not a stream), so buffering is safe.
                        if path == "/v1/video/generations" {
                            let resp_bytes = match resp.bytes().await {
                                Ok(b) => b,
                                Err(e) => {
                                    last_error = format!("Failed to read video gen response: {e}");
                                    continue;
                                }
                            };
                            let task_id = serde_json::from_slice::<serde_json::Value>(&resp_bytes)
                                .ok()
                                .and_then(|v| {
                                    v.get("id")
                                        .and_then(|id| id.as_str())
                                        .map(|s| s.to_string())
                                });
                            // Parse usage if present (Seedance has none, but be safe)
                            if let Ok(resp_json) =
                                serde_json::from_slice::<serde_json::Value>(&resp_bytes)
                            {
                                let resp_usage = parse_response_or_default(
                                    get_parser(channel_type).as_ref(),
                                    &resp_json,
                                    "video-gen",
                                );
                                token_counter.set_from_usage(&resp_usage);
                            }
                            // L2 Shaper success — video-gen non-streaming (actual_tpm available).
                            let actual_tpm = token_counter.get_usage().total_tokens() as u64;
                            let commit_tpm = if actual_tpm > 0 {
                                actual_tpm
                            } else {
                                shaper_ctx.est_tpm
                            };
                            if let Some(g) = budget_guard.take() {
                                g.commit(commit_tpm);
                            }
                            return ProxyResult {
                                response: build_response_with_header(
                                    status,
                                    "content-type",
                                    "application/json",
                                    Body::from(resp_bytes),
                                ),
                                upstream_id: last_upstream_id,
                                final_status: status,
                                pricing_region: selected_pricing_region.clone(),
                                video_task_id: task_id,
                                shaper_outcome: Some((
                                    iter_label.to_string(),
                                    shaper_ctx.color.as_char().to_string(),
                                )),
                                routing_decision: sched_routing_decision.clone(),
                                sched_request_color: shaper_color,
                                error_type: None,
                            request_log_data: None,
                            };
                        }
                        // L2 Shaper success: OpenAI streaming path — keep est_tpm
                        // (actual_tpm not yet available during stream, audit decision D9).
                        if let Some(g) = budget_guard.take() {
                            g.commit(shaper_ctx.est_tpm);
                        }

                        // Peek first chunk to detect errors before sending HTTP response
                        // This allows retry on auth errors instead of sending error to user
                        let peek_timeout = std::time::Duration::from_secs(PEEK_FIRST_CHUNK_TIMEOUT_SECS);
                        let mut peek_error_handled = false;
                        let body_stream = resp.bytes_stream();
                        
                        // Peek and check for errors
                        let body_stream = {
                            let body_stream_for_peek = body_stream;
                            match crate::stream_peek::peek_first_chunk(body_stream_for_peek, peek_timeout).await {
                                crate::stream_peek::PeekResult::HasFirstChunk { first_chunk, remaining_stream } => {
                                    if let Some((error_code, error_msg, is_auth)) = 
                                        crate::stream_peek::check_sse_error_in_chunk(&first_chunk) {
                                        // Error detected - record and mark for retry
                                        let failure_type = if is_auth { FailureType::AuthFailed } else { FailureType::ServerError };
                                        tracing::warn!(
                                            channel_id = %upstream.id,
                                            error_code = error_code,
                                            error_msg = error_msg,
                                            ?failure_type,
                                            "SSE error in first chunk (OpenAI) - retrying"
                                        );
                                        state.circuit_breaker.record_failure_with_type(&upstream.id, failure_type.clone());
                                        state.channel_state_tracker.record_error(
                                            upstream.id.parse().unwrap_or(0),
                                            model_name.as_deref(),
                                            &failure_type,
                                            &format!("SSE error: {}", error_msg),
                                        );
                                        if let Some(model) = model_name {
                                            state.affinity_cache.evict(&session_id.to_string(), model);
                                        }
                                        last_error = format!("SSE error {}: {}", error_code, error_msg);
                                        peek_error_handled = true;
                                        // Return empty stream since we will continue anyway
                                        futures::stream::empty::<Result<axum::body::Bytes, reqwest::Error>>().boxed()
                                    } else {
                                        // No error - prepend first chunk
                                        let first_chunk_stream = futures::stream::once(async move { Ok(first_chunk) });
                                        first_chunk_stream.chain(remaining_stream).boxed()
                                    }
                                }
                                crate::stream_peek::PeekResult::Empty => {
                                    tracing::warn!(channel_id = %upstream.id, "Empty first chunk (OpenAI) - retrying");
                                    state.circuit_breaker.record_failure_with_type(&upstream.id, FailureType::EmptyResponse);
                                    last_error = "Empty response".to_string();
                                    peek_error_handled = true;
                                    futures::stream::empty().boxed()
                                }
                                crate::stream_peek::PeekResult::Error(e) => {
                                    tracing::error!(channel_id = %upstream.id, error = ?e, "Peek error (OpenAI)");
                                    state.circuit_breaker.record_failure_with_type(&upstream.id, FailureType::EmptyResponse);
                                    last_error = format!("Network error: {}", e);
                                    peek_error_handled = true;
                                    futures::stream::empty().boxed()
                                }
                                crate::stream_peek::PeekResult::Timeout { stream } => {
                                    tracing::info!(channel_id = %upstream.id, "Peek timeout (OpenAI) - proceeding");
                                    stream
                                }
                            }
                        };
                        
                        // If peek detected an error, skip this channel and try next
                        if peek_error_handled {
                            continue;
                        }
                        
                        let counter_clone = Arc::clone(&token_counter);
                        let parser = get_parser(channel_type);

                        // Clone state and upstream info for post-stream empty response check
                        let state_clone = state.clone();
                        let upstream_id_str = upstream.id.clone();
                        let upstream_name = upstream.name.clone();
                        let model_name_clone = model_name.map(|s| s.to_string());
                        let session_id_clone = session_id.to_string();
                        let channel_id: i32 = upstream.id.parse().unwrap_or(0);

                        // Track if we've seen any tokens during streaming
                        let seen_tokens = Arc::new(std::sync::atomic::AtomicBool::new(false));
                        let seen_tokens_clone = Arc::clone(&seen_tokens);
                        
                        // Track if SSE error was detected during streaming
                        let sse_error_detected = Arc::new(std::sync::atomic::AtomicBool::new(false));
                        let sse_error_detected_clone = Arc::clone(&sse_error_detected);
                        let sse_error_is_auth = Arc::new(std::sync::atomic::AtomicBool::new(false));
                        let sse_error_is_auth_clone = Arc::clone(&sse_error_is_auth);
                        
                        // Clone for stream closure (original will be used in done closure)
                        let upstream_id_str_for_stream = upstream_id_str.clone();

                        let stream = body_stream.map(move |chunk_result| {
                            match chunk_result {
                                Ok(bytes) => {
                                    let text = String::from_utf8_lossy(&bytes);
                                    if let Some(u) = parse_chunk_or_default(parser.as_ref(), &text, "stream") {
                                        if !u.is_empty() {
                                            seen_tokens_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                                        }
                                        match parser.provider_name() {
                                            "anthropic" => counter_clone.accumulate(&u),
                                            _ => counter_clone.set_from_usage(&u),
                                        }
                                    }
                                    
                                    // Check for SSE errors and content in the stream
                                    if !seen_tokens_clone.load(std::sync::atomic::Ordering::Relaxed) {
                                        for line in text.lines() {
                                            let line = line.trim();
                                            if !line.starts_with("data: ") {
                                                continue;
                                            }
                                            let data = &line[6..];
                                            if data.trim() == "[DONE]" {
                                                continue;
                                            }
                                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                                // Check for SSE error response (e.g., AppIdNoAuthError from Xunfei)
                                                if let Some(error) = json.get("error") {
                                                    let error_msg = error.get("message")
                                                        .and_then(|m| m.as_str())
                                                        .unwrap_or("Unknown SSE error");
                                                    let error_code = error.get("code")
                                                        .and_then(|c| c.as_u64())
                                                        .unwrap_or(400) as u16;
                                                    
                                                    // Check if this is an auth error
                                                    let msg_lower = error_msg.to_lowercase();
                                                    let is_auth_error = msg_lower.contains("auth") 
                                                        || msg_lower.contains("appid") 
                                                        || msg_lower.contains("unauthorized")
                                                        || msg_lower.contains("invalid key")
                                                        || error_code == 401;
                                                    
                                                    tracing::error!(
                                                        channel_id = %upstream_id_str_for_stream,
                                                        error_code,
                                                        error_msg,
                                                        is_auth = is_auth_error,
                                                        "SSE streaming error detected"
                                                    );
                                                    
                                                    // Set error flags for done closure to handle
                                                    sse_error_detected_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                                                    sse_error_is_auth_clone.store(is_auth_error, std::sync::atomic::Ordering::Relaxed);
                                                }
                                                // Then check for actual content in the stream
                                                else if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                                                    for choice in choices {
                                                        if let Some(delta) = choice.get("delta") {
                                                            if delta.get("content").and_then(|c| c.as_str()).is_some() {
                                                                seen_tokens_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Ok(bytes)
                                }
                                Err(e) => Err(std::io::Error::other(e)),
                            }
                        });

                        // Add post-stream check for empty response with sliding window counter
                        let done = futures::stream::once(async move {
                            if !seen_tokens.load(std::sync::atomic::Ordering::Relaxed) {
                                // Use sliding window counter: only penalize after consecutive empty responses
                                let should_penalize = state_clone.empty_response_counter.record_empty(&upstream_id_str);
                                
                                if should_penalize {
                                    tracing::warn!(
                                        channel_id = %upstream_id_str,
                                        model = ?model_name_clone,
                                        "Consecutive empty streaming responses exceeded threshold, treating as failure"
                                    );
                                    
                                    // Threshold exceeded - record failure
                                    state_clone.circuit_breaker.record_failure_with_type(
                                        &upstream_id_str,
                                        FailureType::EmptyResponse,
                                    );
                                    state_clone.channel_state_tracker.record_error(
                                        channel_id,
                                        model_name_clone.as_deref(),
                                        &FailureType::EmptyResponse,
                                        "Consecutive empty streaming responses exceeded threshold",
                                    );

                                    if let Some(model) = &model_name_clone {
                                        state_clone.affinity_cache.evict(&session_id_clone, model);
                                        tracing::debug!(
                                            session_id = %session_id_clone,
                                            model = %model,
                                            channel_id,
                                            "Affinity evicted — consecutive empty responses for {}",
                                            upstream_name
                                        );
                                    }
                                } else {
                                    tracing::debug!(
                                        channel_id = %upstream_id_str,
                                        "Empty streaming response recorded but threshold not yet exceeded"
                                    );
                                }
                            } else {
                                // Successful response - reset the counter and record success
                                state_clone.empty_response_counter.reset(&upstream_id_str);
                                // Now we can safely record success for the streaming response
                                state_clone.circuit_breaker.record_success(&upstream_id_str);
                                if let Some(model) = &model_name_clone {
                                    state_clone.affinity_cache.insert(&session_id_clone, model, channel_id);
                                }
                            }
                            Ok(axum::body::Bytes::new())
                        });

                        let final_stream = stream.chain(done);

                        return ProxyResult {
                            response: Response::builder()
                                .status(status)
                                .header("content-type", "text/event-stream")
                                .header("cache-control", "no-cache")
                                .header("connection", "keep-alive")
                                .body(Body::from_stream(final_stream))
                                .unwrap_or_else(|_| {
                                    build_response(
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        Body::from(r#"{"error":{"message":"Failed to build streaming response","type":"internal_error","code":"stream_build_failed"}}"#),
                                    )
                                }),
                            upstream_id: last_upstream_id,
                            final_status: status,
                            pricing_region: selected_pricing_region.clone(),
                            video_task_id: None,
                            shaper_outcome: Some((
                                iter_label.to_string(),
                                shaper_ctx.color.as_char().to_string(),
                            )),
                            routing_decision: sched_routing_decision.clone(),
                            sched_request_color: shaper_color,
                            error_type: None,
                            request_log_data: None,
                        };
                    }

                    // Handle Streaming for non-OpenAI with peek
                    if is_stream {
                        // Peek first chunk to detect errors before sending HTTP response
                        let peek_timeout = std::time::Duration::from_secs(PEEK_FIRST_CHUNK_TIMEOUT_SECS);
                        let mut peek_error_handled = false;
                        let body_stream = resp.bytes_stream();
                        
                        let body_stream = {
                            let body_stream_for_peek = body_stream;
                            match crate::stream_peek::peek_first_chunk(body_stream_for_peek, peek_timeout).await {
                                crate::stream_peek::PeekResult::HasFirstChunk { first_chunk, remaining_stream } => {
                                    if let Some((error_code, error_msg, is_auth)) = 
                                        crate::stream_peek::check_sse_error_in_chunk(&first_chunk) {
                                        let failure_type = if is_auth { FailureType::AuthFailed } else { FailureType::ServerError };
                                        tracing::warn!(
                                            channel_id = %upstream.id,
                                            error_code = error_code,
                                            error_msg = error_msg,
                                            ?failure_type,
                                            "SSE error in first chunk (non-OpenAI) - retrying"
                                        );
                                        state.circuit_breaker.record_failure_with_type(&upstream.id, failure_type.clone());
                                        state.channel_state_tracker.record_error(
                                            upstream.id.parse().unwrap_or(0),
                                            model_name.as_deref(),
                                            &failure_type,
                                            &format!("SSE error: {}", error_msg),
                                        );
                                        if let Some(model) = model_name {
                                            state.affinity_cache.evict(&session_id.to_string(), model);
                                        }
                                        last_error = format!("SSE error {}: {}", error_code, error_msg);
                                        peek_error_handled = true;
                                        futures::stream::empty::<Result<axum::body::Bytes, reqwest::Error>>().boxed()
                                    } else {
                                        let first_chunk_stream = futures::stream::once(async move { Ok(first_chunk) });
                                        first_chunk_stream.chain(remaining_stream).boxed()
                                    }
                                }
                                crate::stream_peek::PeekResult::Empty => {
                                    tracing::warn!(channel_id = %upstream.id, "Empty first chunk (non-OpenAI) - retrying");
                                    state.circuit_breaker.record_failure_with_type(&upstream.id, FailureType::EmptyResponse);
                                    last_error = "Empty response".to_string();
                                    peek_error_handled = true;
                                    futures::stream::empty().boxed()
                                }
                                crate::stream_peek::PeekResult::Error(e) => {
                                    tracing::error!(channel_id = %upstream.id, error = ?e, "Peek error (non-OpenAI)");
                                    state.circuit_breaker.record_failure_with_type(&upstream.id, FailureType::EmptyResponse);
                                    last_error = format!("Network error: {}", e);
                                    peek_error_handled = true;
                                    futures::stream::empty().boxed()
                                }
                                crate::stream_peek::PeekResult::Timeout { stream } => {
                                    tracing::info!(channel_id = %upstream.id, "Peek timeout (non-OpenAI) - proceeding");
                                    stream
                                }
                            }
                        };
                        
                        if peek_error_handled {
                            continue;
                        }
                        
                        let adaptor_clone = Arc::clone(&adaptor);
                        let counter_clone = Arc::clone(&token_counter);
                        let parser = get_parser(channel_type);

                        // Clone state and upstream info for post-stream empty response check
                        let state_clone = state.clone();
                        let upstream_id_str = upstream.id.clone();
                        let upstream_name = upstream.name.clone();
                        let model_name_clone = model_name.map(|s| s.to_string());
                        let session_id_clone = session_id.to_string();
                        let channel_id: i32 = upstream.id.parse().unwrap_or(0);

                        // Track if we've seen any tokens during streaming
                        let seen_tokens = Arc::new(std::sync::atomic::AtomicBool::new(false));
                        let seen_tokens_clone = Arc::clone(&seen_tokens);
                        
                        // Track if SSE error was detected during streaming
                        let sse_error_detected = Arc::new(std::sync::atomic::AtomicBool::new(false));
                        let sse_error_detected_clone = Arc::clone(&sse_error_detected);
                        let sse_error_is_auth = Arc::new(std::sync::atomic::AtomicBool::new(false));
                        let sse_error_is_auth_clone = Arc::clone(&sse_error_is_auth);
                        
                        // Clone for stream closure (original will be used in done closure)
                        let upstream_id_str_for_stream = upstream_id_str.clone();

                        let stream = body_stream.map(move |chunk_result| {
                            match chunk_result {
                                Ok(bytes) => {
                                    let text = String::from_utf8_lossy(&bytes);

                                    // Parse token usage from streaming response
                                    // Anthropic sends incremental events; others send cumulative totals
                                    if let Some(u) =
                                        parse_chunk_or_default(parser.as_ref(), &text, "stream")
                                    {
                                        if !u.is_empty() {
                                            seen_tokens_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                                        }
                                        match parser.provider_name() {
                                            "anthropic" => counter_clone.accumulate(&u),
                                            _ => counter_clone.set_from_usage(&u),
                                        }
                                    }
                                    
                                    // Check for SSE errors and content in the stream
                                    if !seen_tokens_clone.load(std::sync::atomic::Ordering::Relaxed) {
                                        for line in text.lines() {
                                            let line = line.trim();
                                            if !line.starts_with("data: ") {
                                                continue;
                                            }
                                            let data = &line[6..];
                                            if data.trim() == "[DONE]" {
                                                continue;
                                            }
                                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                                // Check for SSE error response (e.g., AppIdNoAuthError from Xunfei)
                                                if let Some(error) = json.get("error") {
                                                    let error_msg = error.get("message")
                                                        .and_then(|m| m.as_str())
                                                        .unwrap_or("Unknown SSE error");
                                                    let error_code = error.get("code")
                                                        .and_then(|c| c.as_u64())
                                                        .unwrap_or(400) as u16;
                                                    
                                                    // Check if this is an auth error
                                                    let msg_lower = error_msg.to_lowercase();
                                                    let is_auth_error = msg_lower.contains("auth") 
                                                        || msg_lower.contains("appid") 
                                                        || msg_lower.contains("unauthorized")
                                                        || msg_lower.contains("invalid key")
                                                        || error_code == 401;
                                                    
                                                    tracing::error!(
                                                        channel_id = %upstream_id_str_for_stream,
                                                        error_code,
                                                        error_msg,
                                                        is_auth = is_auth_error,
                                                        "SSE streaming error detected"
                                                    );
                                                    
                                                    // Set error flags for done closure to handle
                                                    sse_error_detected_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                                                    sse_error_is_auth_clone.store(is_auth_error, std::sync::atomic::Ordering::Relaxed);
                                                }
                                                // Then check for actual content in the stream
                                                else if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                                                    for choice in choices {
                                                        if let Some(delta) = choice.get("delta") {
                                                            if delta.get("content").and_then(|c| c.as_str()).is_some() {
                                                                seen_tokens_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    if let Some(converted) =
                                        adaptor_clone.convert_stream_response(&text)
                                    {
                                        Ok(axum::body::Bytes::from(converted))
                                    } else {
                                        Ok(axum::body::Bytes::new())
                                    }
                                }
                                Err(e) => Err(std::io::Error::other(e)),
                            }
                        });

                        let done = futures::stream::once(async move {
                            // Check for SSE error first (takes priority over empty response)
                            if sse_error_detected.load(std::sync::atomic::Ordering::Relaxed) {
                                // SSE error was detected during streaming
                                let is_auth = sse_error_is_auth.load(std::sync::atomic::Ordering::Relaxed);
                                let failure_type = if is_auth {
                                    FailureType::AuthFailed
                                } else {
                                    FailureType::ServerError
                                };
                                
                                tracing::warn!(
                                    channel_id = %upstream_id_str,
                                    model = ?model_name_clone,
                                    ?failure_type,
                                    "SSE streaming error - recording failure to circuit breaker"
                                );
                                
                                // Record failure with correct type (AuthFailed triggers 30-min cooldown)
                                state_clone.circuit_breaker.record_failure_with_type(
                                    &upstream_id_str,
                                    failure_type.clone(),
                                );
                                state_clone.channel_state_tracker.record_error(
                                    channel_id,
                                    model_name_clone.as_deref(),
                                    &failure_type,
                                    "SSE streaming error detected",
                                );
                                
                                // Evict affinity
                                if let Some(model) = &model_name_clone {
                                    state_clone.affinity_cache.evict(&session_id_clone, model);
                                }
                            }
                            // Check for empty response after stream ends with sliding window counter
                            else if !seen_tokens.load(std::sync::atomic::Ordering::Relaxed) {
                                // Use sliding window counter: only penalize after consecutive empty responses
                                let should_penalize = state_clone.empty_response_counter.record_empty(&upstream_id_str);
                                
                                if should_penalize {
                                    tracing::warn!(
                                        channel_id = %upstream_id_str,
                                        model = ?model_name_clone,
                                        "Consecutive empty streaming responses exceeded threshold, treating as failure"
                                    );
                                    
                                    // Threshold exceeded - record failure to circuit breaker and channel state
                                    state_clone.circuit_breaker.record_failure_with_type(
                                        &upstream_id_str,
                                        FailureType::EmptyResponse,
                                    );
                                    state_clone.channel_state_tracker.record_error(
                                        channel_id,
                                        model_name_clone.as_deref(),
                                        &FailureType::EmptyResponse,
                                        "Consecutive empty streaming responses exceeded threshold",
                                    );

                                    // Evict affinity so next request tries different channel
                                    if let Some(model) = &model_name_clone {
                                        state_clone.affinity_cache.evict(&session_id_clone, model);
                                        tracing::debug!(
                                            session_id = %session_id_clone,
                                            model = %model,
                                            channel_id,
                                            "Affinity evicted — consecutive empty responses for {}",
                                            upstream_name
                                        );
                                    }
                                } else {
                                    tracing::debug!(
                                        channel_id = %upstream_id_str,
                                        "Empty streaming response recorded but threshold not yet exceeded"
                                    );
                                }
                            } else {
                                // Successful response - reset the counter and record success
                                state_clone.empty_response_counter.reset(&upstream_id_str);
                                // Now we can safely record success for the streaming response
                                state_clone.circuit_breaker.record_success(&upstream_id_str);
                                if let Some(model) = &model_name_clone {
                                    state_clone.affinity_cache.insert(&session_id_clone, model, channel_id);
                                }
                            }
                            Ok(axum::body::Bytes::from(SSE_DONE_MARKER))
                        });
                        let final_stream = stream.chain(done);

                        // L2 Shaper success — non-OpenAI streaming path — keep est_tpm
                        // (actual_tpm not yet available during stream, audit decision D9).
                        if let Some(g) = budget_guard.take() {
                            g.commit(shaper_ctx.est_tpm);
                        }
                        return ProxyResult {
                            response: Response::builder()
                                .status(status)
                                .header("content-type", "text/event-stream")
                                .header("cache-control", "no-cache")
                                .header("connection", "keep-alive")
                                .body(Body::from_stream(final_stream))
                                .unwrap_or_else(|_| {
                                    build_response(
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        Body::from(r#"{"error":{"message":"Failed to build streaming response","type":"internal_error","code":"stream_build_failed"}}"#),
                                    )
                                }),
                            upstream_id: last_upstream_id,
                            final_status: status,
                            pricing_region: selected_pricing_region.clone(),
                            video_task_id: None,
                            shaper_outcome: Some((
                                iter_label.to_string(),
                                shaper_ctx.color.as_char().to_string(),
                            )),
                            routing_decision: sched_routing_decision.clone(),
                            sched_request_color: shaper_color,
                            error_type: None,
                            request_log_data: None,
                        };
                    }

                    // 6. Handle Response Conversion
                    // Read body to memory to convert
                    let resp_json: serde_json::Value = match resp.json().await {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse response JSON from {}: {}",
                                upstream.name,
                                e
                            );
                            serde_json::json!({})
                        }
                    };

                    // Extract token usage from non-streaming response for billing
                    {
                        let resp_usage = parse_response_or_default(
                            get_parser(channel_type).as_ref(),
                            &resp_json,
                            "non-stream",
                        );
                        token_counter.set_from_usage(&resp_usage);
                    }

                    let response_body = if let Some(converted) =
                        adaptor.convert_response(resp_json.clone(), &upstream.name)
                    {
                        // Also extract usage from converted response if not yet captured
                        if token_counter.get_usage().is_empty() {
                            let conv_usage = parse_response_or_default(
                                get_parser(channel_type).as_ref(),
                                &converted,
                                "non-stream-converted",
                            );
                            token_counter.set_from_usage(&conv_usage);
                        }
                        serde_json::to_string(&converted).unwrap_or_else(|_| "{}".to_string())
                    } else {
                        // No conversion needed (e.g. OpenAI), return original body
                        serde_json::to_string(&resp_json).unwrap_or_else(|_| "{}".to_string())
                    };

                    // L2 Shaper success — non-OpenAI non-streaming (actual_tpm available).
                    let actual_tpm = token_counter.get_usage().total_tokens() as u64;
                    let commit_tpm = if actual_tpm > 0 {
                        actual_tpm
                    } else {
                        shaper_ctx.est_tpm
                    };
                    if let Some(g) = budget_guard.take() {
                        g.commit(commit_tpm);
                    }

                    // Check response quality using the new quality detection system
                    // This replaces simple empty check with comprehensive quality analysis
                    let (quality, is_failure) = check_response_quality(
                        state,
                        upstream,
                        model_name,
                        &session_id,
                        &response_body,
                        status,
                        &resp_headers,
                    );
                    
                    if is_failure {
                        tracing::warn!(
                            channel_id = %upstream.id,
                            model = ?model_name,
                            quality = ?quality,
                            "Response quality check failed for non-streaming main path"
                        );
                        continue; // Try next candidate
                    }

                    return ProxyResult {
                        response: build_response_with_header(
                            status,
                            "content-type",
                            "application/json",
                            Body::from(response_body),
                        ),
                        upstream_id: last_upstream_id,
                        final_status: status,
                        pricing_region: selected_pricing_region.clone(),
                        video_task_id: None,
                        shaper_outcome: Some((
                            iter_label.to_string(),
                            shaper_ctx.color.as_char().to_string(),
                        )),
                        routing_decision: sched_routing_decision.clone(),
                        sched_request_color: shaper_color,
                        error_type: None,
                            request_log_data: None,
                    };
                } else {
                    // Handle non-success responses (4xx errors)
                    let body_bytes = match resp.bytes().await {
                        Ok(b) => b,
                        Err(e) => {
                            // If we can't read the body, return a simple error
                            last_error = format!("Failed to read response body: {e}");
                            // 4xx body-read failure: budget_guard drops here
                            // → full est_tpm refund (request reached upstream
                            // but no actual usage was billed).
                            return ProxyResult {
                                response: build_response_with_header(
                                        status,
                                        "content-type",
                                        "application/json",
                                        Body::from(format!(
                                            r#"{{"error":{{"message":"{}","type":"upstream_error","code":"read_error"}}}}"#,
                                            last_error
                                        )),
                                    ),
                                upstream_id: last_upstream_id,
                                final_status: status,
                                pricing_region: selected_pricing_region.clone(),
                                video_task_id: None,
                                shaper_outcome: Some((
                                    iter_label.to_string(),
                                    shaper_ctx.color.as_char().to_string(),
                                )),
                                routing_decision: sched_routing_decision.clone(),
                                sched_request_color: shaper_color,
                                error_type: Some("upstream_error".to_string()),
                        request_log_data: None,
                            };
                        }
                    };
                    let body_str = String::from_utf8_lossy(&body_bytes);

                    // Parse error response
                    let error_info = parse_error_response(&body_str, &upstream.protocol);
                    let error_message = error_info.message.as_deref().unwrap_or("Unknown error");

                    // Determine failure type using shared classifier (D12)
                    let failure_type = classify_upstream_error(status, &resp_headers, &error_info);

                    // Auth/payment failures affect entire channel (model_name=None)
                    let error_model = match status {
                        StatusCode::UNAUTHORIZED | StatusCode::PAYMENT_REQUIRED => None,
                        _ => model_name,
                    };

                    record_upstream_failure(
                        state,
                        upstream,
                        error_model,
                        failure_type.clone(),
                        error_message,
                        &session_id,
                    );

                    // 429: try next ranked candidate (scheduler provides alternatives)
                    if status == StatusCode::TOO_MANY_REQUESTS {
                        tracing::warn!(
                            "Upstream {} rate limited, trying next candidate",
                            upstream.name
                        );
                        continue;
                    }

                    // Check for API version deprecation and auto-update if detected
                    if adaptor::detector::ApiVersionDetector::is_deprecation_error(error_message) {
                        let channel_id_for_detector: i32 = upstream.id.parse().unwrap_or(0);
                        let adaptor_factory_for_detector = state.adaptor_factory.clone();
                        let detector = state.api_version_detector.clone();
                        let error_message_for_detector = error_message.to_string();

                        tokio::spawn(async move {
                            match detector
                                .detect_and_update(
                                    channel_id_for_detector,
                                    &error_message_for_detector,
                                    &adaptor_factory_for_detector,
                                )
                                .await
                            {
                                Ok(Some(new_version)) => {
                                    tracing::info!(
                                        "API version deprecation detected, updated channel {} to version: {}",
                                        channel_id_for_detector, new_version
                                    );
                                }
                                Ok(None) => {
                                    // No deprecation detected or no new version found
                                }
                                Err(e) => {
                                    tracing::error!("Failed to detect/update API version: {}", e);
                                }
                            }
                        });
                    }

                    // Log the error
                    tracing::warn!(
                        "Upstream {} returned {}: {}",
                        upstream.name,
                        status.as_u16(),
                        error_message
                    );

                    // 4xx response from non-OpenAI: budget_guard drops on
                    // return → full est_tpm refund (request reached upstream
                    // but no actual usage was billed).
                    let et = match failure_type {
                        FailureType::AuthFailed => "auth_failed",
                        FailureType::RateLimited { .. } => "rate_limit",
                        FailureType::ModelNotFound => "upstream_error",
                        FailureType::PaymentRequired => "upstream_error",
                        FailureType::ServerError => "upstream_error",
                        FailureType::Timeout => "timeout",
                        FailureType::ConnectionError => "upstream_error",
                        FailureType::EmptyResponse => "empty_response",
                    };
                    return ProxyResult {
                        response: build_response_with_header(
                            status,
                            "content-type",
                            "application/json",
                            Body::from(body_bytes),
                        ),
                        upstream_id: last_upstream_id,
                        final_status: status,
                        pricing_region: selected_pricing_region.clone(),
                        video_task_id: None,
                        shaper_outcome: Some((
                            iter_label.to_string(),
                            shaper_ctx.color.as_char().to_string(),
                        )),
                        routing_decision: sched_routing_decision.clone(),
                        sched_request_color: shaper_color,
                        error_type: Some(et.to_string()),
                            request_log_data: None,
                    };
                }
            }
            Err(e) => {
                last_error = format!("Network Error: {e}");
                let failure_type = if e.is_timeout() {
                    FailureType::Timeout
                } else {
                    FailureType::ConnectionError
                };
                record_upstream_failure(
                    state,
                    upstream,
                    model_name,
                    failure_type,
                    &last_error,
                    &session_id,
                );
                tracing::warn!(
                    "Failover: {} failed with {}, trying next...",
                    upstream.name,
                    e
                );
                continue;
            }
        }
    }

    // After-loop branch: every candidate was rejected by the L2 Shaper
    // (no CB skip, no upstream failure). Return 503 + X-Rejected-By: shaper
    // per audit decision D12 — clients can distinguish a local shaper reject
    // from upstream 5xx and back off via Retry-After.
    if shaper_ctx.rejected_count > 0 && shaper_ctx.rejected_count as usize == total_candidates {
        let body = Body::from(
            r#"{"error":{"message":"All candidate channels rejected by rate budget shaper","type":"service_unavailable","code":"rate_budget_exhausted","rejected_by":"shaper"}}"#,
        );
        let response = Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header("content-type", "application/json")
            .header("X-Rejected-By", "shaper")
            .header("Retry-After", "60")
            .body(body)
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Body::empty())
                    .unwrap_or_else(|_| Response::new(Body::empty()))
            });
        return ProxyResult {
            response,
            upstream_id: None,
            final_status: StatusCode::SERVICE_UNAVAILABLE,
            pricing_region: None,
            video_task_id: None,
            shaper_outcome: Some((
                "shaper_reject".to_string(),
                shaper_ctx.color.as_char().to_string(),
            )),
            routing_decision: sched_routing_decision.clone(),
            sched_request_color: shaper_color,
            error_type: Some("router_reject".to_string()),
                        request_log_data: None,
        };
    }

    ProxyResult {
        response: build_response_with_header(
            StatusCode::BAD_GATEWAY,
            "content-type",
            "application/json",
            Body::from(format!(
                r#"{{"error":{{"message":"All upstreams failed. Last error: {}","type":"upstream_error","code":"all_upstreams_failed"}}}}"#,
                last_error
            )),
        ),
        upstream_id: None,
        final_status: StatusCode::BAD_GATEWAY,
        pricing_region: None,
        video_task_id: None,
        // Mixed shaper-reject + upstream-failure: emit shaper context so
        // RouterLog still records the color/last-iter outcome if any.
        shaper_outcome: shaper_ctx
            .outcome
            .map(|lbl| (lbl.to_string(), shaper_ctx.color.as_char().to_string())),
        routing_decision: sched_routing_decision.clone(),
        sched_request_color: shaper_color,
        error_type: Some("upstream_error".to_string()),
                        request_log_data: None,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::unnecessary_cast,
    clippy::let_and_return,
    clippy::redundant_pattern_matching,
    clippy::identity_op
)]
mod tests {
    use super::inject_video_tokens_if_empty;
    use axum::http::StatusCode;
    use burncloud_service_billing::UnifiedUsage;

    #[test]
    fn test_veo_billing_extracts_duration() {
        // Veo: video_tokens = duration_secs * sample_count = 8 * 1 = 8
        let usage = inject_video_tokens_if_empty(StatusCode::OK, UnifiedUsage::default(), 8, "veo");
        assert_eq!(usage.video_tokens, 8);
    }

    #[test]
    fn test_veo_billing_extracts_duration_and_sample_count() {
        // Veo: video_tokens = 8 * 2 = 16
        let usage =
            inject_video_tokens_if_empty(StatusCode::OK, UnifiedUsage::default(), 16, "veo");
        assert_eq!(usage.video_tokens, 16);
    }

    #[test]
    fn test_veo_billing_defaults_to_8s_when_unspecified() {
        // Caller passes unwrap_or(8) default — verify 8 * 1 = 8
        let usage = inject_video_tokens_if_empty(StatusCode::OK, UnifiedUsage::default(), 8, "veo");
        assert_eq!(usage.video_tokens, 8);
    }

    #[test]
    fn test_veo_billing_no_tokens_on_failure() {
        let usage = inject_video_tokens_if_empty(
            StatusCode::BAD_REQUEST,
            UnifiedUsage::default(),
            8,
            "veo",
        );
        assert_eq!(usage.video_tokens, 0);
    }

    #[test]
    fn test_veo_billing_no_tokens_when_zero() {
        // Non-Veo model: caller computes 0 tokens, inject should skip
        let usage = inject_video_tokens_if_empty(StatusCode::OK, UnifiedUsage::default(), 0, "veo");
        assert_eq!(usage.video_tokens, 0);
    }

    #[test]
    fn test_veo_billing_no_inject_when_usage_not_empty() {
        let existing = UnifiedUsage {
            input_tokens: 100,
            ..Default::default()
        };
        let result = inject_video_tokens_if_empty(StatusCode::OK, existing.clone(), 8, "veo");
        assert_eq!(result, existing);
    }

    #[test]
    fn test_seedance_billing_720p() {
        // Seedance 720p 5s: duration=5, resolution_weight=2 → video_tokens=10
        let tokens = 5i64 * 2;
        let usage = inject_video_tokens_if_empty(
            StatusCode::OK,
            UnifiedUsage::default(),
            tokens,
            "seedance",
        );
        assert_eq!(usage.video_tokens, 10);
    }

    #[test]
    fn test_seedance_billing_480p() {
        // Seedance 480p 5s: duration=5, resolution_weight=1 → video_tokens=5
        let tokens = 5i64;
        let usage = inject_video_tokens_if_empty(
            StatusCode::OK,
            UnifiedUsage::default(),
            tokens,
            "seedance",
        );
        assert_eq!(usage.video_tokens, 5);
    }

    #[test]
    fn test_seedance_billing_no_tokens_on_failure() {
        let tokens = 5i64 * 2;
        let usage = inject_video_tokens_if_empty(
            StatusCode::BAD_REQUEST,
            UnifiedUsage::default(),
            tokens,
            "seedance",
        );
        assert_eq!(usage.video_tokens, 0);
    }

    #[test]
    fn test_video_price_derivation_from_per_second() {
        // Verify the price_sync formula: video_price = price_per_sec_nanos × 1_000_000 / 2
        // For $0.14/s at 720p: price_per_sec = 140_000_000 nanodollars
        // video_price = 140_000_000 × 1_000_000 / 2 = 70_000_000_000_000 nanodollars/MTok
        let price_per_sec_nanos: i64 = 140_000_000; // $0.14/s
        let video_price = (price_per_sec_nanos as i128 * 1_000_000 / 2) as i64;

        // 5s 720p: video_tokens = 10, cost should be $0.70
        let video_tokens_720p = 5i64 * 2;
        let cost_nanos = video_tokens_720p * video_price / 1_000_000;
        let expected_nanos = 700_000_000i64; // $0.70
        assert_eq!(
            cost_nanos, expected_nanos,
            "5s 720p @ $0.14/s should cost $0.70 = 700_000_000 nanodollars"
        );

        // 5s 480p: video_tokens = 5, cost should be $0.35 (same video_price, half cost)
        let video_tokens_480p = 5i64;
        let cost_nanos_480p = video_tokens_480p * video_price / 1_000_000;
        let expected_nanos_480p = 350_000_000i64; // $0.35
        assert_eq!(
            cost_nanos_480p, expected_nanos_480p,
            "5s 480p @ $0.07/s should cost $0.35 = 350_000_000 nanodollars"
        );

        // duration=-1 falls back to 5s default: same as 720p 5s
        let video_tokens_default = 5i64 * 2;
        let cost_nanos_default = video_tokens_default * video_price / 1_000_000;
        assert_eq!(cost_nanos_default, expected_nanos);
    }

    #[test]
    fn test_video_price_derivation_seedance_fast() {
        // doubao-seedance-2-0-fast-260128: $0.07/s at 720p
        let price_per_sec_nanos: i64 = 70_000_000; // $0.07/s
        let video_price = (price_per_sec_nanos as i128 * 1_000_000 / 2) as i64;

        // 5s 720p fast: video_tokens = 10, cost = $0.35
        let cost_nanos = 10i64 * video_price / 1_000_000;
        assert_eq!(
            cost_nanos, 350_000_000i64,
            "5s 720p fast @ $0.07/s should cost $0.35 = 350_000_000 nanodollars"
        );
    }
}

/// Handler for /internal/metrics endpoint - Prometheus metrics
#[allow(clippy::expect_used)]
async fn metrics_handler() -> Response {
    let metrics_output = crate::metrics::export();
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(Body::from(metrics_output))
        .expect("Failed to build metrics response")
}
pub mod smart_circuit_breaker;
pub mod channel_health_manager;
pub mod health_probe;

/// Check response quality using the new ResponseQuality system.
/// 
/// This replaces the simple empty response check with a more sophisticated
/// quality detection that considers:
/// - Empty responses (zero tokens)
/// - Partial responses
/// - Malformed responses
/// - Upstream errors
/// 
/// Returns the response quality and whether the response should be treated as a failure.
fn check_response_quality(
    state: &AppState,
    upstream: &Upstream,
    model_name: Option<&str>,
    session_id: &str,
    response_body: &str,
    status_code: axum::http::StatusCode,
    headers: &axum::http::HeaderMap,
) -> (crate::response_quality::ResponseQuality, bool) {
    use crate::response_quality::ResponseQuality;
    use crate::response_quality::ResponseQualityDetector;
    use crate::response_quality::UpstreamErrorType;
    use crate::response_quality::RateLimitScope as UpstreamRateLimitScope;
    
    let channel_id: i32 = upstream.id.parse().unwrap_or(0);
    let model = model_name.unwrap_or("unknown");
    let http_status: u16 = status_code.as_u16();
    
    tracing::info!("check_response_quality called for channel={}, model={}, status={}", channel_id, model, http_status);
    
    // Detect response quality using the detector
    let detector = ResponseQualityDetector::new();
    
    // Determine channel_type based on upstream protocol
    let channel_type = match upstream.protocol.as_str() {
        "claude" | "anthropic" => "anthropic",
        "gemini" | "vertex" => "gemini",
        _ => "openai",
    };
    
    let quality = detector.detect(http_status, headers, response_body, 0, false, channel_type);
    
    // Process response through health manager (records to circuit breaker)
    state.channel_health_manager.process_response(
        channel_id, model, http_status, headers, response_body, 0, false, channel_type,
    );
    
    let upstream_id_str = upstream.id.clone();
    
    match quality {
        ResponseQuality::Healthy { .. } => {
            // Reset empty response counter
            state.empty_response_counter.reset(&upstream_id_str);
            (quality, false)
        }
        ResponseQuality::Partial { .. } => {
            // Partial response - not a failure but degraded
            state.empty_response_counter.reset(&upstream_id_str);
            (quality, false)
        }
        ResponseQuality::Empty { .. } => {
            // Empty response - treat as failure
            tracing::warn!(
                channel_id = %upstream.id,
                model = ?model_name,
                "Empty response detected (new quality system)"
            );
            
            let should_penalize = state.empty_response_counter.record_empty(&upstream_id_str);
            if should_penalize {
                record_upstream_failure(
                    state,
                    upstream,
                    model_name,
                    FailureType::EmptyResponse,
                    "Empty response with zero tokens",
                    session_id,
                );
            }
            (quality, true)
        }
        ResponseQuality::Malformed { .. } => {
            // Malformed response - treat as failure
            tracing::warn!(
                channel_id = %upstream.id,
                model = ?model_name,
                "Malformed response detected"
            );
            
            record_upstream_failure(
                state,
                upstream,
                model_name,
                FailureType::ServerError,
                "Malformed response",
                session_id,
            );
            (quality, true)
        }
        ResponseQuality::UpstreamError { ref error_type, .. } => {
            // Upstream error - treat as failure
            let failure_type = match error_type {
                UpstreamErrorType::RateLimited { ref scope, ref retry_after } => {
                    FailureType::RateLimited {
                        scope: match scope {
                            UpstreamRateLimitScope::Account => crate::circuit_breaker::RateLimitScope::Account,
                            UpstreamRateLimitScope::Model => crate::circuit_breaker::RateLimitScope::Model,
                            UpstreamRateLimitScope::Unknown => crate::circuit_breaker::RateLimitScope::Unknown,
                        },
                        retry_after: retry_after.clone(),
                    }
                }
                UpstreamErrorType::AuthFailed => FailureType::AuthFailed,
                UpstreamErrorType::ModelNotFound => FailureType::ModelNotFound,
                UpstreamErrorType::PaymentRequired => FailureType::PaymentRequired,
                UpstreamErrorType::ServerError => FailureType::ServerError,
                UpstreamErrorType::GatewayError => FailureType::ConnectionError,
                UpstreamErrorType::Timeout => FailureType::Timeout,
                UpstreamErrorType::ConnectionError => FailureType::ConnectionError,
                UpstreamErrorType::Overloaded { .. } => FailureType::ServerError,
            };
            
            record_upstream_failure(
                state,
                upstream,
                model_name,
                failure_type,
                "Upstream error",
                session_id,
            );
            (quality, true)
        }
    }
}
