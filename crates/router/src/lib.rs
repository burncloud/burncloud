mod config;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Uri},
    response::Response,
    routing::any,
    Router,
};
use burncloud_database::{create_default_database, Database};
use burncloud_database_router::RouterDatabase;
use burncloud_router_aws::{AwsConfig, sign_request};
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

pub async fn start_server(port: u16) -> anyhow::Result<()> {
    // Initialize Database
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;

    // Load Upstreams from DB
    let db_upstreams = RouterDatabase::get_all_upstreams(&db).await?;
    let upstreams = db_upstreams.into_iter().map(|u| Upstream {
        id: u.id,
        name: u.name,
        base_url: u.base_url,
        api_key: u.api_key,
        match_path: u.match_path,
        auth_type: AuthType::from(u.auth_type.as_str()),
        priority: u.priority, // Map priority
    }).collect();

    let config = RouterConfig { upstreams };
    let client = Client::builder().build()?;

    let state = AppState { 
        client,
        config: Arc::new(RwLock::new(config)),
        db: Arc::new(db),
    };

    let app = Router::new()
        .route("/", any(proxy_handler))
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

    // 5. Handle Auth & Body (Special logic for AWS)
    match &upstream.auth_type {
        AuthType::AwsSigV4 => {
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
        auth_type => {
            // Standard Passthrough (Streaming Upload Supported)
            match auth_type {
                AuthType::Bearer => {
                    req_builder = req_builder.bearer_auth(&upstream.api_key);
                }
                AuthType::Azure => {
                    req_builder = req_builder.header("api-key", &upstream.api_key);
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