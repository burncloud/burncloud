pub mod api;

use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use burncloud_database::{create_default_database, Database};
use burncloud_database_router::RouterDatabase;
use std::sync::Arc;
use burncloud_router::create_router_app;
use burncloud_common::constants::API_PREFIX;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
}

pub async fn create_app(db: Arc<Database>) -> anyhow::Result<Router> {
    let state = AppState {
        db: db.clone(),
    };

    // 1. Management API Router
    let api_router = api::routes(state.clone());

    // 2. LiveView Router
    let liveview_router = burncloud_client::liveview_router(db.clone());

    // 3. Data Plane Router (Fallback)
    let router_app = create_router_app(db).await?;

    // Combine them
    // Prioritize:
    // 1. /console/api -> Management API
    // 2. /ws -> LiveView WS (handled inside liveview_router, usually)
    // 3. / -> LiveView HTML (handled inside liveview_router)
    // 4. Fallback -> Router (LLM Traffic)
    
    // Note: liveview_router currently handles "/" and "/ws".
    // We nest api under /console/api
    
    let app = Router::new()
        .merge(api_router)
        .merge(liveview_router) // Handles / and /ws
        .fallback_service(router_app)
        .layer(CorsLayer::permissive());

    Ok(app)
}

pub async fn start_server(port: u16) -> anyhow::Result<()> {
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;
    let db = Arc::new(db);

    let app = create_app(db).await?;

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Unified Gateway listening on {}", addr);
    println!("- Dashboard: http://127.0.0.1:{}", port);
    println!("- LLM API:   http://127.0.0.1:{}/v1/...", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
