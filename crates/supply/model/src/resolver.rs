use async_trait::async_trait;

/// Canonical demand presented to the Supply-owned model resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelResolutionRequest {
    pub model: String,
    pub accelerator_memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModel {
    pub model: String,
    pub artifact_source: String,
    pub artifact_digest: Option<String>,
    pub runtime: String,
    pub runtime_version: Option<String>,
}

/// 通过阶段三兼容性过滤后的模型候选结果。
///
/// 用途：返回模型名称、量化版本和资源要求，供列表展示；同时保留下载与校验所需的
/// Runtime、工件 URI 和 checksum。
///
/// # Examples
///
/// ```
/// use burncloud_supply_model::ResolvedModelCandidate;
///
/// let candidate = ResolvedModelCandidate {
///     model_name: "demo".into(),
///     quantization: "Q4_K_M".into(),
///     min_ram_bytes: 8,
///     min_vram_bytes: 4,
///     disk_size_bytes: 3,
///     runtime: "llama.cpp".into(),
///     artifact_uri: "https://example.invalid/demo.gguf".into(),
///     checksum: "sha256:example".into(),
/// };
/// assert_eq!(candidate.model_name, "demo");
/// ```
///
/// 注意事项：`disk_size_bytes` 只表示模型工件本身大小，不表示整套运行环境的磁盘需求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModelCandidate {
    /// 模型名称。
    pub model_name: String,
    /// 模型量化版本。
    pub quantization: String,
    /// 模型最低 RAM 要求，单位为字节。
    pub min_ram_bytes: u64,
    /// 模型最低 VRAM 要求，单位为字节。
    pub min_vram_bytes: u64,
    /// 模型工件占用的磁盘空间，单位为字节。
    pub disk_size_bytes: u64,
    /// 运行该 Variant 所需的 Runtime 后端。
    pub runtime: String,
    /// 模型工件下载地址。
    pub artifact_uri: String,
    /// 模型工件校验和。
    pub checksum: String,
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

/// 阶段三的候选解析结果。
///
/// 用途：承载硬件、Runtime 和磁盘过滤后的全部候选模型，直接返回给调用方展示。
///
/// # Examples
///
/// ```
/// use burncloud_supply_model::{CandidateResolutionOutcome, ResolvedModelCandidate};
///
/// let outcome =
///     CandidateResolutionOutcome::Candidates(Vec::<ResolvedModelCandidate>::new());
/// assert!(matches!(outcome, CandidateResolutionOutcome::Candidates(_)));
/// ```
///
/// 注意事项：阶段三只负责过滤和保持输入顺序，不在这里进行打分或选择推荐项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateResolutionOutcome {
    /// 过滤后仍然可用的候选模型，顺序与清单中的 Variant 顺序一致。
    Candidates(Vec<ResolvedModelCandidate>),
    /// 没有任何 Variant 同时满足硬件和 Runtime 条件。
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
