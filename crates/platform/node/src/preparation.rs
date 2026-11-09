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
pub use artifact_preparer::{ArtifactPreparation, ArtifactPreparer};
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
    /// 请求准备的运行时名称。
    pub runtime: String,
    /// 可选的运行时版本约束。
    pub version: Option<String>,
}

/// 本机已准备好的运行时可执行文件描述。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRuntime {
    /// 已准备好的运行时可执行文件路径。
    pub executable: String,
}

/// 运行时准备失败时返回的错误。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimePrepareError {
    /// 运行时准备器无法提供可执行文件。
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
    /// readiness 与 health 探测器必须访问的完整 URL。
    pub endpoint: String,
}

/// 运行时适配器生成的完整机器级启动计划。
///
/// 应用编排层应直接使用此计划，不应自行生成运行时参数、端口、就绪地址
/// 或本地服务端点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessPlan {
    /// 适配器生成、并由进程管理器原样启动的程序和参数。
    pub process: ProcessSpec,
    /// 启动后用于等待就绪和执行健康检查的目标。
    pub readiness: ReadinessTarget,
    /// 服务启动后供应用层访问的本地 endpoint，不应由编排层重新拼接。
    pub local_endpoint: String,
}

/// 运行时启动计划生成失败时返回的错误。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimeAdapterError {
    /// 适配器无法把运行时和工件转换为完整启动计划。
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

/// 根据模型地址生成本地运行时进程计划的部署专用接口。
///
/// 实现方负责把地址转换成运行时参数；Node 部署服务不下载或校验模型文件。
///
/// # Examples
///
/// ```
/// use burncloud_node_runtime::ModelUrlRuntimeAdapter;
///
/// fn accepts_model_url_adapter(_: &dyn ModelUrlRuntimeAdapter) {}
/// ```
#[async_trait]
pub trait ModelUrlRuntimeAdapter: Send + Sync {
    /// 根据已准备的运行时和 HTTP(S) 模型地址生成进程启动计划。
    async fn plan_model_url(
        &self,
        runtime: &PreparedRuntime,
        model_url: &str,
    ) -> Result<ProcessPlan, RuntimeAdapterError>;
}

/// 就绪检查失败时返回的错误。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReadinessError {
    /// readiness URL 请求失败或在允许时间内未就绪。
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
    /// health URL 请求或响应解析失败。
    #[error("health check failed: {0}")]
    CheckFailed(String),
}

#[async_trait]
pub trait HealthProbe: Send + Sync {
    /// 检查指定目标当前是否健康。
    async fn is_healthy(&self, target: ReadinessTarget) -> Result<bool, HealthError>;
}
