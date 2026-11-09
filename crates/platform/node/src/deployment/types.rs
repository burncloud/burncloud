// 导入 async_trait 宏：Rust 稳定版 trait 中不能直接写 async fn，
// 该宏会把 async fn 改写成返回 Pin<Box<dyn Future>> 的形式。
use async_trait::async_trait;

// 从 crate 根导入本模块需要的三个类型：
// - NodeState：节点生命周期状态（Ready / Unhealthy / Failed / Starting 等）
// - ProcessHandle：进程句柄（含 pid）
// - ProcessPlan：进程计划（含 ProcessSpec、ReadinessTarget、local_endpoint）
use crate::{NodeState, ProcessHandle, ProcessPlan};

// ============================================================================
// DeploymentStage：部署进度阶段
// ============================================================================

// 三斜线文档注释：说明该枚举的语义。
/// 部署服务发布给应用层的进度阶段，不替代模型运行状态。
///
/// # Examples
///
/// ```
/// use burncloud_node_runtime::deployment::DeploymentStage;
///
/// let stage = DeploymentStage::Queued;
/// assert!(matches!(stage, DeploymentStage::Queued));
/// ```
//
// 文档要点：
//   1. 这是“进度阶段”，描述部署推进到哪一步；
//   2. 它“不替代模型运行状态”——模型状态由 NodeState 表达，
//      DeploymentStage 只是阶段标签，不是生命周期状态；
//   3. doctest 验证 Queued 变体可构造、可 matches 匹配。

// 派生多个 trait：
// - Debug：便于打印/日志
// - Clone / Copy：变体无数据，Copy 廉价，方便按值传递
// - PartialEq / Eq：可比较，便于测试与去重
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// 公开枚举：对外可见
pub enum DeploymentStage {
    /// 请求已进入 FIFO 队列，尚未开始执行。
    Queued,
    /// 同一模型已有旧运行时，当前正在停止该旧进程。
    StoppingOldRuntime,
    /// 正在调用运行时适配器生成 `ProcessPlan`。
    Planning,
    /// 正在把计划交给 `ProcessManager::start` 启动子进程。
    Starting,
    /// 进程已启动，正在等待 `ReadinessProbe::wait_ready` 成功。
    WaitingReady,
    /// readiness 成功后正在执行首次 `HealthProbe::is_healthy` 检查。
    HealthChecking,
}

// ============================================================================
// DeploymentEventKind：事件种类
// ============================================================================

// 三斜线文档：说明事件种类的语义。
/// 描述部署阶段、首次健康结果、后台状态变化或部署失败。
///
/// 首次结果事件只由部署服务使用 `Ready` 或 `Unhealthy` 状态构造。
///
/// # Examples
///
/// ```
/// use burncloud_node_runtime::{deployment::DeploymentEventKind, NodeState};
///
/// let result = DeploymentEventKind::InitialResult(NodeState::Ready);
/// assert!(matches!(
///     result,
///     DeploymentEventKind::InitialResult(NodeState::Ready)
/// ));
/// ```
//
// 文档要点：
//   1. 四种事件：进度、首次结果、状态变化、部署失败；
//   2. 首次结果只允许 Ready / Unhealthy（不是 Starting/Failed 等其他状态）；
//   3. doctest 验证 InitialResult(Ready) 可构造可匹配。

// 派生：Debug / Clone / Copy（变体只含 Copy 的 NodeState/DeploymentStage）/ PartialEq / Eq。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// 公开枚举：对外可见
pub enum DeploymentEventKind {
    /// 发布一个部署阶段进度。
    Progress(DeploymentStage),
    /// 发布首次健康检查结果；状态只能是 `Ready` 或 `Unhealthy`。
    InitialResult(NodeState),
    /// 发布后台监控发现的状态变化。
    StateChanged(NodeState),
    /// 本次存活与健康探测均已完成。
    Heartbeat(NodeState),
    /// 已停止该模型对应的本地运行时。
    Stopped,
    /// 发布部署失败；详细错误位于 [`DeploymentEvent`] 的可选字段中。
    DeploymentFailed,
}

// ============================================================================
// DeploymentEvent：对外发布的事件结构
// ============================================================================

// 三斜线文档：说明事件结构的语义。
/// 部署服务传递给应用层的进度、模型状态或失败事件。
///
/// # Examples
///
/// ```
/// use burncloud_node_runtime::{
///     deployment::{DeploymentEvent, DeploymentEventKind, DeploymentStage},
///     NodeState,
/// };
///
/// let event = DeploymentEvent {
///     model_id: "model-1".to_string(),
///     kind: DeploymentEventKind::Progress(DeploymentStage::Queued),
///     state: None,
///     error_kind: None,
///     local_endpoint: None,
///     pid: None,
///     error: None,
///     cleanup_error: None,
/// };
/// assert_eq!(event.model_id, "model-1");
/// ```
//
// 文档要点：
//   1. 这是“统一事件结构”，所有事件（进度/结果/状态/失败）都用这一个结构承载；
//   2. 通过 kind 区分事件种类，其余字段是可选上下文；
//   3. doctest 展示如何构造一个 Queued 进度事件。

// 派生：Debug / Clone / PartialEq / Eq。
// 注意：不能 Copy（因为含 String）。
#[derive(Debug, Clone, PartialEq, Eq)]
// 公开结构体：对外可见
pub struct DeploymentEvent {
    /// 事件所属的模型标识。
    pub model_id: String,
    /// 事件的阶段、结果、状态变化或失败分类。
    pub kind: DeploymentEventKind,
    /// 事件发生时的 Node 状态；纯进度事件可以为空。
    pub state: Option<NodeState>,
    /// 稳定的部署错误分类；非失败事件为空。
    pub error_kind: Option<DeploymentErrorKind>,
    /// 已启动运行时的本地 endpoint；进程尚未启动时为空。
    pub local_endpoint: Option<String>,
    /// 已启动操作系统进程的真实 PID。
    pub pid: Option<u32>,
    /// 面向应用层的主错误消息。
    pub error: Option<String>,
    /// 进程清理失败时的附加诊断，不覆盖 `error`。
    pub cleanup_error: Option<String>,
}

// ============================================================================
// DeploymentEventPublishError：事件发布错误
// ============================================================================

// 三斜线文档：说明该错误的语义与重试契约。
/// 应用层未接收部署事件时返回的错误。
///
/// 返回此错误的发布器必须确认事件尚未被接收，调用方才能用同一事件安全重试。
///
/// # Examples
///
/// ```
/// use burncloud_node_runtime::deployment::DeploymentEventPublishError;
///
/// let error = DeploymentEventPublishError::PublishFailed("channel closed".to_string());
/// assert!(error.to_string().contains("channel closed"));
/// ```
//
// 文档要点：
//   1. 该错误表示“应用层没收到事件”；
//   2. 关键契约：发布器返回此错误时，必须保证事件未被接收，
//      这样调用方才能用同一事件安全重试（这正是 DeploymentService::publish_with_retry 的前提）；
//   3. doctest 验证 Display 包含原始消息。

// 派生：Debug / Clone / PartialEq / Eq / thiserror::Error（自动实现 Display 与 std::error::Error）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
// 公开枚举：对外可见
pub enum DeploymentEventPublishError {
    // 发布失败：携带原因字符串；#[error] 定义 Display 格式
    #[error("deployment event publish failed: {0}")]
    PublishFailed(String),
}

// ============================================================================
// DeploymentOutcome：部署成功结果
// ============================================================================

// 三斜线文档：说明结果的语义。
/// 首次健康探测完成后的部署结果。
///
/// `state` 只由部署服务设置为 `Ready` 或 `Unhealthy`。
///
/// # Examples
///
/// ```
/// use burncloud_node_runtime::{
///     deployment::DeploymentOutcome, NodeState, ProcessHandle, ProcessPlan, ProcessSpec,
///     ReadinessTarget,
/// };
///
/// let outcome = DeploymentOutcome {
///     plan: ProcessPlan {
///         process: ProcessSpec { program: "runtime".to_string(), args: vec![] },
///         readiness: ReadinessTarget { endpoint: "http://127.0.0.1:8080/ready".to_string() },
///         local_endpoint: "http://127.0.0.1:8080".to_string(),
///     },
///     process: ProcessHandle { pid: 42 },
///     state: NodeState::Ready,
/// };
/// assert_eq!(outcome.state, NodeState::Ready);
/// ```
//
// 文档要点：
//   1. 这是 deploy() 成功时的返回；
//   2. state 只可能是 Ready / Unhealthy（首次健康检查的两种结论）；
//   3. doctest 展示完整构造（ProcessPlan 含 ProcessSpec + ReadinessTarget + local_endpoint）。

// 派生：Debug / Clone / PartialEq / Eq。
#[derive(Debug, Clone, PartialEq, Eq)]
// 公开结构体：对外可见
pub struct DeploymentOutcome {
    /// 适配器实际生成并用于启动的进程计划。
    pub plan: ProcessPlan,
    /// 真实启动进程的句柄和 PID。
    pub process: ProcessHandle,
    /// 首次健康检查得出的状态，只能是 `Ready` 或 `Unhealthy`。
    pub state: NodeState,
}

// ============================================================================
// DeploymentError：部署错误
// ============================================================================

// 三斜线文档：说明错误的语义。
/// 部署调用返回的可克隆错误结果。
///
/// 清理或事件通知错误作为附加诊断保留，不覆盖原始错误分类。
///
/// # Examples
///
/// ```
/// use burncloud_node_runtime::deployment::{DeploymentError, DeploymentErrorKind};
///
/// let error = DeploymentError {
///     kind: DeploymentErrorKind::PlanFailed,
///     message: "no launch plan".to_string(),
///     cleanup_error: None,
///     notification_error: None,
///     preserved_state: None,
/// };
/// assert!(error.to_string().contains("no launch plan"));
/// ```
//
// 文档要点：
//   1. 该错误可 Clone（因为要在 complete_waiters 里广播给多个等待者）；
//   2. 关键设计：cleanup_error / notification_error 是“附加诊断”，
//      不覆盖原始 kind/message —— 原始错误分类始终保留；
//   3. doctest 验证 Display 包含原始 message。

// 派生：Debug / Clone / PartialEq / Eq / thiserror::Error。
// #[error("{kind:?}: {message}")]：Display 格式为 “Kind: message”。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{kind:?}: {message}")]
// 公开结构体：对外可见
pub struct DeploymentError {
    /// 原始、稳定的错误分类。
    pub kind: DeploymentErrorKind,
    /// 原始错误消息，供日志和用户界面展示。
    pub message: String,
    /// 进程清理失败时的附加诊断。
    pub cleanup_error: Option<String>,
    /// 失败事件发布失败时的附加诊断。
    pub notification_error: Option<String>,
    /// 替换失败时仍被保留的旧运行时状态。
    pub preserved_state: Option<NodeState>,
}

// ============================================================================
// DeploymentErrorKind：稳定错误分类
// ============================================================================

// 三斜线文档：说明该枚举的稳定性与用途。
/// 跨部署 API 与事件使用的稳定错误分类。
///
/// # Examples
///
/// ```
/// use burncloud_node_runtime::deployment::DeploymentErrorKind;
///
/// let kind = DeploymentErrorKind::ProcessExited;
/// assert!(matches!(kind, DeploymentErrorKind::ProcessExited));
/// ```
//
// 文档要点：
//   1. “稳定分类”意味着外部可以依赖这些变体做匹配/处理，不轻易变动；
//   2. 同时用于 DeploymentError 与 DeploymentEvent.error_kind，保持跨 API 一致。

// 派生：Debug / Clone / Copy（无数据）/ PartialEq / Eq。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// 公开枚举：对外可见
pub enum DeploymentErrorKind {
    /// 部署请求中的模型标识或模型地址无效。
    InvalidRequest,
    /// 运行时适配器无法生成启动计划。
    PlanFailed,
    /// 进程管理器无法启动计划中的程序。
    ProcessStartFailed,
    /// readiness 探测未能成功。
    ReadinessFailed,
    /// health 探测调用本身失败。
    HealthProbeFailed,
    /// 停止同一模型的旧运行时失败。
    OldRuntimeStopFailed,
    /// 进程存活查询调用失败。
    ProcessLivenessFailed,
    /// 进程存活查询确认进程已退出。
    ProcessExited,
    /// 应用层事件发布失败。
    EventPublishFailed,
}

// ============================================================================
// DeploymentMonitorError：后台监控错误
// ============================================================================

// 三斜线文档：说明该错误的交付方式与语义。
/// 通过错误通道交付给应用层的后台监控失败信息。
///
/// 清理失败诊断保存在 `cleanup_error`，不会覆盖监控的主错误。
///
/// # Examples
///
/// ```
/// use burncloud_node_runtime::deployment::{DeploymentErrorKind, DeploymentMonitorError};
///
/// let error = DeploymentMonitorError {
///     model_id: "model-1".to_string(),
///     pid: 42,
///     kind: DeploymentErrorKind::ProcessExited,
///     message: "runtime exited".to_string(),
///     cleanup_error: None,
/// };
/// assert_eq!(error.pid, 42);
/// ```
//
// 文档要点：
//   1. 该错误不走 deploy() 返回，而是通过 mpsc 错误通道（monitor_error_tx）交付；
//   2. 与 DeploymentError 一样，cleanup_error 是附加诊断，不覆盖主错误；
//   3. doctest 验证字段可访问。

// 派生：Debug / Clone / PartialEq / Eq / thiserror::Error。
// #[error("{kind:?}: {message}")]：Display 格式为 “Kind: message”。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{kind:?}: {message}")]
// 公开结构体：对外可见
pub struct DeploymentMonitorError {
    /// 监控失败所属的模型标识。
    pub model_id: String,
    /// 监控失败所属的真实进程 PID。
    pub pid: u32,
    /// 监控失败的稳定错误分类。
    pub kind: DeploymentErrorKind,
    /// 监控失败的原始错误消息。
    pub message: String,
    /// 清理进程失败时的附加诊断。
    pub cleanup_error: Option<String>,
}

// ============================================================================
// DeploymentEventPublisher：事件发布器 trait
// ============================================================================

// 三斜线文档：说明 trait 的契约。
/// 将部署进度和模型状态事件交付给应用层的异步接口。
///
/// `Err` 必须表示应用层未接收事件，便于调用方用同一事件重试一次。
///
/// # Examples
///
/// ```
/// use burncloud_node_runtime::deployment::{
///     DeploymentEvent, DeploymentEventPublishError, DeploymentEventPublisher,
/// };
///
/// async fn publish_event(
///     publisher: &impl DeploymentEventPublisher,
///     event: DeploymentEvent,
/// ) -> Result<(), DeploymentEventPublishError> {
///     publisher.publish(event).await
/// }
/// ```
//
// 文档要点：
//   1. 这是外部（应用层/订阅侧）要实现的事件接收接口；
//   2. 关键契约：Err 必须表示“应用层未接收”，
//      这正是 publish_with_retry 能用同一事件重试一次的前提；
//      若实现方在“部分接收”后返回 Err，重试会导致重复事件，违反契约。
//   3. doctest 展示如何写一个泛型发布辅助函数。

// async_trait 宏：让 trait 里能写 async fn。
// 约束 Send + Sync：发布器会被 Arc 共享、跨任务调用，必须线程安全。
#[async_trait]
pub trait DeploymentEventPublisher: Send + Sync {
    // 三斜线文档：说明 publish 的契约。
    /// 发布事件；返回错误时必须保证应用层尚未接收该事件。
    async fn publish(&self, event: DeploymentEvent) -> Result<(), DeploymentEventPublishError>;
    // 参数 event 按值传入（事件可能被 move 进通道）；
    // 返回 Result：Ok 表示已接收，Err 表示未接收（可安全重试）。
}
