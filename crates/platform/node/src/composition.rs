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
    pub fn start(
        hardware: H,
        artifacts: A,
        runtimes: R,
        processes: P,
        readiness: Q,
        health: E,
    ) -> Self {
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

    pub const fn context(&self) -> &NodeContext {
        &self.context
    }

    pub const fn reconciler(&self) -> &DemandReconciler {
        &self.reconciler
    }

    pub fn reconciler_mut(&mut self) -> &mut DemandReconciler {
        &mut self.reconciler
    }

    pub const fn hardware(&self) -> &H {
        &self.hardware
    }

    pub const fn artifacts(&self) -> &A {
        &self.artifacts
    }

    pub const fn runtimes(&self) -> &R {
        &self.runtimes
    }

    pub const fn processes(&self) -> &P {
        &self.processes
    }

    pub const fn readiness(&self) -> &Q {
        &self.readiness
    }

    pub const fn health(&self) -> &E {
        &self.health
    }
}
