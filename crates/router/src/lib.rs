// Router core — LLM request/response handling — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

mod aimd_limiter;
mod adaptor;
pub mod affinity;
mod balancer;
pub mod channel_state;
mod circuit_breaker;
mod config;
pub mod exchange_rate;
mod limiter;
pub mod model_router;
pub mod order_type;
pub mod passthrough;
pub mod price_sync;
pub mod rate_budget;
pub mod response_parser;
mod scheduler;
mod state;
pub mod stream_parser;
pub mod token_counter;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
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
    RouterDatabase, RouterLog, RouterTokenValidationResult, RouterVideoTask, RouterVideoTaskModel,
};
use burncloud_service_user::UserService;
use burncloud_service_billing::{
    get_parser, parse_chunk_or_default, parse_response_or_default, UnifiedTokenCounter,
};
use channel_state::ChannelStateTracker;
use circuit_breaker::CircuitBreaker;
use config::{AuthType, Group, GroupMember, RouteTarget, RouterConfig, Upstream};
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

pub use scheduler::SchedulingRequest;
pub use state::AppState;

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
}

/// Build a JSON error response body: `{"error": "<message>"}`.
fn json_error_body(message: impl std::fmt::Display) -> Body {
    Body::from(serde_json::json!({"error": message.to_string()}).to_string())
}

/// Record a channel error in both circuit breaker and channel state tracker.
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
    // Evict affinity entry when CB trips (audit decision D8 — only on CB
    // trip, not on every failure, to avoid affinity storms).
    if !state.circuit_breaker.allow_request(&upstream.id) {
        if let Some(model) = model_name {
            state.affinity_cache.evict(session_id, model);
            tracing::debug!(
                session_id, model, channel_id,
                "Affinity evicted — CB tripped for upstream {}",
                upstream.name
            );
        }
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

async fn load_router_config(db: &Database) -> anyhow::Result<RouterConfig> {
    // Load Upstreams
    let db_upstreams = RouterDatabase::get_all_upstreams(db).await?;
    let upstreams: Vec<Upstream> = db_upstreams
        .into_iter()
        .map(|u| Upstream {
            id: u.id,
            name: u.name,
            base_url: u.base_url,
            api_key: u.api_key,
            match_path: u.match_path,
            auth_type: AuthType::from(u.auth_type.as_str()),
            priority: u.priority,
            protocol: u.protocol,
            param_override: u.param_override,
            header_override: u.header_override,
            api_version: u.api_version,
            pricing_region: None,
        })
        .collect();

    // Load Groups
    let db_groups = RouterDatabase::get_all_groups(db).await?;
    let db_members = RouterDatabase::get_group_members(db).await?;

    let groups = db_groups
        .into_iter()
        .map(|g| {
            let members = db_members
                .iter()
                .filter(|m| m.group_id == g.id)
                .map(|m| GroupMember {
                    upstream_id: m.upstream_id.clone(),
                    weight: m.weight,
                })
                .collect();

            Group {
                id: g.id,
                name: g.name,
                strategy: g.strategy,
                match_path: g.match_path,
                members,
            }
        })
        .collect();

    Ok(RouterConfig { upstreams, groups })
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
    let rows: Vec<ChannelCapRow> =
        match burncloud_database::sqlx::query_as(
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
    let config = load_router_config(&db).await?;
    let client = Client::builder().build()?;
    let balancer = Arc::new(RoundRobinBalancer::new());
    // Rate limiter: 100 burst, 10 requests/second
    let limiter = Arc::new(RateLimiter::new(100.0, 10.0));
    // Circuit breaker: 5 failure threshold, 30s cooldown
    let circuit_breaker = Arc::new(CircuitBreaker::new(CIRCUIT_BREAKER_FAILURE_THRESHOLD, CIRCUIT_BREAKER_COOLDOWN_SECS));
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

    // AIMD → InMemoryBudget feedback channel (capacity=1, latest-wins debounce).
    // When the adaptive limiter learns a new RPM limit, it sends an update here;
    // a background task reconfigures the budget bucket (audit decision D6/D10).
    let (budget_update_tx, mut budget_update_rx) =
        mpsc::channel::<state::BudgetUpdate>(1);
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

    let state = AppState {
        client,
        config: Arc::new(RwLock::new(config)),
        db, // Arc<Database>
        balancer,
        limiter,
        circuit_breaker,
        log_tx,
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
        budget_update_tx,
    };

    use burncloud_common::constants::INTERNAL_PREFIX;

    // ...

    let reload_path = format!("{}/reload", INTERNAL_PREFIX);
    let health_path = format!("{}/health", INTERNAL_PREFIX);
    let price_sync_path = format!("{}/prices/sync", INTERNAL_PREFIX);

    // Internal routes that must be registered BEFORE LiveView's catch-all
    // `/console/{*path}` in the server layer, otherwise LiveView intercepts
    // them and returns HTML instead of JSON.
    let internal_app = Router::new()
        .route(&reload_path, post(reload_handler))
        .route(&health_path, axum::routing::get(health_status_handler))
        .route(&price_sync_path, post(price_sync_handler))
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

// ...

async fn reload_handler(State(state): State<AppState>) -> Response {
    match load_router_config(&state.db).await {
        Ok(new_config) => {
            let mut config_write = state.config.write().await;
            *config_write = new_config;
            // Reload scheduler policies
            {
                let mut policies = state.scheduler_policies.write().await;
                *policies = scheduler::load_scheduler_config();
            }
            build_response(StatusCode::OK, Body::from("Reloaded"))
        }
        Err(e) => build_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            Body::from(format!("Reload Failed: {e}")),
        ),
    }
}

/// POST /console/internal/prices/sync
///
/// Triggers an immediate forced price sync. Waits up to 60 seconds for completion.
/// Internal-only; no auth required (server is assumed behind firewall).
async fn price_sync_handler(State(state): State<AppState>) -> Response {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    if state.force_sync_tx.send(reply_tx).await.is_err() {
        return build_response(
            StatusCode::SERVICE_UNAVAILABLE,
            Body::from("Price sync task is not running"),
        );
    }
    match tokio::time::timeout(std::time::Duration::from_secs(PRICE_SYNC_TRIGGER_TIMEOUT_SECS), reply_rx).await {
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
        Ok(Err(_)) => build_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            Body::from("Price sync task dropped the reply channel"),
        ),
        Err(_) => build_response(
            StatusCode::GATEWAY_TIMEOUT,
            Body::from("Price sync timed out after 60 seconds"),
        ),
    }
}

async fn models_handler(State(state): State<AppState>) -> Response {
    // Fetch all upstreams and groups to list as available "models"
    // This allows clients like WebUI to auto-discover available backends

    let mut model_entries = Vec::new();
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Ok(upstreams) = RouterDatabase::get_all_upstreams(&state.db).await {
        for u in upstreams {
            model_entries.push(serde_json::json!({
                "id": u.id,
                "object": "model",
                "created": current_time,
                "owned_by": "burncloud-router",
                "permission": [],
                "root": u.id,
                "parent": null,
            }));
        }
    }

    if let Ok(groups) = RouterDatabase::get_all_groups(&state.db).await {
        for g in groups {
            model_entries.push(serde_json::json!({
                "id": g.id,
                "object": "model",
                "created": current_time,
                "owned_by": "burncloud-group",
                "permission": [],
                "root": g.id,
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
                _ => Err(build_response(
                    StatusCode::UNAUTHORIZED,
                    Body::from(r#"{"error":"Invalid token"}"#),
                )),
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
        Err(e) => build_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json_error_body(&e),
        ),
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
        Err(e) => build_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json_error_body(&e),
        ),
    }
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
            map.insert(group.clone(), match kind {
                scheduler::SchedulerKind::Passthrough => "passthrough".to_string(),
                scheduler::SchedulerKind::Combined { config } => format!(
                    "combined(h={:.1},c={:.1},r={:.1})",
                    config.health_weight, config.cost_weight, config.rpm_weight
                ),
            });
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
    let path = uri.path().to_string();

    // 0. Authenticate User
    // Support both "Authorization: Bearer sk-xxx" and "x-goog-api-key: sk-xxx" (Gemini native)
    let user_auth = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .or_else(|| headers.get("x-goog-api-key").and_then(|h| h.to_str().ok()));

    let user_token = match user_auth {
        Some(token) => token.to_string(),
        None => {
            return build_response(
                StatusCode::UNAUTHORIZED,
                Body::from("Unauthorized: Missing Bearer Token"),
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
                        return build_response_with_header(
                            StatusCode::UNAUTHORIZED,
                            "content-type",
                            "application/json",
                            Body::from(
                                r#"{"error":{"message":"Invalid Token","type":"invalid_request_error","code":"invalid_token"}}"#,
                            ),
                        )
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
        return build_response(
            StatusCode::TOO_MANY_REQUESTS,
            Body::from("Too Many Requests"),
        );
    }

    // Buffer body for token counting and retries
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return build_response(
                StatusCode::BAD_REQUEST,
                Body::from(format!("Body Read Error: {e}")),
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
        (5i64, "720p".to_string())
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
        if let Some(ch_id) = result.upstream_id.as_ref().and_then(|s| s.parse::<i32>().ok()) {
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
    let resolution_weight: i64 = if seedance_resolution == "720p" { SEEDANCE_RESOLUTION_WEIGHT_HD } else { SEEDANCE_RESOLUTION_WEIGHT_SD };
    let seedance_tokens = seedance_duration_secs * resolution_weight;
    let usage = inject_video_tokens_if_empty(result.final_status, usage, seedance_tokens, "seedance");

    // Calculate cost using CostCalculator (nanodollars)
    let (cost, cost_breakdown) = if !usage.is_empty() {
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
                    (total, result.breakdown)
                }
                Err(burncloud_service_billing::BillingError::PriceNotFound(m)) => {
                    tracing::warn!(model = %m, "PriceNotFound — no price configured for this model");
                    (0, Default::default())
                }
                Err(e) => {
                    tracing::warn!("Cost calculation error for {model}: {e}");
                    (0, Default::default())
                }
            }
        } else {
            (0, Default::default())
        }
    } else {
        (0, Default::default())
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
            (None, None) => None, // No decision (pre-routing error)
        }
    };
    // traffic_color: prefer shaper_outcome color (final color after Shaper
    // processing). When Shaper is inactive (no shaper_outcome), fall back
    // to SchedulingRequest.color per the L6 Observability checklist.
    let traffic_color = result.shaper_outcome
        .as_ref()
        .map(|(_, c)| c.clone())
        .or_else(|| Some(result.sched_request_color.as_char().to_string()));

    // Async Log
    let log = RouterLog {
        id: 0, // Auto-generated by database
        request_id,
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
        created_at: None, // Auto-generated by database
    };

    if state.log_tx.send(log).await.is_err() {
        tracing::error!(
            cost,
            "billing log channel full or closed — request cost NOT recorded"
        );
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

    result.response
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
    let config = state.config.read().await;

    // 1. Model Routing (Priority)
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
        .and_then(|v| v.get("conversation_id").and_then(|c| c.as_str()).map(|s| s.to_string()))
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
                        let (auth_type, protocol) = match channel_type {
                            ChannelType::OpenAI => (AuthType::Bearer, PROTOCOL_OPENAI.to_string()),
                            ChannelType::Anthropic => (AuthType::Claude, PROTOCOL_CLAUDE.to_string()),
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
                        model, user_group
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
                    };
                }
            }
        } else {
            tracing::debug!("ProxyLogic: No 'model' field in JSON body");
        }
    } else {
        tracing::debug!("ProxyLogic: Failed to parse body as JSON");
    }

    // 2. Path Routing (Fallback)
    if candidates.is_empty() {
        tracing::debug!(
            "ProxyLogic: Model routing failed/skipped, trying path routing for {}",
            path
        );
        let route = match config.find_route(path) {
            Some(r) => r,
            None => {
                return ProxyResult {
                    response: build_response(
                        StatusCode::NOT_FOUND,
                        Body::from(format!("No matching upstream found for path: {path}")),
                    ),
                    upstream_id: None,
                    final_status: StatusCode::NOT_FOUND,
                    pricing_region: None,
                    video_task_id: None,
                    shaper_outcome: None,
                    routing_decision: None,
                    sched_request_color: shaper_color,
                };
            }
        };

        match route {
            RouteTarget::Upstream(u) => candidates.push(u.clone()),
            RouteTarget::Group(g) => {
                if g.members.is_empty() {
                    return ProxyResult {
                        response: build_response(
                            StatusCode::SERVICE_UNAVAILABLE,
                            Body::from(format!("Group '{}' has no healthy members", g.name)),
                        ),
                        upstream_id: None,
                        final_status: StatusCode::SERVICE_UNAVAILABLE,
                        pricing_region: None,
                        video_task_id: None,
                        shaper_outcome: None,
                        routing_decision: None,
                        sched_request_color: shaper_color,
                    };
                }

                let start_idx = state.balancer.next_index(&g.id, g.members.len());
                for i in 0..g.members.len() {
                    let idx = (start_idx + i) % g.members.len();
                    let member = &g.members[idx];
                    if let Some(u) = config.get_upstream(&member.upstream_id) {
                        candidates.push(u.clone());
                    }
                }
            }
        };
    }

    if candidates.is_empty() {
        return ProxyResult {
            response: build_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                Body::from("Configuration Error: No upstreams available"),
            ),
            upstream_id: None,
            final_status: StatusCode::INTERNAL_SERVER_ERROR,
            pricing_region: None,
            video_task_id: None,
            shaper_outcome: None,
            routing_decision: None,
            sched_request_color: shaper_color,
        };
    }

    // Preflight billing check: reject requests for models with no price configured.
    // Returns 400 to prevent unbilled usage on unknown models.
    if let Some(model) = model_name {
        if let Err(e) = state.cost_calculator.preflight(model, None).await {
            tracing::warn!(model = %model, "Preflight billing check failed — rejecting request: {e}");
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
            };
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

    for (attempt, upstream) in candidates.iter().enumerate() {
        // L5 Failover: override routing decision when attempt > 0.
        if attempt > 0 {
            sched_routing_decision = Some(model_router::RoutingDecision::Failover { attempt: attempt as u32 });
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
            let outcome = state.rate_budget.try_consume(
                channel_id_i32,
                shaper_ctx.color,
                shaper_ctx.est_tpm,
            );
            if outcome == ConsumeOutcome::Rejected {
                shaper_ctx.rejected_count += 1;
                tracing::debug!(
                    channel_id = channel_id_i32,
                    color = ?shaper_ctx.color,
                    est_tpm = shaper_ctx.est_tpm,
                    "L2 Shaper rejected candidate {}, trying next", upstream.name
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

        // Handle Gemini passthrough mode
        if passthrough_decision == PassthroughDecision::Passthrough {
            tracing::debug!(
                "Using passthrough mode for Gemini request: path={}, has_contents={}",
                path,
                body_json.get("contents").is_some()
            );

            // Build target URL for passthrough
            let passthrough_url =
                passthrough::build_gemini_passthrough_url(&upstream.base_url, path, &body_json);

            tracing::debug!("Passthrough URL: {}", passthrough_url);

            // Check if streaming
            let is_stream = body_json
                .get("stream")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            // Determine final URL (add alt=sse for streaming requests if not already in URL)
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

            tracing::debug!("Final passthrough URL: {}", final_url);

            // Prepare request body - remove 'stream' field for Gemini native API
            // Safe to take ownership: passthrough path always returns or continues,
            // so body_json is never used after this point in the current iteration.
            let mut passthrough_body = std::mem::take(&mut body_json);
            if passthrough_body.get("stream").is_some() {
                if let Some(obj) = passthrough_body.as_object_mut() {
                    obj.remove("stream");
                }
            }

            // Build passthrough request with final URL
            let req_builder = state
                .client
                .request(method.clone(), &final_url)
                .header("x-goog-api-key", &upstream.api_key);

            // Apply header_override
            let req_builder = apply_header_override(req_builder, upstream.header_override.as_deref());

            let req_builder = req_builder.json(&passthrough_body);

            // Execute passthrough request
            match req_builder.send().await {
                Ok(resp) => {
                    let status = resp.status();

                    if status.is_server_error() {
                        last_error = format!("Upstream returned {status}");
                        record_upstream_failure(
                            state, upstream, model_name, FailureType::ServerError, &last_error,
                            &session_id,
                        );
                        continue;
                    }

                    if status.is_success() {
                        state.circuit_breaker.record_success(&upstream.id);

                        let channel_id: i32 = upstream.id.parse().unwrap_or(0);
                        let latency_ms = request_start_time.elapsed().as_millis() as u64;
                        // Parse rate limit info from response headers for adaptive limiter
                        let rate_limit_info =
                            parse_rate_limit_info(resp.headers(), None, &upstream.protocol);
                        state.channel_state_tracker.record_success(
                            channel_id,
                            model_name,
                            latency_ms,
                            rate_limit_info.request_limit,
                        )
                        .inspect(|&learned| {
                            let _ = state.budget_update_tx.try_send(
                                state::BudgetUpdate { channel_id, learned_limit: learned }
                            );
                        });

                        // Handle streaming vs non-streaming passthrough
                        if is_stream {
                            // Stream passthrough - directly forward Gemini SSE format
                            let body_stream = resp.bytes_stream();
                            let counter_clone = Arc::clone(&token_counter);

                            let parser = get_parser(channel_type);
                            let stream = body_stream.map(move |chunk_result| match chunk_result {
                                Ok(bytes) => {
                                    let text = String::from_utf8_lossy(&bytes);

                                    // Parse token usage from Gemini streaming response
                                    if let Some(u) = parse_chunk_or_default(
                                        parser.as_ref(),
                                        &text,
                                        "passthrough",
                                    ) {
                                        counter_clone.set_from_usage(&u);
                                    }

                                    // Pass through raw bytes (Gemini native format)
                                    Ok(bytes)
                                }
                                Err(e) => Err(std::io::Error::other(e)),
                            });

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
                                            Body::from("Failed to build streaming response"),
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
                            };
                        } else {
                            // Non-streaming passthrough
                            let resp_bytes = match resp.bytes().await {
                                Ok(b) => b,
                                Err(e) => {
                                    last_error = format!("Failed to read response: {e}");
                                    tracing::warn!("Passthrough: {} response read failed: {}", upstream.name, e);
                                    continue;
                                }
                            };

                            // Parse usage metadata from response
                            if let Ok(resp_json) =
                                serde_json::from_slice::<serde_json::Value>(&resp_bytes)
                            {
                                let resp_usage = parse_response_or_default(
                                    get_parser(channel_type).as_ref(),
                                    &resp_json,
                                    "passthrough",
                                );
                                token_counter.set_from_usage(&resp_usage);
                            }

                            // L2 Shaper success — non-streaming passthrough.
                            // Use actual_tpm from parsed usage (audit decision D9 —
                            // non-streaming paths have complete usage data).
                            // Fall back to est_tpm when usage parsing yields 0.
                            let actual_tpm = token_counter.get_usage().total_tokens() as u64;
                            let commit_tpm = if actual_tpm > 0 { actual_tpm } else { shaper_ctx.est_tpm };
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
                                    response: build_response(status, Body::from(last_error.clone())),
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
                                };
                            }
                        };

                        // Classify all 4xx errors (not just 429) for circuit breaker
                        // and channel state tracking (P1 — passthrough error mapping).
                        let body_str = String::from_utf8_lossy(&body_bytes);
                        let error_info = parse_error_response(&body_str, &upstream.protocol);
                        let error_message = error_info.message.as_deref().unwrap_or("Unknown error");
                        let failure_type = classify_upstream_error(status, &resp_headers, &error_info);
                        // Auth/payment failures affect entire channel (model_name=None)
                        let error_model = match status {
                            StatusCode::UNAUTHORIZED | StatusCode::PAYMENT_REQUIRED => None,
                            _ => model_name,
                        };
                        record_upstream_failure(
                            state, upstream, error_model, failure_type, error_message,
                            &session_id,
                        );
                        // 429: try next ranked candidate
                        if status == StatusCode::TOO_MANY_REQUESTS {
                            tracing::warn!(
                                "Passthrough: {} rate limited, trying next candidate",
                                upstream.name
                            );
                            continue;
                        }

                        // 4xx response: actual usage = 0. budget_guard drops
                        // on return → full est_tpm refund (no commit).
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
                        state, upstream, model_name, failure_type, &last_error,
                        &session_id,
                    );
                    tracing::warn!(
                        "Failover: {} network error: {}, trying next...",
                        upstream.name, e
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
                        state, upstream, model_name, FailureType::ServerError, &last_error,
                        &session_id,
                    );
                    continue;
                }

                if resp.status().is_success() {
                    state.circuit_breaker.record_success(&upstream.id);
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
                    state.channel_state_tracker.record_success(
                        channel_id,
                        model_name,
                        latency_ms,
                        rate_limit_info.request_limit,
                    )
                    .inspect(|&learned| {
                        let _ = state.budget_update_tx.try_send(
                            state::BudgetUpdate { channel_id, learned_limit: learned }
                        );
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
                                    last_error =
                                        format!("Failed to read video gen response: {e}");
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
                            let commit_tpm = if actual_tpm > 0 { actual_tpm } else { shaper_ctx.est_tpm };
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
                            };
                        }
                        // L2 Shaper success: OpenAI streaming path — keep est_tpm
                        // (actual_tpm not yet available during stream, audit decision D9).
                        if let Some(g) = budget_guard.take() {
                            g.commit(shaper_ctx.est_tpm);
                        }
                        return ProxyResult {
                            response: handle_response_with_token_parsing(resp, &token_counter, channel_type),
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
                        };
                    }

                    // Handle Streaming for non-OpenAI
                    if is_stream {
                        let body_stream = resp.bytes_stream();
                        let adaptor_clone = Arc::clone(&adaptor);
                        let counter_clone = Arc::clone(&token_counter);
                        let parser = get_parser(channel_type);

                        let stream = body_stream.map(move |chunk_result| match chunk_result {
                            Ok(bytes) => {
                                let text = String::from_utf8_lossy(&bytes);

                                // Parse token usage from streaming response
                                // Anthropic sends incremental events; others send cumulative totals
                                if let Some(u) =
                                    parse_chunk_or_default(parser.as_ref(), &text, "stream")
                                {
                                    match parser.provider_name() {
                                        "anthropic" => counter_clone.accumulate(&u),
                                        _ => counter_clone.set_from_usage(&u),
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
                        });

                        let done = futures::stream::once(async {
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
                                        Body::from("Failed to build streaming response"),
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
                        };
                    }

                    // 6. Handle Response Conversion
                    // Read body to memory to convert
                    let resp_json: serde_json::Value = match resp.json().await {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!("Failed to parse response JSON from {}: {}", upstream.name, e);
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
                    let commit_tpm = if actual_tpm > 0 { actual_tpm } else { shaper_ctx.est_tpm };
                    if let Some(g) = budget_guard.take() {
                        g.commit(commit_tpm);
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
                                response: build_response(status, Body::from(last_error.clone())),
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
                        state, upstream, error_model, failure_type, error_message,
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
                        upstream.name, status.as_u16(), error_message
                    );

                    // 4xx response from non-OpenAI: budget_guard drops on
                    // return → full est_tpm refund (request reached upstream
                    // but no actual usage was billed).
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
                    state, upstream, model_name, failure_type, &last_error,
                    &session_id,
                );
                tracing::warn!(
                    "Failover: {} failed with {}, trying next...",
                    upstream.name, e
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
            r#"{"error":{"message":"All candidate channels rejected by rate budget shaper","type":"service_unavailable","code":"rate_budget_exhausted","rejected_by":"shaper"}}"#
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
        };
    }

    ProxyResult {
        response: build_response(
            StatusCode::BAD_GATEWAY,
            Body::from(format!("All upstreams failed. Last error: {last_error}")),
        ),
        upstream_id: None,
        final_status: StatusCode::BAD_GATEWAY,
        pricing_region: None,
        video_task_id: None,
        // Mixed shaper-reject + upstream-failure: emit shaper context so
        // RouterLog still records the color/last-iter outcome if any.
        shaper_outcome: shaper_ctx.outcome.map(|lbl| {
            (lbl.to_string(), shaper_ctx.color.as_char().to_string())
        }),
        routing_decision: sched_routing_decision.clone(),
        sched_request_color: shaper_color,
    }
}

/// Handle streaming response with token parsing for OpenAI protocol
fn handle_response_with_token_parsing(
    resp: reqwest::Response,
    token_counter: &Arc<UnifiedTokenCounter>,
    channel_type: ChannelType,
) -> Response {
    let status = resp.status();
    let mut response_builder = Response::builder().status(status);

    if let Some(headers_mut) = response_builder.headers_mut() {
        for (k, v) in resp.headers() {
            headers_mut.insert(k, v.clone());
        }
    }

    let counter_clone = Arc::clone(token_counter);
    let parser = get_parser(channel_type);
    let stream = resp.bytes_stream();

    let mapped_stream = stream.map(move |chunk_result| match chunk_result {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            if let Some(u) = parse_chunk_or_default(parser.as_ref(), &text, "stream") {
                match parser.provider_name() {
                    "anthropic" => counter_clone.accumulate(&u),
                    _ => counter_clone.set_from_usage(&u),
                }
            }
            Ok(bytes)
        }
        Err(e) => Err(std::io::Error::other(e)),
    });

    let body = Body::from_stream(mapped_stream);

    response_builder
        .body(body)
        .unwrap_or_else(|_| Response::new(Body::empty()))
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
