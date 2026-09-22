//! BurnCloud Node machine-level runtime primitives.
//!
//! Invariant: this crate never binds a network port, creates a second HTTP
//! server, or owns BurnCloud model/provider/router business decisions. It only
//! provides local Node runtime lifecycle and machine-level contracts.

mod composition;
mod context;
mod contracts;
mod fake;
mod lifecycle;
mod preparation;
mod reconciler;
mod state;

pub use composition::NodeComposition;
pub use context::NodeContext;
pub use contracts::{
    AcceleratorKind, AcceleratorProfile, HardwareProbe, HardwareProbeError, HardwareProfile,
    ProcessError, ProcessHandle, ProcessManager, ProcessSpec,
};
pub use fake::{
    FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager,
    FakeReadinessProbe, FakeRuntimeAdapter, FakeRuntimePreparer,
};
pub use lifecycle::NodeRuntime;
pub use preparation::{
    ArtifactPrepareError, ArtifactPreparer, ArtifactRequest, HealthError, HealthProbe,
    PreparedArtifact, PreparedRuntime, ProcessPlan, ReadinessError, ReadinessProbe,
    ReadinessTarget, RuntimeAdapter, RuntimeAdapterError, RuntimePrepareError, RuntimePreparer,
    RuntimeRequest,
};
pub use reconciler::{DemandReconciler, ReconcileAction, ReconcileEvidence};
pub use state::{InvalidNodeTransition, NodeState, NodeStateMachine};
