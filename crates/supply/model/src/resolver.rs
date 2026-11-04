use async_trait::async_trait;
use burncloud_node_runtime::HardwareProfile;

/// 已选模型和本次真实探测结果，供 Supply 执行硬件兼容性判断。
///
/// 硬件容量来自 Node 的公开探测接口；未知显存保留为 `None`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelResolutionRequest {
    /// 已选逻辑模型名。
    pub model: String,
    /// 本次探测的完整硬件快照，包含 RAM 和各加速器的显存。
    pub hardware: HardwareProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModel {
    pub model: String,
    pub artifact_source: String,
    pub artifact_digest: Option<String>,
    pub runtime: String,
    pub runtime_version: Option<String>,
}

/// A normal, expected Supply decision: this machine has no acceptable local
/// variant for the requested model. This is not a BurnCloud routing failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalModelUnsupported {
    pub model: String,
    pub reason: LocalModelUnsupportedReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalModelUnsupportedReason {
    InsufficientAcceleratorMemory {
        required_bytes: u64,
        available_bytes: Option<u64>,
    },
    NoCompatibleVariant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelResolutionOutcome {
    Local(ResolvedModel),
    Unsupported(LocalModelUnsupported),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ModelResolutionError {
    #[error("model resolution failed: {0}")]
    ResolutionFailed(String),
}

#[async_trait]
pub trait ModelResolver: Send + Sync {
    async fn resolve(
        &self,
        request: ModelResolutionRequest,
    ) -> Result<ModelResolutionOutcome, ModelResolutionError>;
}

/// Deterministic fake used only to complete the Node skeleton before the real
/// curated catalog / manifest / hardware-driven variant selection exists.
#[derive(Debug, Clone, Default)]
pub struct FakeModelResolver;

#[async_trait]
impl ModelResolver for FakeModelResolver {
    async fn resolve(
        &self,
        request: ModelResolutionRequest,
    ) -> Result<ModelResolutionOutcome, ModelResolutionError> {
        Ok(ModelResolutionOutcome::Local(ResolvedModel {
            artifact_source: format!(
                "https://modelscope.cn/models/{}/resolve/main/fake.gguf",
                request.model
            ),
            artifact_digest: Some(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
            ),
            runtime: "llama.cpp".into(),
            runtime_version: Some("fake-v0".into()),
            model: request.model,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_is_a_normal_resolution_outcome_not_an_infrastructure_error() {
        let outcome = ModelResolutionOutcome::Unsupported(LocalModelUnsupported {
            model: "qwen-4b".into(),
            reason: LocalModelUnsupportedReason::InsufficientAcceleratorMemory {
                required_bytes: 12 * 1024 * 1024 * 1024,
                available_bytes: Some(8 * 1024 * 1024 * 1024),
            },
        });
        assert!(matches!(outcome, ModelResolutionOutcome::Unsupported(_)));
    }
}
