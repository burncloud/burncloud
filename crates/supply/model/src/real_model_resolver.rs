use async_trait::async_trait;
use burncloud_node_runtime::HardwareProfile;

use crate::{
    CandidateResolutionOutcome, LocalModelUnsupported, LocalModelUnsupportedReason, ModelManifest,
    ModelResolutionError, ModelResolutionOutcome, ModelResolutionRequest, ModelResolver,
    ResolvedModel, ResolvedModelCandidate, Variant,
};

/// 使用调用方提供的模型清单和真实硬件快照解析本地模型。
///
/// 按清单顺序返回首个满足 RAM、单设备显存、磁盘和清单后端约束的变体，
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
/// let resolver = RealModelResolver::new(Vec::new())?;
/// let result = resolver
///     .resolve(ModelResolutionRequest {
///         model: "selected-model".into(),
///         hardware,
///     })
///     .await;
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
    ///
    /// # Examples
    ///
    /// ```
    /// use burncloud_supply_model::RealModelResolver;
    ///
    /// let resolver = RealModelResolver::new(Vec::new()).unwrap();
    /// assert!(resolver.manifests().is_empty());
    /// ```
    pub fn manifests(&self) -> &[ModelManifest] {
        &self.manifests
    }

    /// 返回从已加载 Manifest 中提取的全部 Variant。
    ///
    /// # Examples
    ///
    /// ```
    /// use burncloud_supply_model::RealModelResolver;
    ///
    /// let resolver = RealModelResolver::new(Vec::new()).unwrap();
    /// assert!(resolver.variants().is_empty());
    /// ```
    pub fn variants(&self) -> &[Variant] {
        &self.variants
    }

    /// 保留 RAM 和最大可用加速器显存要求满足设备能力的 Variant。
    ///
    /// # Examples
    ///
    /// ```
    /// use burncloud_node_runtime::HardwareProfile;
    /// use burncloud_supply_model::RealModelResolver;
    ///
    /// let hardware = HardwareProfile {
    ///     cpu: "example".into(),
    ///     memory_bytes: 8,
    ///     disk_available_bytes: 8,
    ///     accelerators: Vec::new(),
    /// };
    /// assert!(RealModelResolver::filter_by_hardware(&[], &hardware).is_empty());
    /// ```
    ///
    /// 注意事项：多加速器设备使用单个加速器中的最大显存进行比较；没有已知显存时按零处理。
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

    /// 保留 Runtime 后端出现在支持列表中的 Variant，并保持输入顺序。
    ///
    /// # Examples
    ///
    /// ```
    /// use burncloud_supply_model::RealModelResolver;
    ///
    /// assert!(RealModelResolver::filter_by_runtime(&[], &[]).is_empty());
    /// ```
    ///
    /// 注意事项：此方法只比较 Runtime 名称，不进行别名解析或版本协商。
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

    // 磁盘条件属于阶段三规则，单独过滤可以保持硬件过滤函数的语义不变。
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
    ///     cpu: "example".into(),
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
        let model = model.trim();
        if model.is_empty() {
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
                    variant_id: variant.id,
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
        let compatible = Self::filter_by_disk(&compatible, hardware.disk_available_bytes);

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
