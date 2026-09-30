use crate::{
    ArtifactPreparer, DemandReconciler, HardwareProbe, HealthProbe, NodeContext, NodeRuntime,
    ProcessManager, ReadinessProbe, RuntimePreparer,
};

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
    processes: P,
    readiness: Q,
    health: E,
}

impl<H, A, R, P, Q, E> NodeComposition<H, A, R, P, Q, E>
where
    H: HardwareProbe,
    A: ArtifactPreparer,
    R: RuntimePreparer,
    P: ProcessManager,
    Q: ReadinessProbe,
    E: HealthProbe,
{
    /// 启动节点运行时，并保存本机能力接口的实现。
    ///
    /// 调用此方法仅初始化节点上下文和依赖，不会启动网络服务或执行编排动作。
    pub fn start(
        hardware: H,
        artifacts: A,
        runtimes: R,
        processes: P,
        readiness: Q,
        health: E,
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
    pub const fn processes(&self) -> &P {
        &self.processes
    }

    /// 返回就绪状态探测接口的只读引用。
    pub const fn readiness(&self) -> &Q {
        &self.readiness
    }

    /// 返回健康状态探测接口的只读引用。
    pub const fn health(&self) -> &E {
        &self.health
    }
}
