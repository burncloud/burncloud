pub mod api;
pub mod logging;
pub use api::auth::{auth_middleware, Claims};

use axum::http::HeaderName;
use axum::{middleware, routing::get, Router};
use burncloud_database::{create_default_database, Database};
use burncloud_database_router::RouterDatabase;
use burncloud_database_user::UserDatabase;
use burncloud_router::create_router_app;
use burncloud_router::price_sync::SyncResult;
use burncloud_service_monitor::SystemMonitorService;
use burncloud_service_user::{JwtSecret, UserService};
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tower_http::cors::CorsLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

/// Validated shared secret for trusted internal HTTP calls.
///
/// The inner value is private and its `Debug` implementation is redacted so
/// application state and diagnostics cannot accidentally expose it.
#[derive(Clone)]
pub struct InternalSecret(Arc<str>);

impl InternalSecret {
    /// Validate an explicitly supplied internal API secret.
    pub fn new(value: impl Into<String>) -> anyhow::Result<Self> {
        let value = value.into();
        let value = value.trim();
        if value.is_empty() {
            anyhow::bail!("BURNCLOUD_INTERNAL_SECRET is missing or empty");
        }
        if value.parse::<axum::http::HeaderValue>().is_err() {
            anyhow::bail!("BURNCLOUD_INTERNAL_SECRET is invalid");
        }

        Ok(Self(Arc::from(value)))
    }

    pub(crate) fn header_value(&self) -> &str {
        &self.0
    }

    pub(crate) fn matches(&self, provided: &str) -> bool {
        provided == self.0.as_ref()
    }
}

impl fmt::Debug for InternalSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("InternalSecret([REDACTED])")
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub monitor: Arc<SystemMonitorService>,
    pub user_service: Arc<UserService>,
    pub jwt_secret: JwtSecret,
    pub internal_secret: InternalSecret,
    pub force_sync_tx: mpsc::Sender<oneshot::Sender<SyncResult>>,
    /// Ready-to-serve data-plane router used by authenticated console smoke tests.
    /// Requests sent through this router still pass the router's bearer-token validation
    /// and routing logic without exposing the bearer secret to the console client.
    pub data_plane: Router,
}

#[tracing::instrument(skip(db))]
pub async fn create_app(
    db: Arc<Database>,
    enable_liveview: bool,
    jwt_secret: JwtSecret,
    internal_secret: InternalSecret,
) -> anyhow::Result<Router> {
    let monitor = Arc::new(SystemMonitorService::new());
    // Start auto collection in background
    let _ = monitor.start_auto_update().await;

    // 3. Data Plane Router (Fallback) — must be created first to get force_sync_tx
    let (router_app, internal_app, force_sync_tx) =
        create_router_app(db.clone(), jwt_secret.clone()).await?;

    let state = AppState {
        db: db.clone(),
        monitor,
        user_service: Arc::new(UserService::new(jwt_secret.clone())),
        jwt_secret,
        internal_secret,
        force_sync_tx,
        data_plane: router_app.clone(),
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

    let x_request_id = HeaderName::from_static("x-request-id");

    let app = app
        .fallback_service(router_app)
        .layer(SetRequestIdLayer::new(
            x_request_id.clone(),
            MakeRequestUuid,
        ))
        .layer(PropagateRequestIdLayer::new(x_request_id.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        // Security boundary is intentionally global so it protects both the
        // explicitly merged internal routes and the data-plane fallback.
        .layer(middleware::from_fn_with_state(
            state,
            api::auth::security_boundary_middleware,
        ));

    Ok(app)
}

#[tracing::instrument(skip_all)]
pub async fn start_server(host: &str, port: u16, enable_liveview: bool) -> anyhow::Result<()> {
    let jwt_secret = std::env::var("JWT_SECRET")
        .map_err(|_| anyhow::anyhow!("JWT_SECRET is missing or empty"))?;
    let jwt_secret = JwtSecret::new(jwt_secret)?;
    let internal_secret = std::env::var("BURNCLOUD_INTERNAL_SECRET")
        .map_err(|_| anyhow::anyhow!("BURNCLOUD_INTERNAL_SECRET is missing or empty"))?;
    let internal_secret = InternalSecret::new(internal_secret)?;

    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;
    UserDatabase::init(&db).await?;
    let db = Arc::new(db);

    let app = create_app(db, enable_liveview, jwt_secret, internal_secret).await?;

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

#[cfg(test)]
mod tests {
    use super::InternalSecret;

    #[test]
    fn internal_secret_rejects_empty_and_invalid_values() {
        for value in ["", "   ", "\n\t", "secret\nheader"] {
            assert!(InternalSecret::new(value).is_err());
        }
    }

    #[test]
    fn internal_secret_debug_is_redacted() {
        let plaintext = "must-never-appear";
        let secret = InternalSecret::new(plaintext)
            .unwrap_or_else(|e| panic!("test internal secret must be valid: {e}"));
        let rendered = format!("{secret:?}");

        assert!(!rendered.contains(plaintext));
        assert_eq!(rendered, "InternalSecret([REDACTED])");
    }
}
