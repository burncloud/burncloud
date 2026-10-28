use std::{
    collections::{HashMap, VecDeque},
    mem,
    sync::Arc,
    time::Duration,
};

use tokio::{
    sync::{mpsc, oneshot, watch, Mutex, Notify},
    task::JoinHandle,
    time::{Instant, MissedTickBehavior},
};

use crate::{
    HealthProbe, NodeState, PreparedArtifact, PreparedRuntime, ProcessHandle, ProcessManager,
    ProcessPlan, ReadinessProbe, RuntimeAdapter,
};

use super::{
    DeploymentError, DeploymentErrorKind, DeploymentEvent, DeploymentEventKind,
    DeploymentEventPublishError, DeploymentEventPublisher, DeploymentMonitorError,
    DeploymentOutcome, DeploymentStage,
};

/// 串行协调 Node 进程内的模型部署请求。
///
/// 一个 Node 生命周期应共享一个服务实例。丢弃服务会通知队列和运行时监控任务退出，
/// 但不会隐式停止由进程管理器持有的运行时进程。构造服务时需要处于 Tokio 运行时中。
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
    generation: u64,
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
    /// #[tokio::main]
    /// async fn main() {
    ///     let (_service, _monitor_errors) = DeploymentService::new(
    ///         Arc::new(FakeProcessManager::default()),
    ///         Arc::new(FakeReadinessProbe),
    ///         Arc::new(FakeHealthProbe),
    ///         Arc::new(Publisher),
    ///     );
    /// }
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
            generation: 0,
            worker: None,
        };

        let service = Self {
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
        };
        let (worker_start_tx, worker_start_rx) = oneshot::channel();
        let worker_inner = Arc::clone(&service.inner);
        let mut worker_shutdown = service.inner.shutdown.subscribe();
        let worker = tokio::spawn(async move {
            if worker_start_rx.await.is_ok() {
                Self::run_queue(worker_inner, &mut worker_shutdown).await;
            }
        });
        match service.inner.coordinator.try_lock() {
            Ok(mut coordinator) => coordinator.worker = Some(worker),
            Err(_) => {
                worker.abort();
                panic!("deployment coordinator was locked during construction");
            }
        }
        if worker_start_tx.send(()).is_err() {
            tracing::error!("deployment queue worker stopped before startup");
        }

        (service, monitor_error_rx)
    }

    /// 将模型部署请求加入 FIFO 队列，并等待首次健康结果或部署错误。
    ///
    /// 同一 `model_id` 的在途调用共享规范请求结果。工件已验证、运行时已准备以及
    /// `model_id` 可用作关联键均由调用方保证。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use burncloud_node_runtime::{
    ///     deployment::DeploymentService, ArtifactVerificationStatus, PreparedArtifact,
    ///     PreparedRuntime, RuntimeAdapter,
    /// };
    ///
    /// async fn deploy(
    ///     service: &DeploymentService,
    ///     adapter: Arc<dyn RuntimeAdapter>,
    /// ) {
    ///     let artifact = match PreparedArtifact::new(
    ///         "model.gguf",
    ///         ArtifactVerificationStatus::Verified,
    ///     ) {
    ///         Ok(artifact) => artifact,
    ///         Err(_) => return,
    ///     };
    ///     let _result = service
    ///         .deploy(
    ///             "model-1".to_string(),
    ///             artifact,
    ///             PreparedRuntime { executable: "runtime".to_string() },
    ///             adapter,
    ///         )
    ///         .await;
    /// }
    /// ```
    pub async fn deploy(
        &self,
        model_id: String,
        artifact: PreparedArtifact,
        runtime: PreparedRuntime,
        adapter: Arc<dyn RuntimeAdapter>,
    ) -> Result<DeploymentOutcome, DeploymentError> {
        let result = self
            .enqueue_or_join(QueuedDeployment {
                model_id,
                artifact,
                runtime,
                adapter,
            })
            .await;
        match result.await {
            Ok(result) => result,
            Err(_) => Err(DeploymentError {
                kind: DeploymentErrorKind::EventPublishFailed,
                message: "deployment worker stopped before completing the request".to_string(),
                cleanup_error: None,
                notification_error: None,
                preserved_state: None,
            }),
        }
    }

    async fn run_queue(inner: Arc<DeploymentServiceInner>, shutdown: &mut watch::Receiver<bool>) {
        loop {
            if *shutdown.borrow() {
                return;
            }

            let request = {
                let mut coordinator = inner.coordinator.lock().await;
                take_next_deployment(&mut coordinator)
            };

            if let Some(request) = request {
                let model_id = request.model_id.clone();
                let result = tokio::select! {
                    result = Self::run_deployment(Arc::clone(&inner), request) => result,
                    _ = shutdown.changed() => return,
                };

                let mut coordinator = inner.coordinator.lock().await;
                if let Some(in_flight) = coordinator.in_flight.get_mut(&model_id) {
                    complete_waiters(mem::take(&mut in_flight.waiters), result);
                }
                coordinator.in_flight.remove(&model_id);
            } else {
                tokio::select! {
                    _ = inner.queue_ready.notified() => {}
                    _ = shutdown.changed() => return,
                }
            }
        }
    }

    async fn run_deployment(
        inner: Arc<DeploymentServiceInner>,
        request: QueuedDeployment,
    ) -> Result<DeploymentOutcome, DeploymentError> {
        let preserved_state = Self::stop_old_runtime(Arc::clone(&inner), &request.model_id).await?;

        if let Err(error) = publish_progress(
            &inner,
            &request.model_id,
            DeploymentStage::Planning,
            preserved_state,
            None,
            None,
        )
        .await
        {
            return Err(event_publish_error(error, None));
        }

        let plan = match request
            .adapter
            .plan(&request.runtime, &request.artifact)
            .await
        {
            Ok(plan) => plan,
            Err(error) => {
                let mut deployment_error =
                    deployment_error(DeploymentErrorKind::PlanFailed, error.to_string(), None);
                publish_deployment_failure(
                    &inner,
                    &request.model_id,
                    &mut deployment_error,
                    None,
                    None,
                )
                .await;
                return Err(deployment_error);
            }
        };

        if let Err(error) = publish_progress(
            &inner,
            &request.model_id,
            DeploymentStage::Starting,
            preserved_state,
            Some(&plan),
            None,
        )
        .await
        {
            return Err(event_publish_error(error, None));
        }

        let handle = match inner.process_manager.start(plan.process.clone()).await {
            Ok(handle) => handle,
            Err(error) => {
                let mut deployment_error = deployment_error(
                    DeploymentErrorKind::ProcessStartFailed,
                    error.to_string(),
                    None,
                );
                publish_deployment_failure(
                    &inner,
                    &request.model_id,
                    &mut deployment_error,
                    Some(&plan),
                    None,
                )
                .await;
                return Err(deployment_error);
            }
        };

        if let Err(error) = publish_progress(
            &inner,
            &request.model_id,
            DeploymentStage::WaitingReady,
            preserved_state,
            Some(&plan),
            Some(handle),
        )
        .await
        {
            let error = event_publish_error(error, None);
            return Err(Self::cleanup_failed_runtime(
                inner.process_manager.as_ref(),
                handle,
                error,
            )
            .await);
        }

        if let Err(error) = inner
            .readiness_probe
            .wait_ready(plan.readiness.clone())
            .await
        {
            let error = deployment_error(
                DeploymentErrorKind::ReadinessFailed,
                error.to_string(),
                None,
            );
            let mut error =
                Self::cleanup_failed_runtime(inner.process_manager.as_ref(), handle, error).await;
            publish_deployment_failure(
                &inner,
                &request.model_id,
                &mut error,
                Some(&plan),
                Some(handle),
            )
            .await;
            return Err(error);
        }

        if let Err(error) = publish_progress(
            &inner,
            &request.model_id,
            DeploymentStage::HealthChecking,
            preserved_state,
            Some(&plan),
            Some(handle),
        )
        .await
        {
            let error = event_publish_error(error, None);
            return Err(Self::cleanup_failed_runtime(
                inner.process_manager.as_ref(),
                handle,
                error,
            )
            .await);
        }

        let healthy = match inner.health_probe.is_healthy(plan.readiness.clone()).await {
            Ok(healthy) => healthy,
            Err(error) => {
                let error = deployment_error(
                    DeploymentErrorKind::HealthProbeFailed,
                    error.to_string(),
                    None,
                );
                let mut error =
                    Self::cleanup_failed_runtime(inner.process_manager.as_ref(), handle, error)
                        .await;
                publish_deployment_failure(
                    &inner,
                    &request.model_id,
                    &mut error,
                    Some(&plan),
                    Some(handle),
                )
                .await;
                return Err(error);
            }
        };

        let state = if healthy {
            NodeState::Ready
        } else {
            NodeState::Unhealthy
        };
        let initial_result = DeploymentEvent {
            model_id: request.model_id.clone(),
            kind: DeploymentEventKind::InitialResult(state),
            state: Some(state),
            error_kind: None,
            local_endpoint: Some(plan.local_endpoint.clone()),
            pid: Some(handle.pid),
            error: None,
            cleanup_error: None,
        };
        if let Err(error) =
            Self::publish_with_retry(inner.event_publisher.as_ref(), initial_result).await
        {
            let error = event_publish_error(error, None);
            return Err(Self::cleanup_failed_runtime(
                inner.process_manager.as_ref(),
                handle,
                error,
            )
            .await);
        }

        Self::spawn_monitor(
            Arc::clone(&inner),
            request.model_id,
            plan.clone(),
            handle,
            state,
        )
        .await;

        Ok(DeploymentOutcome {
            plan,
            process: handle,
            state,
        })
    }

    async fn stop_old_runtime(
        inner: Arc<DeploymentServiceInner>,
        model_id: &str,
    ) -> Result<Option<NodeState>, DeploymentError> {
        let previous = {
            let coordinator = inner.coordinator.lock().await;
            coordinator
                .monitored
                .get(model_id)
                .map(|runtime| (runtime.state, runtime.plan.clone(), runtime.handle))
        };
        let Some((state, plan, handle)) = previous else {
            return Ok(None);
        };

        let stopping_event = DeploymentEvent {
            model_id: model_id.to_string(),
            kind: DeploymentEventKind::Progress(DeploymentStage::StoppingOldRuntime),
            state: Some(state),
            error_kind: None,
            local_endpoint: Some(plan.local_endpoint.clone()),
            pid: Some(handle.pid),
            error: None,
            cleanup_error: None,
        };
        if let Err(error) =
            Self::publish_with_retry(inner.event_publisher.as_ref(), stopping_event).await
        {
            return Err(event_publish_error(error, Some(state)));
        }

        let Some(runtime) = inner.coordinator.lock().await.monitored.remove(model_id) else {
            return Ok(None);
        };
        let MonitoredRuntime {
            handle,
            plan,
            state,
            cancel,
            task,
            ..
        } = runtime;
        cancel.send_replace(true);
        if let Err(error) = task.await {
            tracing::warn!(%error, model_id, "old runtime monitor task stopped unexpectedly");
        }

        match inner.process_manager.stop(handle).await {
            Ok(()) => Ok(Some(state)),
            Err(error) => {
                Self::spawn_monitor(
                    Arc::clone(&inner),
                    model_id.to_string(),
                    plan,
                    handle,
                    state,
                )
                .await;
                Err(deployment_error(
                    DeploymentErrorKind::OldRuntimeStopFailed,
                    error.to_string(),
                    Some(state),
                ))
            }
        }
    }

    async fn publish_with_retry(
        event_publisher: &dyn DeploymentEventPublisher,
        event: DeploymentEvent,
    ) -> Result<(), DeploymentEventPublishError> {
        match event_publisher.publish(event.clone()).await {
            Ok(()) => Ok(()),
            Err(_) => event_publisher.publish(event).await,
        }
    }

    async fn cleanup_failed_runtime(
        process_manager: &dyn ProcessManager,
        handle: ProcessHandle,
        mut error: DeploymentError,
    ) -> DeploymentError {
        if let Err(cleanup_error) = process_manager.stop(handle).await {
            error.cleanup_error = Some(cleanup_error.to_string());
        }
        error
    }

    async fn spawn_monitor(
        inner: Arc<DeploymentServiceInner>,
        model_id: String,
        plan: ProcessPlan,
        handle: ProcessHandle,
        initial_state: NodeState,
    ) {
        assert!(matches!(
            initial_state,
            NodeState::Ready | NodeState::Unhealthy
        ));
        let (cancel, cancel_rx) = watch::channel(false);
        let monitor_plan = plan.clone();
        let mut coordinator = inner.coordinator.lock().await;
        assert!(
            !coordinator.monitored.contains_key(&model_id),
            "a runtime must be stopped before its monitor is replaced"
        );
        coordinator.generation = coordinator.generation.wrapping_add(1);
        let generation = coordinator.generation;
        let task_inner = Arc::clone(&inner);
        let task_model_id = model_id.clone();
        let mut shutdown = inner.shutdown.subscribe();
        let task = tokio::spawn(async move {
            let result = Self::monitor_runtime(
                Arc::clone(&task_inner),
                task_model_id.clone(),
                monitor_plan,
                handle,
                initial_state,
                generation,
                cancel_rx,
                &mut shutdown,
            )
            .await;

            {
                let mut coordinator = task_inner.coordinator.lock().await;
                if coordinator
                    .monitored
                    .get(&task_model_id)
                    .is_some_and(|runtime| runtime.generation == generation)
                {
                    coordinator.monitored.remove(&task_model_id);
                }
            }

            if let Err(error) = result {
                if let Err(send_error) = task_inner.monitor_error_tx.send(error) {
                    tracing::error!(
                        error = %send_error.0,
                        "deployment monitor error receiver closed"
                    );
                }
            }
        });
        coordinator.monitored.insert(
            model_id,
            MonitoredRuntime {
                handle,
                plan,
                state: initial_state,
                generation,
                cancel,
                task,
            },
        );
    }

    async fn monitor_runtime(
        inner: Arc<DeploymentServiceInner>,
        model_id: String,
        plan: ProcessPlan,
        handle: ProcessHandle,
        mut state: NodeState,
        generation: u64,
        mut cancel: watch::Receiver<bool>,
        shutdown: &mut watch::Receiver<bool>,
    ) -> Result<(), DeploymentMonitorError> {
        // 固定周期避免把单次探测耗时叠加到下一次等待间隔中。
        let mut interval = tokio::time::interval_at(
            Instant::now() + Duration::from_secs(10),
            Duration::from_secs(10),
        );
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            if *cancel.borrow() || *shutdown.borrow() {
                return Ok(());
            }

            tokio::select! {
                _ = interval.tick() => {}
                _ = cancel.changed() => return Ok(()),
                _ = shutdown.changed() => return Ok(()),
            }

            let running = tokio::select! {
                result = inner.process_manager.is_running(handle) => result,
                _ = cancel.changed() => return Ok(()),
                _ = shutdown.changed() => return Ok(()),
            };

            match running {
                Ok(false) => {
                    return report_monitor_failure(
                        &inner,
                        &model_id,
                        &plan,
                        handle,
                        DeploymentError {
                            kind: DeploymentErrorKind::ProcessExited,
                            message: "runtime process exited".to_string(),
                            cleanup_error: None,
                            notification_error: None,
                            preserved_state: None,
                        },
                        false,
                        true,
                    )
                    .await;
                }
                Err(error) => {
                    return report_monitor_failure(
                        &inner,
                        &model_id,
                        &plan,
                        handle,
                        DeploymentError {
                            kind: DeploymentErrorKind::ProcessLivenessFailed,
                            message: error.to_string(),
                            cleanup_error: None,
                            notification_error: None,
                            preserved_state: None,
                        },
                        true,
                        true,
                    )
                    .await;
                }
                Ok(true) => {}
            }

            let health = tokio::select! {
                result = inner.health_probe.is_healthy(plan.readiness.clone()) => result,
                _ = cancel.changed() => return Ok(()),
                _ = shutdown.changed() => return Ok(()),
            };
            let next_state = match health {
                Ok(true) => NodeState::Ready,
                Ok(false) => NodeState::Unhealthy,
                Err(error) => {
                    return report_monitor_failure(
                        &inner,
                        &model_id,
                        &plan,
                        handle,
                        DeploymentError {
                            kind: DeploymentErrorKind::HealthProbeFailed,
                            message: error.to_string(),
                            cleanup_error: None,
                            notification_error: None,
                            preserved_state: None,
                        },
                        true,
                        true,
                    )
                    .await;
                }
            };

            if next_state == state {
                continue;
            }

            let event = monitor_state_event(&model_id, &plan, handle, next_state, None, None, None);
            if let Err(error) =
                Self::publish_with_retry(inner.event_publisher.as_ref(), event).await
            {
                return report_monitor_failure(
                    &inner,
                    &model_id,
                    &plan,
                    handle,
                    DeploymentError {
                        kind: DeploymentErrorKind::EventPublishFailed,
                        message: error.to_string(),
                        cleanup_error: None,
                        notification_error: None,
                        preserved_state: None,
                    },
                    true,
                    false,
                )
                .await;
            }

            let mut coordinator = inner.coordinator.lock().await;
            match coordinator.monitored.get_mut(&model_id) {
                Some(runtime) if runtime.generation == generation => {
                    runtime.state = next_state;
                    state = next_state;
                }
                _ => return Ok(()),
            }
        }
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

        if let Err(error) =
            Self::publish_with_retry(self.inner.event_publisher.as_ref(), queued_event).await
        {
            let mut coordinator = self.inner.coordinator.lock().await;
            coordinator
                .queue
                .retain(|queued| queued.model_id != model_id);
            let preserved_state = coordinator
                .monitored
                .get(&model_id)
                .map(|runtime| runtime.state);
            let deployment_error = DeploymentError {
                kind: DeploymentErrorKind::EventPublishFailed,
                message: error.to_string(),
                cleanup_error: None,
                notification_error: None,
                preserved_state,
            };
            if let Some(in_flight) = coordinator.in_flight.get_mut(&model_id) {
                complete_waiters(mem::take(&mut in_flight.waiters), Err(deployment_error));
            }
            coordinator.in_flight.remove(&model_id);
            drop(coordinator);
            self.inner.queue_ready.notify_one();
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

fn take_next_deployment(coordinator: &mut DeploymentCoordinator) -> Option<QueuedDeployment> {
    let ready = coordinator.queue.front().is_some_and(|request| {
        coordinator
            .in_flight
            .get(&request.model_id)
            .is_some_and(|in_flight| in_flight.queue_event_published && !in_flight.is_executing)
    });
    if !ready {
        return None;
    }

    let request = coordinator.queue.pop_front()?;
    if let Some(in_flight) = coordinator.in_flight.get_mut(&request.model_id) {
        in_flight.is_executing = true;
    }
    Some(request)
}

async fn publish_progress(
    inner: &DeploymentServiceInner,
    model_id: &str,
    stage: DeploymentStage,
    state: Option<NodeState>,
    plan: Option<&ProcessPlan>,
    handle: Option<ProcessHandle>,
) -> Result<(), DeploymentEventPublishError> {
    let event = DeploymentEvent {
        model_id: model_id.to_string(),
        kind: DeploymentEventKind::Progress(stage),
        state,
        error_kind: None,
        local_endpoint: plan.map(|plan| plan.local_endpoint.clone()),
        pid: handle.map(|handle| handle.pid),
        error: None,
        cleanup_error: None,
    };
    DeploymentService::publish_with_retry(inner.event_publisher.as_ref(), event).await
}

fn deployment_error(
    kind: DeploymentErrorKind,
    message: String,
    preserved_state: Option<NodeState>,
) -> DeploymentError {
    DeploymentError {
        kind,
        message,
        cleanup_error: None,
        notification_error: None,
        preserved_state,
    }
}

fn event_publish_error(
    error: DeploymentEventPublishError,
    preserved_state: Option<NodeState>,
) -> DeploymentError {
    deployment_error(
        DeploymentErrorKind::EventPublishFailed,
        error.to_string(),
        preserved_state,
    )
}

async fn publish_deployment_failure(
    inner: &DeploymentServiceInner,
    model_id: &str,
    error: &mut DeploymentError,
    plan: Option<&ProcessPlan>,
    handle: Option<ProcessHandle>,
) {
    if error.kind == DeploymentErrorKind::EventPublishFailed || error.preserved_state.is_some() {
        return;
    }

    let event = DeploymentEvent {
        model_id: model_id.to_string(),
        kind: DeploymentEventKind::DeploymentFailed,
        state: Some(NodeState::Failed),
        error_kind: Some(error.kind),
        local_endpoint: plan.map(|plan| plan.local_endpoint.clone()),
        pid: handle.map(|handle| handle.pid),
        error: Some(error.message.clone()),
        cleanup_error: error.cleanup_error.clone(),
    };
    if let Err(notification_error) =
        DeploymentService::publish_with_retry(inner.event_publisher.as_ref(), event).await
    {
        error.notification_error = Some(notification_error.to_string());
    }
}

async fn report_monitor_failure(
    inner: &DeploymentServiceInner,
    model_id: &str,
    plan: &ProcessPlan,
    handle: ProcessHandle,
    mut error: DeploymentError,
    stop_process: bool,
    publish_failed_state: bool,
) -> Result<(), DeploymentMonitorError> {
    let kind = error.kind;
    if stop_process {
        error = DeploymentService::cleanup_failed_runtime(
            inner.process_manager.as_ref(),
            handle,
            error,
        )
        .await;
    }

    let mut monitor_error = DeploymentMonitorError {
        model_id: model_id.to_string(),
        pid: handle.pid,
        kind,
        message: error.message.clone(),
        cleanup_error: error.cleanup_error.clone(),
    };

    if publish_failed_state {
        let event = monitor_state_event(
            model_id,
            plan,
            handle,
            NodeState::Failed,
            Some(kind),
            Some(error.message),
            error.cleanup_error,
        );
        if let Err(publish_error) =
            DeploymentService::publish_with_retry(inner.event_publisher.as_ref(), event).await
        {
            monitor_error.message = format!(
                "{}; failed to publish Failed state: {}",
                monitor_error.message, publish_error
            );
            return Err(monitor_error);
        }
        return Ok(());
    }

    Err(monitor_error)
}

fn monitor_state_event(
    model_id: &str,
    plan: &ProcessPlan,
    handle: ProcessHandle,
    state: NodeState,
    error_kind: Option<DeploymentErrorKind>,
    error: Option<String>,
    cleanup_error: Option<String>,
) -> DeploymentEvent {
    DeploymentEvent {
        model_id: model_id.to_string(),
        kind: DeploymentEventKind::StateChanged(state),
        state: Some(state),
        error_kind,
        local_endpoint: Some(plan.local_endpoint.clone()),
        pid: Some(handle.pid),
        error,
        cleanup_error,
    }
}
