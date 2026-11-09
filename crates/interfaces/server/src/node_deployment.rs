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
use serde::Serialize;

// 引入标准库工具
use std::{
    // HashMap：用于按 model_id 索引快照；VecDeque：用于保存有限长度的事件队列
    collections::{HashMap, VecDeque},
    // PathBuf：文件系统路径类型
    path::PathBuf,
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
    // 事件历史队列（最多 128 条，见 push_event）
    pub events: VecDeque<DeploymentEventRecord>,
}

// 定义部署事件存储（内存中的"数据库"，按 model_id 索引快照）
// Debug：调试；Default：默认构造（空 HashMap）
#[derive(Debug, Default)]
pub(crate) struct DeploymentEventStore {
    // RwLock 包裹的 HashMap：异步读写锁保护，允许多读单写
    snapshots: RwLock<HashMap<String, DeploymentSnapshot>>,
}

// 为 DeploymentEventStore 实现方法
impl DeploymentEventStore {
    // 插入一个 "queued" 状态的初始快照（部署刚入队时调用）
    pub(crate) async fn insert_queued(&self, model_id: &str) {
        // 获取写锁（await 因为 tokio RwLock 是异步的）
        let mut snapshots = self.snapshots.write().await;
        // entry API：若 key 不存在则用闭包构造新快照插入；若已存在则不动
        snapshots
            .entry(model_id.to_string())
            .or_insert_with(|| DeploymentSnapshot {
                // 设置 model_id
                model_id: model_id.to_string(),
                // 初始状态为 queued
                status: "queued".to_string(),
                // 其余字段用 Default 填充
                ..DeploymentSnapshot::default()
            });
    }

    // 读取某个 model_id 的快照
    pub(crate) async fn get(&self, model_id: &str) -> Option<DeploymentSnapshot> {
        // 获取读锁（多个读者可并发）
        // .get() 返回 Option<&DeploymentSnapshot>
        // .cloned() 克隆成拥有所有权的快照返回（避免锁释放后引用失效）
        self.snapshots.read().await.get(model_id).cloned()
    }

    // 将某个部署标记为失败，并追加一条错误事件
    pub(crate) async fn set_error(&self, model_id: &str, message: String) {
        // 获取写锁
        let mut snapshots = self.snapshots.write().await;
        // entry API：不存在则用 Default 构造（status 为空串等），存在则取出可变引用
        let snapshot = snapshots.entry(model_id.to_string()).or_default();
        // 补齐 model_id（Default 构造时为空）
        snapshot.model_id = model_id.to_string();
        // 状态置为 failed
        snapshot.status = "failed".to_string();
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
        // 获取写锁
        let mut snapshots = self.snapshots.write().await;
        // 若该 model_id 存在，取出可变引用
        if let Some(snapshot) = snapshots.get_mut(model_id) {
            // 状态置为 stopped
            snapshot.status = "stopped".to_string();
            // 节点状态置为 "Absent"（表示进程已不存在）
            snapshot.state = Some("Absent".to_string());
        }
    }
}

// 用 async_trait 宏实现 DeploymentEventPublisher trait
#[async_trait]
impl DeploymentEventPublisher for DeploymentEventStore {
    // 发布一个部署事件：更新快照 + 追加事件记录
    async fn publish(&self, event: DeploymentEvent) -> Result<(), DeploymentEventPublishError> {
        // 根据事件种类推导三元组 (kind 字符串, stage 可选, status 字符串)
        let (kind, stage, status) = match event.kind {
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

        // 记录当前时间戳（毫秒）
        let now = now_unix_ms();
        // 构造要追加到事件队列的记录
        let record = DeploymentEventRecord {
            // kind 转 String
            kind: kind.to_string(),
            // stage 克隆（后面还要用 stage 更新 snapshot）
            stage: stage.clone(),
            // 把 event.state 转成 Debug 字符串；event.state 是 Option<NodeState>
            state: event.state.map(|state| format!("{state:?}")),
            // 克隆端点字符串
            endpoint: event.local_endpoint.clone(),
            // 复制 PID（Option<u32> 是 Copy）
            pid: event.pid,
            // 优先用 event.error，若为 None 则回退到 cleanup_error
            error: event.error.clone().or(event.cleanup_error.clone()),
            // 记录时间
            occurred_at_unix_ms: now,
        };

        // 获取写锁
        let mut snapshots = self.snapshots.write().await;
        // 取出（或创建默认）快照
        let snapshot = snapshots.entry(event.model_id.clone()).or_default();
        // 更新 model_id（把所有权移入 snapshot）
        snapshot.model_id = event.model_id;
        // 更新整体状态
        snapshot.status = status.to_string();
        // 若本次事件带 stage，则更新快照的 stage
        if stage.is_some() {
            snapshot.stage = stage;
        }
        // 若本次事件带 state，则更新快照的 state（Debug 格式）
        if let Some(state) = event.state {
            snapshot.state = Some(format!("{state:?}"));
        }
        // 若本次事件带 endpoint，则更新
        if let Some(endpoint) = event.local_endpoint {
            snapshot.endpoint = Some(endpoint);
        }
        // 若本次事件带 pid，则更新
        if let Some(pid) = event.pid {
            snapshot.pid = Some(pid);
        }
        // 若是心跳事件，记录最后心跳时间
        if matches!(event.kind, DeploymentEventKind::Heartbeat(_)) {
            snapshot.last_heartbeat_unix_ms = Some(now);
        }
        // 若是停止事件，清空 endpoint 和 pid（进程已不存在）
        if matches!(event.kind, DeploymentEventKind::Stopped) {
            snapshot.endpoint = None;
            snapshot.pid = None;
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
        // 就绪
        burncloud_node_runtime::NodeState::Ready => "ready",
        // 不健康
        burncloud_node_runtime::NodeState::Unhealthy => "unhealthy",
        // 失败
        burncloud_node_runtime::NodeState::Failed => "failed",
        // 其他（non_exhaustive 或未来新增变体）统一返回 unknown
        _ => "unknown",
    }
}
