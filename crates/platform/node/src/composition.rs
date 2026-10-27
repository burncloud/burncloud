use crate::{
    deployment::DeploymentMonitorError, ArtifactPreparation, DemandReconciler, HardwareProbe,
    HealthProbe, NodeContext, NodeRuntime, ProcessManager, ReadinessProbe, RuntimePreparer,
};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Composition root for machine-level Node capabilities.
///
/// This is the machine-side socket board. It owns no model/provider/router
/// business truth and performs no orchestration by itself. Higher application
/// layers may replace every capability independently with a real or fake
/// adapter without changing the wiring contract.
#[derive(Debug)]
pub struct NodeComposition<H, A, R, P, Q, E> {
    context: NodeContext,
    reconciler: DemandReconciler,
    hardware: H,
    artifacts: A,
    runtimes: R,
    processes: Arc<P>,
    readiness: Arc<Q>,
    health: Arc<E>,
    deployment_monitor_errors: mpsc::UnboundedReceiver<DeploymentMonitorError>,
}

impl<H, A, R, P, Q, E> NodeComposition<H, A, R, P, Q, E>
where
    H: HardwareProbe,
    A: ArtifactPreparation,
    R: RuntimePreparer,
    P: ProcessManager,
    Q: ReadinessProbe,
    E: HealthProbe,
{
    /// 启动节点运行时，并保存本机能力接口的实现。
    ///
    /// 调用此方法仅初始化节点上下文和依赖，不会启动网络服务或执行编排动作。
    /// 此兼容入口不创建部署服务；需要接收部署监控错误时使用
    /// [`start_with_deployment_monitor_errors`](Self::start_with_deployment_monitor_errors)。
    pub fn start(
        hardware: H,
        artifacts: A,
        runtimes: R,
        processes: P,
        readiness: Q,
        health: E,
    ) -> Self {
        let (_, deployment_monitor_errors) = mpsc::unbounded_channel();
        Self::start_with_deployment_monitor_errors(
            hardware,
            artifacts,
            runtimes,
            Arc::new(processes),
            Arc::new(readiness),
            Arc::new(health),
            deployment_monitor_errors,
        )
    }

    /// 启动节点运行时，并接收部署服务的后台监控错误通道。
    ///
    /// 调用方需在组合根的生命周期内保留部署服务，并将相同的进程管理器和探测器 `Arc`
    /// 同时传给部署服务与此方法。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use burncloud_node_runtime::{
    ///     deployment::{
    ///         DeploymentEventPublisher, DeploymentService, HttpProbe, LocalProcessManager,
    ///     },
    ///     ArtifactPreparation, FakeArtifactPreparer, FakeHardwareProbe, FakeRuntimePreparer,
    ///     HardwareProbe, NodeComposition, RuntimePreparer,
    /// };
    ///
    /// fn wire_node(
    ///     hardware: impl HardwareProbe,
    ///     artifacts: impl ArtifactPreparation,
    ///     runtimes: impl RuntimePreparer,
    ///     publisher: Arc<dyn DeploymentEventPublisher>,
    /// ) {
    ///     let processes = Arc::new(LocalProcessManager::default());
    ///     let probe = Arc::new(HttpProbe::default());
    ///     let (_deployment, monitor_errors) = DeploymentService::new(
    ///         processes.clone(),
    ///         probe.clone(),
    ///         probe.clone(),
    ///         publisher,
    ///     );
    ///     let _node = NodeComposition::start_with_deployment_monitor_errors(
    ///         hardware,
    ///         artifacts,
    ///         runtimes,
    ///         processes,
    ///         probe.clone(),
    ///         probe,
    ///         monitor_errors,
    ///     );
    /// }
    /// ```
    pub fn start_with_deployment_monitor_errors(
        hardware: H,
        artifacts: A,
        runtimes: R,
        processes: Arc<P>,
        readiness: Arc<Q>,
        health: Arc<E>,
        deployment_monitor_errors: mpsc::UnboundedReceiver<DeploymentMonitorError>,
    ) -> Self {
        // 由生命周期入口创建已启动上下文，再将各能力实现保存在组合根中。
        let context = NodeRuntime::new().start();
        Self {
            context,
            reconciler: DemandReconciler::new(),
            hardware,
            artifacts,
            runtimes,
            processes,
            readiness,
            health,
            deployment_monitor_errors,
        }
    }

    /// 返回节点运行上下文的只读引用。
    pub const fn context(&self) -> &NodeContext {
        &self.context
    }

    /// 返回需求协调器的只读引用。
    pub const fn reconciler(&self) -> &DemandReconciler {
        &self.reconciler
    }

    /// 返回需求协调器的可变引用，以便提交阶段证据。
    pub fn reconciler_mut(&mut self) -> &mut DemandReconciler {
        &mut self.reconciler
    }

    /// 返回硬件探测接口的只读引用。
    pub const fn hardware(&self) -> &H {
        &self.hardware
    }

    /// 返回工件准备接口的只读引用。
    pub const fn artifacts(&self) -> &A {
        &self.artifacts
    }

    /// 返回运行时准备接口的只读引用。
    pub const fn runtimes(&self) -> &R {
        &self.runtimes
    }

    /// 返回本地进程管理接口的只读引用。
    pub fn processes(&self) -> &P {
        self.processes.as_ref()
    }

    /// 返回就绪状态探测接口的只读引用。
    pub fn readiness(&self) -> &Q {
        self.readiness.as_ref()
    }

    /// 返回健康状态探测接口的只读引用。
    pub fn health(&self) -> &E {
        self.health.as_ref()
    }

    /// 等待并读取一条后台部署监控错误；接收通道关闭时返回 `None`。
    ///
    /// # Examples
    ///
    /// ```
    /// use burncloud_node_runtime::{
    ///     deployment::DeploymentMonitorError,
    ///     FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager,
    ///     FakeReadinessProbe, FakeRuntimePreparer, NodeComposition,
    /// };
    ///
    /// async fn next_error(
    ///     node: &mut NodeComposition<
    ///         FakeHardwareProbe,
    ///         FakeArtifactPreparer,
    ///         FakeRuntimePreparer,
    ///         FakeProcessManager,
    ///         FakeReadinessProbe,
    ///         FakeHealthProbe,
    ///     >,
    /// ) -> Option<DeploymentMonitorError> {
    ///     node.next_deployment_monitor_error().await
    /// }
    /// ```
    pub async fn next_deployment_monitor_error(&mut self) -> Option<DeploymentMonitorError> {
        self.deployment_monitor_errors.recv().await
    }
}
