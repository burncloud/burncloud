// ============================================================================
// 依赖导入区
// ============================================================================

// 从标准库导入多个模块，用大括号分组写法。
use std::{
    // 从 std::collections 导入 HashMap 与 VecDeque。
    collections::{HashMap, VecDeque},
    // 为什么用 HashMap：需要按 model_id 做 O(1) 查找（合并同模型请求、查旧运行时、查在途记录）。
    // 为什么用 VecDeque：FIFO 部署队列，队首取下一个执行、队尾追加新请求。

    // 从 std 导入 mem 模块。
    mem,
    // 用 mem::take 把 Vec 取走（置空原值），避免移动整个容器；在持锁时借用检查更友好。

    // 从 std::sync 导入 Arc。
    sync::Arc,
    // Arc：DeploymentServiceInner 被 service 本体、队列 worker、每个监控任务共享；
    //       Arc 是原子引用计数，跨线程/跨任务共享的标准做法。

    // 从 std::time 导入 Duration。
    time::Duration,
    // Duration：监控轮询周期（10 秒）以及 interval 的周期参数。
};

// 从 tokio 导入多个模块。
use tokio::{
    // 从 tokio::sync 导入多个同步原语。
    sync::{
        // 导入 mpsc（多生产者单消费者通道）。
        mpsc,
        // mpsc：监控任务 -> 外部（NodeComposition）的上报通道；
        //       用 unbounded 避免背压阻塞监控任务。

        // 导入 oneshot（一次性通道）。
        oneshot,
        // oneshot：deploy() 调用方等待结果的一次性回执通道；每个等待者一个 Sender。

        // 导入 watch（广播式通道）。
        watch,
        // watch：广播式“关闭/取消”信号；
        //        shutdown 是服务级（Drop 时置 true）；
        //        cancel 是每个监控任务级（stop_old_runtime 时置 true）。

        // 导入 Mutex（异步互斥锁）。
        Mutex,
        // Mutex：保护 coordinator（持锁期间会 .await，不能用 std::sync::Mutex）。

        // 导入 Notify（异步通知）。
        Notify,
        // Notify：队列有新任务时唤醒 worker，避免 worker 忙等自旋。
    },

    // 从 tokio::task 导入 JoinHandle。
    task::JoinHandle,
    // JoinHandle：队列 worker 与监控任务的任务句柄，用于登记与 await 退出。

    // 从 tokio::time 导入 Instant 与 MissedTickBehavior。
    time::{Instant, MissedTickBehavior},
    // Instant：监控定时器起点（interval_at 的第一个参数）。
    // MissedTickBehavior：错过 tick 时的补偿策略（Delay 表示延后下一次，不补发）。
};

// 从 crate 根导入运行时抽象与数据类型（trait 在 crate 根定义，实现在 deployment 子模块）。
use crate::{
    // 导入 HealthProbe trait。
    HealthProbe,
    // ReadinessProbe：就绪探测 trait，启动阶段阻塞等待就绪。

    // 模型地址适配器只把 URL 转换为进程计划，不负责下载模型文件。
    ModelUrlRuntimeAdapter,
    // HealthProbe：健康探测 trait，运行期单次判定目标是否健康。

    // 导入 NodeState 枚举。
    NodeState,
    PreparedArtifact,
    // NodeState：节点生命周期状态（Ready / Unhealthy / Failed / Starting 等）。

    // 导入 PreparedRuntime。
    PreparedRuntime,
    // PreparedRuntime：已准备好的运行时（可执行文件路径等）。

    // 导入 ProcessHandle。
    ProcessHandle,
    // ProcessHandle：进程句柄（含 pid）。

    // 导入 ProcessManager trait。
    ProcessManager,
    // ProcessManager：进程管理 trait（start / stop / is_running）。

    // 导入 ProcessPlan。
    ProcessPlan,
    // ProcessPlan：进程计划（含启动规格、就绪目标 ReadinessTarget、本地 endpoint）。

    // 导入 ReadinessProbe trait。
    ReadinessProbe,
    RuntimeAdapter,
};

// 从父模块（deployment）导入契约类型。
use super::{
    // 导入 DeploymentError。
    DeploymentError,
    // DeploymentError：部署顶层错误（含 kind / message / cleanup_error / notification_error / preserved_state）。

    // 导入 DeploymentErrorKind。
    DeploymentErrorKind,
    // DeploymentErrorKind：错误种类枚举（PlanFailed / ProcessStartFailed / ReadinessFailed 等）。

    // 导入 DeploymentEvent。
    DeploymentEvent,
    // DeploymentEvent：部署事件（对外发布的统一结构，含 model_id / kind / state / endpoint / pid / error 等）。

    // 导入 DeploymentEventKind。
    DeploymentEventKind,
    // DeploymentEventKind：事件种类（Progress(stage) / InitialResult(state) / StateChanged(state) / DeploymentFailed）。

    // 导入 DeploymentEventPublishError。
    DeploymentEventPublishError,
    // DeploymentEventPublishError：事件发布错误。

    // 导入 DeploymentEventPublisher trait。
    DeploymentEventPublisher,
    // DeploymentEventPublisher：事件发布器 trait（外部订阅侧实现 publish）。

    // 导入 DeploymentMonitorError。
    DeploymentMonitorError,
    // DeploymentMonitorError：监控通道错误（监控任务失败时通过 mpsc 上报）。

    // 导入 DeploymentOutcome。
    DeploymentOutcome,
    // DeploymentOutcome：部署最终结果（plan / process / state）。

    // 导入 DeploymentStage。
    DeploymentStage,
    // DeploymentStage：部署阶段（Queued / Planning / Starting / WaitingReady / HealthChecking / StoppingOldRuntime）。
};

// ============================================================================
// DeploymentService：对外门面
// ============================================================================

// 三斜线文档注释：说明这个结构体的职责与使用方式。
/// 串行协调 Node 进程内的模型部署请求。
///
/// 一个 Node 生命周期应共享一个服务实例。丢弃服务会通知队列和运行时监控任务退出，
/// 但不会隐式停止由进程管理器持有的运行时进程。构造服务时需要处于 Tokio 运行时中。
///
/// # Examples
/// （doctest：用 Fake 实现构造服务，验证 API 形状与 Publisher 需实现 async_trait）
pub struct DeploymentService {
    // 公开结构体，字段私有（外部无法直接构造/访问 inner）。
    // 只持有一个 Arc<Inner>：
    //   - Clone 廉价（直接 Arc::clone），适合跨任务共享；
    //   - Drop 语义集中在 Inner 的 Arc 上（最后一个 Arc 释放时才触发 Inner 的 drop，
    //     但本文件的 Drop 实现在 DeploymentService 上，见后文）。
    inner: Arc<DeploymentServiceInner>,
}

// ============================================================================
// DeploymentServiceInner：服务的全部共享状态（被 worker/监控任务共享）
// ============================================================================

// 私有结构体：只在模块内部可见。
struct DeploymentServiceInner {
    // 进程管理器：启动/停止/查询子进程。dyn trait object，便于注入 Fake 或真实实现。
    process_manager: Arc<dyn ProcessManager>,

    // 就绪探测：启动阶段 wait_ready 阻塞等待。
    readiness_probe: Arc<dyn ReadinessProbe>,

    // 健康探测：运行期 is_healthy 单次判定。
    health_probe: Arc<dyn HealthProbe>,

    // 事件发布器：把状态变化通知外部（外部实现 publish）。
    event_publisher: Arc<dyn DeploymentEventPublisher>,

    // 监控任务出错时上报的通道发送端。
    //   接收端由 NodeComposition 独占保存（new() 返回给调用方），
    //   这样监控错误不会混入部署结果通道。
    monitor_error_tx: mpsc::UnboundedSender<DeploymentMonitorError>,

    // 协调器状态（队列、在途、受监控运行时），异步锁保护。
    //   用 Arc 是为了让监控任务也能拿到同一把锁（它们需要在退出时清理 monitored）。
    coordinator: Arc<Mutex<DeploymentCoordinator>>,

    // 队列有新任务时唤醒 worker。
    queue_ready: Notify,

    // 服务级关闭广播：Drop 时置 true，worker 与监控任务据此退出。
    shutdown: watch::Sender<bool>,
}

// ============================================================================
// DeploymentCoordinator：串行执行的调度状态（受 Mutex 保护）
// ============================================================================

// 私有结构体：协调器状态。
struct DeploymentCoordinator {
    // FIFO 队列：等待执行的部署请求。
    queue: VecDeque<QueuedDeployment>,

    // 在途部署：model_id -> 执行状态 + 等待回执的调用方。
    //   作用：合并同模型并发请求、防止重复执行、回填结果。
    in_flight: HashMap<String, InFlightDeployment>,

    // 正在被监控的已启动运行时：model_id -> 监控信息。
    //   作用：停旧运行时、监控状态查询、防止同一模型重复 spawn 监控。
    monitored: HashMap<String, MonitoredRuntime>,

    // 世代号：每次 spawn_monitor 自增。
    //   作用：监控任务退出时判断自己是否“过期”（防止旧任务误删新监控记录）。
    generation: u64,

    // 队列 worker 任务句柄（构造后填入；Drop 时不显式 await，靠 shutdown 通知退出）。
    worker: Option<JoinHandle<()>>,
}

// 队列中的一次部署请求（已具备全部输入，等待 worker 执行）。
struct QueuedDeployment {
    // 模型标识（也用作关联键：in_flight / monitored 的 key）。
    model_id: String,

    // 本地工件或模型地址及其对应适配器。
    input: DeploymentInput,

    // 已准备好的运行时。
    runtime: PreparedRuntime,
}

enum DeploymentInput {
    PreparedArtifact {
        artifact: PreparedArtifact,
        adapter: Arc<dyn RuntimeAdapter>,
    },
    ModelUrl {
        model_url: String,
        adapter: Arc<dyn ModelUrlRuntimeAdapter>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DeploymentInputKey {
    PreparedArtifact(PreparedArtifact),
    ModelUrl(String),
}

impl DeploymentInput {
    fn key(&self) -> DeploymentInputKey {
        match self {
            Self::PreparedArtifact { artifact, .. } => {
                DeploymentInputKey::PreparedArtifact(artifact.clone())
            }
            Self::ModelUrl { model_url, .. } => DeploymentInputKey::ModelUrl(model_url.clone()),
        }
    }

    async fn plan(
        &self,
        runtime: &PreparedRuntime,
    ) -> Result<ProcessPlan, crate::RuntimeAdapterError> {
        match self {
            Self::PreparedArtifact { artifact, adapter } => adapter.plan(runtime, artifact).await,
            Self::ModelUrl { model_url, adapter } => {
                adapter.plan_model_url(runtime, model_url).await
            }
        }
    }
}

// 在途部署的簿记。
struct InFlightDeployment {
    // 作为去重依据，避免相同模型标识但不同输入的请求共享错误结果。
    input_key: DeploymentInputKey,
    // worker 是否已开始执行（防止 take_next_deployment 重复取同一个）。
    is_executing: bool,

    // Queued 事件是否已成功发布。
    //   作用：take_next_deployment 只在“已发布且未执行”时才允许取，
    //   保证事件顺序（先对外宣布 Queued，再真正执行）。
    queue_event_published: bool,

    // 等待结果的调用方列表（同一个 model_id 的并发 deploy 共享结果）。
    waiters: Vec<oneshot::Sender<Result<DeploymentOutcome, DeploymentError>>>,
}

// 受监控的运行时。
struct MonitoredRuntime {
    // 进程句柄。
    handle: ProcessHandle,

    // 进程计划（含 endpoint/readiness，监控任务需要）。
    plan: ProcessPlan,

    // 当前生命周期状态（Ready / Unhealthy）。
    state: crate::NodeState,

    // 世代号：监控任务退出时判断自己是否过期。
    generation: u64,

    // 取消该监控任务的信号发送端。
    cancel: watch::Sender<bool>,

    // 监控任务句柄。
    task: JoinHandle<()>,
}

// ============================================================================
// impl DeploymentService
// ============================================================================

// 为 DeploymentService 实现方法。
impl DeploymentService {
    // 三斜线文档：new 的职责与返回。
    /// 创建空的部署协调状态和后台监控错误接收端。
    ///
    /// 所有依赖通过共享接口持有；返回的接收端由 NodeComposition 独占保存。
    ///
    /// # Examples
    /// （doctest：在 tokio runtime 里用 Fake 实现构造服务）
    pub fn new(
        // 进程管理器（Arc<dyn> 便于注入）。
        process_manager: Arc<dyn ProcessManager>,
        // 就绪探测。
        readiness_probe: Arc<dyn ReadinessProbe>,
        // 健康探测。
        health_probe: Arc<dyn HealthProbe>,
        // 事件发布器。
        event_publisher: Arc<dyn DeploymentEventPublisher>,
        // 返回服务本体 + 监控错误接收端。
    ) -> (Self, mpsc::UnboundedReceiver<DeploymentMonitorError>) {
        // 创建监控错误通道：发送端放进 Inner，接收端返回给调用方。
        let (monitor_error_tx, monitor_error_rx) = mpsc::unbounded_channel();

        // 创建关闭广播通道：初值 false；发送端留在 Inner，接收端由各任务 subscribe()。
        // 用 `_` 丢弃初始 Receiver（没人用初始那个）。
        let (shutdown, _) = watch::channel(false);

        // 初始化空协调器。
        let coordinator = DeploymentCoordinator {
            // 空队列。
            queue: VecDeque::new(),
            // 空在途表。
            in_flight: HashMap::new(),
            // 空监控表。
            monitored: HashMap::new(),
            // 世代号从 0 开始。
            generation: 0,
            // worker 句柄稍后填入。
            worker: None,
        };

        // 组装服务（Arc 包裹 Inner）。
        let service = Self {
            inner: Arc::new(DeploymentServiceInner {
                // 依赖注入。
                process_manager,
                readiness_probe,
                health_probe,
                event_publisher,
                // 监控错误发送端。
                monitor_error_tx,
                // 协调器用 Mutex + Arc 包裹。
                coordinator: Arc::new(Mutex::new(coordinator)),
                // Notify 新建。
                queue_ready: Notify::new(),
                // 关闭广播发送端。
                shutdown,
            }),
        };

        // ---- 启动队列 worker（带“启动门闩”避免竞态）----
        // oneshot 作为门闩：worker 先等信号，确保 worker 句柄先被登记到 coordinator，
        // 再放行 worker 进入循环。否则 worker 可能先跑起来，而 coordinator.worker 还没写入。
        let (worker_start_tx, worker_start_rx) = oneshot::channel();

        // 克隆 Inner 给 worker 任务。
        let worker_inner = Arc::clone(&service.inner);

        // 订阅关闭信号（worker 循环里会检查）。
        let mut worker_shutdown = service.inner.shutdown.subscribe();

        // spawn worker 任务。
        let worker = tokio::spawn(async move {
            // 收到启动信号后才进入队列循环；若发送端被丢弃（is_err）则直接结束。
            if worker_start_rx.await.is_ok() {
                // 进入队列主循环。
                Self::run_queue(worker_inner, &mut worker_shutdown).await;
            }
        });

        // 把 worker 句柄登记到协调器。
        match service.inner.coordinator.try_lock() {
            // 成功拿锁：登记句柄。
            Ok(mut coordinator) => coordinator.worker = Some(worker),
            // 构造期不应被锁；若被锁说明有 bug（比如别处提前拿了锁）。
            Err(_) => {
                // 取消任务并 panic，避免留下半初始化状态。
                worker.abort();
                panic!("deployment coordinator was locked during construction");
            }
        }

        // 放行 worker（发送启动信号）。
        if worker_start_tx.send(()).is_err() {
            // 发送失败说明 worker 已提前退出（极罕见），仅记录错误。
            tracing::error!("deployment queue worker stopped before startup");
        }

        // 返回服务与监控错误接收端。
        (service, monitor_error_rx)
    }

    // 三斜线文档：deploy 的职责。
    /// 将模型部署请求加入 FIFO 队列，并等待首次健康结果或部署错误。
    ///
    /// 同一 `model_id` 的在途调用共享部署结果。模型地址由运行时处理；本服务不下载
    /// 模型文件或校验模型摘要。
    ///
    /// # Examples
    ///
    /// ```
    /// use burncloud_node_runtime::deployment::DeploymentService;
    ///
    /// fn accepts_deployment_service(_: &DeploymentService) {}
    /// ```
    pub async fn deploy(
        // &self：服务是共享的（内部 Arc）。
        &self,
        // 模型标识。
        model_id: String,
        // 模型地址。
        model_url: String,
        // 已准备好的运行时。
        runtime: PreparedRuntime,
        // 适配器。
        adapter: Arc<dyn ModelUrlRuntimeAdapter>,
        // 返回部署结果或错误。
    ) -> Result<DeploymentOutcome, DeploymentError> {
        let model_id = model_id.trim().to_owned();
        let model_url = model_url.trim().to_owned();
        if model_id.is_empty() {
            return Err(deployment_error(
                DeploymentErrorKind::InvalidRequest,
                "model_id must not be empty".to_string(),
                None,
            ));
        }
        let parsed_url = url::Url::parse(&model_url).map_err(|error| {
            deployment_error(
                DeploymentErrorKind::InvalidRequest,
                format!("invalid model_url: {error}"),
                None,
            )
        })?;
        if !matches!(parsed_url.scheme(), "http" | "https") || parsed_url.host_str().is_none() {
            return Err(deployment_error(
                DeploymentErrorKind::InvalidRequest,
                "model_url must be an HTTP(S) URL".to_string(),
                None,
            ));
        }

        self.enqueue_and_wait(QueuedDeployment {
            model_id,
            input: DeploymentInput::ModelUrl { model_url, adapter },
            runtime,
        })
        .await
    }

    /// 使用已经校验的本地工件部署模型。
    ///
    /// 工件必须标记为 `Verified`，且其路径必须指向现存文件。适配器根据该工件和运行时
    /// 生成真实进程计划；部署服务随后启动进程、等待就绪并执行首次健康检查。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use burncloud_node_runtime::{
    ///     deployment::{DeploymentError, DeploymentService},
    ///     PreparedArtifact, PreparedRuntime, RuntimeAdapter,
    /// };
    ///
    /// async fn deploy_local(
    ///     service: &DeploymentService,
    ///     artifact: PreparedArtifact,
    ///     runtime: PreparedRuntime,
    ///     adapter: Arc<dyn RuntimeAdapter>,
    /// ) -> Result<(), DeploymentError> {
    ///     service
    ///         .deploy_prepared("local-model".to_string(), artifact, runtime, adapter)
    ///         .await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn deploy_prepared(
        &self,
        model_id: String,
        artifact: PreparedArtifact,
        runtime: PreparedRuntime,
        adapter: Arc<dyn RuntimeAdapter>,
    ) -> Result<DeploymentOutcome, DeploymentError> {
        let model_id = model_id.trim().to_owned();
        if model_id.is_empty() {
            return Err(deployment_error(
                DeploymentErrorKind::InvalidRequest,
                "model_id must not be empty".to_string(),
                None,
            ));
        }
        if !artifact.is_verified() {
            return Err(deployment_error(
                DeploymentErrorKind::InvalidRequest,
                "prepared artifact has not been verified".to_string(),
                None,
            ));
        }
        if !artifact.local_path().is_file() {
            return Err(deployment_error(
                DeploymentErrorKind::InvalidRequest,
                format!(
                    "prepared artifact is not a file: {}",
                    artifact.local_path().display()
                ),
                None,
            ));
        }

        self.enqueue_and_wait(QueuedDeployment {
            model_id,
            input: DeploymentInput::PreparedArtifact { artifact, adapter },
            runtime,
        })
        .await
    }

    async fn enqueue_and_wait(
        &self,
        request: QueuedDeployment,
    ) -> Result<DeploymentOutcome, DeploymentError> {
        match self.enqueue_or_join(request).await.await {
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

    /// 停止一个已进入运行期监控的本地模型进程。
    ///
    /// 返回 `true` 表示找到并停止了运行时，`false` 表示该模型当前没有受监控进程。
    ///
    /// # Examples
    ///
    /// ```
    /// use burncloud_node_runtime::deployment::DeploymentService;
    ///
    /// async fn stop_model(service: &DeploymentService) {
    ///     let _ = service.stop_model("model-1").await;
    /// }
    /// ```
    pub async fn stop_model(&self, model_id: &str) -> Result<bool, DeploymentError> {
        let model_id = model_id.trim();
        if model_id.is_empty() {
            return Err(deployment_error(
                DeploymentErrorKind::InvalidRequest,
                "model_id must not be empty".to_string(),
                None,
            ));
        }

        let stopped = Self::stop_old_runtime(Arc::clone(&self.inner), model_id).await?;
        if stopped.is_some() {
            let event = DeploymentEvent {
                model_id: model_id.to_string(),
                kind: DeploymentEventKind::Stopped,
                state: Some(NodeState::Absent),
                error_kind: None,
                local_endpoint: None,
                pid: None,
                error: None,
                cleanup_error: None,
            };
            if let Err(error) =
                Self::publish_with_retry(self.inner.event_publisher.as_ref(), event).await
            {
                tracing::error!(%error, model_id, "runtime stopped but stop event could not be published");
            }
        }
        Ok(stopped.is_some())
    }

    // ------------------------------------------------------------------------
    // 队列主循环：串行取出并执行部署
    // ------------------------------------------------------------------------
    async fn run_queue(
        // 共享 Inner。
        inner: Arc<DeploymentServiceInner>,
        // 关闭信号接收端。
        shutdown: &mut watch::Receiver<bool>,
    ) {
        // 无限循环，直到收到关闭信号。
        loop {
            // 已收到关闭信号则退出。
            if *shutdown.borrow() {
                return;
            }

            // 尝试取出下一个可执行请求（持锁时间最短，避免阻塞其他锁使用者）。
            let request = {
                // 拿锁。
                let mut coordinator = inner.coordinator.lock().await;
                // 取下一个（可能 None）。
                take_next_deployment(&mut coordinator)
            };

            // 若取到请求。
            if let Some(request) = request {
                // 克隆 model_id，稍后回填结果时用。
                let model_id = request.model_id.clone();

                // 执行部署；期间若收到 shutdown 则直接退出。
                // 用 select! 保证 shutdown 能立即打断长时间运行的部署。
                let result = tokio::select! {
                    // 正常执行部署。
                    result = Self::run_deployment(Arc::clone(&inner), request) => result,
                    // 收到关闭信号：直接退出循环。
                    _ = shutdown.changed() => return,
                };

                // 回填结果给所有等待者，并清理在途记录。
                let mut coordinator = inner.coordinator.lock().await;
                // 若在途记录还在，回填结果。
                if let Some(in_flight) = coordinator.in_flight.get_mut(&model_id) {
                    // mem::take 取走 waiters，避免移动整个 in_flight。
                    complete_waiters(mem::take(&mut in_flight.waiters), result);
                }
                // 清理在途记录。
                coordinator.in_flight.remove(&model_id);
            } else {
                // 队列空（或队首未就绪）：等待“有新任务”或“关闭”信号。
                // 不会忙等：Notify 只有 notify_one/notify_waiters 才唤醒。
                tokio::select! {
                    // 有新任务通知。
                    _ = inner.queue_ready.notified() => {}
                    // 有关闭信号：退出。
                    _ = shutdown.changed() => return,
                }
            }
        }
    }

    // ------------------------------------------------------------------------
    // 执行单次部署：停旧 -> 规划 -> 启动 -> 等就绪 -> 健康检查 -> 发布结果 -> 开监控
    // ------------------------------------------------------------------------
    async fn run_deployment(
        // 共享 Inner。
        inner: Arc<DeploymentServiceInner>,
        // 本次请求（已从队列取出）。
        request: QueuedDeployment,
    ) -> Result<DeploymentOutcome, DeploymentError> {
        // 1) 若该模型已有旧运行时在跑，先停掉，拿到被保留的旧状态。
        //    失败时（如停旧失败）直接返回错误，不继续启新。
        //    ? 运算符：错误直接冒泡。
        let preserved_state = Self::stop_old_runtime(Arc::clone(&inner), &request.model_id).await?;

        // 2) 发布 Planning 进度事件（带 preserved_state，让外部知道旧状态被保留）。
        //    发布失败 => 返回 EventPublishFailed（此时还未启动进程，无需清理）。
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
            // 把发布错误翻译成 DeploymentError 返回。
            return Err(event_publish_error(error, None));
        }

        // 3) 让对应适配器根据部署输入生成本地进程计划。
        let plan = match request.input.plan(&request.runtime).await {
            // 规划成功：拿到 plan。
            Ok(plan) => plan,
            // 规划失败：构造 PlanFailed 错误，发布 DeploymentFailed 事件，返回错误。
            Err(error) => {
                // 构造错误。
                let mut deployment_error =
                    deployment_error(DeploymentErrorKind::PlanFailed, error.to_string(), None);
                // 发布失败事件（此时还没有进程，无需清理）。
                publish_deployment_failure(
                    &inner,
                    &request.model_id,
                    &mut deployment_error,
                    None,
                    None,
                )
                .await;
                // 返回错误。
                return Err(deployment_error);
            }
        };

        // 4) 发布 Starting 进度事件（带 plan）。
        //    发布失败 => 返回 EventPublishFailed（此时还未启动进程，无需清理）。
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

        // 5) 启动子进程。
        let handle = match inner.process_manager.start(plan.process.clone()).await {
            // 启动成功：拿到句柄。
            Ok(handle) => handle,
            // 启动失败：构造 ProcessStartFailed 错误，发布 DeploymentFailed 事件，
            // 返回错误（进程未启动，无需清理）。
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

        // 6) 发布 WaitingReady 进度事件（带 plan + handle）。
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
            // 事件发布失败：进程已启动，必须清理（stop）后返回。
            let error = event_publish_error(error, None);
            return Err(Self::cleanup_failed_runtime(
                inner.process_manager.as_ref(),
                handle,
                error,
            )
            .await);
        }

        // 7) 等待就绪（readiness 探测，会阻塞直到就绪或永久错误）。
        if let Err(error) = inner
            .readiness_probe
            .wait_ready(plan.readiness.clone())
            .await
        {
            // 就绪失败：先清理进程（尽力 stop），再发布 DeploymentFailed 事件，
            // 最后返回带 cleanup_error 的错误。
            let error = deployment_error(
                DeploymentErrorKind::ReadinessFailed,
                error.to_string(),
                None,
            );
            // 清理进程并把清理错误附加到 error。
            let mut error =
                Self::cleanup_failed_runtime(inner.process_manager.as_ref(), handle, error).await;
            // 发布失败事件。
            publish_deployment_failure(
                &inner,
                &request.model_id,
                &mut error,
                Some(&plan),
                Some(handle),
            )
            .await;
            // 返回错误。
            return Err(error);
        }

        // 8) 发布 HealthChecking 进度事件（带 plan + handle）。
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
            // 事件发布失败：进程已启动且就绪，但事件发不出去，仍需清理后返回。
            let error = event_publish_error(error, None);
            return Err(Self::cleanup_failed_runtime(
                inner.process_manager.as_ref(),
                handle,
                error,
            )
            .await);
        }

        // 9) 首次健康检查。
        let healthy = match inner.health_probe.is_healthy(plan.readiness.clone()).await {
            // 探测成功：拿到 bool。
            Ok(healthy) => healthy,
            // 健康探测本身失败（如网络错误）：清理进程并发布 DeploymentFailed。
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

        // 10) 根据健康结果决定初始状态：
        //     - Ok(true)  -> Ready
        //     - Ok(false) -> Unhealthy（进程还在，但不健康，仍进入监控）
        let state = if healthy {
            NodeState::Ready
        } else {
            NodeState::Unhealthy
        };
        // 构造 InitialResult 事件（带 endpoint 与 pid，让外部知道怎么路由/观测）。
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
        // 发布 InitialResult（失败重试一次）。
        if let Err(error) =
            Self::publish_with_retry(inner.event_publisher.as_ref(), initial_result).await
        {
            // 发布失败：清理进程后返回 EventPublishFailed。
            let error = event_publish_error(error, None);
            return Err(Self::cleanup_failed_runtime(
                inner.process_manager.as_ref(),
                handle,
                error,
            )
            .await);
        }

        // 11) 启动运行期监控任务（周期检查进程存活 + 健康）。
        //     注意：即使 state 是 Unhealthy，也进入监控，以便后续恢复时发 StateChanged。
        Self::spawn_monitor(
            Arc::clone(&inner),
            request.model_id,
            plan.clone(),
            handle,
            state,
        )
        .await;

        // 12) 返回部署成功结果（plan / process / state）。
        Ok(DeploymentOutcome {
            plan,
            process: handle,
            state,
        })
    }

    // ------------------------------------------------------------------------
    // 停止旧运行时（若存在），返回其状态用于“保留状态”
    // ------------------------------------------------------------------------
    async fn stop_old_runtime(
        // 共享 Inner。
        inner: Arc<DeploymentServiceInner>,
        // 模型标识。
        model_id: &str,
    ) -> Result<Option<NodeState>, DeploymentError> {
        // 先查该模型是否已有受监控运行时（持锁时间短，只做查询）。
        let previous = {
            // 拿锁。
            let coordinator = inner.coordinator.lock().await;
            // 查 monitored。
            coordinator
                .monitored
                .get(model_id)
                // 拷贝出 (state, plan, handle)，避免持锁做后续异步操作。
                .map(|runtime| (runtime.state, runtime.plan.clone(), runtime.handle))
        };
        // 没有旧运行时：直接返回 None（表示无 preserved_state）。
        let Some((state, plan, handle)) = previous else {
            return Ok(None);
        };

        // 发布 StoppingOldRuntime 进度事件（带旧 state/plan/pid）。
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
        // 发布事件（失败重试一次）。
        if let Err(error) =
            Self::publish_with_retry(inner.event_publisher.as_ref(), stopping_event).await
        {
            // 事件发布失败：保留旧状态并返回错误（旧运行时保持不动，可重试）。
            return Err(event_publish_error(error, Some(state)));
        }

        // 从 monitored 中移除旧运行时（拿到所有权，便于取消任务与停止进程）。
        let Some(runtime) = inner.coordinator.lock().await.monitored.remove(model_id) else {
            // 极端竞态：查询与移除之间被别的路径移除，直接返回 None。
            return Ok(None);
        };
        // 解构出各字段。
        let MonitoredRuntime {
            handle,
            plan,
            state,
            cancel,
            task,
            // `..` 忽略其余字段（generation 不需要）。
            ..
        } = runtime;
        // 取消旧监控任务。
        cancel.send_replace(true);
        // 等它退出（失败仅告警，不影响后续停进程流程）。
        if let Err(error) = task.await {
            tracing::warn!(%error, model_id, "old runtime monitor task stopped unexpectedly");
        }

        // 停止旧进程。
        match inner.process_manager.stop(handle).await {
            // 停止成功：返回旧状态作为 preserved_state。
            Ok(()) => Ok(Some(state)),
            // 停止失败：重新挂回监控（保持可观测，避免“失联”的运行时），
            // 然后返回 OldRuntimeStopFailed 错误（带 preserved_state=Some(state)）。
            Err(error) => {
                // 重新挂监控。
                Self::spawn_monitor(
                    Arc::clone(&inner),
                    model_id.to_string(),
                    plan,
                    handle,
                    state,
                )
                .await;
                // 返回错误。
                Err(deployment_error(
                    DeploymentErrorKind::OldRuntimeStopFailed,
                    error.to_string(),
                    Some(state),
                ))
            }
        }
    }

    // ------------------------------------------------------------------------
    // 事件发布：失败重试一次（clone 一份）
    // ------------------------------------------------------------------------
    async fn publish_with_retry(
        // 事件发布器 trait 对象引用。
        event_publisher: &dyn DeploymentEventPublisher,
        // 要发布的事件。
        event: DeploymentEvent,
    ) -> Result<(), DeploymentEventPublishError> {
        // 先发一次（clone 一份给可能的重试）。
        match event_publisher.publish(event.clone()).await {
            // 首次成功：返回 Ok。
            Ok(()) => Ok(()),
            // 首次失败：用 clone 再发一次（事件实现 Clone；重试一次避免瞬时故障）。
            Err(_) => event_publisher.publish(event).await,
        }
    }

    // ------------------------------------------------------------------------
    // 清理失败运行时：尽力 stop 进程，把清理错误记到 DeploymentError
    // ------------------------------------------------------------------------
    async fn cleanup_failed_runtime(
        // 进程管理器引用。
        process_manager: &dyn ProcessManager,
        // 要停止的进程句柄。
        handle: ProcessHandle,
        // 原始错误（会被补充 cleanup_error 后返回）。
        mut error: DeploymentError,
    ) -> DeploymentError {
        // 尝试 stop；失败不覆盖原始错误，只把清理错误附加到 cleanup_error 字段。
        if let Err(cleanup_error) = process_manager.stop(handle).await {
            error.cleanup_error = Some(cleanup_error.to_string());
        }
        // 返回带 cleanup_error 的错误（原始 kind/message 保留）。
        error
    }

    // ------------------------------------------------------------------------
    // 启动运行期监控任务并登记到 monitored
    // ------------------------------------------------------------------------
    async fn spawn_monitor(
        // 共享 Inner。
        inner: Arc<DeploymentServiceInner>,
        // 模型标识。
        model_id: String,
        // 进程计划。
        plan: ProcessPlan,
        // 进程句柄。
        handle: ProcessHandle,
        // 初始状态。
        initial_state: NodeState,
    ) {
        // 断言：只允许 Ready / Unhealthy 进入监控（其他状态是非法调用）。
        assert!(matches!(
            initial_state,
            NodeState::Ready | NodeState::Unhealthy
        ));
        // 创建取消通道（初值 false）。
        let (cancel, cancel_rx) = watch::channel(false);
        // 克隆 plan 给任务用（plan 本体要存进 MonitoredRuntime）。
        let monitor_plan = plan.clone();
        // 持锁登记（持锁期间会 insert，短操作）。
        let mut coordinator = inner.coordinator.lock().await;
        // 断言：同一模型不能同时有两个监控（必须先 stop_old_runtime 移除旧的）。
        assert!(
            !coordinator.monitored.contains_key(&model_id),
            "a runtime must be stopped before its monitor is replaced"
        );
        // 世代号自增（wrapping 防溢出；u64 实际不会溢出）。
        coordinator.generation = coordinator.generation.wrapping_add(1);
        // 取出本次世代号。
        let generation = coordinator.generation;
        // 克隆 Inner 与 model_id 给任务。
        let task_inner = Arc::clone(&inner);
        let task_model_id = model_id.clone();
        // 订阅服务级关闭信号。
        let mut shutdown = inner.shutdown.subscribe();
        // 启动监控任务。
        let task = tokio::spawn(async move {
            // 运行监控循环（直到取消/关闭/失败）。
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

            // 监控任务退出后：仅当 generation 仍匹配才从 monitored 移除。
            // 作用：防止“旧任务晚退出”误删“新任务刚插入”的记录。
            {
                // 拿锁。
                let mut coordinator = task_inner.coordinator.lock().await;
                if coordinator
                    .monitored
                    .get(&task_model_id)
                    // is_some_and：存在且 generation 匹配。
                    .is_some_and(|runtime| runtime.generation == generation)
                {
                    // 移除记录。
                    coordinator.monitored.remove(&task_model_id);
                }
            }

            // 若监控返回错误：通过 monitor_error_tx 上报给外部。
            if let Err(error) = result {
                if let Err(send_error) = task_inner.monitor_error_tx.send(error) {
                    // 接收端已关闭：记录错误（不 panic，避免影响服务）。
                    tracing::error!(
                        error = %send_error.0,
                        "deployment monitor error receiver closed"
                    );
                }
            }
        });
        // 登记到 monitored（持有 cancel 发送端与 task 句柄，便于后续 stop_old_runtime 取消/await）。
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

    // ------------------------------------------------------------------------
    // 运行期监控循环：周期检查进程存活 + 健康，状态变化时发布事件
    // ------------------------------------------------------------------------
    async fn monitor_runtime(
        // 共享 Inner。
        inner: Arc<DeploymentServiceInner>,
        // 模型标识。
        model_id: String,
        // 进程计划。
        plan: ProcessPlan,
        // 进程句柄。
        handle: ProcessHandle,
        // 当前状态（可变，会在循环中更新）。
        mut state: NodeState,
        // 本次监控的世代号。
        generation: u64,
        // 取消信号接收端。
        mut cancel: watch::Receiver<bool>,
        // 服务级关闭信号接收端。
        shutdown: &mut watch::Receiver<bool>,
    ) -> Result<(), DeploymentMonitorError> {
        // 固定周期 10s：用 interval_at 从“现在 + 10s”开始，避免第一次立即 tick。
        // 用固定周期而非 sleep，是为了避免“探测耗时”叠加到下一次等待间隔中。
        let mut interval = tokio::time::interval_at(
            // 起点：现在 + 10s。
            Instant::now() + Duration::from_secs(10),
            // 周期：10s。
            Duration::from_secs(10),
        );
        // 错过 tick 时用 Delay 策略：不补发、不连发，而是延后下一次 tick。
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        // 监控循环。
        loop {
            // 已取消/已关闭：正常退出。
            if *cancel.borrow() || *shutdown.borrow() {
                return Ok(());
            }

            // 等待下一个 tick，或取消/关闭信号（保证能立即被打断）。
            tokio::select! {
                // 定时器 tick。
                _ = interval.tick() => {}
                // 取消信号。
                _ = cancel.changed() => return Ok(()),
                // 关闭信号。
                _ = shutdown.changed() => return Ok(()),
            }

            // 检查进程是否存活（可被取消/关闭打断）。
            let running = tokio::select! {
                // 调用 is_running。
                result = inner.process_manager.is_running(handle) => result,
                // 取消信号。
                _ = cancel.changed() => return Ok(()),
                // 关闭信号。
                _ = shutdown.changed() => return Ok(()),
            };

            // 根据存活结果分派。
            match running {
                // 进程已退出：上报 ProcessExited（不 stop、发布 Failed 状态）。
                //   stop_process=false：进程已退出，无需再 stop；
                //   publish_failed_state=true：让外部看到 Failed。
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
                // 存活查询失败：上报 ProcessLivenessFailed（stop、发布 Failed）。
                //   stop_process=true：查询失败后尽力 stop，避免残留；
                //   publish_failed_state=true：让外部看到 Failed。
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
                // 仍存活：继续。
                Ok(true) => {}
            }

            // 健康检查（可被取消/关闭打断）。
            let health = tokio::select! {
                // 调用 is_healthy。
                result = inner.health_probe.is_healthy(plan.readiness.clone()) => result,
                // 取消信号。
                _ = cancel.changed() => return Ok(()),
                // 关闭信号。
                _ = shutdown.changed() => return Ok(()),
            };
            // 把健康结果翻译成下一状态。
            let next_state = match health {
                // 健康：Ready。
                Ok(true) => NodeState::Ready,
                // 不健康：Unhealthy。
                Ok(false) => NodeState::Unhealthy,
                // 探测失败：上报 HealthProbeFailed（stop、发布 Failed）。
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

            let heartbeat = DeploymentEvent {
                model_id: model_id.clone(),
                kind: DeploymentEventKind::Heartbeat(next_state),
                state: Some(next_state),
                error_kind: None,
                local_endpoint: Some(plan.local_endpoint.clone()),
                pid: Some(handle.pid),
                error: None,
                cleanup_error: None,
            };
            if let Err(error) =
                Self::publish_with_retry(inner.event_publisher.as_ref(), heartbeat).await
            {
                tracing::warn!(%error, model_id, "runtime heartbeat could not be published");
            }

            // 状态未变：跳过发布（避免刷屏）。
            if next_state == state {
                continue;
            }

            // 状态变化：发布 StateChanged 事件。
            let event = monitor_state_event(&model_id, &plan, handle, next_state, None, None, None);
            // 发布事件（失败重试一次）。
            if let Err(error) =
                Self::publish_with_retry(inner.event_publisher.as_ref(), event).await
            {
                // 发布失败：上报 EventPublishFailed（stop、不发布 Failed）。
                //   publish_failed_state=false：避免“发布失败事件本身又失败”的循环。
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

            // 更新协调器中的状态；若 generation 不匹配说明已被替换，直接退出。
            let mut coordinator = inner.coordinator.lock().await;
            match coordinator.monitored.get_mut(&model_id) {
                // generation 匹配：更新状态。
                Some(runtime) if runtime.generation == generation => {
                    runtime.state = next_state;
                    state = next_state;
                }
                // 已被替换：退出。
                _ => return Ok(()),
            }
        }
    }

    // ------------------------------------------------------------------------
    // 入队或加入在途请求：返回一个 oneshot 接收端
    // ------------------------------------------------------------------------
    async fn enqueue_or_join(
        // &self：服务共享。
        &self,
        // 要入队的请求。
        request: QueuedDeployment,
    ) -> oneshot::Receiver<Result<DeploymentOutcome, DeploymentError>> {
        // 克隆 model_id（后面多处用）。
        let model_id = request.model_id.clone();
        let input_key = request.input.key();
        // 创建一次性回执通道（发送端放入 in_flight.waiters，接收端返回给调用方）。
        let (waiter, result) = oneshot::channel();
        {
            // 拿锁。
            let mut coordinator = self.inner.coordinator.lock().await;
            // 已有同模型在途：直接挂到其 waiters 上（结果共享），不重复入队。
            if let Some(in_flight) = coordinator.in_flight.get_mut(&model_id) {
                if in_flight.input_key != input_key {
                    if waiter
                        .send(Err(deployment_error(
                        DeploymentErrorKind::InvalidRequest,
                        "a deployment for this model_id is already running with a different input"
                            .to_string(),
                        None,
                        )))
                        .is_err()
                    {
                        tracing::debug!("deployment waiter closed before conflicting request response");
                    }
                    return result;
                }
                // 追加等待者。
                in_flight.waiters.push(waiter);
                // 直接返回接收端。
                return result;
            }

            // 新建在途记录并入队。
            coordinator.in_flight.insert(
                model_id.clone(),
                InFlightDeployment {
                    input_key,
                    // 尚未执行。
                    is_executing: false,
                    // Queued 事件尚未发布。
                    queue_event_published: false,
                    // 初始只有本等待者。
                    waiters: vec![waiter],
                },
            );
            // 追加到队列尾。
            coordinator.queue.push_back(request);
        }

        // 构造 Queued 事件。
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

        // 发布 Queued 事件；失败则回滚入队并通知所有等待者。
        if let Err(error) =
            Self::publish_with_retry(self.inner.event_publisher.as_ref(), queued_event).await
        {
            // 拿锁回滚。
            let mut coordinator = self.inner.coordinator.lock().await;
            // 从队列移除该请求（按 model_id retain 掉）。
            coordinator
                .queue
                .retain(|queued| queued.model_id != model_id);
            // 记录被保留的旧状态（若该模型已有旧运行时）。
            let preserved_state = coordinator
                .monitored
                .get(&model_id)
                .map(|runtime| runtime.state);
            // 构造 EventPublishFailed 错误。
            let deployment_error = DeploymentError {
                kind: DeploymentErrorKind::EventPublishFailed,
                message: error.to_string(),
                cleanup_error: None,
                notification_error: None,
                preserved_state,
            };
            // 通知所有等待者失败。
            if let Some(in_flight) = coordinator.in_flight.get_mut(&model_id) {
                complete_waiters(mem::take(&mut in_flight.waiters), Err(deployment_error));
            }
            // 清理在途记录。
            coordinator.in_flight.remove(&model_id);
            // 释放锁。
            drop(coordinator);
            // 唤醒 worker（让它处理可能的其他任务）。
            self.inner.queue_ready.notify_one();
            // 返回接收端。
            return result;
        }

        // 发布成功：标记 queue_event_published（take_next_deployment 依赖此标记）。
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
        // 唤醒 worker 去执行。
        self.inner.queue_ready.notify_one();
        // 返回接收端。
        result
    }
}

// ============================================================================
// Drop：服务丢弃时广播关闭
// ============================================================================

impl Drop for DeploymentService {
    fn drop(&mut self) {
        // 置 true：worker 与监控任务据此退出。
        // Drop 无法等待异步任务；进程最终如何清理由注入的进程管理器决定。
        self.inner.shutdown.send_replace(true);
    }
}

// ============================================================================
// 自由函数（辅助）
// ============================================================================

// 把结果回填给所有等待者。
fn complete_waiters(
    // 等待者列表。
    waiters: Vec<oneshot::Sender<Result<DeploymentOutcome, DeploymentError>>>,
    // 要广播的结果。
    result: Result<DeploymentOutcome, DeploymentError>,
) {
    // 逐个发送。
    for waiter in waiters {
        // 发送失败说明调用方已不等待（如 future 被 drop），仅记录 debug 日志。
        if waiter.send(result.clone()).is_err() {
            tracing::debug!("deployment result waiter closed before completion");
        }
    }
}

// 从队列取出下一个可执行请求。
// 前置条件（“就绪”）：队首请求的 Queued 事件已发布，且尚未在执行。
fn take_next_deployment(coordinator: &mut DeploymentCoordinator) -> Option<QueuedDeployment> {
    // 判断队首是否“就绪”。
    let ready = coordinator.queue.front().is_some_and(|request| {
        coordinator
            .in_flight
            .get(&request.model_id)
            // 已发布且未执行。
            .is_some_and(|in_flight| in_flight.queue_event_published && !in_flight.is_executing)
    });
    // 未就绪：返回 None。
    if !ready {
        return None;
    }

    // 弹出队首并标记为执行中（防止重复取）。
    let request = coordinator.queue.pop_front()?;
    if let Some(in_flight) = coordinator.in_flight.get_mut(&request.model_id) {
        in_flight.is_executing = true;
    }
    // 返回请求。
    Some(request)
}

// 发布 Progress 事件（带可选 state/plan/handle）。
async fn publish_progress(
    // 共享 Inner。
    inner: &DeploymentServiceInner,
    // 模型标识。
    model_id: &str,
    // 阶段。
    stage: DeploymentStage,
    // 可选状态。
    state: Option<NodeState>,
    // 可选 plan。
    plan: Option<&ProcessPlan>,
    // 可选句柄。
    handle: Option<ProcessHandle>,
) -> Result<(), DeploymentEventPublishError> {
    // 构造事件。
    let event = DeploymentEvent {
        model_id: model_id.to_string(),
        kind: DeploymentEventKind::Progress(stage),
        state,
        error_kind: None,
        // plan 存在时带 endpoint。
        local_endpoint: plan.map(|plan| plan.local_endpoint.clone()),
        // handle 存在时带 pid。
        pid: handle.map(|handle| handle.pid),
        error: None,
        cleanup_error: None,
    };
    // 发布（失败重试一次）。
    DeploymentService::publish_with_retry(inner.event_publisher.as_ref(), event).await
}

// 构造 DeploymentError（各字段默认 None）。
fn deployment_error(
    // 错误种类。
    kind: DeploymentErrorKind,
    // 错误消息。
    message: String,
    // 保留状态。
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

// 把“事件发布错误”翻译成 DeploymentError（kind 固定为 EventPublishFailed）。
fn event_publish_error(
    // 发布错误。
    error: DeploymentEventPublishError,
    // 保留状态。
    preserved_state: Option<NodeState>,
) -> DeploymentError {
    deployment_error(
        DeploymentErrorKind::EventPublishFailed,
        error.to_string(),
        preserved_state,
    )
}

// 发布 DeploymentFailed 事件（带清理错误与通知错误回填）。
async fn publish_deployment_failure(
    // 共享 Inner。
    inner: &DeploymentServiceInner,
    // 模型标识。
    model_id: &str,
    // 错误（可变，可能回填 notification_error）。
    error: &mut DeploymentError,
    // 可选 plan。
    plan: Option<&ProcessPlan>,
    // 可选句柄。
    handle: Option<ProcessHandle>,
) {
    // 若已是发布失败或已保留状态，则不重复发布（避免循环/覆盖）。
    if error.kind == DeploymentErrorKind::EventPublishFailed || error.preserved_state.is_some() {
        return;
    }

    // 构造失败事件。
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
    // 发布失败时把通知错误回填到 error.notification_error（不覆盖原始错误）。
    if let Err(notification_error) =
        DeploymentService::publish_with_retry(inner.event_publisher.as_ref(), event).await
    {
        error.notification_error = Some(notification_error.to_string());
    }
}

// 监控任务失败上报：可选 stop 进程、可选发布 Failed 状态。
async fn report_monitor_failure(
    // 共享 Inner。
    inner: &DeploymentServiceInner,
    // 模型标识。
    model_id: &str,
    // 进程计划。
    plan: &ProcessPlan,
    // 进程句柄。
    handle: ProcessHandle,
    // 错误（可变，可能补充 cleanup_error）。
    mut error: DeploymentError,
    // 是否 stop 进程。
    stop_process: bool,
    // 是否发布 Failed 状态。
    publish_failed_state: bool,
) -> Result<(), DeploymentMonitorError> {
    // 记录错误种类（后面构造 monitor_error 用）。
    let kind = error.kind;
    // 需要时先清理进程（把清理错误附加到 error.cleanup_error）。
    if stop_process {
        error = DeploymentService::cleanup_failed_runtime(
            inner.process_manager.as_ref(),
            handle,
            error,
        )
        .await;
    }

    // 构造监控错误（通过 mpsc 上报给外部）。
    let mut monitor_error = DeploymentMonitorError {
        model_id: model_id.to_string(),
        pid: handle.pid,
        kind,
        message: error.message.clone(),
        cleanup_error: error.cleanup_error.clone(),
    };

    // 需要时发布 Failed 状态事件。
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
            // 发布失败：把原因拼到 message，仍返回监控错误（让外部知道“事件也没发出去”）。
            monitor_error.message = format!(
                "{}; failed to publish Failed state: {}",
                monitor_error.message, publish_error
            );
            return Err(monitor_error);
        }
        // 发布成功：本次监控失败已被“事件”消化，返回 Ok（不再上报 mpsc）。
        return Ok(());
    }

    // 不发布 Failed 状态：把错误交给 mpsc 上报通道。
    Err(monitor_error)
}

// 构造 StateChanged 事件。
fn monitor_state_event(
    // 模型标识。
    model_id: &str,
    // 进程计划。
    plan: &ProcessPlan,
    // 进程句柄。
    handle: ProcessHandle,
    // 状态。
    state: NodeState,
    // 可选错误种类。
    error_kind: Option<DeploymentErrorKind>,
    // 可选错误消息。
    error: Option<String>,
    // 可选清理错误。
    cleanup_error: Option<String>,
) -> DeploymentEvent {
    // 构造 StateChanged 事件。
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
