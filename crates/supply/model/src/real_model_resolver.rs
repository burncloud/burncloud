use async_trait::async_trait;
use burncloud_node_runtime::HardwareProfile;

use crate::{
    CandidateResolutionOutcome, LocalModelUnsupported, LocalModelUnsupportedReason, ModelManifest,
    ModelResolutionError, ModelResolutionOutcome, ModelResolutionRequest, ModelResolver,
    ResolvedModelCandidate, Variant,
};

/// 基于 Manifest 的模型 Resolver。
///
/// 本类型只保存调用方传入的 Manifest，并执行 Variant 兼容性过滤。它不扫描本地或远程
/// 模型文件，不筛选 GGUF 文件，不构建 artifact URI，不下载工件，也不启动 Runtime。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealModelResolver {
    manifests: Vec<ModelManifest>,
    variants: Vec<Variant>,
}

impl RealModelResolver {
    fn validate_manifests(manifests: &[ModelManifest]) -> Result<(), ModelResolutionError> {
        for manifest in manifests {
            for variant in &manifest.variants {
                if !manifest
                    .supported_backends
                    .iter()
                    .any(|backend| backend == &variant.backend)
                {
                    return Err(ModelResolutionError::ResolutionFailed(format!(
                        "MANIFEST_BACKEND_MISMATCH: model '{}' variant '{}' backend '{}' is not declared in supported_backends",
                        manifest.model_name, variant.id, variant.backend
                    )));
                }
            }
        }

        Ok(())
    }

    /// 使用调用方已经加载好的 Manifest 创建 Resolver。
    ///
    /// 用途：校验 Manifest 的后端声明，并创建只执行本地兼容性过滤的 Resolver。
    ///
    /// 参数：
    /// - `manifests`：调用方已经加载好的模型清单。
    ///
    /// 返回值：返回通过校验的 Resolver；如果某个 Variant 的 `backend` 不在所属 Manifest
    /// 的 `supported_backends` 中，则返回错误。
    ///
    /// # Examples
    ///
    /// ```
    /// use burncloud_supply_model::{ModelManifest, RealModelResolver};
    ///
    /// # fn main() -> Result<(), burncloud_supply_model::ModelResolutionError> {
    /// let resolver = RealModelResolver::new(vec![ModelManifest {
    ///     model_name: "demo".into(),
    ///     description: "测试模型".into(),
    ///     supported_backends: vec!["llama.cpp".into()],
    ///     version: "1".into(),
    ///     variants: Vec::new(),
    /// }])?;
    /// assert_eq!(resolver.manifests().len(), 1);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// 注意事项：该方法不会执行网络请求、文件扫描或 URI 构建。
    pub fn new(manifests: Vec<ModelManifest>) -> Result<Self, ModelResolutionError> {
        Self::validate_manifests(&manifests)?;
        let variants = manifests
            .iter()
            .flat_map(|manifest| manifest.variants.iter().cloned())
            .collect();

        Ok(Self {
            manifests,
            variants,
        })
    }

    /// 返回已加载的 Manifest，不执行 I/O。
    pub fn manifests(&self) -> &[ModelManifest] {
        &self.manifests
    }

    /// 返回从已加载 Manifest 中提取的全部 Variant。
    pub fn variants(&self) -> &[Variant] {
        &self.variants
    }

    /// 保留 RAM 和最大可用加速器显存要求满足设备能力的 Variant。
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

    /// 保留 Runtime 后端出现在设备支持列表中的 Variant，并保持输入顺序。
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

    // 磁盘条件属于阶段三新增规则，单独过滤可以保持阶段二硬件规则的语义不变。
    fn filter_by_disk(variants: &[Variant], disk_available_bytes: u64) -> Vec<Variant> {
        variants
            .iter()
            .filter(|variant| variant.disk_size_bytes <= disk_available_bytes)
            .cloned()
            .collect()
    }

    /// 根据设备能力和支持的 Runtime 过滤模型候选。
    ///
    /// 用途：执行阶段三的硬件、Runtime 和磁盘过滤，并直接返回全部兼容候选。
    ///
    /// 参数：
    /// - `model`：模型清单中的模型名称。
    /// - `hardware`：设备可用 RAM、VRAM 和磁盘信息；磁盘值只用于比较模型工件大小。
    /// - `supported_runtimes`：设备支持的 Runtime 后端名称列表。
    ///
    /// 返回值：返回保持原始 Variant 顺序的候选列表；没有候选时返回
    /// `CandidateResolutionOutcome::Unsupported`。
    ///
    /// # Examples
    ///
    /// ```
    /// use burncloud_node_runtime::HardwareProfile;
    /// use burncloud_supply_model::{ModelManifest, RealModelResolver};
    ///
    /// # fn main() -> Result<(), burncloud_supply_model::ModelResolutionError> {
    /// let resolver = RealModelResolver::new(vec![ModelManifest {
    ///     model_name: "demo".into(),
    ///     description: "测试模型".into(),
    ///     supported_backends: vec!["llama.cpp".into()],
    ///     version: "1".into(),
    ///     variants: Vec::new(),
    /// }])?;
    /// let hardware = HardwareProfile {
    ///     cpu_threads: 4,
    ///     memory_bytes: 8,
    ///     disk_available_bytes: 8,
    ///     accelerators: Vec::new(),
    /// };
    /// let result = resolver.resolve_with_capabilities("demo", &hardware, &[]);
    /// assert!(result.is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// 注意事项：此方法不下载模型、不写入缓存、不启动 Runtime，不打分、不排序，也不选择推荐项。
    pub fn resolve_with_capabilities(
        &self,
        model: &str,
        hardware: &HardwareProfile,
        supported_runtimes: &[String],
    ) -> Result<CandidateResolutionOutcome, ModelResolutionError> {
        if model.trim().is_empty() {
            return Err(ModelResolutionError::ResolutionFailed(
                "model name must not be empty".into(),
            ));
        }
        if supported_runtimes
            .iter()
            .any(|runtime| runtime.trim().is_empty())
        {
            return Err(ModelResolutionError::ResolutionFailed(
                "supported runtime names must not be empty".into(),
            ));
        }

        let manifest = self
            .manifests
            .iter()
            .find(|manifest| manifest.model_name == model)
            .ok_or_else(|| {
                ModelResolutionError::ResolutionFailed(format!(
                    "NO_MODEL_MANIFEST: model '{model}'"
                ))
            })?;

        let hardware_compatible = Self::filter_by_hardware(&manifest.variants, hardware);
        let runtime_compatible = Self::filter_by_runtime(&hardware_compatible, supported_runtimes);
        let compatible = Self::filter_by_disk(&runtime_compatible, hardware.disk_available_bytes);

        if compatible.is_empty() {
            return Ok(CandidateResolutionOutcome::Unsupported(
                LocalModelUnsupported {
                    model: model.into(),
                    reason: LocalModelUnsupportedReason::NoCompatibleVariant,
                },
            ));
        }

        Ok(CandidateResolutionOutcome::Candidates(
            compatible
                .into_iter()
                .map(|variant| ResolvedModelCandidate {
                    model_name: manifest.model_name.clone(),
                    quantization: variant.quantization,
                    min_ram_bytes: variant.min_ram_bytes,
                    min_vram_bytes: variant.min_vram_bytes,
                    disk_size_bytes: variant.disk_size_bytes,
                    runtime: variant.backend,
                    artifact_uri: variant.artifact_uri,
                    checksum: variant.checksum,
                })
                .collect(),
        ))
    }
}

#[async_trait]
impl ModelResolver for RealModelResolver {
    async fn resolve(
        &self,
        request: ModelResolutionRequest,
    ) -> Result<ModelResolutionOutcome, ModelResolutionError> {
        Err(ModelResolutionError::ResolutionFailed(format!(
            "real resolution is not implemented for model '{}'",
            request.model
        )))
    }
}
