use async_trait::async_trait;

use crate::ProcessSpec;

/// Machine-level artifact prepared for a local runtime.
///
/// This contract deliberately carries no BurnCloud model/provider/router
/// semantics. A higher-level owner decides which artifact is required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRequest {
    pub source: String,
    pub expected_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedArtifact {
    pub local_path: String,
    pub verified: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ArtifactPrepareError {
    #[error("artifact preparation failed: {0}")]
    PrepareFailed(String),
}

#[async_trait]
pub trait ArtifactPreparer: Send + Sync {
    async fn prepare(
        &self,
        request: ArtifactRequest,
    ) -> Result<PreparedArtifact, ArtifactPrepareError>;
}

/// Machine-level runtime requirement selected by a higher-level owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRequest {
    pub runtime: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRuntime {
    pub executable: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimePrepareError {
    #[error("runtime preparation failed: {0}")]
    PrepareFailed(String),
}

#[async_trait]
pub trait RuntimePreparer: Send + Sync {
    async fn prepare(
        &self,
        request: RuntimeRequest,
    ) -> Result<PreparedRuntime, RuntimePrepareError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadinessTarget {
    pub endpoint: String,
}

/// Complete machine-level launch receipt produced by a runtime adapter.
///
/// The application orchestrator must consume this plan instead of inventing
/// runtime arguments, ports, readiness URLs, or local serving endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessPlan {
    pub process: ProcessSpec,
    pub readiness: ReadinessTarget,
    pub local_endpoint: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimeAdapterError {
    #[error("runtime launch planning failed: {0}")]
    PlanFailed(String),
}

/// Runtime-specific adapter boundary. Real implementations may understand
/// llama.cpp/vLLM command lines; application orchestration must not.
#[async_trait]
pub trait RuntimeAdapter: Send + Sync {
    async fn plan(
        &self,
        runtime: &PreparedRuntime,
        artifact: &PreparedArtifact,
    ) -> Result<ProcessPlan, RuntimeAdapterError>;
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReadinessError {
    #[error("readiness check failed: {0}")]
    CheckFailed(String),
}

#[async_trait]
pub trait ReadinessProbe: Send + Sync {
    async fn wait_ready(&self, target: ReadinessTarget) -> Result<(), ReadinessError>;
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HealthError {
    #[error("health check failed: {0}")]
    CheckFailed(String),
}

#[async_trait]
pub trait HealthProbe: Send + Sync {
    async fn is_healthy(&self, target: ReadinessTarget) -> Result<bool, HealthError>;
}
