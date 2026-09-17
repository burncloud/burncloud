use burncloud_node_runtime::{
    ArtifactPreparer, HardwareProbe, HealthProbe, NodeComposition, ProcessManager, ReadinessProbe,
    RuntimePreparer,
};
use burncloud_service_models::ModelResolver;

/// Application-level composition root for BurnCloud Node.
///
/// This is the cross-domain wiring board, not a business implementation. It
/// connects the Supply-owned resolver to the machine-level Node composition.
/// Traffic attachment remains an application seam and must always target the
/// existing BurnCloud router.
#[derive(Debug)]
pub struct NodeOrchestrator<M, H, A, R, P, Q, E> {
    resolver: M,
    machine: NodeComposition<H, A, R, P, Q, E>,
}

impl<M, H, A, R, P, Q, E> NodeOrchestrator<M, H, A, R, P, Q, E>
where
    M: ModelResolver,
    H: HardwareProbe,
    A: ArtifactPreparer,
    R: RuntimePreparer,
    P: ProcessManager,
    Q: ReadinessProbe,
    E: HealthProbe,
{
    pub const fn new(resolver: M, machine: NodeComposition<H, A, R, P, Q, E>) -> Self {
        Self { resolver, machine }
    }

    pub const fn resolver(&self) -> &M { &self.resolver }

    pub const fn machine(&self) -> &NodeComposition<H, A, R, P, Q, E> { &self.machine }

    pub fn machine_mut(&mut self) -> &mut NodeComposition<H, A, R, P, Q, E> {
        &mut self.machine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use burncloud_node_runtime::{
        FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager,
        FakeReadinessProbe, FakeRuntimePreparer, NodeState,
    };
    use burncloud_service_models::{FakeModelResolver, ModelResolutionRequest};

    #[tokio::test]
    async fn application_composition_connects_supply_to_machine_without_owning_either() {
        let machine = NodeComposition::start(
            FakeHardwareProbe::default(),
            FakeArtifactPreparer,
            FakeRuntimePreparer,
            FakeProcessManager::default(),
            FakeReadinessProbe,
            FakeHealthProbe,
        );
        let orchestrator = NodeOrchestrator::new(FakeModelResolver, machine);

        let resolved = orchestrator
            .resolver()
            .resolve(ModelResolutionRequest {
                model: "qwen-4b".into(),
                accelerator_memory_bytes: Some(24 * 1024 * 1024 * 1024),
            })
            .await
            .unwrap();

        assert_eq!(resolved.model, "qwen-4b");
        assert_eq!(orchestrator.machine().reconciler().state(), NodeState::Absent);
    }
}
