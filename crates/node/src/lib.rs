//! BurnCloud Node local orchestration core.
//!
//! The Node crate owns local-machine capabilities that sit behind the stable BurnCloud API
//! boundary. It intentionally does not duplicate the data-plane router: the existing router
//! remains responsible for request routing and proxy execution.

mod error;
pub mod hardware;

pub use error::{NodeError, Result};
pub use hardware::{GpuDevice, GpuProbeState, GpuProbeStatus, HardwareDetector, HardwareProfile};
