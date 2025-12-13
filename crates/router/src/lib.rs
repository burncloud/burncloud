mod adaptor;
mod balancer;
mod circuit_breaker;
mod config;
mod limiter;
mod model_router;

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
use burncloud_database_router::{DbRouterLog, RouterDatabase};
use circuit_breaker::CircuitBreaker;
use config::{AuthType, Group, GroupMember, RouteTarget, RouterConfig, Upstream};
use http_body_util::BodyExt;
use limiter::RateLimiter;
use model_router::ModelRouter;
use reqwest::Client;
use std::{sync::Arc, time::Instant};
use tokio::sync::{mpsc, RwLock};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    client: Client,
    config: Arc<RwLock<RouterConfig>>,
    db: Arc<Database>,
    balancer: Arc<RoundRobinBalancer>,
    limiter: Arc<RateLimiter>,
    circuit_breaker: Arc<CircuitBreaker>,
    log_tx: mpsc::Sender<DbRouterLog>,
    model_router: Arc<ModelRouter>,
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
            param_override: None,
            header_override: None,
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
            Response::builder()
                .status(200)
                .body(Body::from("Reloaded"))
                .unwrap()
        }
        Err(e) => Response::builder()
            .status(500)
            .body(Body::from(format!("Reload Failed: {}", e)))
            .unwrap(),
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

    Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&response_json).unwrap()))
        .unwrap()
}

async fn health_status_handler(State(state): State<AppState>) -> Response {
    let status_map = state.circuit_breaker.get_status_map();
    let json = serde_json::to_string(&status_map).unwrap_or_else(|_| "{}".to_string());

    Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(Body::from(json))
        .unwrap()
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

    // DEBUG: Immediate return
    // return Response::builder().status(200).body(Body::from("DEBUG_OK")).unwrap();
    let path = uri.path().to_string();

    println!("Proxy Handler: {} {}, Headers: {:?}", method, path, headers);

    // 0. Authenticate User
    let user_auth = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    let user_token = match user_auth {
        Some(token) => token,
        None => {
            return Response::builder()
                .status(401)
                .body(Body::from("Unauthorized: Missing Bearer Token"))
                .unwrap();
        }
    };

    // Check against DB
    let (user_id, user_group, quota_limit, used_quota) =
        match RouterDatabase::validate_token_and_get_info(&state.db, user_token).await {
            Ok(Some(info)) => (info.0.to_string(), info.1, info.2, info.3),
            Ok(None) => {
                // Fallback to old token table logic
                match RouterDatabase::validate_token(&state.db, user_token).await {
                    Ok(Some(t)) => (
                        t.user_id,
                        "default".to_string(),
                        t.quota_limit,
                        t.used_quota,
                    ),
                    _ => {
                        return Response::builder()
                            .status(401)
                            .body(Body::from("Unauthorized: Invalid Token"))
                            .unwrap()
                    }
                }
            }
            Err(e) => {
                return Response::builder()
                    .status(500)
                    .body(Body::from(format!("Internal Auth Error: {}", e)))
                    .unwrap()
            }
        };

    if quota_limit >= 0 && used_quota >= quota_limit {
        return Response::builder()
            .status(429)
            .body(Body::from("Quota Exceeded"))
            .unwrap();
    }

    // Rate Limiting Check
    if !state.limiter.check(&user_id, 1.0) {
        return Response::builder()
            .status(429)
            .body(Body::from("Too Many Requests"))
            .unwrap();
    }

    // Buffer body for token counting and retries
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return Response::builder()
                .status(400)
                .body(Body::from(format!("Body Read Error: {}", e)))
                .unwrap()
        }
    };

    // Estimate Prompt Tokens (Simple approximation: 1 token ~= 4 bytes)
    // TODO: Integrate tiktoken-rs for precise counting
    let prompt_tokens = (body_bytes.len() as f32 / 4.0).ceil() as i32;

    // Perform Proxy Logic
    let (response, upstream_id, final_status) =
        proxy_logic(&state, method, uri, headers, body_bytes, &path, &user_group).await;

    // Estimate Completion Tokens (If header present, else 0 for streaming)
    // For streaming, we can't easily know without wrapping the stream.
    let completion_tokens = 0;

    // Async Log
    let log = DbRouterLog {
        request_id,
        user_id: Some(user_id),
        path,
        upstream_id,
        status_code: final_status.as_u16() as i32,
        latency_ms: start_time.elapsed().as_millis() as i64,
        prompt_tokens,
        completion_tokens,
    };

    let _ = state.log_tx.send(log).await;

    response
}

use adaptor::factory::AdaptorFactory;
use burncloud_common::types::ChannelType;

async fn proxy_logic(
    state: &AppState,
    method: Method,
    uri: Uri,
    _headers: HeaderMap,
    body_bytes: axum::body::Bytes,
    path: &str,
    user_group: &str,
) -> (Response, Option<String>, StatusCode) {
    let config = state.config.read().await;

    // 1. Model Routing (Priority)
    let mut candidates: Vec<Upstream> = Vec::new();

    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
        if let Some(model) = json.get("model").and_then(|v| v.as_str()) {
            println!(
                "ProxyLogic: Attempting to route model '{}' for group '{}'",
                model, user_group
            );
            match state.model_router.route(user_group, model).await {
                Ok(Some(channel)) => {
                    println!("ModelRouter: Routed {} -> Channel {}", model, channel.name);
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
                    });
                }
                Ok(None) => {
                    println!(
                        "ModelRouter: No route found for {} (Group: {})",
                        model, user_group
                    );
                }
                Err(e) => {
                    println!("ModelRouter: Error querying DB: {}", e);
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
                    Response::builder()
                        .status(404)
                        .body(Body::from(format!(
                            "No matching upstream found for path: {}",
                            path
                        )))
                        .unwrap(),
                    None,
                    StatusCode::NOT_FOUND,
                );
            }
        };

        match route {
            RouteTarget::Upstream(u) => candidates.push(u.clone()),
            RouteTarget::Group(g) => {
                if g.members.is_empty() {
                    return (
                        Response::builder()
                            .status(503)
                            .body(Body::from(format!(
                                "Group '{}' has no healthy members",
                                g.name
                            )))
                            .unwrap(),
                        None,
                        StatusCode::SERVICE_UNAVAILABLE,
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
            Response::builder()
                .status(500)
                .body(Body::from("Configuration Error: No upstreams available"))
                .unwrap(),
            None,
            StatusCode::INTERNAL_SERVER_ERROR,
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

        // Determine Adaptor
        // Currently Upstream struct stores protocol string. We should map it to ChannelType.
        // Simple heuristic map for now.
        let channel_type = match upstream.protocol.as_str() {
            "claude" => ChannelType::Anthropic,
            "gemini" => ChannelType::Gemini,
            _ => ChannelType::OpenAI,
        };

        let adaptor = AdaptorFactory::get_adaptor(channel_type);

        // 3. Prepare Request Body
        let request_body_json: Option<serde_json::Value> =
            if let Ok(req) = serde_json::from_slice::<OpenAIChatRequest>(&body_bytes) {
                adaptor
                    .convert_request(&req)
                    .or_else(|| Some(serde_json::json!(req))) // Use converted or original
            } else {
                // Failed to parse as OpenAI request, use raw bytes if possible?
                // Adaptor interface expects Value for build_request.
                // If body is not valid JSON, we might fail here for complex adaptors.
                // For OpenAI passthrough, we might want raw bytes.
                // Let's assume it's JSON for now as most LLM APIs are.
                serde_json::from_slice(&body_bytes).ok()
            };

        if request_body_json.is_none() {
            // If we can't parse body and we need to (implied by using adaptors), fail?
            // Or just pass empty?
            // If protocol is "openai", maybe we don't need to parse?
            // But `build_request` takes Value.
            // We should probably update `build_request` to take Option<Value> or Bytes?
            // For now, fail if not JSON.
            last_error = "Invalid JSON body".to_string();
            continue;
        }
        let mut request_body_json = request_body_json.unwrap();

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

        let req_builder = adaptor.build_request(req_builder, &upstream.api_key, &request_body_json);

        // 5. Execute
        match req_builder.send().await {
            Ok(resp) => {
                if resp.status().is_server_error() {
                    last_error = format!("Upstream returned {}", resp.status());
                    state.circuit_breaker.record_failure(&upstream.id);
                    continue;
                }

                state.circuit_breaker.record_success(&upstream.id);

                if resp.status().is_success() {
                    let status = resp.status();

                    // Optimization: If protocol is OpenAI, we can stream directly without parsing/buffering
                    // This satisfies the "Passthrough Principle" and enables streaming.
                    if upstream.protocol == "openai" {
                        return (handle_response(resp), last_upstream_id, status);
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
                        Response::builder()
                            .status(status)
                            .header("content-type", "application/json")
                            .body(Body::from(response_body))
                            .unwrap(),
                        last_upstream_id,
                        status,
                    );
                } else {
                    let status = resp.status();
                    return (handle_response(resp), last_upstream_id, status);
                }
            }
            Err(e) => {
                last_error = format!("Network Error: {}", e);
                state.circuit_breaker.record_failure(&upstream.id);
                eprintln!(
                    "Failover: {} failed with {}, trying next...",
                    upstream.name, e
                );
                continue;
            }
        }
    }

    (
        Response::builder()
            .status(502)
            .body(Body::from(format!(
                "All upstreams failed. Last error: {}",
                last_error
            )))
            .unwrap(),
        None,
        StatusCode::BAD_GATEWAY,
    )
}

fn handle_response(resp: reqwest::Response) -> Response {
    let status = resp.status();
    let mut response_builder = Response::builder().status(status);

    if let Some(headers_mut) = response_builder.headers_mut() {
        for (k, v) in resp.headers() {
            headers_mut.insert(k, v.clone());
        }
    }

    let stream = resp.bytes_stream();
    let body = Body::from_stream(stream);

    response_builder
        .body(body)
        .unwrap_or_else(|_| Response::new(Body::empty()))
}
