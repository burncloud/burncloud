//! BurnCloud Node machine-level runtime primitives.
//!
//! Invariant: this crate never binds a network port, creates a second HTTP
//! server, or owns BurnCloud model/provider/router business decisions. It only
//! provides local Node runtime lifecycle and machine-level contracts.

mod composition;
mod context;
mod contracts;
mod error;
mod fake;
mod lifecycle;
mod preparation;
mod probe;
mod reconciler;
mod state;
mod types;

pub use composition::NodeComposition;
pub use context::NodeContext;
pub use contracts::{
    HardwareProbe, HardwareProbeError, HardwareProfile, ProcessError, ProcessHandle,
    ProcessManager, ProcessSpec,
};
pub use fake::{
    FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager,
    FakeReadinessProbe, FakeRuntimeAdapter, FakeRuntimePreparer,
};
pub use lifecycle::NodeRuntime;
pub use preparation::{
    ArtifactPreparation, ArtifactPrepareError, ArtifactPreparer, ArtifactSource,
    ArtifactSourceError, ArtifactVerificationStatus, Digest, DigestAlgorithm, DigestError,
    HealthError, HealthProbe, PreparedArtifact, PreparedRuntime, ProcessPlan, ReadinessError,
    ReadinessProbe, ReadinessTarget, RuntimeAdapter, RuntimeAdapterError, RuntimePrepareError,
    RuntimePreparer, RuntimeRequest,
};
pub use probe::SystemHardwareProbe;
pub use reconciler::{DemandReconciler, ReconcileAction, ReconcileEvidence};
pub use state::{InvalidNodeTransition, NodeState, NodeStateMachine};
pub use types::{AcceleratorKind, AcceleratorProfile, Platform};

/// 返回当前编译目标的平台。
///
/// # 示例
///
/// ```
/// use burncloud_node_runtime::{detect_platform, Platform};
///
/// assert_ne!(detect_platform(), Platform::Other);
/// ```
pub const fn detect_platform() -> Platform {
    types::detect_platform()
}

/// 返回 Rust 编译目标的操作系统标识。
///
/// # 示例
///
/// ```
/// use burncloud_node_runtime::current_os;
///
/// assert_eq!(current_os(), std::env::consts::OS);
/// ```
pub const fn current_os() -> &'static str {
    types::current_os()
}

/// 返回 Rust 编译目标的体系结构标识。
///
/// # 示例
///
/// ```
/// use burncloud_node_runtime::current_arch;
///
/// assert_eq!(current_arch(), std::env::consts::ARCH);
/// ```
pub const fn current_arch() -> &'static str {
    types::current_arch()
}
