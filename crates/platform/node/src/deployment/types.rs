use async_trait::async_trait;

use crate::{NodeState, ProcessHandle, ProcessPlan};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentStage {
    Queued,
    StoppingOldRuntime,
    Planning,
    Starting,
    WaitingReady,
    HealthChecking,
}

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentEventKind {
    Progress(DeploymentStage),
    InitialResult(NodeState),
    StateChanged(NodeState),
    DeploymentFailed,
}

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentEvent {
    pub model_id: String,
    pub kind: DeploymentEventKind,
    pub state: Option<NodeState>,
    pub error_kind: Option<DeploymentErrorKind>,
    pub local_endpoint: Option<String>,
    pub pid: Option<u32>,
    pub error: Option<String>,
    pub cleanup_error: Option<String>,
}

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
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeploymentEventPublishError {
    #[error("deployment event publish failed: {0}")]
    PublishFailed(String),
}

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentOutcome {
    pub plan: ProcessPlan,
    pub process: ProcessHandle,
    pub state: NodeState,
}

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
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{kind:?}: {message}")]
pub struct DeploymentError {
    pub kind: DeploymentErrorKind,
    pub message: String,
    pub cleanup_error: Option<String>,
    pub notification_error: Option<String>,
    pub preserved_state: Option<NodeState>,
}

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentErrorKind {
    PlanFailed,
    ProcessStartFailed,
    ReadinessFailed,
    HealthProbeFailed,
    OldRuntimeStopFailed,
    ProcessLivenessFailed,
    ProcessExited,
    EventPublishFailed,
}

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
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{kind:?}: {message}")]
pub struct DeploymentMonitorError {
    pub model_id: String,
    pub pid: u32,
    pub kind: DeploymentErrorKind,
    pub message: String,
    pub cleanup_error: Option<String>,
}

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
#[async_trait]
pub trait DeploymentEventPublisher: Send + Sync {
    /// 发布事件；返回错误时必须保证应用层尚未接收该事件。
    async fn publish(&self, event: DeploymentEvent) -> Result<(), DeploymentEventPublishError>;
}
