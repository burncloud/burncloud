mod config;
mod adaptor;
mod balancer;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Uri},
    response::Response,
    routing::post,
    Router,
};
use burncloud_database::{create_default_database, Database};
use burncloud_database_router::{RouterDatabase, DbRouterLog};
use burncloud_router_aws::{AwsConfig, sign_request};
use burncloud_common::types::OpenAIChatRequest;
use adaptor::gemini::GeminiAdaptor;
use adaptor::claude::ClaudeAdaptor;
use balancer::RoundRobinBalancer;
use config::{AuthType, RouterConfig, Upstream, Group, GroupMember, RouteTarget};
use reqwest::Client;
use std::{net::SocketAddr, sync::Arc, time::Instant};
use tokio::sync::{RwLock, mpsc};
use tower_http::cors::CorsLayer;
use http_body_util::BodyExt;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    client: Client,
    config: Arc<RwLock<RouterConfig>>,
    db: Arc<Database>,
    balancer: Arc<RoundRobinBalancer>,
    log_tx: mpsc::Sender<DbRouterLog>,
}

async fn load_router_config(db: &Database) -> anyhow::Result<RouterConfig> {
    // Load Upstreams
    let db_upstreams = RouterDatabase::get_all_upstreams(db).await?;
    let upstreams: Vec<Upstream> = db_upstreams.into_iter().map(|u| Upstream {
        id: u.id,
        name: u.name,
        base_url: u.base_url,
        api_key: u.api_key,
        match_path: u.match_path,
        auth_type: AuthType::from(u.auth_type.as_str()),
        priority: u.priority,
    }).collect();

    // Load Groups
    let db_groups = RouterDatabase::get_all_groups(db).await?;
    let db_members = RouterDatabase::get_group_members(db).await?;

    let groups = db_groups.into_iter().map(|g| {
        let members = db_members.iter()
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
    }).collect();

    Ok(RouterConfig { upstreams, groups })
}

pub async fn create_router_app(db: Arc<Database>) -> anyhow::Result<Router> {
    let config = load_router_config(&db).await?;
    let client = Client::builder().build()?;
    let balancer = Arc::new(RoundRobinBalancer::new());

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
        db: db, // Arc<Database>
        balancer,
        log_tx,
    };

use burncloud_common::constants::INTERNAL_PREFIX;

// ...

    let reload_path = format!("{}/reload", INTERNAL_PREFIX);

    let app = Router::new()
        .route(&reload_path, post(reload_handler))
        .fallback(proxy_handler)
        .layer(CorsLayer::permissive())
        .with_state(state);

    Ok(app)
}

pub async fn start_server(port: u16) -> anyhow::Result<()> {
    // Initialize Database
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;
    let db = Arc::new(db); // Wrap in Arc for sharing

    let app = create_router_app(db).await?;

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Router listening on {}", addr);
    println!("Ready to handle requests. Try: curl -H 'Authorization: Bearer sk-burncloud-demo' http://127.0.0.1:{}/v1/messages", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn reload_handler(
    State(state): State<AppState>,
) -> Response {
    println!("Reloading router configuration...");
    match load_router_config(&state.db).await {
        Ok(new_config) => {
            let mut config_write = state.config.write().await;
            *config_write = new_config;
            println!("Configuration reloaded successfully.");
            Response::builder().status(200).body(Body::from("Reloaded")).unwrap()
        }
        Err(e) => {
             eprintln!("Configuration reload failed: {}", e);
             Response::builder().status(500).body(Body::from(format!("Reload failed: {}", e))).unwrap()
        }
    }
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
    let user_auth = headers.get("authorization")
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
    let user_id = match RouterDatabase::validate_token(&state.db, user_token).await {
        Ok(Some(token_data)) => {
             if token_data.quota_limit >= 0 && token_data.used_quota >= token_data.quota_limit {
                 return Response::builder()
                    .status(429)
                    .body(Body::from("Quota Exceeded"))
                    .unwrap();
             }
             token_data.user_id
        },
        Ok(None) => {
             return Response::builder()
                .status(401)
                .body(Body::from("Unauthorized: Invalid Token"))
                .unwrap();
        },
        Err(e) => {
             return Response::builder()
                .status(500)
                .body(Body::from(format!("Internal Auth Error: {}", e)))
                .unwrap();
        }
    };

    // Buffer body for token counting and retries
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => return Response::builder().status(400).body(Body::from(format!("Body Read Error: {}", e))).unwrap(),
    };

    // Estimate Prompt Tokens (Simple approximation: 1 token ~= 4 bytes)
    // TODO: Integrate tiktoken-rs for precise counting
    let prompt_tokens = (body_bytes.len() as f32 / 4.0).ceil() as i32;

    // Perform Proxy Logic
    let (response, upstream_id, final_status) = proxy_logic(&state, method, uri, headers, body_bytes, &path).await;

    // Estimate Completion Tokens (If header present, else 0 for streaming)
    // For streaming, we can't easily know without wrapping the stream.
    let completion_tokens = 0; 

    // Async Log
    let log = DbRouterLog {
        request_id,
        user_id: Some(user_id),
        path,
        upstream_id,
        status_code: final_status.as_u16(),
        latency_ms: start_time.elapsed().as_millis() as i64,
        prompt_tokens,
        completion_tokens,
    };

    let _ = state.log_tx.send(log).await;

    response
}

async fn proxy_logic(
    state: &AppState,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body_bytes: axum::body::Bytes,
    path: &str,
) -> (Response, Option<String>, axum::http::StatusCode) {
    let config = state.config.read().await;
    
    // 1. Routing
    let route = match config.find_route(path) {
        Some(r) => r,
        None => {
            return (Response::builder()
                .status(404)
                .body(Body::from(format!("No matching upstream found for path: {}", path)))
                .unwrap(), None, axum::http::StatusCode::NOT_FOUND);
        }
    };
    
    // Resolve Route Target -> Ordered Candidates for Retry
    let candidates: Vec<&Upstream> = match route {
        RouteTarget::Upstream(u) => vec![u],
        RouteTarget::Group(g) => {
            if g.members.is_empty() {
                 return (Response::builder()
                    .status(503)
                    .body(Body::from(format!("Group '{}' has no healthy members", g.name)))
                    .unwrap(), None, axum::http::StatusCode::SERVICE_UNAVAILABLE);
            }
            
            let start_idx = state.balancer.next_index(&g.id, g.members.len());
            
            let mut ordered_members = Vec::with_capacity(g.members.len());
            for i in 0..g.members.len() {
                let idx = (start_idx + i) % g.members.len();
                let member = &g.members[idx];
                if let Some(u) = config.get_upstream(&member.upstream_id) {
                    ordered_members.push(u);
                }
            }
            
            if ordered_members.is_empty() {
                 return (Response::builder()
                    .status(500)
                    .body(Body::from("Configuration Error: Group members not found in upstream list"))
                    .unwrap(), None, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
            }
            ordered_members
        }
    };
    
    let use_adaptor = headers.contains_key("x-use-adaptor");
    let mut last_error = String::new();
    let mut last_upstream_id = None;

    for (attempt, upstream) in candidates.iter().enumerate() {
        last_upstream_id = Some(upstream.id.clone());

        // 2. Construct Target URL
        let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
        let target_url = format!("{}{}{}", upstream.base_url, path, query);

        println!("Proxying {} -> {} (via {}) [Attempt {}]", path, target_url, upstream.name, attempt + 1);

        // 3. Build Downstream Request
        let mut req_builder = state.client.request(method.clone(), &target_url);

        // 4. Forward Headers
        for (key, value) in &headers {
            let key_str = key.as_str();
            if key_str == "host" || key_str == "content-length" || key_str == "transfer-encoding" 
               || key_str == "authorization" || key_str == "x-api-key" || key_str == "api-key" {
                continue;
            }
            req_builder = req_builder.header(key, value);
        }

        // 5. Handle Auth & Body
        let result = match &upstream.auth_type {
            AuthType::AwsSigV4 => {
                let aws_config = match AwsConfig::from_colon_string(&upstream.api_key) {
                    Ok(c) => c,
                    Err(e) => {
                        last_error = format!("AWS Config Error: {}", e);
                        continue; 
                    }
                };
                
                req_builder = req_builder.body(body_bytes.clone());
                let mut request = match req_builder.build() {
                    Ok(r) => r,
                    Err(e) => { last_error = format!("Req Build Error: {}", e); continue; }
                };
                
                if let Err(e) = sign_request(&mut request, &aws_config, &body_bytes) {
                     last_error = format!("AWS Signing Error: {}", e);
                     continue;
                }
                state.client.execute(request).await
            },
            AuthType::Claude if use_adaptor => {
                let openai_req: Result<OpenAIChatRequest, _> = serde_json::from_slice(&body_bytes);
                match openai_req {
                    Ok(req) => {
                        let claude_json = ClaudeAdaptor::convert_request(req);
                        req_builder = req_builder.header("x-api-key", &upstream.api_key);
                        req_builder = req_builder.header("anthropic-version", "2023-06-01");
                        req_builder = req_builder.json(&claude_json);
                        state.client.execute(req_builder.build().unwrap()).await
                    },
                    Err(e) => {
                        return (Response::builder().status(400).body(Body::from(format!("Invalid OpenAI JSON: {}", e))).unwrap(), last_upstream_id, axum::http::StatusCode::BAD_REQUEST);
                    }
                }
            },
            AuthType::GoogleAI if use_adaptor => {
                let openai_req: Result<OpenAIChatRequest, _> = serde_json::from_slice(&body_bytes);
                match openai_req {
                    Ok(req) => {
                        let gemini_json = GeminiAdaptor::convert_request(req);
                        req_builder = req_builder.header("x-goog-api-key", &upstream.api_key);
                        req_builder = req_builder.json(&gemini_json);
                        state.client.execute(req_builder.build().unwrap()).await
                    },
                    Err(e) => {
                        return (Response::builder().status(400).body(Body::from(format!("Invalid OpenAI JSON: {}", e))).unwrap(), last_upstream_id, axum::http::StatusCode::BAD_REQUEST);
                    }
                }
            },
            auth_type => {
                match auth_type {
                    AuthType::Bearer => { req_builder = req_builder.bearer_auth(&upstream.api_key); }
                    AuthType::Azure => { req_builder = req_builder.header("api-key", &upstream.api_key); }
                    AuthType::GoogleAI => { req_builder = req_builder.header("x-goog-api-key", &upstream.api_key); }
                    AuthType::Claude => { 
                        req_builder = req_builder.header("x-api-key", &upstream.api_key); 
                        req_builder = req_builder.header("anthropic-version", "2023-06-01");
                    }
                    AuthType::Vertex => { req_builder = req_builder.bearer_auth(&upstream.api_key); }
                    AuthType::DeepSeek => { req_builder = req_builder.bearer_auth(&upstream.api_key); }
                    AuthType::Qwen => { req_builder = req_builder.bearer_auth(&upstream.api_key); }
                    AuthType::Header(h) => { req_builder = req_builder.header(h, &upstream.api_key); }
                    _ => {}
                }
                req_builder = req_builder.body(body_bytes.clone());
                req_builder.send().await
            }
        };

        match result {
            Ok(resp) => {
                if resp.status().is_server_error() { // 500-599
                    last_error = format!("Upstream returned {}", resp.status());
                    eprintln!("Failover: {} failed with {}, trying next...", upstream.name, resp.status());
                    continue; 
                }
                
                if use_adaptor && resp.status().is_success() {
                     let status = resp.status();
                     let resp_json: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!({}));
                     let converted_json = match upstream.auth_type {
                         AuthType::GoogleAI => GeminiAdaptor::convert_response(resp_json, &upstream.name),
                         AuthType::Claude => ClaudeAdaptor::convert_response(resp_json, &upstream.name),
                         _ => resp_json
                     };
                     return (Response::builder()
                        .status(status)
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_string(&converted_json).unwrap()))
                        .unwrap(), last_upstream_id, status);
                } else {
                    let status = resp.status();
                    return (handle_response(resp), last_upstream_id, status);
                }
            },
            Err(e) => {
                last_error = format!("Network Error: {}", e);
                eprintln!("Failover: {} failed with {}, trying next...", upstream.name, e);
                continue;
            }
        }
    }

    (Response::builder()
        .status(502)
        .body(Body::from(format!("All upstreams failed. Last error: {}", last_error)))
        .unwrap(), None, axum::http::StatusCode::BAD_GATEWAY)
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
    
    response_builder.body(body).unwrap_or_else(|_| Response::new(Body::empty()))
}