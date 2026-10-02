use async_trait::async_trait;

use crate::ProcessSpec;

mod artifact_prepare_error;
mod artifact_preparer;
mod artifact_request;
mod artifact_source;
mod artifact_source_error;
mod artifact_verification_status;
mod digest;
mod digest_algorithm;
mod digest_error;
mod prepared_artifact;

pub use artifact_prepare_error::ArtifactPrepareError;
pub use artifact_preparer::ArtifactPreparer;
pub use artifact_request::ArtifactRequest;
pub use artifact_source::ArtifactSource;
pub use artifact_source_error::ArtifactSourceError;
pub use artifact_verification_status::ArtifactVerificationStatus;
pub use digest::Digest;
pub use digest_algorithm::DigestAlgorithm;
pub use digest_error::DigestError;
pub use prepared_artifact::PreparedArtifact;

/// 由上层所有者选择的机器级运行时需求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRequest {
    pub runtime: String,
    pub version: Option<String>,
}

/// 本机已准备好的运行时可执行文件描述。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRuntime {
    pub executable: String,
}

/// 运行时准备失败时返回的错误。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimePrepareError {
    #[error("runtime preparation failed: {0}")]
    PrepareFailed(String),
}

#[async_trait]
pub trait RuntimePreparer: Send + Sync {
    /// 准备请求指定的运行时，并返回可执行文件描述。
    async fn prepare(
        &self,
        request: RuntimeRequest,
    ) -> Result<PreparedRuntime, RuntimePrepareError>;
}

/// 用于验证运行时服务是否就绪的目标地址。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadinessTarget {
    pub endpoint: String,
}

/// 运行时适配器生成的完整机器级启动计划。
///
/// 应用编排层应直接使用此计划，不应自行生成运行时参数、端口、就绪地址
/// 或本地服务端点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessPlan {
    pub process: ProcessSpec,
    pub readiness: ReadinessTarget,
    pub local_endpoint: String,
}

/// 运行时启动计划生成失败时返回的错误。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimeAdapterError {
    #[error("runtime launch planning failed: {0}")]
    PlanFailed(String),
}

/// 隔离运行时专属启动细节的适配器接口。
#[async_trait]
pub trait RuntimeAdapter: Send + Sync {
    /// 根据已准备的运行时和工件生成本地进程启动计划。
    async fn plan(
        &self,
        runtime: &PreparedRuntime,
        artifact: &PreparedArtifact,
    ) -> Result<ProcessPlan, RuntimeAdapterError>;
}

/// 就绪检查失败时返回的错误。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReadinessError {
    #[error("readiness check failed: {0}")]
    CheckFailed(String),
}

#[async_trait]
pub trait ReadinessProbe: Send + Sync {
    /// 等待指定目标达到就绪状态。
    async fn wait_ready(&self, target: ReadinessTarget) -> Result<(), ReadinessError>;
}

/// 健康检查失败时返回的错误。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HealthError {
    #[error("health check failed: {0}")]
    CheckFailed(String),
}

#[async_trait]
pub trait HealthProbe: Send + Sync {
    /// 检查指定目标当前是否健康。
    async fn is_healthy(&self, target: ReadinessTarget) -> Result<bool, HealthError>;
}
