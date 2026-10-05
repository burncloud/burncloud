use async_trait::async_trait;

/// Canonical model manifest owned by Supply.
///
/// A manifest describes the model identity and every artifact/runtime variant that
/// BurnCloud may later consider. Selection, caching, downloading and runtime launch
/// deliberately live outside this stage.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ModelManifest {
    /// Stable model identifier presented by callers (for example `qwen/qwen3-4b`).
    pub model: String,
    /// Optional upstream/catalog version of the model definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Concrete runnable/downloadable variants for this model.
    #[serde(default)]
    pub variants: Vec<Variant>,
}

/// Resource requirements used to describe, but not yet select, a model variant.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VariantResources {
    /// Required accelerator family when the artifact is hardware-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accelerator: Option<String>,
    /// Minimum accelerator memory required by this variant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accelerator_memory_bytes: Option<u64>,
    /// Minimum host/system memory required by this variant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_memory_bytes: Option<u64>,
}

/// One concrete artifact/runtime form of a model.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Variant {
    /// Stable identifier within the manifest (for example `gguf-q4-k-m`).
    pub id: String,
    /// Runtime/backend expected to execute the artifact (for example `llama.cpp`).
    pub backend: String,
    /// Optional backend/runtime version constraint or known-good version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_version: Option<String>,
    /// URI of the immutable or versioned model artifact.
    pub artifact_uri: String,
    /// Optional content checksum, including its algorithm prefix when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    /// Hardware/resource facts used by the later selection stage.
    #[serde(default)]
    pub resources: VariantResources,
}

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
            artifact_source: format!("{}/fake.gguf", request.model),
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

    #[test]
    fn model_manifest_json_round_trip_preserves_variant_contract() {
        let manifest = ModelManifest {
            model: "qwen/qwen3-4b".into(),
            version: Some("2026-10-05".into()),
            variants: vec![Variant {
                id: "gguf-q4-k-m".into(),
                backend: "llama.cpp".into(),
                backend_version: Some("b6500".into()),
                artifact_uri: "https://example.invalid/qwen3-4b-q4_k_m.gguf".into(),
                checksum: Some(
                    "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                        .into(),
                ),
                resources: VariantResources {
                    accelerator: Some("cuda".into()),
                    accelerator_memory_bytes: Some(6 * 1024 * 1024 * 1024),
                    system_memory_bytes: Some(8 * 1024 * 1024 * 1024),
                },
            }],
        };

        let json = serde_json::to_string(&manifest)
            .unwrap_or_else(|error| panic!("manifest should serialize to JSON: {error}"));
        let reparsed: ModelManifest = serde_json::from_str(&json)
            .unwrap_or_else(|error| panic!("manifest JSON should deserialize: {error}"));

        assert_eq!(reparsed, manifest);
        assert_eq!(reparsed.variants[0].backend, "llama.cpp");
        assert_eq!(
            reparsed.variants[0].resources.accelerator_memory_bytes,
            Some(6 * 1024 * 1024 * 1024)
        );
    }
}
