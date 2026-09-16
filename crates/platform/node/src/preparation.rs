use async_trait::async_trait;

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
