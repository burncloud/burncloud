use async_trait::async_trait;

/// Canonical demand presented to the Supply-owned model resolver.
///
/// Supply decides which concrete model artifact/runtime variant satisfies the
/// requested model. Platform must not make this decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelResolutionRequest {
    pub model: String,
    pub accelerator_memory_bytes: Option<u64>,
}

/// Stable result consumed by later preparation stages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModel {
    pub model: String,
    pub artifact_source: String,
    pub artifact_digest: Option<String>,
    pub runtime: String,
    pub runtime_version: Option<String>,
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
    ) -> Result<ResolvedModel, ModelResolutionError>;
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
    ) -> Result<ResolvedModel, ModelResolutionError> {
        Ok(ResolvedModel {
            artifact_source: format!("{}/fake.gguf", request.model),
            artifact_digest: Some("sha256:fake".into()),
            runtime: "llama.cpp".into(),
            runtime_version: Some("fake-v0".into()),
            model: request.model,
        })
    }
}
