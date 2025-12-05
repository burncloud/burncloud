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
use config::{AuthType, RouterConfig, Upstream};
use reqwest::Client;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

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

    // 4. Forward Headers & Inject Auth
    for (key, value) in headers {
        if let Some(key) = key {
            let key_str = key.as_str();
            // Filter hop-by-hop and auth
            if key_str == "host" || key_str == "content-length" || key_str == "transfer-encoding" || key_str == "authorization" || key_str == "x-api-key" {
                continue;
            }
            req_builder = req_builder.header(key, value);
        }
    }

    // Inject Real Auth
    match upstream.auth_type {
        AuthType::Bearer => {
            req_builder = req_builder.bearer_auth(&upstream.api_key);
        }
        AuthType::XApiKey => {
             req_builder = req_builder.header("x-api-key", &upstream.api_key);
        }
        AuthType::Query(ref _param) => {
             // TODO: Append to URL query
        }
    }

    let client_body = reqwest::Body::wrap_stream(body.into_data_stream());
    req_builder = req_builder.body(client_body);

    // 5. Execute & Stream Response
    match req_builder.send().await {
        Ok(resp) => {
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
        Err(e) => {
            Response::builder()
                .status(502)
                .body(Body::from(format!("Proxy Error: {}", e)))
                .unwrap()
        }
    }
}
