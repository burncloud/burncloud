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
use burncloud_service_cache::CacheService;
use burncloud_service_monitor::SystemMonitorService;
use burncloud_service_user::UserService;
use std::net::{IpAddr, SocketAddr};
use std::process::Command;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tower_http::cors::CorsLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub monitor: Arc<SystemMonitorService>,
    pub user_service: Arc<UserService>,
    pub cache: CacheService,
    pub force_sync_tx: mpsc::Sender<oneshot::Sender<SyncResult>>,
    pub(crate) bootstrap_token: Arc<String>,
    pub(crate) bootstrap_token_required: bool,
}

fn is_loopback_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    host.parse::<IpAddr>()
        .map(|address| address.is_loopback())
        .unwrap_or(false)
}

fn resolve_bootstrap_token() -> anyhow::Result<(String, bool)> {
    if let Ok(configured) = std::env::var("BURNCLOUD_BOOTSTRAP_TOKEN") {
        let configured = configured.trim().to_string();
        if configured.len() < 16 {
            anyhow::bail!("BURNCLOUD_BOOTSTRAP_TOKEN must contain at least 16 characters");
        }
        return Ok((configured, false));
    }

    Ok((api::registration::generate_bootstrap_token(), true))
}

fn try_open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let result = Command::new("cmd").args(["/C", "start", "", url]).spawn();

    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(url).spawn();

    #[cfg(all(unix, not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(url).spawn();

    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    let result: std::io::Result<std::process::Child> = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "automatic browser launch is not supported on this platform",
    ));

    if let Err(error) = result {
        tracing::debug!(%error, %url, "Could not open first-run browser automatically");
    }
}

async fn build_app(
    db: Arc<Database>,
    enable_liveview: bool,
    bootstrap_token_required: bool,
) -> anyhow::Result<(Router, bool)> {
    // The marker is created before public registration is reachable. Existing
    // installations are marked complete here so upgrades never reopen setup.
    let bootstrap_complete = api::registration::initialize_bootstrap_state(&db).await?;
    let setup_required = !bootstrap_complete;
    let (bootstrap_token, generated_bootstrap_token) = resolve_bootstrap_token()?;

    if setup_required {
        if bootstrap_token_required {
            if generated_bootstrap_token {
                tracing::warn!(
                    setup_code = %bootstrap_token,
                    "First-time remote setup: enter this one-time setup code on the Create Administrator page"
                );
            } else {
                tracing::info!(
                    "First-time remote setup will use the configured BURNCLOUD_BOOTSTRAP_TOKEN"
                );
            }
        } else {
            tracing::info!(
                "First-time local setup ready: no bootstrap token or environment configuration is required"
            );
        }
    }

    let monitor = Arc::new(SystemMonitorService::new());
    let _ = monitor.start_auto_update().await;

    let cache = CacheService::new().await?;
    if cache.is_available().await {
        tracing::info!("Redis cache enabled and connected");
    } else {
        tracing::info!("Redis cache disabled");
    }

    let (router_app, internal_app, force_sync_tx) = create_router_app(db.clone()).await?;

    let state = AppState {
        db: db.clone(),
        monitor,
        user_service: Arc::new(UserService::new()),
        cache,
        force_sync_tx,
        bootstrap_token: Arc::new(bootstrap_token),
        bootstrap_token_required,
    };

    let api_router = api::routes(state.clone());

    let mut app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(api_router)
        .merge(internal_app);

    if enable_liveview {
        let liveview_router = burncloud_client::liveview_router(db.clone());
        app = app.merge(liveview_router);
    }

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
        .layer(middleware::from_fn(
            api::auth::security_boundary_middleware,
        ));

    Ok((app, setup_required))
}

/// Build an embeddable application with the conservative remote-safe bootstrap
/// policy. Library callers that expose this Router directly must provide the
/// one-time setup code; the zero-configuration shortcut is enabled only by
/// `start_server` when BurnCloud is actually bound to a loopback address.
#[tracing::instrument(skip(db))]
pub async fn create_app(db: Arc<Database>, enable_liveview: bool) -> anyhow::Result<Router> {
    let (app, _) = build_app(db, enable_liveview, true).await?;
    Ok(app)
}

#[tracing::instrument(skip_all)]
pub async fn start_server(host: &str, port: u16, enable_liveview: bool) -> anyhow::Result<()> {
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;
    UserDatabase::init(&db).await?;
    let db = Arc::new(db);

    // BurnCloud defaults to 127.0.0.1. On that private local-only binding the
    // first registration can safely become admin with no manual setup secret.
    // Explicitly exposing the server on a non-loopback address switches back
    // to a one-time setup code that BurnCloud generates automatically.
    let local_only = is_loopback_host(host);
    let (app, setup_required) = build_app(db, enable_liveview, !local_only).await?;

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("Unified Gateway listening on {}", addr);
    if enable_liveview {
        tracing::info!("- Dashboard: http://{}:{}/", host, port);
    }
    tracing::info!("- LLM API:   http://{}:{}/v1/...", host, port);

    if setup_required && local_only {
        let setup_url = format!("http://127.0.0.1:{port}/register");
        tracing::info!(%setup_url, "First-time setup: create your administrator account");
        if enable_liveview {
            try_open_browser(&setup_url);
        }
    }

    axum::serve(listener, app).await?;

    Ok(())
}
