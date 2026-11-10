pub mod api;
mod local_routes;
pub mod logging;
mod model_selection;
pub mod node_attachment;
#[expect(
    clippy::expect_used,
    clippy::too_many_lines,
    reason = "node_orchestrator is a legacy state machine whose mutable workload lookups rely on the invariant that each demand is inserted before transition handling; keep the debt explicit until that state machine is decomposed"
)]
pub mod node_orchestrator;
pub mod node_request;
pub mod node_test;
pub use api::auth::{auth_middleware, Claims};

use axum::http::HeaderName;
use axum::{middleware, routing::get, Router};
use burncloud_database::{create_default_database, Database};
use burncloud_node_runtime::{NodeContext, NodeRuntime};
use burncloud_router::create_router_app_with_route_miss;
use burncloud_router::local_attachment::LocalRouteManager;
use burncloud_router::price_sync::SyncResult;
use burncloud_router::RouterDatabase;
use burncloud_service_monitor::SystemMonitorService;
use burncloud_service_user::UserDatabase;
use burncloud_service_user::{JwtSecret, UserService};
use burncloud_supply_channel::ChannelProtocolConfigModel;
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
    pub node_context: NodeContext,
    pub node_request_state: node_request::NodeRequestState,
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

/// 构建统一网关，将模型选择接口、管理 API 和可选 LiveView 挂到同一服务。
///
/// # Examples
/// ```no_run
/// # use std::sync::Arc;
/// # use burncloud_database::Database;
/// # use burncloud_service_user::JwtSecret;
/// # use burncloud_server::{create_app, InternalSecret};
/// # async fn example(db: Arc<Database>, jwt: JwtSecret, secret: InternalSecret) -> anyhow::Result<()> {
/// let app = create_app(db, false, jwt, secret).await?;
/// // 即使关闭 LiveView，模型选择三个接口仍由该 Router 提供。
/// # drop(app);
/// # Ok(())
/// # }
/// ```
///
/// # Notes
/// 本函数不额外绑定模型服务端口；资源页面可访问不表示模型已下载或已部署。
#[tracing::instrument(skip(db))]
pub async fn create_app(
    db: Arc<Database>,
    enable_liveview: bool,
    jwt_secret: JwtSecret,
    internal_secret: InternalSecret,
) -> anyhow::Result<Router> {
    create_app_with_node_request_state(
        db,
        enable_liveview,
        jwt_secret,
        internal_secret,
        node_request::NodeRequestState::default(),
    )
    .await
}

#[tracing::instrument(skip(db, node_request_state))]
pub async fn create_app_with_node_request_state(
    db: Arc<Database>,
    enable_liveview: bool,
    jwt_secret: JwtSecret,
    internal_secret: InternalSecret,
    node_request_state: node_request::NodeRequestState,
) -> anyhow::Result<Router> {
    let monitor = Arc::new(SystemMonitorService::new());
    // Auto collection is auxiliary to serving traffic. Keep startup available
    // if it cannot start, but make the degraded observability state explicit.
    if let Err(error) = monitor.start_auto_update().await {
        tracing::warn!(%error, "system monitor auto-update failed to start");
    }

    // Keep the Node runtime and its request-visible status alive with the server.
    let node_context = NodeRuntime::new().start();
    let route_miss_state = node_request_state.clone();

    // 3. Data Plane Router (Fallback) — must be created first to get force_sync_tx
    let (router_app, internal_app, force_sync_tx) = create_router_app_with_route_miss(
        db.clone(),
        jwt_secret.clone(),
        Arc::new(move |model| route_miss_state.route_miss_response(model)),
    )
    .await?;

    let state = AppState {
        db: db.clone(),
        node_context,
        node_request_state,
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
        .merge(model_selection::routes()?)
        .merge(internal_app);

    if enable_liveview {
        // 2. LiveView Router
        let liveview_router = burncloud_client::liveview_router(db.clone());
        app = app.merge(liveview_router);
    }

    // Note: If LiveView is disabled, "/" requests will hit the fallback (router_app)
    // which usually returns 404 for unknown paths, or handles LLM requests.

    let x_request_id = HeaderName::from_static("x-request-id");

    let mut app = app
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

    // #567 human black-box harness. It is deliberately merged after the
    // production security stack because it is not a product API and must be
    // reachable by simple local curl commands. The explicit env gate is the
    // boundary: without it these routes do not exist at all.
    if node_test::enabled_from_env() {
        tracing::warn!(
            "BURNCLOUD_NODE_TEST_API is enabled; mounting test-only Node routes on the existing server"
        );
        app = app.merge(node_test::router());
    }

    Ok(app)
}

/// 启动统一网关，并在同一生命周期内管理配置的本机路由挂载。
///
/// # Examples
/// ```no_run
/// # async fn example() -> anyhow::Result<()> {
/// // 需预先配置 JWT_SECRET 与 BURNCLOUD_INTERNAL_SECRET。
/// burncloud_server::start_server("127.0.0.1", 3000, false).await?;
/// # Ok(()) }
/// ```
///
/// # Notes
/// `BURNCLOUD_LOCAL_ROUTE_ATTACHMENTS` 为仅含 `model`/`base_url` 的 JSON 数组；
/// 也可使用 `BURNCLOUD_LOCAL_ROUTE_DEPLOYMENTS` 导入一个部署结果对象或数组，
/// 必须包含 `model_id`、`endpoint`、`status="ready"` 和 `state="Ready"`，
/// `error` 只能缺省或为 null。两项配置互斥；均未设置或唯一来源为 `[]` 时不创建挂载。
/// 全量校验先于探测与挂载。部署 Ready、PID、历史心跳和事件不能代替当前端点的
/// `/v1/models` 直接探测，也不改变认证、计费或模型报价。
/// 取消本 future 同步关闭本机挂载准入，不自动删除持久化记录；重启残留须核对后
/// 显式 detach。低层 `create_app` 不读取这两项配置，也不启动管理器。
#[tracing::instrument(skip_all)]
pub async fn start_server(host: &str, port: u16, enable_liveview: bool) -> anyhow::Result<()> {
    let attachments = local_routes::configured_attachments()?;
    let node_test_api_enabled = node_test::enabled_from_env();
    node_test::validate_bind_host(host, node_test_api_enabled)?;

    let jwt_secret = std::env::var("JWT_SECRET")
        .map_err(|_| anyhow::anyhow!("JWT_SECRET is missing or empty"))?;
    let jwt_secret = JwtSecret::new(jwt_secret)?;
    let internal_secret = std::env::var("BURNCLOUD_INTERNAL_SECRET")
        .map_err(|_| anyhow::anyhow!("BURNCLOUD_INTERNAL_SECRET is missing or empty"))?;
    let internal_secret = InternalSecret::new(internal_secret)?;

    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;
    UserDatabase::init(&db).await?;
    // Domain-owned default records (#842). Platform only creates schema and historic
    // fixups; Identity owns the demo account plus demo API key and Supply owns the default
    // protocol configs, so the application bootstrap seeds them here, in the order the
    // records depend on: account before the key that references it. Both owner calls are
    // idempotent, so a restart over an existing database inserts nothing.
    UserDatabase::seed_demo_defaults(&db).await?;
    ChannelProtocolConfigModel::seed_default_protocol_configs(&db).await?;
    let db = Arc::new(db);

    let manager = LocalRouteManager::new(db.clone(), attachments)?;
    let app = create_app(db, enable_liveview, jwt_secret, internal_secret).await?;

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    tracing::info!("Unified Gateway listening on {}", addr);
    if enable_liveview {
        tracing::info!("- Dashboard: http://{}:{}/", host, port);
    }
    tracing::info!("- LLM API:   http://{}:{}/v1/...", host, port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    // 两个 future 由启动入口直接持有，退出/取消会回收首检，不遗留脱离的挂载任务。
    tokio::select! {
        result = async { axum::serve(listener, app).await } => result?,
        result = manager.run() => result?,
    }

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
