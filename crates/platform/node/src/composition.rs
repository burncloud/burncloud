use crate::{DemandReconciler, HardwareProbe, NodeContext, NodeRuntime, ProcessManager};

/// Composition root for machine-level Node runtime capabilities.
///
/// It wires lifecycle, reconciliation, hardware inspection, and process
/// lifecycle together without creating any of them internally. Real or fake
/// adapters can therefore be swapped without changing orchestration contracts.
#[derive(Debug)]
pub struct NodeComposition<H, P> {
    context: NodeContext,
    reconciler: DemandReconciler,
    hardware: H,
    processes: P,
}

impl<H, P> NodeComposition<H, P>
where
    H: HardwareProbe,
    P: ProcessManager,
{
    pub fn start(hardware: H, processes: P) -> Self {
        let context = NodeRuntime::new().start();
        Self {
            context,
            reconciler: DemandReconciler::new(),
            hardware,
            processes,
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

    pub const fn processes(&self) -> &P {
        &self.processes
    }
}
