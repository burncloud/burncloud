pub mod api;
pub mod logging;
pub use api::auth::{auth_middleware, Claims};

use axum::{routing::get, Router};
use burncloud_database::{create_default_database, Database};
use burncloud_database_router::RouterDatabase;
use burncloud_database_user::UserDatabase;
use burncloud_router::create_router_app;
use burncloud_router::price_sync::SyncResult;
use burncloud_service_monitor::SystemMonitorService;
use burncloud_service_user::UserService;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub monitor: Arc<SystemMonitorService>,
    pub user_service: Arc<UserService>,
    pub force_sync_tx: mpsc::Sender<oneshot::Sender<SyncResult>>,
}

pub async fn create_app(db: Arc<Database>, enable_liveview: bool) -> anyhow::Result<Router> {
    let monitor = Arc::new(SystemMonitorService::new());
    // Start auto collection in background
    let _ = monitor.start_auto_update().await;

    // 3. Data Plane Router (Fallback) — must be created first to get force_sync_tx
    let (router_app, internal_app, force_sync_tx) = create_router_app(db.clone()).await?;

    let state = AppState {
        db: db.clone(),
        monitor,
        user_service: Arc::new(UserService::new()),
        force_sync_tx,
    };

    // 1. Management API Router
    let api_router = api::routes(state.clone());

    // Top-level liveness probe (unauthenticated, used by deploy validators and
    // external uptime monitors). The richer report lives at
    // `/console/internal/health`.
    // Internal routes (health, reload, price-sync) must be registered BEFORE
    // LiveView's catch-all `/console/{*path}` so they return JSON instead of
    // the SPA HTML shell.
    let mut app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(api_router)
        .merge(internal_app);

    if enable_liveview {
        // 2. LiveView Router
        let liveview_router = burncloud_client::liveview_router(db.clone());
        app = app.merge(liveview_router);
    }

    // Note: If LiveView is disabled, "/" requests will hit the fallback (router_app)
    // which usually returns 404 for unknown paths, or handles LLM requests.

    let app = app
        .fallback_service(router_app)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    Ok(app)
}

pub async fn start_server(host: &str, port: u16, enable_liveview: bool) -> anyhow::Result<()> {
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;
    UserDatabase::init(&db).await?;
    let db = Arc::new(db);

    let app = create_app(db, enable_liveview).await?;

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    tracing::info!("Unified Gateway listening on {}", addr);
    if enable_liveview {
        tracing::info!("- Dashboard: http://{}:{}/", host, port);
    }
    tracing::info!("- LLM API:   http://{}:{}/v1/...", host, port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
