use thiserror::Error;

/// Errors produced while building or operating the local BurnCloud Node state.
#[derive(Debug, Error)]
pub enum NodeError {
    #[error("failed to collect system metrics: {0}")]
    SystemMetrics(String),
}

pub type Result<T> = std::result::Result<T, NodeError>;
