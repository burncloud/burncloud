// 声明 api 子模块（公开）：包含各业务 API 路由与认证中间件
pub mod api;
// 声明 logging 子模块（公开）：日志相关
pub mod logging;
// 声明 node_attachment 子模块（公开）：节点附件相关
pub mod node_attachment;
// 声明 node_deployment 子模块（私有，仅本模块可见）：节点部署状态管理（前面分析过的 DeploymentEventStore / NodeDeploymentState）
mod node_deployment;
// 豁免两条 clippy lint：
// - expect_used：禁止使用 .expect()（因为会 panic）
// - too_many_lines：禁止函数过长
// 原因：node_orchestrator 是一个遗留状态机，其可变 workload 查找依赖
// "每条 demand 在状态转换处理前已被插入"这个不变量；在状态机拆分前
// 把技术债显式记录下来，而不是静默忽略。
#[expect(
    clippy::expect_used,
    clippy::too_many_lines,
    reason = "node_orchestrator is a legacy state machine whose mutable workload lookups rely on the invariant that each demand is inserted before transition handling; keep the debt explicit until that state machine is decomposed"
)]
// 声明 node_orchestrator 子模块（公开）：节点编排状态机
pub mod node_orchestrator;
// 声明 node_request 子模块（公开）：节点请求相关
pub mod node_request;
// 声明 node_runtime 子模块（私有）：llama.cpp 运行时适配器与准备器
mod node_runtime;
// 声明 node_test 子模块（公开）：测试用路由（见文件末尾的 harness）
pub mod node_test;
// 从 api::auth 重新导出 auth_middleware 和 Claims，使外部可直接通过本 crate 根访问
// （api::routes 中用的是 crate::auth_middleware，即这里导出的）
pub use api::auth::{auth_middleware, Claims};

// 引入 HTTP 头名称类型
use axum::http::HeaderName;
// 引入 axum 的中间件工具、GET 路由辅助、路由器
use axum::{middleware, routing::get, Router};
// 引入数据库创建函数与 Database 类型
use burncloud_database::{create_default_database, Database};
// 引入节点运行时相关类型
use burncloud_node_runtime::{
    deployment::{
        // 部署事件发布者 trait
        DeploymentEventPublisher,
        // 部署服务
        DeploymentService,
        // HTTP 探测器（前面分析过的 HttpProbe）
        HttpProbe,
        // 本地进程管理器
        LocalProcessManager,
    },
    // 节点上下文（运行时启动后返回，持有运行时句柄）
    NodeContext,
    // 节点运行时
    NodeRuntime,
};
// 引入路由应用创建函数（带 route_miss 回调）与 SyncResult
use burncloud_router::create_router_app_with_route_miss;
use burncloud_router::price_sync::SyncResult;
// 引入路由数据库初始化
use burncloud_router::RouterDatabase;
// 引入系统监控服务
use burncloud_service_monitor::SystemMonitorService;
// 引入用户数据库与用户服务相关类型
use burncloud_service_user::UserDatabase;
use burncloud_service_user::{JwtSecret, UserService};
// 引入渠道协议配置模型（用于 seed 默认协议配置）
use burncloud_supply_channel::ChannelProtocolConfigModel;
// 引入 fmt（用于自定义 Debug 实现）
use std::fmt;
// 引入 SocketAddr（监听地址）
use std::net::SocketAddr;
// 引入 Arc（共享所有权）
use std::sync::Arc;
// 引入 tokio 的 mpsc 和 oneshot 通道
use tokio::sync::{mpsc, oneshot};
// 引入 CORS 中间件
use tower_http::cors::CorsLayer;
// 引入请求 ID 相关中间件：
// - MakeRequestUuid：生成 UUID 作为请求 ID
// - PropagateRequestIdLayer：把请求 ID 传播到响应头
// - SetRequestIdLayer：把请求 ID 写入请求头
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
// 引入 HTTP 追踪中间件（记录请求日志、耗时）
use tower_http::trace::TraceLayer;

/// Validated shared secret for trusted internal HTTP calls.
///
/// The inner value is private and its `Debug` implementation is redacted so
/// application state and diagnostics cannot accidentally expose it.
// 内部共享密钥（用于可信内部 HTTP 调用）
// 派生 Clone：可以低成本复制
#[derive(Clone)]
pub struct InternalSecret(Arc<str>);

// 为 InternalSecret 实现方法
impl InternalSecret {
    /// Validate an explicitly supplied internal API secret.
    // 校验外部传入的密钥是否合法
    pub fn new(value: impl Into<String>) -> anyhow::Result<Self> {
        // 转成 String
        let value = value.into();
        // 去除首尾空白
        let value = value.trim();
        // 空字符串直接报错
        if value.is_empty() {
            anyhow::bail!("BURNCLOUD_INTERNAL_SECRET is missing or empty");
        }
        // 校验能否作为合法 HTTP HeaderValue（防止换行注入等）
        if value.parse::<axum::http::HeaderValue>().is_err() {
            anyhow::bail!("BURNCLOUD_INTERNAL_SECRET is invalid");
        }

        // 包装成 Arc<str> 返回
        Ok(Self(Arc::from(value)))
    }

    // 返回密钥的 &str 视图（crate 内可见）
    pub(crate) fn header_value(&self) -> &str {
        &self.0
    }

    // 比较外部提供的密钥是否与内部一致
    // 注意：这里是普通 == 比较，非恒定时间；注释未声明安全级别
    pub(crate) fn matches(&self, provided: &str) -> bool {
        provided == self.0.as_ref()
    }
}

// 自定义 Debug 实现：红act 掉真实密钥，防止日志泄露
impl fmt::Debug for InternalSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("InternalSecret([REDACTED])")
    }
}

// 应用全局状态：被注入到所有 handler 与中间件
#[derive(Clone)]
pub struct AppState {
    // 数据库连接（Arc 共享）
    pub db: Arc<Database>,
    // 节点上下文（运行时句柄）
    pub node_context: NodeContext,
    // 节点请求状态
    pub node_request_state: node_request::NodeRequestState,
    // 系统监控服务
    pub monitor: Arc<SystemMonitorService>,
    // 用户服务
    pub user_service: Arc<UserService>,
    // JWT 密钥
    pub jwt_secret: JwtSecret,
    // 内部共享密钥
    pub internal_secret: InternalSecret,
    // 强制同步通道发送端（向数据面发起价格同步，用 oneshot 回传结果）
    pub force_sync_tx: mpsc::Sender<oneshot::Sender<SyncResult>>,
    /// Ready-to-serve data-plane router used by authenticated console smoke tests.
    /// Requests sent through this router still pass the router's bearer-token validation
    /// and routing logic without exposing the bearer secret to the console client.
    // 数据面路由器（供控制台冒烟测试使用，仍会走 bearer token 校验）
    pub data_plane: Router,
    // 节点部署状态（crate 内可见）
    pub(crate) node_deployment: Arc<node_deployment::NodeDeploymentState>,
}

// 创建应用的便捷入口（使用默认的 NodeRequestState）
// #[tracing::instrument] 自动为函数生成 tracing span；skip(db) 表示不记录 db（不可 Debug/敏感）
#[tracing::instrument(skip(db))]
pub async fn create_app(
    db: Arc<Database>,
    enable_liveview: bool,
    jwt_secret: JwtSecret,
    internal_secret: InternalSecret,
) -> anyhow::Result<Router> {
    // 委托给带自定义 NodeRequestState 的版本
    create_app_with_node_request_state(
        db,
        enable_liveview,
        jwt_secret,
        internal_secret,
        node_request::NodeRequestState::default(),
    )
    .await
}

// 创建应用的核心实现（允许注入 NodeRequestState，便于测试）
#[tracing::instrument(skip(db, node_request_state))]
pub async fn create_app_with_node_request_state(
    db: Arc<Database>,
    enable_liveview: bool,
    jwt_secret: JwtSecret,
    internal_secret: InternalSecret,
    node_request_state: node_request::NodeRequestState,
) -> anyhow::Result<Router> {
    // 创建系统监控服务
    let monitor = Arc::new(SystemMonitorService::new());
    // Auto collection is auxiliary to serving traffic. Keep startup available
    // if it cannot start, but make the degraded observability state explicit.
    // 自动采集失败不影响主流程，只打 warn 日志
    if let Err(error) = monitor.start_auto_update().await {
        tracing::warn!(%error, "system monitor auto-update failed to start");
    }

    // Keep the Node runtime and its request-visible status alive with the server.
    // 启动节点运行时（返回 NodeContext，持有运行时句柄，需随服务器存活）
    let node_context = NodeRuntime::new().start();

    // 创建部署事件存储
    let deployment_events = Arc::new(node_deployment::DeploymentEventStore::default());
    // 转为 trait 对象（供 DeploymentService 使用）
    let event_publisher: Arc<dyn DeploymentEventPublisher> = deployment_events.clone();
    // 创建本地进程管理器（with_kill_on_drop：Drop 时杀进程，防止泄漏）
    let processes = Arc::new(LocalProcessManager::with_kill_on_drop());
    // 创建 HTTP 探测器（reqwest client）
    let probe = Arc::new(HttpProbe::default());
    // 创建部署服务：参数为进程管理器、两个探测器（readiness + health）、事件发布者
    // 返回服务实例和错误接收端
    let (deployment_service, mut monitor_errors) =
        DeploymentService::new(processes, probe.clone(), probe, event_publisher);
    // 后台任务：持续消费监控错误并记录 error 日志
    tokio::spawn(async move {
        while let Some(error) = monitor_errors.recv().await {
            tracing::error!(%error, "local model runtime monitor failed");
        }
    });

    // 创建节点部署状态门面（服务 + 运行时准备器 + 事件存储）
    let node_deployment = Arc::new(node_deployment::NodeDeploymentState::new(
        Arc::new(deployment_service),
        Arc::new(node_runtime::LlamaCppRuntimePreparer),
        deployment_events,
    ));

    // 克隆 route_miss 状态（供闭包捕获）
    let route_miss_state = node_request_state.clone();

    // 3. Data Plane Router (Fallback) — must be created first to get force_sync_tx
    // 创建数据面路由（返回主路由、内部路由、强制同步通道发送端）
    // route_miss 闭包：当路由未命中时，调用 route_miss_response 返回响应
    let (router_app, internal_app, force_sync_tx) = create_router_app_with_route_miss(
        db.clone(),
        jwt_secret.clone(),
        Arc::new(move |model| route_miss_state.route_miss_response(model)),
    )
    .await?;

    // 组装应用全局状态
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
        node_deployment,
    };

    // 1. Management API Router
    // 管理 API 路由（即前面分析的 api::routes，包含 public/protected/admin）
    let api_router = api::routes(state.clone());

    // Top-level liveness probe (unauthenticated, used by deploy validators and
    // external uptime monitors). The richer report lives at
    // `/console/internal/health`.
    // Internal routes (health, reload, price-sync) must be registered BEFORE
    // LiveView's catch-all `/console/{*path}` so they return JSON instead of
    // the SPA HTML shell.
    // 构建顶层路由：
    // - /health：无认证存活探针（供部署校验器/外部监控使用）
    // - 合并管理 API
    // - 合并内部路由（必须在 LiveView catch-all 之前注册，避免被 SPA HTML 吞掉）
    let mut app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(api_router)
        .merge(internal_app);

    // 若启用 LiveView，则合并前端路由
    if enable_liveview {
        // 2. LiveView Router
        let liveview_router = burncloud_client::liveview_router(db.clone());
        app = app.merge(liveview_router);
    }

    // Note: If LiveView is disabled, "/" requests will hit the fallback (router_app)
    // which usually returns 404 for unknown paths, or handles LLM requests.

    // 定义 x-request-id 头
    let x_request_id = HeaderName::from_static("x-request-id");

    // 叠加各层中间件（注意 axum 中后叠加的先执行，即"洋葱模型"）
    let mut app = app
        // 兜底服务：未匹配的请求交给数据面路由（处理 LLM 请求或返回 404）
        .fallback_service(router_app)
        // 为每个请求生成 UUID 并写入 x-request-id 请求头
        .layer(SetRequestIdLayer::new(
            x_request_id.clone(),
            MakeRequestUuid,
        ))
        // 把 x-request-id 传播到响应头
        .layer(PropagateRequestIdLayer::new(x_request_id.clone()))
        // HTTP 追踪层（记录请求方法、路径、状态码、耗时）
        .layer(TraceLayer::new_for_http())
        // CORS 全放开（permissive：允许所有来源/方法/头）
        .layer(CorsLayer::permissive())
        // Security boundary is intentionally global so it protects both the
        // explicitly merged internal routes and the data-plane fallback.
        // 安全边界中间件：全局生效，同时保护内部路由和数据面 fallback
        .layer(middleware::from_fn_with_state(
            state,
            api::auth::security_boundary_middleware,
        ));

    // #567 human black-box harness. It is deliberately merged after the
    // production security stack because it is not a product API and must be
    // reachable by simple local curl commands. The explicit env gate is the
    // boundary: without it these routes do not exist at all.
    // 若环境变量开启 node_test，则挂载测试路由（在生产安全层之后 merge，
    // 但环境变量是唯一的开关边界；未开启时这些路由根本不存在）
    if node_test::enabled_from_env() {
        tracing::warn!(
            "BURNCLOUD_NODE_TEST_API is enabled; mounting test-only Node routes on the existing server"
        );
        app = app.merge(node_test::router());
    }

    // 返回最终路由
    Ok(app)
}

// 启动服务器（读取环境变量、初始化数据库、构建应用、监听端口）
// skip_all：不记录任何参数（含 host/port 之外的敏感信息）
#[tracing::instrument(skip_all)]
pub async fn start_server(host: &str, port: u16, enable_liveview: bool) -> anyhow::Result<()> {
    // 读取 node_test 开关
    let node_test_api_enabled = node_test::enabled_from_env();
    // 校验监听 host 是否安全（例如启用测试 API 时禁止绑定 0.0.0.0 等）
    node_test::validate_bind_host(host, node_test_api_enabled)?;

    // 读取 JWT_SECRET 环境变量（缺失报错）
    let jwt_secret = std::env::var("JWT_SECRET")
        .map_err(|_| anyhow::anyhow!("JWT_SECRET is missing or empty"))?;
    // 校验 JWT 密钥合法性
    let jwt_secret = JwtSecret::new(jwt_secret)?;
    // 读取内部密钥环境变量
    let internal_secret = std::env::var("BURNCLOUD_INTERNAL_SECRET")
        .map_err(|_| anyhow::anyhow!("BURNCLOUD_INTERNAL_SECRET is missing or empty"))?;
    // 校验内部密钥
    let internal_secret = InternalSecret::new(internal_secret)?;

    // 创建默认数据库连接
    let db = create_default_database().await?;
    // 初始化路由数据库 schema
    RouterDatabase::init(&db).await?;
    // 初始化用户数据库 schema
    UserDatabase::init(&db).await?;
    // Domain-owned default records (#842). Platform only creates schema and historic
    // fixups; Identity owns the demo account plus demo API key and Supply owns the default
    // protocol configs, so the application bootstrap seeds them here, in the order the
    // records depend on: account before the key that references it. Both owner calls are
    // idempotent, so a restart over an existing database inserts nothing.
    // seed 演示账号（幂等，重复启动不会重复插入）
    UserDatabase::seed_demo_defaults(&db).await?;
    // seed 默认协议配置（幂等）
    ChannelProtocolConfigModel::seed_default_protocol_configs(&db).await?;
    // 包成 Arc
    let db = Arc::new(db);

    // 构建应用
    let app = create_app(db, enable_liveview, jwt_secret, internal_secret).await?;

    // 解析监听地址
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    // 打印启动日志
    tracing::info!("Unified Gateway listening on {}", addr);
    if enable_liveview {
        tracing::info!("- Dashboard: http://{}:{}/", host, port);
    }
    tracing::info!("- LLM API:   http://{}:{}/v1/...", host, port);

    // 绑定 TCP 监听器
    let listener = tokio::net::TcpListener::bind(addr).await?;
    // 启动 axum 服务
    axum::serve(listener, app).await?;

    Ok(())
}

// 测试模块
#[cfg(test)]
mod tests {
    // 引入被测类型
    use super::InternalSecret;

    // 测试：空值和非法值应被拒绝
    #[test]
    fn internal_secret_rejects_empty_and_invalid_values() {
        // 空串、纯空白、换行、含换行的值都应报错
        for value in ["", "   ", "\n\t", "secret\nheader"] {
            assert!(InternalSecret::new(value).is_err());
        }
    }

    // 测试：Debug 输出应被脱敏
    #[test]
    fn internal_secret_debug_is_redacted() {
        // 明文不应出现在 Debug 输出中
        let plaintext = "must-never-appear";
        let secret = InternalSecret::new(plaintext)
            .unwrap_or_else(|e| panic!("test internal secret must be valid: {e}"));
        let rendered = format!("{secret:?}");

        assert!(!rendered.contains(plaintext));
        assert_eq!(rendered, "InternalSecret([REDACTED])");
    }
}
