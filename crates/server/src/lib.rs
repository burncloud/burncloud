pub mod api;
pub use api::auth::{auth_middleware, Claims};

use axum::Router;
use burncloud_database::{create_default_database, Database};
use burncloud_database_router::RouterDatabase;
use burncloud_database_user::UserDatabase;
use burncloud_router::create_router_app;
use burncloud_service_monitor::SystemMonitorService;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub monitor: Arc<SystemMonitorService>,
}

pub async fn create_app(db: Arc<Database>, enable_liveview: bool) -> anyhow::Result<Router> {
    let monitor = Arc::new(SystemMonitorService::new());
    // Start auto collection in background
    let _ = monitor.start_auto_update().await;

    let state = AppState {
        db: db.clone(),
        monitor,
    };

    // 1. Management API Router
    let api_router = api::routes(state.clone());

    // 3. Data Plane Router (Fallback)
    let router_app = create_router_app(db.clone()).await?;

    let mut app = Router::new().merge(api_router);

    if enable_liveview {
        // 2. LiveView Router
        let liveview_router = burncloud_client::liveview_router(db.clone());
        app = app.merge(liveview_router);
    }

    // Note: If LiveView is disabled, "/" requests will hit the fallback (router_app)
    // which usually returns 404 for unknown paths, or handles LLM requests.

    let app = app
        .fallback_service(router_app)
        .layer(CorsLayer::permissive());

    Ok(app)
}

pub async fn start_server(port: u16, enable_liveview: bool) -> anyhow::Result<()> {
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;
    UserDatabase::init(&db).await?;
    let db = Arc::new(db);

    let app = create_app(db, enable_liveview).await?;

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Unified Gateway listening on {}", addr);
    if enable_liveview {
        println!("- Dashboard: http://127.0.0.1:{}", port);
    }
    println!("- LLM API:   http://127.0.0.1:{}/v1/...", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
