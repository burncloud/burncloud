mod config;
mod adaptor;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Uri},
    response::Response,
    routing::{any, post},
    Router,
};
use burncloud_database::{create_default_database, Database};
use burncloud_database_router::RouterDatabase;
use burncloud_router_aws::{AwsConfig, sign_request};
use burncloud_common::types::OpenAIChatRequest;
use adaptor::gemini::GeminiAdaptor;
use config::{AuthType, RouterConfig, Upstream};
use reqwest::Client;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use http_body_util::BodyExt;

#[derive(Clone)]
struct AppState {
    client: Client,
    config: Arc<RwLock<RouterConfig>>,
    db: Arc<Database>,
}

async fn load_router_config(db: &Database) -> anyhow::Result<RouterConfig> {
    // Load Upstreams from DB
    let db_upstreams = RouterDatabase::get_all_upstreams(db).await?;
    let upstreams = db_upstreams.into_iter().map(|u| Upstream {
        id: u.id,
        name: u.name,
        base_url: u.base_url,
        api_key: u.api_key,
        match_path: u.match_path,
        auth_type: AuthType::from(u.auth_type.as_str()),
        priority: u.priority,
    }).collect();
    Ok(RouterConfig { upstreams })
}

pub async fn start_server(port: u16) -> anyhow::Result<()> {
    // Initialize Database
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;

    let config = load_router_config(&db).await?;
    let client = Client::builder().build()?;

    let state = AppState { 
        client,
        config: Arc::new(RwLock::new(config)),
        db: Arc::new(db),
    };

    let app = Router::new()
        .route("/", any(proxy_handler))
        .route("/_internal/reload", post(reload_handler))
        .route("/*path", any(proxy_handler)) 
        .layer(CorsLayer::permissive())
        .with_state(state);

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
    match RouterDatabase::validate_token(&state.db, user_token).await {
        Ok(Some(_)) => { /* Valid */ },
        Ok(None) => {
             return Response::builder()
                .status(401)
                .body(Body::from("Unauthorized: Invalid Token"))
                .unwrap();
        }
        Err(e) => {
             return Response::builder()
                .status(500)
                .body(Body::from(format!("Internal Auth Error: {}", e)))
                .unwrap();
        }
    }

    let path = uri.path();
    let config = state.config.read().await;
    
    // 1. Routing
    let upstream = match config.find_upstream(path) {
        Some(u) => u,
        None => {
            return Response::builder()
                .status(404)
                .body(Body::from(format!("No matching upstream found for path: {}", path)))
                .unwrap();
        }
    };
    
    // Check for explicit adaptor trigger header (before headers are moved)
    let use_adaptor = headers.contains_key("x-use-adaptor");

    // 2. Construct Target URL
    let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
    let target_url = format!("{}{}{}", upstream.base_url, path, query);

    println!("Proxying {} -> {} (via {})", path, target_url, upstream.name);

    // 3. Build Downstream Request
    let mut req_builder = state.client.request(method, &target_url);

    // 4. Forward Headers
    for (key, value) in headers {
        if let Some(key) = key {
            let key_str = key.as_str();
            // Filter hop-by-hop and auth
            // Also filter upstream-specific auth headers if they came from client
            if key_str == "host" || key_str == "content-length" || key_str == "transfer-encoding" 
               || key_str == "authorization" || key_str == "x-api-key" || key_str == "api-key" {
                continue;
            }
            req_builder = req_builder.header(key, value);
        }
    }

    // 5. Handle Auth & Body (Special logic for AWS and Adaptors)
    
    match &upstream.auth_type {
        AuthType::AwsSigV4 => {
            // ... (AWS logic remains same) ...
            // For AWS SigV4, we MUST buffer the body to calculate SHA256 hash
            let body_bytes = match body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => return Response::builder().status(400).body(Body::from(format!("Body Read Error: {}", e))).unwrap(),
            };
            
            let aws_config = match AwsConfig::from_colon_string(&upstream.api_key) {
                Ok(c) => c,
                Err(e) => return Response::builder().status(500).body(Body::from(format!("AWS Config Error: {}", e))).unwrap(),
            };
            
            req_builder = req_builder.body(body_bytes.clone());
            
            let mut request = match req_builder.build() {
                Ok(r) => r,
                Err(e) => return Response::builder().status(500).body(Body::from(format!("Request Build Error: {}", e))).unwrap(),
            };
            
            if let Err(e) = sign_request(&mut request, &aws_config, &body_bytes) {
                 return Response::builder().status(500).body(Body::from(format!("AWS Signing Error: {}", e))).unwrap();
            }
            
            match state.client.execute(request).await {
                 Ok(resp) => handle_response(resp),
                 Err(e) => Response::builder().status(502).body(Body::from(format!("Proxy Error: {}", e))).unwrap()
            }
        },
        AuthType::GoogleAI if use_adaptor => {
            // Protocol Adaptation: OpenAI -> Gemini
            // 1. Read Body (OpenAI JSON)
            let body_bytes = match body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => return Response::builder().status(400).body(Body::from(format!("Body Read Error: {}", e))).unwrap(),
            };

            let openai_req: OpenAIChatRequest = match serde_json::from_slice(&body_bytes) {
                Ok(req) => req,
                Err(e) => return Response::builder().status(400).body(Body::from(format!("Invalid OpenAI Request JSON: {}", e))).unwrap(),
            };

            // 2. Convert to Gemini Request
            let gemini_json = GeminiAdaptor::convert_request(openai_req);

            // 3. Send to Upstream
            req_builder = req_builder.header("x-goog-api-key", &upstream.api_key);
            req_builder = req_builder.json(&gemini_json);

            match req_builder.send().await {
                Ok(resp) => {
                    // 4. Convert Response back (Gemini -> OpenAI)
                    if resp.status().is_success() {
                        let gemini_resp_json: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!({}));
                        let openai_resp = GeminiAdaptor::convert_response(gemini_resp_json, &upstream.name); // Using upstream name as model name for now
                        Response::builder()
                            .status(200)
                            .header("content-type", "application/json")
                            .body(Body::from(serde_json::to_string(&openai_resp).unwrap()))
                            .unwrap()
                    } else {
                        // Forward error as is
                        handle_response(resp)
                    }
                },
                Err(e) => Response::builder().status(502).body(Body::from(format!("Proxy Error: {}", e))).unwrap()
            }
        }
        auth_type => {
            // Standard Passthrough (Streaming Upload Supported)
            match auth_type {
                AuthType::Bearer => {
                    req_builder = req_builder.bearer_auth(&upstream.api_key);
                }
                AuthType::Azure => {
                    req_builder = req_builder.header("api-key", &upstream.api_key);
                }
                AuthType::GoogleAI => {
                    req_builder = req_builder.header("x-goog-api-key", &upstream.api_key);
                }
                AuthType::Vertex => {
                    // TODO: For Vertex, we should ideally generate a token from Service Account JSON.
                    // Current implementation assumes the user provided a valid Bearer token (e.g. via gcloud auth print-access-token).
                    req_builder = req_builder.bearer_auth(&upstream.api_key);
                }
                AuthType::DeepSeek => {
                    req_builder = req_builder.bearer_auth(&upstream.api_key);
                }
                AuthType::Qwen => {
                    // Alibaba Cloud Qwen (DashScope) uses Bearer auth
                    req_builder = req_builder.bearer_auth(&upstream.api_key);
                }
                AuthType::Header(header_name) => {
                     req_builder = req_builder.header(header_name, &upstream.api_key);
                }
                AuthType::Query(ref _param) => {
                     // TODO: Append to URL query
                }
                _ => {} // AWS SigV4 handled above
            }

            let client_body = reqwest::Body::wrap_stream(body.into_data_stream());
            req_builder = req_builder.body(client_body);

            match req_builder.send().await {
                Ok(resp) => handle_response(resp),
                Err(e) => Response::builder().status(502).body(Body::from(format!("Proxy Error: {}", e))).unwrap()
            }
        }
    }
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