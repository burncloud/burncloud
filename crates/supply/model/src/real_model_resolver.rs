use async_trait::async_trait;
use burncloud_node_runtime::HardwareProfile;

use crate::{
    LocalModelUnsupported, LocalModelUnsupportedReason, ModelManifest, ModelResolutionError,
    ModelResolutionOutcome, ModelResolutionRequest, ModelResolver, ResolvedModel, Variant,
};

/// Manifest-backed resolver foundation.
///
/// This stage owns only manifest loading and compatibility filtering. It does
/// not inspect local files, rank variants, download artifacts, or start a
/// runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealModelResolver {
    manifests: Vec<ModelManifest>,
    variants: Vec<Variant>,
}

impl RealModelResolver {
    /// Creates a resolver from manifests that were loaded by the caller.
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

    /// Returns the preloaded manifests without performing I/O.
    pub fn manifests(&self) -> &[ModelManifest] {
        &self.manifests
    }

    /// Returns all variants extracted from the preloaded manifests.
    pub fn variants(&self) -> &[Variant] {
        &self.variants
    }

    /// Keeps variants whose RAM and maximum available accelerator VRAM needs
    /// fit the supplied machine profile.
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

    /// Keeps variants whose runtime backend appears in the supported runtime
    /// list. The order of the input variants is preserved.
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

    /// Resolves a model using the supplied machine capabilities.
    ///
    /// The existing [`ModelResolver::resolve`] contract is intentionally kept
    /// unchanged, so this capability-aware entry point carries the complete
    /// inputs needed by the local decision flow. It applies both filters and
    /// returns the first compatible variant without ranking or sorting.
    pub fn resolve_with_capabilities(
        &self,
        model: &str,
        hardware: &HardwareProfile,
        supported_runtimes: &[String],
    ) -> Result<ModelResolutionOutcome, ModelResolutionError> {
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
        let compatible = Self::filter_by_runtime(&hardware_compatible, supported_runtimes);

        let Some(variant) = compatible.first() else {
            return Ok(ModelResolutionOutcome::Unsupported(LocalModelUnsupported {
                model: model.into(),
                reason: LocalModelUnsupportedReason::NoCompatibleVariant,
            }));
        };

        Ok(ModelResolutionOutcome::Local(ResolvedModel {
            model: manifest.model_name.clone(),
            artifact_source: variant.artifact_uri.clone(),
            artifact_digest: Some(variant.checksum.clone()),
            runtime: variant.backend.clone(),
            runtime_version: None,
        }))
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

#[cfg(test)]
mod tests {
    use super::*;
    use burncloud_node_runtime::{AcceleratorKind, AcceleratorProfile};

    fn variant(id: &str, backend: &str, min_ram_bytes: u64, min_vram_bytes: u64) -> Variant {
        Variant {
            id: id.into(),
            model_size: Some("7b".into()),
            quantization: "q4_k_m".into(),
            backend: backend.into(),
            min_ram_bytes,
            min_vram_bytes,
            disk_size_bytes: 1,
            checksum: format!("sha256:{id}"),
            artifact_uri: format!("https://example.invalid/{id}.gguf"),
        }
    }

    fn manifest(name: &str, variants: Vec<Variant>) -> ModelManifest {
        ModelManifest {
            model_name: name.into(),
            description: "test manifest".into(),
            supported_backends: vec!["llama.cpp".into(), "vllm".into()],
            version: "test".into(),
            variants,
        }
    }

    fn hardware(memory_bytes: u64, vram_bytes: Option<u64>) -> HardwareProfile {
        HardwareProfile {
            cpu_threads: 8,
            memory_bytes,
            disk_available_bytes: 100,
            accelerators: vram_bytes
                .map(|memory_bytes| {
                    vec![AcceleratorProfile {
                        kind: AcceleratorKind::Other,
                        name: "test-accelerator".into(),
                        memory_bytes: Some(memory_bytes),
                    }]
                })
                .unwrap_or_default(),
        }
    }

    #[tokio::test]
    async fn resolver_can_be_instantiated_and_called() {
        let resolver = RealModelResolver::new(Vec::new());

        let result = resolver
            .resolve(ModelResolutionRequest {
                model: "deepseek-r1".into(),
                accelerator_memory_bytes: None,
            })
            .await;

        assert!(matches!(
            result,
            Err(ModelResolutionError::ResolutionFailed(_))
        ));
    }

    #[test]
    fn constructor_extracts_all_variants_from_all_manifests() {
        let resolver = RealModelResolver::new(vec![
            manifest(
                "model-a",
                vec![
                    variant("a-q4", "llama.cpp", 1, 1),
                    variant("a-q8", "llama.cpp", 1, 1),
                ],
            ),
            manifest("model-b", vec![variant("b-q4", "vllm", 1, 1)]),
        ]);

        assert_eq!(resolver.manifests().len(), 2);
        assert_eq!(
            resolver
                .variants()
                .iter()
                .map(|variant| variant.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a-q4", "a-q8", "b-q4"]
        );
    }

    #[test]
    fn hardware_filter_keeps_variant_when_ram_and_vram_are_sufficient() {
        let variants = vec![variant("compatible", "llama.cpp", 8, 6)];
        let filtered = RealModelResolver::filter_by_hardware(&variants, &hardware(8, Some(6)));

        assert_eq!(
            filtered
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["compatible"]
        );
    }

    #[test]
    fn hardware_filter_removes_variant_when_vram_is_insufficient() {
        let variants = vec![variant("needs-more-vram", "llama.cpp", 8, 8)];
        let filtered = RealModelResolver::filter_by_hardware(&variants, &hardware(8, Some(6)));

        assert!(filtered.is_empty());
    }

    #[test]
    fn hardware_filter_removes_variant_when_ram_is_insufficient() {
        let variants = vec![variant("needs-more-ram", "llama.cpp", 16, 6)];
        let filtered = RealModelResolver::filter_by_hardware(&variants, &hardware(8, Some(6)));

        assert!(filtered.is_empty());
    }

    #[test]
    fn runtime_filter_keeps_only_supported_backends() {
        let variants = vec![
            variant("llama", "llama.cpp", 1, 1),
            variant("vllm", "vllm", 1, 1),
        ];
        let supported = vec!["llama.cpp".into()];
        let filtered = RealModelResolver::filter_by_runtime(&variants, &supported);

        assert_eq!(
            filtered
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["llama"]
        );
    }

    #[test]
    fn capability_resolution_applies_hardware_and_runtime_filters() {
        let resolver = RealModelResolver::new(vec![manifest(
            "deepseek-r1",
            vec![
                variant("too-large", "llama.cpp", 8, 16),
                variant("unsupported-runtime", "vllm", 8, 4),
                variant("compatible", "llama.cpp", 8, 4),
            ],
        )]);
        let supported_runtimes = vec!["llama.cpp".into()];

        let result = resolver
            .resolve_with_capabilities("deepseek-r1", &hardware(8, Some(8)), &supported_runtimes)
            .unwrap();

        match result {
            ModelResolutionOutcome::Local(resolved) => {
                assert_eq!(resolved.model, "deepseek-r1");
                assert!(resolved.artifact_source.ends_with("compatible.gguf"));
                assert_eq!(resolved.runtime, "llama.cpp");
            }
            ModelResolutionOutcome::Unsupported(reason) => {
                panic!("expected a compatible variant, got {reason:?}");
            }
        }
    }

    #[test]
    fn capability_resolution_returns_no_compatible_variant() {
        let resolver = RealModelResolver::new(vec![manifest(
            "deepseek-r1",
            vec![variant("too-large", "llama.cpp", 8, 16)],
        )]);
        let supported_runtimes = vec!["llama.cpp".into()];

        let result = resolver
            .resolve_with_capabilities("deepseek-r1", &hardware(8, Some(8)), &supported_runtimes)
            .unwrap();

        assert!(matches!(
            result,
            ModelResolutionOutcome::Unsupported(LocalModelUnsupported {
                reason: LocalModelUnsupportedReason::NoCompatibleVariant,
                ..
            })
        ));
    }
}
