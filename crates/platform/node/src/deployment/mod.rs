//! Node 部署编排对外提供的事件、结果和错误契约。
//!
//! # Examples
//!
//! ```
//! use burncloud_node_runtime::deployment::{DeploymentEventKind, DeploymentStage};
//!
//! let event_kind = DeploymentEventKind::Progress(DeploymentStage::Queued);
//! assert_eq!(event_kind, DeploymentEventKind::Progress(DeploymentStage::Queued));
//! ```

mod types;
mod process;
mod http_probe;

pub use process::LocalProcessManager;
pub use http_probe::HttpProbe;
pub use types::{
    DeploymentError, DeploymentErrorKind, DeploymentEvent, DeploymentEventKind,
    DeploymentEventPublishError, DeploymentEventPublisher, DeploymentMonitorError,
    DeploymentOutcome, DeploymentStage,
};
