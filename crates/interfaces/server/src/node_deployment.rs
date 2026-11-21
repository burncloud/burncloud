// 引入 async_trait 宏：让 trait 中可以使用 async fn（稳定版 Rust 的限制）
use async_trait::async_trait;

// 从 burncloud_node_runtime crate 引入部署相关类型
use burncloud_node_runtime::{
    // deployment 模块下的类型：
    deployment::{
        // 部署错误类型
        DeploymentError,
        // 部署事件（包含 model_id、kind、state、endpoint、pid、error 等字段）
        DeploymentEvent,
        // 部署事件种类枚举（Progress、InitialResult、StateChanged、Heartbeat、Stopped、DeploymentFailed）
        DeploymentEventKind,
        // 发布事件失败时的错误类型
        DeploymentEventPublishError,
        // 发布事件的 trait（本文件要实现它）
        DeploymentEventPublisher,
        // 部署服务：负责实际的部署编排逻辑
        DeploymentService,
    },
    // 工件验证状态枚举（如 Verified）
    ArtifactVerificationStatus,
    // 已准备好的工件（模型文件路径 + 验证状态）
    PreparedArtifact,
    // 运行时适配器 trait（用于与具体推理运行时交互，如 llama.cpp）
    RuntimeAdapter,
    // 运行时准备器 trait（负责准备/下载/校验运行时二进制）
    RuntimePreparer,
    // 运行时请求（指定运行时名称和版本）
    RuntimeRequest,
};

// 引入 serde 的 Serialize trait，用于把快照序列化为 JSON 返回给前端
use burncloud_router::local_attachment::{
    ExistingRouterLocalAttacher, LocalRouteAttacher, LocalRouteAttachment, LocalRouteAttachmentId,
    LocalRouteCapacity,
};

use serde::Serialize;

// 引入标准库工具
use std::{
    // HashMap：用于按 model_id 索引快照；VecDeque：用于保存有限长度的事件队列
    collections::{HashMap, VecDeque},
    // PathBuf：文件系统路径类型
    path::PathBuf,
    sync::Arc,
    // 时间相关：SystemTime、UNIX_EPOCH 用于生成 Unix 毫秒时间戳
    time::{SystemTime, UNIX_EPOCH},
};

// 引入 tokio 的异步读写锁 RwLock（读多写少场景，允许多个读者并发）
use tokio::sync::RwLock;

// 定义单条部署事件记录（对外序列化给前端展示）
// Debug：调试打印；Clone：可克隆；Serialize：可序列化为 JSON；Default：可默认构造
#[derive(Debug, Clone, Serialize, Default)]
pub(crate) struct DeploymentEventRecord {
    // 事件类型名称（如 "progress"、"state_changed"、"heartbeat"）
    pub kind: String,
    // 部署阶段（如 "Downloading"），仅 Progress 事件有值
    pub stage: Option<String>,
    // 节点状态（如 "ready"、"unhealthy"），部分事件有值
    pub state: Option<String>,
    // 本地推理服务端点（如 "http://127.0.0.1:8080"）
    pub endpoint: Option<String>,
    // 推理进程 PID
    pub pid: Option<u32>,
    // 错误信息（部署失败或清理错误时）
    pub error: Option<String>,
    // 事件发生的时间戳（Unix 毫秒）
    pub occurred_at_unix_ms: u64,
    pub generation: u64,
    pub capacity: Option<CapacitySnapshot>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub(crate) struct CapacitySnapshot {
    pub total_slots: u32,
    pub free_slots: u32,
    pub observed_at_unix_ms: u64,
}

struct RouteBinding {
    id: LocalRouteAttachmentId,
    generation: u64,
    endpoint: String,
    quarantined: bool,
}

// 定义部署快照（某 model_id 的当前完整状态，供查询接口返回）
#[derive(Debug, Clone, Serialize, Default)]
pub(crate) struct DeploymentSnapshot {
    // 模型 ID
    pub model_id: String,
    // 整体状态（"queued"、"deploying"、"ready"、"failed"、"stopped" 等）
    pub status: String,
    // 当前阶段
    pub stage: Option<String>,
    // 当前节点状态
    pub state: Option<String>,
    // 当前端点
    pub endpoint: Option<String>,
    // 进程 PID
    pub pid: Option<u32>,
    // 最后一次心跳的 Unix 毫秒时间戳
    pub last_heartbeat_unix_ms: Option<u64>,
    // 最近一次错误信息
    pub error: Option<String>,
    pub route_status: String,
    pub route_attachment_id: Option<i32>,
    pub generation: u64,
    pub capacity: Option<CapacitySnapshot>,
    // 事件历史队列（最多 128 条，见 push_event）
    pub events: VecDeque<DeploymentEventRecord>,
}

// 定义部署事件存储（内存中的"数据库"，按 model_id 索引快照）
// Debug：调试；Default：默认构造（空 HashMap）
#[derive(Default)]
pub(crate) struct DeploymentEventStore {
    snapshots: RwLock<HashMap<String, DeploymentSnapshot>>,
    attacher: Option<Arc<ExistingRouterLocalAttacher>>,
    routes: RwLock<HashMap<String, RouteBinding>>,
}

// 为 DeploymentEventStore 实现方法
impl DeploymentEventStore {
    pub(crate) fn with_attacher(attacher: Arc<ExistingRouterLocalAttacher>) -> Self {
        Self {
            snapshots: RwLock::new(HashMap::new()),
            attacher: Some(attacher),
            routes: RwLock::new(HashMap::new()),
        }
    }

    // 插入一个 "queued" 状态的初始快照（部署刚入队时调用）
    pub(crate) async fn insert_queued(&self, model_id: &str) {
        // 获取写锁（await 因为 tokio RwLock 是异步的）
        let mut snapshots = self.snapshots.write().await;
        // 每次重新部署都必须建立新的快照边界，避免旧终态的 endpoint
        // 被新代次的 Starting 事件误判为同代次端点冲突。
        snapshots.insert(
            model_id.to_string(),
            DeploymentSnapshot {
                model_id: model_id.to_string(),
                status: "queued".to_string(),
                route_status: "unroutable".to_string(),
                ..DeploymentSnapshot::default()
            },
        );
    }

    // 读取某个 model_id 的快照
    pub(crate) async fn get(&self, model_id: &str) -> Option<DeploymentSnapshot> {
        // 获取读锁（多个读者可并发）
        // .get() 返回 Option<&DeploymentSnapshot>
        // .cloned() 克隆成拥有所有权的快照返回（避免锁释放后引用失效）
        let mut snapshot = self.snapshots.read().await.get(model_id).cloned()?;
        if matches!(
            snapshot.route_status.as_str(),
            "routable" | "saturated" | "unroutable"
        ) && snapshot
            .last_heartbeat_unix_ms
            .is_some_and(|heartbeat| now_unix_ms().saturating_sub(heartbeat) >= 30_000)
        {
            snapshot.route_status = "lost".to_string();
        }
        Some(snapshot)
    }

    // 将某个部署标记为失败，并追加一条错误事件
    pub(crate) async fn set_error(&self, model_id: &str, message: String) {
        self.cleanup_route(model_id, "deployment error").await;
        // 获取写锁
        let mut snapshots = self.snapshots.write().await;
        // entry API：不存在则用 Default 构造（status 为空串等），存在则取出可变引用
        let snapshot = snapshots.entry(model_id.to_string()).or_default();
        // 补齐 model_id（Default 构造时为空）
        snapshot.model_id = model_id.to_string();
        // 状态置为 failed
        snapshot.status = "failed".to_string();
        snapshot.route_status = "failed".to_string();
        // 记录错误信息（克隆一份，因为下面事件里还要用 message）
        snapshot.error = Some(message.clone());
        // 向快照追加一条 deployment_error 事件
        push_event(
            snapshot,
            DeploymentEventRecord {
                // 事件类型
                kind: "deployment_error".to_string(),
                // 携带错误信息
                error: Some(message),
                // 记录当前时间
                occurred_at_unix_ms: now_unix_ms(),
                // 其余字段默认
                ..DeploymentEventRecord::default()
            },
        );
    }

    // 将某个部署标记为已停止
    pub(crate) async fn set_stopped(&self, model_id: &str) {
        self.cleanup_route(model_id, "deployment stopped").await;
        // 获取写锁
        let mut snapshots = self.snapshots.write().await;
        // 若该 model_id 存在，取出可变引用
        if let Some(snapshot) = snapshots.get_mut(model_id) {
            // 状态置为 stopped
            snapshot.status = "stopped".to_string();
            snapshot.route_status = "detached".to_string();
            snapshot.route_attachment_id = None;
            // 节点状态置为 "Absent"（表示进程已不存在）
            snapshot.state = Some("absent".to_string());
        }
    }

    async fn cleanup_route(&self, model_id: &str, reason: &str) {
        let Some(binding) = self.routes.write().await.remove(model_id) else {
            return;
        };
        let Some(attacher) = &self.attacher else {
            return;
        };
        if let Err(error) = attacher.quarantine(binding.id).await {
            tracing::error!(%error, model = %model_id, attachment_id = binding.id.0, reason, "failed to quarantine local route during cleanup");
        }
        if let Err(error) = attacher.detach(binding.id).await {
            tracing::error!(%error, model = %model_id, attachment_id = binding.id.0, reason, "failed to detach local route during cleanup");
        }
    }
    async fn route_view(&self, event: &DeploymentEvent) -> (String, Option<i32>) {
        match event.kind {
            DeploymentEventKind::Progress(
                burncloud_node_runtime::deployment::DeploymentStage::StoppingOldRuntime,
            ) => {
                let routes = self.routes.read().await;
                return routes
                    .get(&event.model_id)
                    .map_or(("detaching".to_string(), None), |binding| {
                        ("detaching".to_string(), Some(binding.id.0))
                    });
            }
            DeploymentEventKind::Stopped => return ("detached".to_string(), None),
            DeploymentEventKind::DeploymentFailed
            | DeploymentEventKind::StateChanged(burncloud_node_runtime::NodeState::Failed) => {
                return ("failed".to_string(), None);
            }
            DeploymentEventKind::Heartbeat(burncloud_node_runtime::NodeState::Unhealthy)
            | DeploymentEventKind::StateChanged(burncloud_node_runtime::NodeState::Unhealthy) => {
                let routes = self.routes.read().await;
                return routes
                    .get(&event.model_id)
                    .map_or(("quarantined".to_string(), None), |binding| {
                        ("quarantined".to_string(), Some(binding.id.0))
                    });
            }
            _ => {}
        }
        let routes = self.routes.read().await;
        let Some(binding) = routes.get(&event.model_id) else {
            return ("unroutable".to_string(), None);
        };
        if binding.quarantined {
            return ("quarantined".to_string(), Some(binding.id.0));
        }
        if event.generation != 0
            && (binding.generation != event.generation
                || event.local_endpoint.as_deref() != Some(binding.endpoint.as_str()))
        {
            return ("unroutable".to_string(), None);
        }
        let status = match event.capacity {
            Some(capacity) if capacity.free_slots > 0 => "routable",
            Some(_) => "saturated",
            None => "unroutable",
        };
        (status.to_string(), Some(binding.id.0))
    }

    async fn is_stale_event(&self, event: &DeploymentEvent) -> bool {
        if event.generation == 0 {
            return false;
        }
        let snapshots = self.snapshots.read().await;
        let Some(snapshot) = snapshots.get(&event.model_id) else {
            return false;
        };
        if snapshot.generation > event.generation {
            return true;
        }
        if event.generation != 0
            && event.generation <= snapshot.generation
            && matches!(snapshot.status.as_str(), "stopped" | "failed")
        {
            // 终态快照是旧运行代次的墓碑；普通旧心跳不能重新激活它。
            return true;
        }
        snapshot.generation == event.generation
            && snapshot.endpoint.is_some()
            && event.local_endpoint.as_ref() != snapshot.endpoint.as_ref()
    }

    #[expect(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "路由生命周期事件必须集中保证先隔离、再清理、后替换的顺序"
    )]
    async fn apply_route_event(&self, event: &DeploymentEvent) {
        let Some(attacher) = &self.attacher else {
            return;
        };
        let model_id = &event.model_id;
        match event.kind {
            DeploymentEventKind::InitialResult(burncloud_node_runtime::NodeState::Ready)
            | DeploymentEventKind::Heartbeat(burncloud_node_runtime::NodeState::Ready) => {
                let Some(endpoint) = event.local_endpoint.as_deref() else {
                    return;
                };
                let Some(generation) = (event.generation != 0).then_some(event.generation) else {
                    return;
                };
                let capacity = event.capacity.map(local_capacity);
                let mut routes = self.routes.write().await;
                let matches_current = routes.get(model_id).is_some_and(|binding| {
                    binding.generation == generation && binding.endpoint == endpoint
                });
                if capacity.is_none() {
                    // 新代次已经接管模型时，旧挂载不能因为容量探测失败而残留；
                    // 当前代次则只记录容量未知，保持已启动但不可派发。
                    if !matches_current {
                        if let Some(old) = routes.remove(model_id) {
                            if let Err(error) = attacher.quarantine(old.id).await {
                                tracing::error!(%error, model = %model_id, attachment_id = old.id.0, "failed to quarantine replaced local route");
                            }
                            if let Err(error) = attacher.detach(old.id).await {
                                tracing::error!(%error, model = %model_id, attachment_id = old.id.0, "failed to detach replaced local route");
                            }
                        }
                    } else if let Some(binding) = routes.get(model_id) {
                        if let Err(error) =
                            attacher.record_heartbeat(binding.id, generation, endpoint, true, None)
                        {
                            tracing::warn!(%error, model = %model_id, "local route heartbeat rejected");
                            let id = binding.id;
                            if let Some(current) = routes.get_mut(model_id) {
                                current.quarantined = true;
                            }
                            if let Err(error) = attacher.quarantine(id).await {
                                tracing::error!(%error, model = %model_id, attachment_id = id.0, "failed to quarantine rejected local route heartbeat");
                            }
                        }
                    }
                    return;
                }
                if !matches_current {
                    if let Some(old) = routes.remove(model_id) {
                        if let Err(error) = attacher.quarantine(old.id).await {
                            tracing::error!(%error, model = %model_id, attachment_id = old.id.0, "failed to quarantine replaced local route");
                        }
                        if let Err(error) = attacher.detach(old.id).await {
                            tracing::error!(%error, model = %model_id, attachment_id = old.id.0, "failed to detach replaced local route");
                        }
                    }
                    match attacher
                        .attach_with_runtime(
                            LocalRouteAttachment {
                                model: model_id.clone(),
                                base_url: endpoint.to_string(),
                            },
                            generation,
                            capacity,
                        )
                        .await
                    {
                        Ok(id) => {
                            routes.insert(
                                model_id.clone(),
                                RouteBinding {
                                    id,
                                    generation,
                                    endpoint: endpoint.to_string(),
                                    quarantined: false,
                                },
                            );
                        }
                        Err(error) => {
                            tracing::warn!(%error, model = %model_id, "local route attachment is not routable yet");
                        }
                    }
                } else if let Some(binding) = routes.get(model_id) {
                    if let Err(error) =
                        attacher.record_heartbeat(binding.id, generation, endpoint, true, capacity)
                    {
                        tracing::warn!(%error, model = %model_id, "local route heartbeat rejected");
                        let id = binding.id;
                        if let Some(current) = routes.get_mut(model_id) {
                            current.quarantined = true;
                        }
                        if let Err(error) = attacher.quarantine(id).await {
                            tracing::error!(%error, model = %model_id, attachment_id = id.0, "failed to quarantine rejected local route heartbeat");
                        }
                    }
                }
            }
            DeploymentEventKind::Progress(
                burncloud_node_runtime::deployment::DeploymentStage::StoppingOldRuntime,
            )
            | DeploymentEventKind::Heartbeat(burncloud_node_runtime::NodeState::Unhealthy)
            | DeploymentEventKind::StateChanged(burncloud_node_runtime::NodeState::Unhealthy) => {
                let binding = {
                    let mut routes = self.routes.write().await;
                    let Some(binding) = routes.get_mut(model_id) else {
                        return;
                    };
                    binding.quarantined = true;
                    binding.id
                };
                if let Err(error) = attacher.quarantine(binding).await {
                    tracing::error!(%error, model = %model_id, attachment_id = binding.0, "failed to quarantine local route");
                }
            }
            DeploymentEventKind::DeploymentFailed
            | DeploymentEventKind::StateChanged(burncloud_node_runtime::NodeState::Failed)
            | DeploymentEventKind::Stopped => {
                if let Some(binding) = self.routes.write().await.remove(model_id) {
                    if let Err(error) = attacher.quarantine(binding.id).await {
                        tracing::error!(%error, model = %model_id, attachment_id = binding.id.0, "failed to quarantine stopped local route");
                    }
                    if let Err(error) = attacher.detach(binding.id).await {
                        tracing::error!(%error, model = %model_id, attachment_id = binding.id.0, "failed to detach stopped local route");
                    }
                }
            }
            _ => {}
        }
    }
}

// 用 async_trait 宏实现 DeploymentEventPublisher trait
#[async_trait]
impl DeploymentEventPublisher for DeploymentEventStore {
    // 发布一个部署事件：更新快照 + 追加事件记录
    async fn publish(&self, event: DeploymentEvent) -> Result<(), DeploymentEventPublishError> {
        if self.is_stale_event(&event).await {
            tracing::warn!(
                model = %event.model_id,
                generation = event.generation,
                endpoint = ?event.local_endpoint,
                "ignored stale local deployment event"
            );
            return Ok(());
        }
        // 根据事件种类推导三元组 (kind 字符串, stage 可选, status 字符串)
        let event_kind = event.kind;
        let (kind, stage, status) = match event_kind {
            // 进度事件：阶段名用 Debug 格式化为字符串，状态视为 deploying
            DeploymentEventKind::Progress(stage) => {
                ("progress", Some(format!("{stage:?}")), "deploying")
            }
            // 初始结果事件：无 stage，状态由 state_name 转成字符串
            DeploymentEventKind::InitialResult(state) => {
                ("initial_result", None, state_name(state))
            }
            // 状态变化事件
            DeploymentEventKind::StateChanged(state) => ("state_changed", None, state_name(state)),
            // 心跳事件
            DeploymentEventKind::Heartbeat(state) => ("heartbeat", None, state_name(state)),
            // 停止事件：状态直接为 "stopped"
            DeploymentEventKind::Stopped => ("stopped", None, "stopped"),
            // 部署失败事件：状态为 "failed"
            DeploymentEventKind::DeploymentFailed => ("deployment_failed", None, "failed"),
        };

        self.apply_route_event(&event).await;
        let (route_status, route_attachment_id) = self.route_view(&event).await;

        // 记录当前时间戳（毫秒）
        let now = now_unix_ms();
        // 构造要追加到事件队列的记录
        let record = DeploymentEventRecord {
            // kind 转 String
            kind: kind.to_string(),
            // stage 克隆（后面还要用 stage 更新 snapshot）
            stage: stage.clone(),
            // 把 event.state 转成 Debug 字符串；event.state 是 Option<NodeState>
            state: event.state.map(|state| state_name(state).to_string()),
            // 克隆端点字符串
            endpoint: event.local_endpoint.clone(),
            // 复制 PID（Option<u32> 是 Copy）
            pid: event.pid,
            // 优先用 event.error，若为 None 则回退到 cleanup_error
            error: event.error.clone().or(event.cleanup_error.clone()),
            // 记录时间
            occurred_at_unix_ms: now,
            generation: event.generation,
            capacity: event.capacity.map(capacity_snapshot),
        };

        // 获取写锁
        let mut snapshots = self.snapshots.write().await;
        // 取出（或创建默认）快照
        let snapshot = snapshots.entry(event.model_id.clone()).or_default();
        // 更新 model_id（把所有权移入 snapshot）
        snapshot.model_id = event.model_id;
        // 更新整体状态
        snapshot.status = status.to_string();
        snapshot.route_status = route_status;
        snapshot.route_attachment_id = route_attachment_id;
        // 若本次事件带 stage，则更新快照的 stage
        if stage.is_some() {
            snapshot.stage = stage;
        }
        // 若本次事件带 state，则更新快照的稳定小写状态名
        if let Some(state) = event.state {
            snapshot.state = Some(state_name(state).to_string());
        }
        // 若本次事件带 endpoint，则更新
        if let Some(endpoint) = event.local_endpoint {
            snapshot.endpoint = Some(endpoint);
        }
        // 若本次事件带 pid，则更新
        if let Some(pid) = event.pid {
            snapshot.pid = Some(pid);
        }
        if event.generation != 0 {
            snapshot.generation = event.generation;
        }
        if let Some(capacity) = event.capacity {
            snapshot.capacity = Some(capacity_snapshot(capacity));
        }
        // 首次健康结果也代表一次有效运行证据，避免尚未收到第一个周期心跳时无法计算 TTL。
        if matches!(
            event.kind,
            DeploymentEventKind::InitialResult(burncloud_node_runtime::NodeState::Ready)
                | DeploymentEventKind::Heartbeat(_)
        ) {
            snapshot.last_heartbeat_unix_ms = Some(now);
        }
        // 若是停止事件，清空 endpoint 和 pid（进程已不存在）
        if matches!(event.kind, DeploymentEventKind::Stopped) {
            snapshot.endpoint = None;
            snapshot.pid = None;
            snapshot.route_status = "detached".to_string();
            snapshot.route_attachment_id = None;
        }
        // 若本次事件带错误信息，则更新快照的 error
        if event.error.is_some() {
            snapshot.error = event.error;
        }
        // 把事件记录追加到快照的事件队列
        push_event(snapshot, record);
        // 发布成功
        Ok(())
    }
}

fn capacity_snapshot(
    capacity: burncloud_node_runtime::deployment::DeploymentCapacity,
) -> CapacitySnapshot {
    CapacitySnapshot {
        total_slots: capacity.total_slots,
        free_slots: capacity.free_slots,
        observed_at_unix_ms: capacity.observed_at_unix_ms,
    }
}

fn local_capacity(
    capacity: burncloud_node_runtime::deployment::DeploymentCapacity,
) -> LocalRouteCapacity {
    LocalRouteCapacity {
        total_slots: capacity.total_slots,
        free_slots: capacity.free_slots,
        observed_at_unix_ms: capacity.observed_at_unix_ms,
    }
}

// 定义节点部署状态（供 HTTP 层使用的门面结构）
pub(crate) struct NodeDeploymentState {
    // 部署服务（Arc 共享，多任务可并发访问）
    service: std::sync::Arc<DeploymentService>,
    // 运行时准备器（trait 对象，Arc 共享）
    runtime_preparer: std::sync::Arc<dyn RuntimePreparer>,
    // 事件存储（Arc 共享）
    events: std::sync::Arc<DeploymentEventStore>,
}

// 为 NodeDeploymentState 实现方法
impl NodeDeploymentState {
    // 构造函数
    pub(crate) fn new(
        service: std::sync::Arc<DeploymentService>,
        runtime_preparer: std::sync::Arc<dyn RuntimePreparer>,
        events: std::sync::Arc<DeploymentEventStore>,
    ) -> Self {
        Self {
            service,
            runtime_preparer,
            events,
        }
    }

    // 启动本地部署：入参是模型文件路径，返回部署快照
    pub(crate) async fn start_local(
        &self,
        model_path: PathBuf,
    ) -> Result<DeploymentSnapshot, String> {
        // 从路径中提取文件名主干（不含扩展名）作为 model_id
        let model_id = model_path
            .file_stem()
            // 转成 &str（要求文件名是合法 UTF-8）
            .and_then(|stem| stem.to_str())
            // 过滤空字符串（避免文件名只有扩展名等边界情况）
            .filter(|stem| !stem.is_empty())
            // 若以上任一环节失败，返回错误
            .ok_or_else(|| "model_path must have a UTF-8 file name".to_string())?
            // 转成 String
            .to_string();
        // 当前接口接收已准备好的本地工件地址，部署层继续沿用 PreparedArtifact 契约。
        // 构造 PreparedArtifact：路径 + 已验证状态
        let artifact = PreparedArtifact::new(&model_path, ArtifactVerificationStatus::Verified)
            // 失败则把错误转字符串返回
            .map_err(|error| error.to_string())?;
        // 调用运行时准备器准备 llama.cpp 运行时
        let runtime = self
            .runtime_preparer
            .prepare(RuntimeRequest {
                // 指定运行时名称
                runtime: "llama.cpp".to_string(),
                // 版本为 None 表示使用默认/最新版本
                version: None,
            })
            .await
            // 失败则把错误转字符串返回
            .map_err(|error| error.to_string())?;

        // 在事件存储中插入 queued 初始快照
        self.events.insert_queued(&model_id).await;
        // 构造运行时适配器（Arc<dyn RuntimeAdapter>），使用 llama.cpp 适配器
        let adapter: std::sync::Arc<dyn RuntimeAdapter> = std::sync::Arc::new(
            crate::node_runtime::LlamaCppRuntimeAdapter::new(model_id.clone()),
        );
        // 调用部署服务执行部署
        match self
            .service
            .deploy_prepared(model_id.clone(), artifact, runtime, adapter)
            .await
        {
            // 部署成功：从事件存储读取最终快照返回
            Ok(_) => self
                .events
                .get(&model_id)
                .await
                .ok_or_else(|| "deployment completed without a status record".to_string()),
            // 部署失败：记录错误事件，然后把错误转字符串返回
            Err(error) => {
                self.events.set_error(&model_id, error.to_string()).await;
                Err(error.to_string())
            }
        }
    }

    // 停止部署
    pub(crate) async fn stop(&self, model_id: &str) -> Result<bool, DeploymentError> {
        // 调用服务停止模型，返回是否真的停止了
        let stopped = self.service.stop_model(model_id).await?;
        // 若确实停止了，更新事件存储状态为 stopped
        if stopped {
            self.events.set_stopped(model_id).await;
        }
        // 返回结果
        Ok(stopped)
    }

    // 查询部署状态
    pub(crate) async fn status(&self, model_id: &str) -> Option<DeploymentSnapshot> {
        // 直接从事件存储读取快照
        self.events.get(model_id).await
    }
}

// 向快照的事件队列追加一条事件，限制最大长度为 128
fn push_event(snapshot: &mut DeploymentSnapshot, event: DeploymentEventRecord) {
    // 若已达上限 128，弹出最旧的一条（FIFO）
    if snapshot.events.len() == 128 {
        snapshot.events.pop_front();
    }
    // 把新事件追加到队尾
    snapshot.events.push_back(event);
}

// 获取当前 Unix 毫秒时间戳
fn now_unix_ms() -> u64 {
    // SystemTime::now() 获取当前系统时间
    SystemTime::now()
        // 计算与 Unix 纪元的间隔
        .duration_since(UNIX_EPOCH)
        // 若系统时间早于 1970（异常）则返回 0
        // 否则把毫秒数（u128）截断为 u64（并防止溢出）
        .map_or(0, |duration| {
            duration.as_millis().min(u64::MAX as u128) as u64
        })
}

// 把 NodeState 枚举转成稳定字符串（用于展示/持久化）
fn state_name(state: burncloud_node_runtime::NodeState) -> &'static str {
    match state {
        burncloud_node_runtime::NodeState::Absent => "absent",
        burncloud_node_runtime::NodeState::Resolving => "resolving",
        burncloud_node_runtime::NodeState::LocalUnsupported => "local_unsupported",
        burncloud_node_runtime::NodeState::PreparingArtifact => "preparing_artifact",
        burncloud_node_runtime::NodeState::ArtifactReady => "artifact_ready",
        burncloud_node_runtime::NodeState::PreparingRuntime => "preparing_runtime",
        burncloud_node_runtime::NodeState::Starting => "starting",
        burncloud_node_runtime::NodeState::WaitingReady => "waiting_ready",
        burncloud_node_runtime::NodeState::Ready => "ready",
        burncloud_node_runtime::NodeState::Routable => "routable",
        burncloud_node_runtime::NodeState::Failed => "failed",
        burncloud_node_runtime::NodeState::Unhealthy => "unhealthy",
    }
}

#[cfg(test)]
#[path = "node_deployment_tests.rs"]
mod tests;
