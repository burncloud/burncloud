use async_trait::async_trait;
use burncloud_node_runtime::HardwareProfile;

use crate::{
    LocalModelUnsupported, LocalModelUnsupportedReason, ModelManifest, ModelResolutionError,
    ModelResolutionOutcome, ModelResolutionRequest, ModelResolver, ResolvedModel, Variant,
};

/// 使用调用方提供的模型清单和真实硬件快照解析本地模型。
///
/// 按清单顺序返回首个满足 RAM、单设备显存和清单后端约束的变体，
/// 不执行下载或启动运行时。运行时是否已安装由后续准备阶段负责。
///
/// # Examples
///
/// ```no_run
/// use burncloud_node_runtime::{HardwareProbe, SystemHardwareProbe};
/// use burncloud_supply_model::{ModelResolutionRequest, ModelResolver, RealModelResolver};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let hardware = SystemHardwareProbe::new().inspect().await?;
/// let resolver = RealModelResolver::new(Vec::new());
/// let result = resolver.resolve(ModelResolutionRequest {
///     model: "selected-model".into(),
///     hardware,
/// }).await;
/// assert!(result.is_err()); // 空目录中没有该模型。
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealModelResolver {
    manifests: Vec<ModelManifest>,
    variants: Vec<Variant>,
}

impl RealModelResolver {
    /// 保存调用方已加载的清单；不执行 I/O，空目录在解析时报告模型不存在。
    pub fn new(manifests: Vec<ModelManifest>) -> Self {
        let variants = manifests
            .iter()
            .flat_map(|manifest| manifest.variants.iter().cloned())
            .collect();

        Self {
            manifests,
            variants,
        }
    }

    /// 返回已加载的清单。
    pub fn manifests(&self) -> &[ModelManifest] {
        &self.manifests
    }

    /// 返回按清单顺序展开的全部变体。
    pub fn variants(&self) -> &[Variant] {
        &self.variants
    }

    /// 根据真实 RAM 和最大可报告的单设备显存过滤变体，保留输入顺序。
    ///
    /// 无已知显存时，仅保留显存要求为零的变体；不累计多设备显存。
    pub fn filter_by_hardware(variants: &[Variant], hardware: &HardwareProfile) -> Vec<Variant> {
        let available_vram_bytes = hardware
            .accelerators
            .iter()
            .filter_map(|accelerator| accelerator.memory_bytes)
            .max()
            .unwrap_or(0);

        variants
            .iter()
            .filter(|variant| {
                variant.min_ram_bytes <= hardware.memory_bytes
                    && variant.min_vram_bytes <= available_vram_bytes
            })
            .cloned()
            .collect()
    }

    /// 按允许的后端名称过滤变体，保留输入顺序；不检查运行时是否已安装。
    pub fn filter_by_runtime(variants: &[Variant], supported_runtimes: &[String]) -> Vec<Variant> {
        variants
            .iter()
            .filter(|variant| {
                supported_runtimes
                    .iter()
                    .any(|runtime| runtime == &variant.backend)
            })
            .cloned()
            .collect()
    }

    fn resolve_model(
        &self,
        model: &str,
        hardware: &HardwareProfile,
    ) -> Result<ModelResolutionOutcome, ModelResolutionError> {
        let manifest = self
            .manifests
            .iter()
            .find(|manifest| manifest.model_name == model)
            .ok_or_else(|| {
                ModelResolutionError::ResolutionFailed(format!(
                    "model '{model}' is not in the catalog"
                ))
            })?;
        let compatible = Self::filter_by_hardware(&manifest.variants, hardware);
        let compatible = Self::filter_by_runtime(&compatible, &manifest.supported_backends);

        match compatible.into_iter().next() {
            Some(variant) => Ok(ModelResolutionOutcome::Local(ResolvedModel {
                model: model.to_owned(),
                artifact_source: variant.artifact_uri,
                artifact_digest: Some(variant.checksum),
                runtime: variant.backend,
                runtime_version: None,
            })),
            None => Ok(ModelResolutionOutcome::Unsupported(LocalModelUnsupported {
                model: model.to_owned(),
                reason: LocalModelUnsupportedReason::NoCompatibleVariant,
            })),
        }
    }
}

#[async_trait]
impl ModelResolver for RealModelResolver {
    /// 校验模型名后，用请求携带的硬件快照执行兼容性判断。
    async fn resolve(
        &self,
        request: ModelResolutionRequest,
    ) -> Result<ModelResolutionOutcome, ModelResolutionError> {
        let model = request.model.trim();
        if model.is_empty() {
            return Err(ModelResolutionError::ResolutionFailed(
                "model name must not be empty".into(),
            ));
        }
        self.resolve_model(model, &request.hardware)
    }
}
