use async_trait::async_trait;
use burncloud_node_runtime::HardwareProfile;

/// 已选模型和本次真实探测结果，供 Supply 执行硬件兼容性判断。
///
/// 硬件容量来自 Node 的公开探测接口；未知显存保留为 `None`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelResolutionRequest {
    /// 已选逻辑模型名。
    pub model: String,
    /// 本次探测的完整硬件快照，包含 RAM、磁盘和各加速器的显存。
    pub hardware: HardwareProfile,
}

/// 已完成执行选择的模型结果。
///
/// 用途：承载下游启动或编排流程使用的模型工件和 Runtime 信息。
///
/// 注意事项：`artifact_source` 和 `artifact_digest` 应来自上游准备好的模型元数据；
/// 本 crate 不负责扫描文件或拼接下载地址。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModel {
    /// 模型名称。
    pub model: String,
    /// 模型工件来源。
    pub artifact_source: String,
    /// 模型工件摘要。
    pub artifact_digest: Option<String>,
    /// 运行模型所需的 Runtime。
    pub runtime: String,
    /// Runtime 版本。
    pub runtime_version: Option<String>,
}

/// 通过阶段三兼容性过滤后的模型候选结果。
///
/// 用途：返回 Manifest 中的模型编号、Variant 编号、量化版本和资源要求，供列表展示；
/// 同时保留下载与校验所需的 Runtime、工件 URI 和 checksum。
///
/// # Examples
///
/// ```
/// use burncloud_supply_model::ResolvedModelCandidate;
///
/// let candidate = ResolvedModelCandidate {
///     model_name: "demo".into(),
///     variant_id: "demo-q4".into(),
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
    /// Manifest 中的模型编号，来源于 `ModelManifest.model_name`。
    pub model_name: String,
    /// Manifest 中的 Variant 编号，来源于 `Variant.id`。
    pub variant_id: String,
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

/// 本地设备不支持请求模型的正常结果。
///
/// 这不是 BurnCloud 路由或基础设施故障，而是当前设备没有可用 Variant。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalModelUnsupported {
    /// 不支持的模型名称。
    pub model: String,
    /// 不支持的具体原因。
    pub reason: LocalModelUnsupportedReason,
}

/// 本地模型不支持的原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalModelUnsupportedReason {
    /// 加速器显存不足。
    InsufficientAcceleratorMemory {
        /// Variant 要求的显存，单位为字节。
        required_bytes: u64,
        /// 设备检测到的显存，单位为字节。
        available_bytes: Option<u64>,
    },
    /// 没有 Variant 同时满足兼容性条件。
    NoCompatibleVariant,
}

/// 执行编排层使用的模型解析结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelResolutionOutcome {
    /// 已解析出可供下游使用的本地模型。
    Local(ResolvedModel),
    /// 当前设备不支持请求的模型。
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
    /// 没有任何 Variant 同时满足硬件、Runtime 和磁盘条件。
    Unsupported(LocalModelUnsupported),
}

/// 模型解析失败。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ModelResolutionError {
    /// 解析流程失败并附带原因。
    #[error("model resolution failed: {0}")]
    ResolutionFailed(String),
}

/// 模型解析的异步契约。
///
/// 用途：为 Node 编排层提供统一的模型解析入口。
///
/// 实现必须基于已经加载的模型元数据和调用方提供的硬件快照工作，不得在 `resolve`
/// 内部递归扫描模型文件、查询 HuggingFace 文件树或构建下载 URL。阶段三的通用候选入口
/// 是 `resolve_candidates`，它接收调用方的 Runtime 能力并返回全部兼容 Variant；
/// `RealModelResolver::resolve_with_capabilities` 是该实现的同步便捷入口。
///
/// # Examples
///
/// ```
/// use burncloud_node_runtime::HardwareProfile;
/// use burncloud_supply_model::{FakeModelResolver, ModelResolutionRequest, ModelResolver};
///
/// # #[tokio::main]
/// # async fn main() {
/// let result = FakeModelResolver
///     .resolve(ModelResolutionRequest {
///         model: "demo".into(),
///         hardware: HardwareProfile {
///             cpu: "example".into(),
///             memory_bytes: 8,
///             disk_available_bytes: 8,
///             accelerators: Vec::new(),
///         },
///     })
///     .await;
/// assert!(result.is_ok());
/// # }
/// ```
#[async_trait]
pub trait ModelResolver: Send + Sync {
    /// 解析请求并返回最终模型结果或正常的不支持结果。
    async fn resolve(
        &self,
        request: ModelResolutionRequest,
    ) -> Result<ModelResolutionOutcome, ModelResolutionError>;

    /// 根据调用方提供的 Runtime 能力返回全部兼容候选。
    ///
    /// 用途：为 Node 的候选选择和失败重试提供统一入口。
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use burncloud_node_runtime::HardwareProfile;
    /// # use burncloud_supply_model::{
    /// #     FakeModelResolver, ModelResolutionRequest, ModelResolver,
    /// # };
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let candidates = FakeModelResolver
    ///     .resolve_candidates(
    ///         ModelResolutionRequest {
    ///             model: "demo".into(),
    ///             hardware: HardwareProfile {
    ///                 cpu: "example".into(),
    ///                 memory_bytes: 8,
    ///                 disk_available_bytes: 8,
    ///                 accelerators: Vec::new(),
    ///             },
    ///         },
    ///         &["llama.cpp".into()],
    ///     )
    ///     .await?;
    /// # let _ = candidates;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// 注意事项：旧版 Resolver 如果没有覆盖此方法，会把 `resolve()` 的单个结果
    /// 适配成一个候选；阶段三的真实 Resolver 应返回完整候选列表。
    async fn resolve_candidates(
        &self,
        request: ModelResolutionRequest,
        supported_runtimes: &[String],
    ) -> Result<CandidateResolutionOutcome, ModelResolutionError> {
        if supported_runtimes
            .iter()
            .any(|runtime| runtime.trim().is_empty())
        {
            return Err(ModelResolutionError::ResolutionFailed(
                "supported runtime names must not be empty".into(),
            ));
        }

        let outcome = self.resolve(request).await?;
        match outcome {
            ModelResolutionOutcome::Local(resolved) => {
                if !supported_runtimes
                    .iter()
                    .any(|runtime| runtime == &resolved.runtime)
                {
                    return Ok(CandidateResolutionOutcome::Unsupported(
                        LocalModelUnsupported {
                            model: resolved.model,
                            reason: LocalModelUnsupportedReason::NoCompatibleVariant,
                        },
                    ));
                }

                let model_name = resolved.model.clone();
                Ok(CandidateResolutionOutcome::Candidates(vec![
                    ResolvedModelCandidate {
                        model_name,
                        variant_id: resolved.model,
                        quantization: String::new(),
                        min_ram_bytes: 0,
                        min_vram_bytes: 0,
                        disk_size_bytes: 0,
                        runtime: resolved.runtime,
                        artifact_uri: resolved.artifact_source,
                        checksum: resolved.artifact_digest.unwrap_or_default(),
                    },
                ]))
            }
            ModelResolutionOutcome::Unsupported(reason) => {
                Ok(CandidateResolutionOutcome::Unsupported(reason))
            }
        }
    }
}

/// 用于 Node 编排测试的确定性 Resolver。
///
/// 注意事项：该类型只生成测试结果，不读取 Manifest，不扫描模型文件，也不执行下载。
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
