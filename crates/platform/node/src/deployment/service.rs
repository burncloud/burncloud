use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use tokio::{
    sync::{mpsc, oneshot, watch, Mutex, Notify},
    task::JoinHandle,
};

use crate::{
    HealthProbe, PreparedArtifact, PreparedRuntime, ProcessHandle, ProcessManager, ProcessPlan,
    ReadinessProbe, RuntimeAdapter,
};

use super::{
    DeploymentError, DeploymentErrorKind, DeploymentEvent, DeploymentEventKind,
    DeploymentEventPublishError, DeploymentEventPublisher, DeploymentMonitorError,
    DeploymentOutcome, DeploymentStage,
};

/// 串行协调 Node 进程内的模型部署请求。
///
/// 一个 Node 生命周期应共享一个服务实例。丢弃服务会通知队列和运行时监控任务退出，
/// 但不会隐式停止由进程管理器持有的运行时进程。
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use async_trait::async_trait;
/// use burncloud_node_runtime::{
///     deployment::{
///         DeploymentEvent, DeploymentEventPublishError, DeploymentEventPublisher,
///         DeploymentService,
///     },
///     FakeHealthProbe, FakeProcessManager, FakeReadinessProbe,
/// };
///
/// struct Publisher;
///
/// #[async_trait]
/// impl DeploymentEventPublisher for Publisher {
///     async fn publish(
///         &self,
///         _event: DeploymentEvent,
///     ) -> Result<(), DeploymentEventPublishError> {
///         Ok(())
///     }
/// }
///
/// fn create_service() {
///     let (_service, _monitor_errors) = DeploymentService::new(
///         Arc::new(FakeProcessManager::default()),
///         Arc::new(FakeReadinessProbe),
///         Arc::new(FakeHealthProbe),
///         Arc::new(Publisher),
///     );
/// }
/// ```
pub struct DeploymentService {
    inner: Arc<DeploymentServiceInner>,
}

struct DeploymentServiceInner {
    process_manager: Arc<dyn ProcessManager>,
    readiness_probe: Arc<dyn ReadinessProbe>,
    health_probe: Arc<dyn HealthProbe>,
    event_publisher: Arc<dyn DeploymentEventPublisher>,
    monitor_error_tx: mpsc::UnboundedSender<DeploymentMonitorError>,
    coordinator: Arc<Mutex<DeploymentCoordinator>>,
    queue_ready: Notify,
    shutdown: watch::Sender<bool>,
}

struct DeploymentCoordinator {
    queue: VecDeque<QueuedDeployment>,
    in_flight: HashMap<String, InFlightDeployment>,
    monitored: HashMap<String, MonitoredRuntime>,
    worker: Option<JoinHandle<()>>,
}

struct QueuedDeployment {
    model_id: String,
    artifact: PreparedArtifact,
    runtime: PreparedRuntime,
    adapter: Arc<dyn RuntimeAdapter>,
}

struct InFlightDeployment {
    is_executing: bool,
    queue_event_published: bool,
    waiters: Vec<oneshot::Sender<Result<DeploymentOutcome, DeploymentError>>>,
}

struct MonitoredRuntime {
    handle: ProcessHandle,
    plan: ProcessPlan,
    state: crate::NodeState,
    generation: u64,
    cancel: watch::Sender<bool>,
    task: JoinHandle<()>,
}

impl DeploymentService {
    /// 创建空的部署协调状态和后台监控错误接收端。
    ///
    /// 所有依赖通过共享接口持有；返回的接收端由 NodeComposition 独占保存。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use async_trait::async_trait;
    /// use burncloud_node_runtime::{
    ///     deployment::{
    ///         DeploymentEvent, DeploymentEventPublishError, DeploymentEventPublisher,
    ///         DeploymentService,
    ///     },
    ///     FakeHealthProbe, FakeProcessManager, FakeReadinessProbe,
    /// };
    ///
    /// struct Publisher;
    ///
    /// #[async_trait]
    /// impl DeploymentEventPublisher for Publisher {
    ///     async fn publish(
    ///         &self,
    ///         _event: DeploymentEvent,
    ///     ) -> Result<(), DeploymentEventPublishError> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let (_service, _monitor_errors) = DeploymentService::new(
    ///     Arc::new(FakeProcessManager::default()),
    ///     Arc::new(FakeReadinessProbe),
    ///     Arc::new(FakeHealthProbe),
    ///     Arc::new(Publisher),
    /// );
    /// ```
    pub fn new(
        process_manager: Arc<dyn ProcessManager>,
        readiness_probe: Arc<dyn ReadinessProbe>,
        health_probe: Arc<dyn HealthProbe>,
        event_publisher: Arc<dyn DeploymentEventPublisher>,
    ) -> (Self, mpsc::UnboundedReceiver<DeploymentMonitorError>) {
        let (monitor_error_tx, monitor_error_rx) = mpsc::unbounded_channel();
        let (shutdown, _) = watch::channel(false);
        let coordinator = DeploymentCoordinator {
            queue: VecDeque::new(),
            in_flight: HashMap::new(),
            monitored: HashMap::new(),
            worker: None,
        };

        (
            Self {
                inner: Arc::new(DeploymentServiceInner {
                    process_manager,
                    readiness_probe,
                    health_probe,
                    event_publisher,
                    monitor_error_tx,
                    coordinator: Arc::new(Mutex::new(coordinator)),
                    queue_ready: Notify::new(),
                    shutdown,
                }),
            },
            monitor_error_rx,
        )
    }

    async fn publish_with_retry(
        &self,
        event: DeploymentEvent,
    ) -> Result<(), DeploymentEventPublishError> {
        match self.inner.event_publisher.publish(event.clone()).await {
            Ok(()) => Ok(()),
            Err(_) => self.inner.event_publisher.publish(event).await,
        }
    }

    async fn cleanup_failed_runtime(
        &self,
        handle: ProcessHandle,
        mut error: DeploymentError,
    ) -> DeploymentError {
        if let Err(cleanup_error) = self.inner.process_manager.stop(handle).await {
            error.cleanup_error = Some(cleanup_error.to_string());
        }
        error
    }

    async fn enqueue_or_join(
        &self,
        request: QueuedDeployment,
    ) -> oneshot::Receiver<Result<DeploymentOutcome, DeploymentError>> {
        let model_id = request.model_id.clone();
        let (waiter, result) = oneshot::channel();
        {
            let mut coordinator = self.inner.coordinator.lock().await;
            if let Some(in_flight) = coordinator.in_flight.get_mut(&model_id) {
                in_flight.waiters.push(waiter);
                return result;
            }

            coordinator.in_flight.insert(
                model_id.clone(),
                InFlightDeployment {
                    is_executing: false,
                    queue_event_published: false,
                    waiters: vec![waiter],
                },
            );
            coordinator.queue.push_back(request);
        }

        let queued_event = DeploymentEvent {
            model_id: model_id.clone(),
            kind: DeploymentEventKind::Progress(DeploymentStage::Queued),
            state: None,
            error_kind: None,
            local_endpoint: None,
            pid: None,
            error: None,
            cleanup_error: None,
        };

        if let Err(error) = self.publish_with_retry(queued_event).await {
            let (waiters, preserved_state) = {
                let mut coordinator = self.inner.coordinator.lock().await;
                coordinator
                    .queue
                    .retain(|queued| queued.model_id != model_id);
                let preserved_state = coordinator
                    .monitored
                    .get(&model_id)
                    .map(|runtime| runtime.state);
                let waiters = coordinator
                    .in_flight
                    .remove(&model_id)
                    .map(|in_flight| in_flight.waiters)
                    .unwrap_or_default();
                (waiters, preserved_state)
            };
            let deployment_error = DeploymentError {
                kind: DeploymentErrorKind::EventPublishFailed,
                message: error.to_string(),
                cleanup_error: None,
                notification_error: None,
                preserved_state,
            };
            complete_waiters(waiters, Err(deployment_error));
            return result;
        }

        if let Some(in_flight) = self
            .inner
            .coordinator
            .lock()
            .await
            .in_flight
            .get_mut(&model_id)
        {
            in_flight.queue_event_published = true;
        }
        self.inner.queue_ready.notify_one();
        result
    }
}

impl Drop for DeploymentService {
    fn drop(&mut self) {
        self.inner.shutdown.send_replace(true);
    }
}

fn complete_waiters(
    waiters: Vec<oneshot::Sender<Result<DeploymentOutcome, DeploymentError>>>,
    result: Result<DeploymentOutcome, DeploymentError>,
) {
    for waiter in waiters {
        if waiter.send(result.clone()).is_err() {
            tracing::debug!("deployment result waiter closed before completion");
        }
    }
}
