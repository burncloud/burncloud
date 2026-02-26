mod adaptive_limit;
mod adaptor;
mod balancer;
pub mod billing;
mod channel_state;
mod circuit_breaker;
mod config;
pub mod exchange_rate;
mod limiter;
mod model_router;
pub mod notification;
pub mod passthrough;
pub mod price_sync;
pub mod pricing_loader;
pub mod response_parser;
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
use burncloud_database::Database;
use burncloud_database_router::{DbRouterLog, RouterDatabase, TokenValidationResult};
use channel_state::ChannelStateTracker;
use circuit_breaker::CircuitBreaker;
use config::{AuthType, Group, GroupMember, RouteTarget, RouterConfig, Upstream};
use futures::stream::StreamExt;
use http_body_util::BodyExt;
use limiter::RateLimiter;
use model_router::ModelRouter;
use reqwest::Client;
use std::sync::Arc;
use std::time::Instant;
use stream_parser::StreamingTokenParser;
use token_counter::StreamingTokenCounter;
use tokio::sync::{mpsc, RwLock};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

pub use state::AppState;

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

pub async fn create_router_app(db: Arc<Database>) -> anyhow::Result<Router> {
    let config = load_router_config(&db).await?;
    let client = Client::builder().build()?;
    let balancer = Arc::new(RoundRobinBalancer::new());
    // Default Limit: 100 burst, 10 requests/second
    let limiter = Arc::new(RateLimiter::new(100.0, 10.0));
    // Circuit Breaker: 5 failures, 30s cooldown
    let circuit_breaker = Arc::new(CircuitBreaker::new(5, 30));
    let model_router = Arc::new(ModelRouter::new(db.clone()));
    // Channel State Tracker for health monitoring
    let channel_state_tracker = Arc::new(ChannelStateTracker::new());
    // Dynamic Adaptor Factory for protocol adaptation
    let adaptor_factory = Arc::new(adaptor::factory::DynamicAdaptorFactory::new(db.clone()));
    // API Version Detector for handling deprecated versions
    let api_version_detector = Arc::new(adaptor::detector::ApiVersionDetector::new(db.clone()));

    // Setup Async Logging Channel
    let (log_tx, mut log_rx) = mpsc::channel::<DbRouterLog>(1000);
    let db_for_logger = db.clone(); // Clone Arc

    // Spawn Logging Task
    tokio::spawn(async move {
        println!("Logging task started");
        while let Some(log) = log_rx.recv().await {
            // Need to create a new default database or use the shared one?
            // Since Database struct isn't thread-safe or Clone by default, we rely on Arc<Database>.
            // But RouterDatabase::insert_log takes &Database.
            if let Err(e) = RouterDatabase::insert_log(&db_for_logger, &log).await {
                eprintln!("Failed to insert log: {}", e);
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
    };

    use burncloud_common::constants::INTERNAL_PREFIX;

    // ...

    let reload_path = format!("{}/reload", INTERNAL_PREFIX);
    let health_path = format!("{}/health", INTERNAL_PREFIX);

    let app = Router::new()
        .route(&reload_path, post(reload_handler))
        .route(&health_path, axum::routing::get(health_status_handler))
        .route("/v1/models", axum::routing::get(models_handler))
        .fallback(proxy_handler)
        .layer(CorsLayer::permissive())
        .with_state(state);

    Ok(app)
}

// ...

async fn reload_handler(State(state): State<AppState>) -> Response {
    match load_router_config(&state.db).await {
        Ok(new_config) => {
            let mut config_write = state.config.write().await;
            *config_write = new_config;
            build_response(StatusCode::OK, Body::from("Reloaded"))
        }
        Err(e) => build_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            Body::from(format!("Reload Failed: {}", e)),
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

async fn health_status_handler(State(state): State<AppState>) -> Response {
    let circuit_breaker_status = state.circuit_breaker.get_status_map();
    let channel_states = state.channel_state_tracker.get_all_states();

    // Build comprehensive health report
    let health_report = serde_json::json!({
        "circuit_breaker": circuit_breaker_status,
        "channels": channel_states.iter().map(|(ch_id, ch_state)| {
            let models: Vec<_> = ch_state.models.iter().map(|m| {
                let model_state = m.value();
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
        }).collect::<std::collections::HashMap<_, _>>()
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

    println!("Proxy Handler: {} {}, Headers: {:?}", method, path, headers);

    // 0. Authenticate User
    let user_auth = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

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
    let (user_id, user_group, quota_limit, used_quota) =
        match RouterDatabase::validate_token_and_get_info(&state.db, &user_token).await {
            Ok(Some(info)) => {
                // Update accessed_time non-blocking
                let db = state.db.clone();
                let token = user_token.clone();
                tokio::spawn(async move {
                    let _ = RouterDatabase::update_token_accessed_time(&db, &token).await;
                });
                (info.0.to_string(), info.1, info.2, info.3)
            }
            Ok(None) => {
                // Fallback to old token table logic with detailed validation
                match RouterDatabase::validate_token_detailed(&state.db, &user_token).await {
                    Ok(TokenValidationResult::Valid(t)) => {
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
                        )
                    }
                    Ok(TokenValidationResult::Expired) => {
                        return build_response_with_header(
                            StatusCode::UNAUTHORIZED,
                            "content-type",
                            "application/json",
                            Body::from(
                                r#"{"error":{"message":"Token has expired","type":"invalid_request_error","code":"token_expired"}}"#,
                            ),
                        )
                    }
                    Ok(TokenValidationResult::Invalid) => {
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
                Body::from(format!("Body Read Error: {}", e)),
            )
        }
    };

    // Estimate Prompt Tokens (Simple approximation: 1 token ~= 4 bytes)
    // TODO(issue): Integrate tiktoken-rs for precise counting
    //   - Current approximation is inaccurate for non-ASCII text
    //   - tiktoken-rs provides accurate token counting for OpenAI models
    //   - Consider: model-specific tokenizers (cl100k_base, o200k_base, etc.)
    let estimated_prompt_tokens = (body_bytes.len() as f32 / 4.0).ceil() as i32;

    // Extract model name for pricing before proxy_logic consumes body_bytes
    let model_name = serde_json::from_slice::<serde_json::Value>(&body_bytes)
        .ok()
        .and_then(|v| {
            v.get("model")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
        });

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

    // Create token counter for streaming response parsing
    let token_counter = Arc::new(StreamingTokenCounter::with_prompt_tokens(
        estimated_prompt_tokens as u32,
    ));

    // Perform Proxy Logic
    let (response, upstream_id, final_status, pricing_region) = proxy_logic(
        &state,
        method,
        uri,
        headers,
        body_bytes,
        &path,
        &user_group,
        token_counter.clone(),
        model_name.as_deref(),
        start_time,
    )
    .await;

    // Get final token counts
    let (prompt_tokens, completion_tokens) = token_counter.get_usage();

    // Get cache token counts
    let (cache_read_tokens, cache_creation_tokens) = token_counter.get_cache_usage();

    // Calculate cost if we have token usage (returns i64 nanodollars)
    let cost: i64 = if prompt_tokens > 0 || completion_tokens > 0 {
        if let Some(model) = &model_name {
            // Try to get price and check for tiered pricing
            // Use channel's pricing_region for region-specific pricing
            let price_result = PriceModel::get(
                &state.db,
                model,
                "USD",
                pricing_region.as_deref(),
            )
            .await;
            let tiered_result =
                burncloud_database_models::TieredPriceModel::has_tiered_pricing(&state.db, model)
                    .await;

            // Check if model has tiered pricing
            let has_tiered = matches!(tiered_result, Ok(true));

            if let Ok(Some(price)) = price_result {
                // Build advanced pricing struct (prices are already in i64 nanodollars)
                let pricing = billing::AdvancedPricing {
                    input_price: price.input_price,
                    output_price: price.output_price,
                    cache_read_price: price.cache_read_input_price,
                    cache_creation_price: price.cache_creation_input_price,
                    batch_input_price: price.batch_input_price,
                    batch_output_price: price.batch_output_price,
                    priority_input_price: price.priority_input_price,
                    priority_output_price: price.priority_output_price,
                    audio_input_price: price.audio_input_price,
                };

                // Determine billing type with priority: cache > tiered > priority > batch > standard
                if cache_read_tokens > 0 || cache_creation_tokens > 0 {
                    // Use advanced pricing with cache cost calculation
                    let usage = billing::TokenUsage {
                        prompt_tokens: prompt_tokens as u64,
                        completion_tokens: completion_tokens as u64,
                        cache_read_tokens: cache_read_tokens as u64,
                        cache_creation_tokens: cache_creation_tokens as u64,
                        audio_tokens: 0,
                    };

                    billing::calculate_cache_cost_nano(&usage, &pricing)
                } else if has_tiered {
                    // Use tiered pricing for models with usage-based tiers (e.g., Qwen)
                    match burncloud_database_models::TieredPriceModel::get_tiers(
                        &state.db,
                        model,
                        pricing_region.as_deref(),
                    )
                    .await
                    {
                        Ok(tiers) if !tiers.is_empty() => {
                            // Convert database TieredPrice to billing TieredPrice
                            let billing_tiers: Vec<burncloud_common::types::TieredPrice> = tiers
                                .into_iter()
                                .map(|t| burncloud_common::types::TieredPrice {
                                    id: t.id,
                                    model: t.model,
                                    region: t.region,
                                    tier_start: t.tier_start,
                                    tier_end: t.tier_end,
                                    input_price: t.input_price,
                                    output_price: t.output_price,
                                })
                                .collect();

                            match billing::calculate_tiered_cost_full_nano(
                                prompt_tokens as u64,
                                completion_tokens as u64,
                                &billing_tiers,
                                None,
                            ) {
                                Ok(cost) => cost,
                                Err(_) => PriceModel::calculate_cost(
                                    &price,
                                    prompt_tokens as u64,
                                    completion_tokens as u64,
                                ),
                            }
                        }
                        _ => PriceModel::calculate_cost(
                            &price,
                            prompt_tokens as u64,
                            completion_tokens as u64,
                        ),
                    }
                } else if is_priority_request {
                    // Use priority pricing for high-priority requests
                    billing::calculate_priority_cost_nano(
                        prompt_tokens as u64,
                        completion_tokens as u64,
                        &pricing,
                    )
                } else if is_batch_request {
                    // Use batch pricing for batch API requests
                    billing::calculate_batch_cost_nano(
                        prompt_tokens as u64,
                        completion_tokens as u64,
                        &pricing,
                    )
                } else {
                    // Fall back to simple calculation
                    PriceModel::calculate_cost(
                        &price,
                        prompt_tokens as u64,
                        completion_tokens as u64,
                    )
                }
            } else {
                0
            }
        } else {
            0
        }
    } else {
        0
    };

    // Async Log
    let log = DbRouterLog {
        id: 0, // Auto-generated by database
        request_id,
        user_id: Some(user_id.clone()),
        path,
        upstream_id,
        status_code: final_status.as_u16() as i32,
        latency_ms: start_time.elapsed().as_millis() as i64,
        prompt_tokens: prompt_tokens as i32,
        completion_tokens: completion_tokens as i32,
        cost,
        created_at: None, // Auto-generated by database
    };

    let _ = state.log_tx.send(log).await;

    // Deduct quota (non-blocking)
    let total_tokens = prompt_tokens + completion_tokens;
    if total_tokens > 0 {
        let db = state.db.clone();
        let token_for_quota = user_token.to_string();
        let user_id_for_quota = user_id.clone();
        tokio::spawn(async move {
            // Deduct quota in nanodollars (i64)
            let quota_cost = cost; // cost is already in nanodollars
            let _ =
                RouterDatabase::deduct_quota(&db, &user_id_for_quota, &token_for_quota, quota_cost)
                    .await;
        });
    }

    response
}

use burncloud_common::types::ChannelType;
use burncloud_database_models::PriceModel;
use circuit_breaker::FailureType;
use passthrough::{should_passthrough, PassthroughDecision};
use response_parser::{parse_error_response, parse_rate_limit_info};

#[allow(clippy::too_many_arguments)]
async fn proxy_logic(
    state: &AppState,
    method: Method,
    uri: Uri,
    _headers: HeaderMap,
    body_bytes: axum::body::Bytes,
    path: &str,
    user_group: &str,
    token_counter: Arc<StreamingTokenCounter>,
    model_name: Option<&str>,
    request_start_time: Instant,
) -> (Response, Option<String>, StatusCode, Option<String>) {
    let config = state.config.read().await;

    // 1. Model Routing (Priority)
    let mut candidates: Vec<Upstream> = Vec::new();
    // Track pricing_region from selected channel for billing
    let mut selected_pricing_region: Option<String> = None;

    // Try to extract model from Gemini native path first
    let gemini_path_model = passthrough::extract_model_from_gemini_path(path);

    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
        // Prefer model from body, fall back to path-extracted model for Gemini native paths
        let model_ref = json.get("model").and_then(|v| v.as_str());
        let model_opt = model_ref.or(gemini_path_model.as_deref());

        if let Some(model) = model_opt {
            println!(
                "ProxyLogic: Attempting to route model '{}' for group '{}'",
                model, user_group
            );
            // Use state-aware routing to filter out unavailable channels
            match state
                .model_router
                .route_with_state(user_group, model, &state.channel_state_tracker)
                .await
            {
                Ok(Some(channel)) => {
                    println!(
                        "ModelRouter: Routed {} -> Channel {} (state-filtered)",
                        model, channel.name
                    );
                    // Map Channel Type
                    let channel_type = ChannelType::from(channel.type_);

                    // Map Channel Type to AuthType/Protocol (Still needed for legacy config struct compatibility if used elsewhere)
                    // But AdaptorFactory will handle logic based on ChannelType.
                    let (auth_type, protocol) = match channel_type {
                        ChannelType::OpenAI => (AuthType::Bearer, "openai".to_string()),
                        ChannelType::Anthropic => (AuthType::Claude, "claude".to_string()),
                        ChannelType::Gemini | ChannelType::VertexAi => {
                            (AuthType::GoogleAI, "gemini".to_string())
                        }
                        _ => (AuthType::Bearer, "openai".to_string()),
                    };

                    // Save pricing_region for billing
                    selected_pricing_region = channel.pricing_region.clone();

                    candidates.push(Upstream {
                        id: channel.id.to_string(),
                        name: channel.name,
                        base_url: channel.base_url.unwrap_or_default(),
                        api_key: channel.key,
                        match_path: "".to_string(),
                        auth_type,
                        priority: channel.priority as i32,
                        protocol, // Ideally we should store ChannelType in Upstream too
                        param_override: channel.param_override.clone(),
                        header_override: channel.header_override.clone(),
                        api_version: channel.api_version.clone(),
                    });
                }
                Ok(None) => {
                    println!(
                        "ModelRouter: No route found for {} (Group: {})",
                        model, user_group
                    );
                }
                Err(e) => {
                    // NoAvailableChannelsError - all channels are unavailable
                    println!("ModelRouter: No available channels for {}: {}", model, e);
                    return (
                        build_response_with_header(
                            StatusCode::SERVICE_UNAVAILABLE,
                            "content-type",
                            "application/json",
                            Body::from(format!(
                                r#"{{"error":{{"message":"{}","type":"service_unavailable","code":"no_available_channels"}}}}"#,
                                e
                            )),
                        ),
                        None,
                        StatusCode::SERVICE_UNAVAILABLE,
                        None,
                    );
                }
            }
        } else {
            println!("ProxyLogic: No 'model' field in JSON body");
        }
    } else {
        println!("ProxyLogic: Failed to parse body as JSON");
    }

    // 2. Path Routing (Fallback)
    if candidates.is_empty() {
        println!(
            "ProxyLogic: Model routing failed/skipped, trying path routing for {}",
            path
        );
        let route = match config.find_route(path) {
            Some(r) => r,
            None => {
                return (
                    build_response(
                        StatusCode::NOT_FOUND,
                        Body::from(format!("No matching upstream found for path: {}", path)),
                    ),
                    None,
                    StatusCode::NOT_FOUND,
                    None,
                );
            }
        };

        match route {
            RouteTarget::Upstream(u) => candidates.push(u.clone()),
            RouteTarget::Group(g) => {
                if g.members.is_empty() {
                    return (
                        build_response(
                            StatusCode::SERVICE_UNAVAILABLE,
                            Body::from(format!("Group '{}' has no healthy members", g.name)),
                        ),
                        None,
                        StatusCode::SERVICE_UNAVAILABLE,
                        None,
                    );
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
        return (
            build_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                Body::from("Configuration Error: No upstreams available"),
            ),
            None,
            StatusCode::INTERNAL_SERVER_ERROR,
            None,
        );
    }

    let mut last_error = String::new();
    #[allow(unused_assignments)]
    let mut last_upstream_id = None;

    for (attempt, upstream) in candidates.iter().enumerate() {
        last_upstream_id = Some(upstream.id.clone());

        // Circuit Breaker Check
        if !state.circuit_breaker.allow_request(&upstream.id) {
            println!("Skipping upstream {} (Circuit Open)", upstream.name);
            last_error = "Circuit Breaker Open".to_string();
            continue;
        }

        // 2. Construct Target URL
        // Note: Some adaptors might override URL, but we set base here.
        let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
        let target_url = format!("{}{}{}", upstream.base_url, path, query);

        println!(
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
            _ => ChannelType::OpenAI,
        };

        // 3. Parse Request Body early for passthrough detection
        let body_json: serde_json::Value = match serde_json::from_slice(&body_bytes) {
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
            println!(
                "Using passthrough mode for Gemini request: path={}, has_contents={}",
                path,
                body_json.get("contents").is_some()
            );

            // Build target URL for passthrough
            let passthrough_url =
                passthrough::build_gemini_passthrough_url(&upstream.base_url, path, &body_json);

            println!("Passthrough URL: {}", passthrough_url);

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
                format!("{}{}alt=sse", passthrough_url, separator)
            } else {
                passthrough_url.clone()
            };

            println!("Final passthrough URL: {}", final_url);

            // Prepare request body - remove 'stream' field for Gemini native API
            let mut passthrough_body = body_json.clone();
            if passthrough_body.get("stream").is_some() {
                if let Some(obj) = passthrough_body.as_object_mut() {
                    obj.remove("stream");
                }
            }

            // Build passthrough request with final URL
            let mut req_builder = state
                .client
                .request(method.clone(), &final_url)
                .header("x-goog-api-key", &upstream.api_key);

            // Apply header_override
            if let Some(ref override_str) = upstream.header_override {
                if let Ok(header_map) =
                    serde_json::from_str::<std::collections::HashMap<String, String>>(override_str)
                {
                    for (k, v) in header_map {
                        req_builder = req_builder.header(k, v);
                    }
                }
            }

            let req_builder = req_builder.json(&passthrough_body);

            // Execute passthrough request
            match req_builder.send().await {
                Ok(resp) => {
                    let status = resp.status();

                    if status.is_server_error() {
                        last_error = format!("Upstream returned {}", status);
                        let channel_id: i32 = upstream.id.parse().unwrap_or(0);
                        state
                            .circuit_breaker
                            .record_failure_with_type(&upstream.id, FailureType::ServerError);
                        state.channel_state_tracker.record_error(
                            channel_id,
                            model_name,
                            &FailureType::ServerError,
                            &last_error,
                        );
                        continue;
                    }

                    state.circuit_breaker.record_success(&upstream.id);

                    if status.is_success() {
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
                        );

                        // Handle streaming vs non-streaming passthrough
                        if is_stream {
                            // Stream passthrough - directly forward Gemini SSE format
                            let body_stream = resp.bytes_stream();
                            let counter_clone = token_counter.clone();

                            let stream = body_stream.map(move |chunk_result| match chunk_result {
                                Ok(bytes) => {
                                    let text = String::from_utf8_lossy(&bytes);

                                    // Parse token usage from Gemini streaming response
                                    let (prompt, completion) =
                                        passthrough::parse_gemini_streaming_usage(&text);
                                    if prompt > 0 || completion > 0 {
                                        counter_clone.add_tokens(prompt, completion);
                                    }

                                    // Pass through raw bytes (Gemini native format)
                                    Ok(bytes)
                                }
                                Err(e) => Err(std::io::Error::other(e)),
                            });

                            return (
                                Response::builder()
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
                                last_upstream_id,
                                status,
                                selected_pricing_region.clone(),
                            );
                        } else {
                            // Non-streaming passthrough
                            let resp_bytes = match resp.bytes().await {
                                Ok(b) => b,
                                Err(e) => {
                                    last_error = format!("Failed to read response: {}", e);
                                    continue;
                                }
                            };

                            // Parse usage metadata from response
                            if let Ok(resp_json) =
                                serde_json::from_slice::<serde_json::Value>(&resp_bytes)
                            {
                                let (prompt, completion) =
                                    passthrough::parse_gemini_usage(&resp_json);
                                if prompt > 0 || completion > 0 {
                                    token_counter.add_tokens(prompt, completion);
                                }
                            }

                            return (
                                build_response_with_header(
                                    status,
                                    "content-type",
                                    "application/json",
                                    Body::from(resp_bytes),
                                ),
                                last_upstream_id,
                                status,
                                selected_pricing_region.clone(),
                            );
                        }
                    } else {
                        // Non-success status (4xx)
                        let body_bytes = match resp.bytes().await {
                            Ok(b) => b,
                            Err(e) => {
                                last_error = format!("Failed to read error response: {}", e);
                                return (
                                    build_response(status, Body::from(last_error.clone())),
                                    last_upstream_id,
                                    status,
                                    selected_pricing_region.clone(),
                                );
                            }
                        };

                        // Record rate limit errors
                        if status.as_u16() == 429 {
                            let channel_id: i32 = upstream.id.parse().unwrap_or(0);
                            state.channel_state_tracker.record_error(
                                channel_id,
                                model_name,
                                &FailureType::RateLimited {
                                    scope: circuit_breaker::RateLimitScope::Unknown,
                                    retry_after: None,
                                },
                                "Rate limited",
                            );
                            state.circuit_breaker.record_failure_with_type(
                                &upstream.id,
                                FailureType::RateLimited {
                                    scope: circuit_breaker::RateLimitScope::Unknown,
                                    retry_after: None,
                                },
                            );
                        }

                        return (
                            build_response_with_header(
                                status,
                                "content-type",
                                "application/json",
                                Body::from(body_bytes),
                            ),
                            last_upstream_id,
                            status,
                            selected_pricing_region.clone(),
                        );
                    }
                }
                Err(e) => {
                    last_error = format!("Network Error: {}", e);
                    let channel_id: i32 = upstream.id.parse().unwrap_or(0);
                    state
                        .circuit_breaker
                        .record_failure_with_type(&upstream.id, FailureType::Timeout);
                    state.channel_state_tracker.record_error(
                        channel_id,
                        model_name,
                        &FailureType::Timeout,
                        &last_error,
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
        let request_body_json: Option<serde_json::Value> =
            if let Ok(req) = serde_json::from_slice::<OpenAIChatRequest>(&body_bytes) {
                adaptor
                    .convert_request(&req)
                    .or_else(|| Some(serde_json::json!(req))) // Use converted or original
            } else {
                // Use the already parsed JSON
                Some(body_json)
            };

        if request_body_json.is_none() {
            last_error = "Failed to prepare request body".to_string();
            continue;
        }
        // SAFETY: We just checked that request_body_json is Some
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
                    println!("Applied param_override for {}", upstream.name);
                }
            }
        }

        // 4. Build Request via Adaptor
        let mut req_builder = state.client.request(method.clone(), &target_url);

        // Apply header_override
        if let Some(ref override_str) = upstream.header_override {
            if let Ok(header_map) =
                serde_json::from_str::<std::collections::HashMap<String, String>>(override_str)
            {
                for (k, v) in header_map {
                    req_builder = req_builder.header(k, v);
                }
                println!("Applied header_override for {}", upstream.name);
            }
        }

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
                    last_error = format!("Upstream returned {}", status);
                    let channel_id: i32 = upstream.id.parse().unwrap_or(0);
                    state
                        .circuit_breaker
                        .record_failure_with_type(&upstream.id, FailureType::ServerError);
                    state.channel_state_tracker.record_error(
                        channel_id,
                        model_name,
                        &FailureType::ServerError,
                        &last_error,
                    );
                    continue;
                }

                state.circuit_breaker.record_success(&upstream.id);

                if resp.status().is_success() {
                    let status = resp.status();
                    let resp_headers = resp.headers().clone();

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
                    );

                    // Log rate limit info for debugging/monitoring
                    if rate_limit_info.request_limit.is_some()
                        || rate_limit_info.token_limit.is_some()
                    {
                        println!(
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
                        return (
                            handle_response_with_token_parsing(resp, &token_counter, "openai"),
                            last_upstream_id,
                            status,
                            selected_pricing_region.clone(),
                        );
                    }

                    // Handle Streaming for non-OpenAI
                    if is_stream {
                        let body_stream = resp.bytes_stream();
                        let adaptor_clone = adaptor.clone();
                        let counter_clone = token_counter.clone();
                        let protocol = upstream.protocol.clone();

                        let stream = body_stream.map(move |chunk_result| match chunk_result {
                            Ok(bytes) => {
                                let text = String::from_utf8_lossy(&bytes);

                                // Parse token usage from streaming response
                                match protocol.as_str() {
                                    "claude" => {
                                        StreamingTokenParser::parse_anthropic_chunk(
                                            &text,
                                            &counter_clone,
                                        );
                                    }
                                    "gemini" | "vertex" => {
                                        StreamingTokenParser::parse_gemini_chunk(
                                            &text,
                                            &counter_clone,
                                        );
                                    }
                                    _ => {
                                        StreamingTokenParser::parse_openai_chunk(
                                            &text,
                                            &counter_clone,
                                        );
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
                            Ok(axum::body::Bytes::from("data: [DONE]\n\n"))
                        });
                        let final_stream = stream.chain(done);

                        return (
                            Response::builder()
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
                            last_upstream_id,
                            status,
                            selected_pricing_region.clone(),
                        );
                    }

                    // 6. Handle Response Conversion
                    // Read body to memory to convert
                    let resp_json: serde_json::Value =
                        resp.json().await.unwrap_or(serde_json::json!({}));

                    let response_body = if let Some(converted) =
                        adaptor.convert_response(resp_json.clone(), &upstream.name)
                    {
                        serde_json::to_string(&converted).unwrap_or_else(|_| "{}".to_string())
                    } else {
                        // No conversion needed (e.g. OpenAI), return original body
                        serde_json::to_string(&resp_json).unwrap_or_else(|_| "{}".to_string())
                    };

                    return (
                        build_response_with_header(
                            status,
                            "content-type",
                            "application/json",
                            Body::from(response_body),
                        ),
                        last_upstream_id,
                        status,
                        selected_pricing_region.clone(),
                    );
                } else {
                    // Handle non-success responses (4xx errors)
                    let status_code = status.as_u16();
                    let body_bytes = match resp.bytes().await {
                        Ok(b) => b,
                        Err(e) => {
                            // If we can't read the body, return a simple error
                            last_error = format!("Failed to read response body: {}", e);
                            return (
                                build_response(status, Body::from(last_error.clone())),
                                last_upstream_id,
                                status,
                                selected_pricing_region.clone(),
                            );
                        }
                    };
                    let body_str = String::from_utf8_lossy(&body_bytes);

                    // Parse error response
                    let error_info = parse_error_response(&body_str, &upstream.protocol);
                    let error_message = error_info.message.as_deref().unwrap_or("Unknown error");

                    let channel_id: i32 = upstream.id.parse().unwrap_or(0);

                    // Determine failure type based on status code
                    let failure_type = match status_code {
                        401 => {
                            // Authentication failed - channel-level issue
                            let ft = FailureType::AuthFailed;
                            state.channel_state_tracker.record_error(
                                channel_id,
                                None, // Auth failure affects entire channel
                                &ft,
                                error_message,
                            );
                            ft
                        }
                        402 => {
                            // Payment required - balance exhausted
                            let ft = FailureType::PaymentRequired;
                            state.channel_state_tracker.record_error(
                                channel_id,
                                None,
                                &ft,
                                error_message,
                            );
                            ft
                        }
                        429 => {
                            // Rate limited - extract retry_after from headers or error info
                            let retry_after = resp_headers
                                .get("retry-after")
                                .and_then(|v| v.to_str().ok())
                                .and_then(|v| v.parse::<u64>().ok());

                            // Determine scope from error response
                            let scope = error_info
                                .scope
                                .unwrap_or(crate::circuit_breaker::RateLimitScope::Unknown);

                            let ft = FailureType::RateLimited { scope, retry_after };
                            state.channel_state_tracker.record_error(
                                channel_id,
                                model_name,
                                &ft,
                                error_message,
                            );
                            ft
                        }
                        404 => {
                            // Model not found
                            let ft = FailureType::ModelNotFound;
                            state.channel_state_tracker.record_error(
                                channel_id,
                                model_name,
                                &ft,
                                error_message,
                            );
                            ft
                        }
                        _ => {
                            // Other client errors - treat as server error for retry logic
                            FailureType::ServerError
                        }
                    };

                    // Record failure with circuit breaker
                    state
                        .circuit_breaker
                        .record_failure_with_type(&upstream.id, failure_type);

                    // Check for API version deprecation and auto-update if detected
                    if adaptor::detector::ApiVersionDetector::is_deprecation_error(error_message) {
                        let channel_id_for_detector = channel_id;
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
                                    println!(
                                        "API version deprecation detected, updated channel {} to version: {}",
                                        channel_id_for_detector, new_version
                                    );
                                }
                                Ok(None) => {
                                    // No deprecation detected or no new version found
                                }
                                Err(e) => {
                                    eprintln!("Failed to detect/update API version: {}", e);
                                }
                            }
                        });
                    }

                    // Log the error
                    println!(
                        "Upstream {} returned {}: {}",
                        upstream.name, status_code, error_message
                    );

                    return (
                        build_response_with_header(
                            status,
                            "content-type",
                            "application/json",
                            Body::from(body_bytes),
                        ),
                        last_upstream_id,
                        status,
                        selected_pricing_region.clone(),
                    );
                }
            }
            Err(e) => {
                last_error = format!("Network Error: {}", e);
                let channel_id: i32 = upstream.id.parse().unwrap_or(0);
                state
                    .circuit_breaker
                    .record_failure_with_type(&upstream.id, FailureType::Timeout);
                state.channel_state_tracker.record_error(
                    channel_id,
                    model_name,
                    &FailureType::Timeout,
                    &last_error,
                );
                eprintln!(
                    "Failover: {} failed with {}, trying next...",
                    upstream.name, e
                );
                continue;
            }
        }
    }

    (
        build_response(
            StatusCode::BAD_GATEWAY,
            Body::from(format!("All upstreams failed. Last error: {}", last_error)),
        ),
        None,
        StatusCode::BAD_GATEWAY,
        None,
    )
}

/// Handle streaming response with token parsing for OpenAI protocol
fn handle_response_with_token_parsing(
    resp: reqwest::Response,
    token_counter: &Arc<StreamingTokenCounter>,
    protocol: &str,
) -> Response {
    let status = resp.status();
    let mut response_builder = Response::builder().status(status);

    if let Some(headers_mut) = response_builder.headers_mut() {
        for (k, v) in resp.headers() {
            headers_mut.insert(k, v.clone());
        }
    }

    let counter_clone = Arc::clone(token_counter);
    let protocol = protocol.to_string();
    let stream = resp.bytes_stream();

    let mapped_stream = stream.map(move |chunk_result| match chunk_result {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);

            // Parse token usage from streaming response
            match protocol.as_str() {
                "claude" => {
                    StreamingTokenParser::parse_anthropic_chunk(&text, &counter_clone);
                }
                "gemini" | "vertex" => {
                    StreamingTokenParser::parse_gemini_chunk(&text, &counter_clone);
                }
                _ => {
                    StreamingTokenParser::parse_openai_chunk(&text, &counter_clone);
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
