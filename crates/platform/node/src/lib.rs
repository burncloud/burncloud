mod context;
mod contracts;
mod lifecycle;

pub use context::NodeContext;
pub use contracts::{
    AcceleratorKind, AcceleratorProfile, HardwareProbe, HardwareProbeError, HardwareProfile,
    ProcessError, ProcessHandle, ProcessManager, ProcessSpec,
};
pub use lifecycle::NodeRuntime;

/// Node runtime skeleton invariant:
///
/// This crate never binds a network port, creates a second HTTP server,
/// or owns BurnCloud model/provider/router business decisions.
/// It only provides local node runtime lifecycle and machine-level contracts.
