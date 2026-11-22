use axum::{
    extract::{rejection::JsonRejection, Json, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use burncloud_node_runtime::HardwareProbeError;
use burncloud_supply_model::{
    load_default_manifests, CandidateResolutionOutcome, LocalModelUnsupportedReason, ModelManifest,
    ModelResolutionError, ResolvedModelCandidate, Variant,
};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::{
    api::hardware::HardwareSnapshot,
    model_selection::presets::{IdentityStatus, Preset},
    node_orchestrator::ModelDemandError,
    AppState,
};

const MODEL_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(15);
const LLAMA_CPP: &str = "llama.cpp";

/// 构建服务端模型解析接口。
///
/// 该路由在一次请求内完成外部模型仓库文件发现、服务端资源配置合并、硬件检测和兼容 Variant 过滤。
/// 前端必须提交当前选中的仓库模型、来源和修订号；工件元数据来自上游，资源门槛优先使用匹配的内置清单配置。
pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/api/node/models/resolve", post(resolve_model))
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ModelSource {
    #[serde(rename = "modelscope")]
    ModelScope,
    #[serde(rename = "hf-mirror")]
    HfMirror,
    #[serde(rename = "huggingface")]
    HuggingFace,
}

impl ModelSource {
    fn base_url(self) -> &'static str {
        match self {
            Self::ModelScope => "https://modelscope.cn",
            Self::HfMirror => "https://hf-mirror.com",
            Self::HuggingFace => "https://huggingface.co",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::ModelScope => "ModelScope",
            Self::HfMirror => "Hugging Face 镜像",
            Self::HuggingFace => "Hugging Face",
        }
    }
}

#[derive(Debug, Deserialize)]
struct ResolveModelRequest {
    model: String,
    source: ModelSource,
    revision: String,
}

#[derive(Debug, Serialize)]
struct ResolveModelResponse {
    success: bool,
    data: ResolveModelData,
}

#[derive(Debug, Serialize)]
struct ResolveModelData {
    status: &'static str,
    model: String,
    hardware: HardwareSnapshot,
    candidates: Vec<ModelCandidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<ResolveReason>,
}

#[derive(Debug, Serialize)]
struct ModelCandidate {
    model_name: String,
    variant_id: String,
    quantization: String,
    min_ram_bytes: u64,
    min_vram_bytes: u64,
    disk_size_bytes: u64,
    runtime: String,
    artifact_uri: String,
    checksum: String,
}

#[derive(Debug, Serialize)]
struct ResolveReason {
    code: &'static str,
    message: String,
}

#[derive(Debug, Serialize)]
struct ResolveModelErrorResponse {
    success: bool,
    error: ResolveModelErrorBody,
}

#[derive(Debug, Serialize)]
struct ResolveModelErrorBody {
    code: &'static str,
    message: &'static str,
}

#[derive(Debug)]
pub(crate) enum DiscoveryError {
    Invalid(String),
    NotFound,
    Unavailable(String),
}

#[derive(Debug, Deserialize)]
struct ModelScopeEnvelope {
    #[serde(rename = "Code")]
    code: u16,
    #[serde(rename = "Data")]
    data: Option<ModelScopeData>,
}

#[derive(Debug, Deserialize)]
struct ModelScopeData {
    #[serde(rename = "Files", default)]
    files: Vec<ModelScopeFile>,
}

#[derive(Debug, Deserialize)]
struct ModelScopeFile {
    #[serde(rename = "Path")]
    path: String,
    #[serde(rename = "Size", default)]
    size: u64,
    #[serde(rename = "Sha256", default)]
    sha256: String,
    #[serde(rename = "Type", default)]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct HuggingFaceFile {
    #[serde(rename = "type")]
    kind: String,
    path: String,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    lfs: Option<HuggingFaceLfs>,
}

#[derive(Debug, Deserialize)]
struct HuggingFaceLfs {
    oid: String,
    #[serde(default)]
    size: Option<u64>,
}

#[derive(Debug)]
struct DiscoveredArtifact {
    path: String,
    size: u64,
    sha256: String,
}

impl From<ResolvedModelCandidate> for ModelCandidate {
    fn from(candidate: ResolvedModelCandidate) -> Self {
        Self {
            model_name: candidate.model_name,
            variant_id: candidate.variant_id,
            quantization: candidate.quantization,
            min_ram_bytes: candidate.min_ram_bytes,
            min_vram_bytes: candidate.min_vram_bytes,
            disk_size_bytes: candidate.disk_size_bytes,
            runtime: candidate.runtime,
            artifact_uri: candidate.artifact_uri,
            checksum: candidate.checksum,
        }
    }
}

async fn resolve_model(
    State(state): State<AppState>,
    payload: Result<Json<ResolveModelRequest>, JsonRejection>,
) -> Response {
    let (source, model, revision) = match validate_request(payload) {
        Ok(request) => request,
        Err(()) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid_model",
                "模型名称、来源或修订号无效",
            )
        }
    };
    let (manifest, _) = match discover_manifest(&source, &model, &revision, None).await {
        Ok(discovery) => discovery,
        Err(error) => return discovery_error_response(error, &model, &source),
    };
    let resolution = state
        .production_node
        .resolve_selected_model_candidates(&model, vec![manifest])
        .await;
    resolution_response(model, resolution)
}

fn validate_request(
    payload: Result<Json<ResolveModelRequest>, JsonRejection>,
) -> Result<(ModelSource, String, String), ()> {
    let Json(request) = payload.map_err(|error| {
        tracing::debug!(%error, "模型解析请求体无效");
    })?;
    let model = request.model.trim().to_owned();
    let revision = request.revision.trim().to_owned();
    if !valid_model_id(&model) || !valid_segment(&revision) {
        return Err(());
    }
    Ok((request.source, model, revision))
}

fn discovery_error_response(error: DiscoveryError, model: &str, source: &ModelSource) -> Response {
    match error {
        DiscoveryError::Invalid(reason) => {
            tracing::debug!(%reason, "模型仓库参数无效");
            error_response(
                StatusCode::BAD_REQUEST,
                "invalid_model",
                "模型名称、来源或修订号无效",
            )
        }
        DiscoveryError::NotFound => error_response(
            StatusCode::NOT_FOUND,
            "model_repository_not_found",
            "模型仓库或指定修订不存在",
        ),
        DiscoveryError::Unavailable(reason) => {
            tracing::warn!(%reason, model = %model, source = source.label(), "模型仓库文件发现失败");
            error_response(
                StatusCode::BAD_GATEWAY,
                "model_repository_unavailable",
                "模型仓库暂时不可访问",
            )
        }
    }
}

fn resolution_response(
    model: String,
    resolution: anyhow::Result<(
        burncloud_node_runtime::HardwareProfile,
        CandidateResolutionOutcome,
    )>,
) -> Response {
    match resolution {
        Ok((hardware, CandidateResolutionOutcome::Candidates(candidates))) => {
            success_response(ResolveModelData {
                status: "resolved",
                model,
                hardware: hardware.into(),
                candidates: candidates.into_iter().map(ModelCandidate::from).collect(),
                reason: None,
            })
        }
        Ok((hardware, CandidateResolutionOutcome::Unsupported(unsupported))) => {
            success_response(ResolveModelData {
                status: "unsupported",
                model: unsupported.model,
                hardware: hardware.into(),
                candidates: Vec::new(),
                reason: Some(resolve_reason(&unsupported.reason)),
            })
        }
        Err(error) => resolution_error_response(error),
    }
}

async fn discover_manifest(
    source: &ModelSource,
    model: &str,
    revision: &str,
    preset: Option<&'static Preset>,
) -> Result<(ModelManifest, Vec<(String, u64)>), DiscoveryError> {
    let client = Client::builder()
        .timeout(MODEL_DISCOVERY_TIMEOUT)
        .build()
        .map_err(|error| DiscoveryError::Unavailable(error.to_string()))?;
    discover_manifest_at(&client, *source, source.base_url(), model, revision, preset).await
}

/// 从指定站点读取有效 GGUF 清单，并返回每个 Variant 对应的真实文件大小。
///
/// # Examples
/// 传入 `ModelScope`、`Qwen/Qwen3-8B` 和 `master` 可发现仓库文件并生成直接 `/resolve/` 地址。
///
/// # Notes
/// 仓库页面地址不会作为工件地址；缺少有效 SHA-256 或正文件大小的文件会被排除。
pub(crate) async fn discover_manifest_at(
    client: &Client,
    source: ModelSource,
    base: &str,
    model: &str,
    revision: &str,
    preset: Option<&'static Preset>,
) -> Result<(ModelManifest, Vec<(String, u64)>), DiscoveryError> {
    let url = repository_url(source, model, revision, base)?;
    let artifacts = match source {
        ModelSource::ModelScope => discover_modelscope(client, url).await?,
        ModelSource::HfMirror | ModelSource::HuggingFace => {
            discover_huggingface(client, url).await?
        }
    };
    let sizes = artifacts
        .iter()
        .map(|artifact| (artifact.path.clone(), artifact.size))
        .collect();
    let variants = artifacts
        .into_iter()
        .map(|artifact| build_variant(source, model, revision, artifact))
        .collect::<Result<Vec<_>, _>>()?;
    let mut manifest = ModelManifest {
        model_name: model.to_owned(),
        description: format!(
            "从 {} 仓库实时发现的 GGUF 工件；资源门槛优先使用匹配的服务端内置配置，未配置的量化版本按工件大小保守估算。",
            source.label()
        ),
        supported_backends: vec![LLAMA_CPP.into()],
        version: revision.to_owned(),
        variants,
    };
    apply_builtin_resource_config(&mut manifest)?;
    apply_preset_hardware_requirements(&mut manifest, preset)?;
    Ok((manifest, sizes))
}

fn apply_builtin_resource_config(manifest: &mut ModelManifest) -> Result<(), DiscoveryError> {
    let builtin_manifests = load_default_manifests()
        .map_err(|error| DiscoveryError::Unavailable(format!("内置模型资源配置无效：{error}")))?;
    let Some(builtin) = builtin_manifests.iter().find(|candidate| {
        let repository_key = model_key(&manifest.model_name);
        model_key(&candidate.model_name) == repository_key
            || candidate.variants.iter().any(|variant| {
                artifact_repository_key(&variant.artifact_uri)
                    .is_some_and(|key| key == repository_key)
            })
    }) else {
        return Ok(());
    };

    for discovered in &mut manifest.variants {
        let configured = builtin
            .variants
            .iter()
            .find(|candidate| {
                quantization_matches(&candidate.quantization, &discovered.quantization)
            })
            .or_else(|| (builtin.variants.len() == 1).then(|| &builtin.variants[0]));
        let Some(configured) = configured else {
            continue;
        };
        // 上游文件大小是 RAM 与磁盘要求的下限，内置配置不能把它降低。
        discovered.min_ram_bytes = discovered.min_ram_bytes.max(configured.min_ram_bytes);
        discovered.min_vram_bytes = discovered.min_vram_bytes.max(configured.min_vram_bytes);
        discovered.disk_size_bytes = discovered.disk_size_bytes.max(configured.disk_size_bytes);
    }
    Ok(())
}

fn apply_preset_hardware_requirements(
    manifest: &mut ModelManifest,
    preset: Option<&'static Preset>,
) -> Result<(), DiscoveryError> {
    let Some(preset) = preset else {
        return Ok(());
    };
    if preset.identity_status == IdentityStatus::PendingValidation {
        // 未核实身份只允许显示仓库信息，不能把推测门槛当作可下载准入条件。
        return Ok(());
    }

    let vram_min = gibibytes_to_bytes(&preset.hardware.vram_min)?;
    let ram_min = gibibytes_to_bytes(&preset.hardware.ram_min)?;
    let disk_min = gibibytes_to_bytes(&preset.hardware.disk_min)?;
    for variant in &mut manifest.variants {
        variant.min_vram_bytes = variant.min_vram_bytes.max(vram_min);
        variant.min_ram_bytes = variant.min_ram_bytes.max(ram_min);
        variant.disk_size_bytes = variant.disk_size_bytes.max(disk_min);
    }
    Ok(())
}

fn gibibytes_to_bytes(value: &str) -> Result<u64, DiscoveryError> {
    let invalid = || DiscoveryError::Invalid("预置模型硬件容量无效".into());
    let number = value.strip_suffix('G').ok_or_else(invalid)?;
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || (number.contains('.')
            && (fraction.is_empty()
                || fraction.len() > 18
                || !fraction.bytes().all(|byte| byte.is_ascii_digit())))
    {
        return Err(invalid());
    }

    let whole = whole.parse::<u128>().map_err(|_| invalid())?;
    let whole_bytes = whole.checked_mul(1_u128 << 30).ok_or_else(invalid)?;
    let fraction_bytes = if fraction.is_empty() {
        0
    } else {
        let numerator = fraction.parse::<u128>().map_err(|_| invalid())?;
        let scale = 10_u128
            .checked_pow(fraction.len() as u32)
            .ok_or_else(invalid)?;
        let scaled = numerator.checked_mul(1_u128 << 30).ok_or_else(invalid)?;
        // 向上取整以避免小数 GiB 换算后低估最低硬件容量。
        scaled / scale + if scaled % scale == 0 { 0 } else { 1 }
    };
    let bytes = whole_bytes
        .checked_add(fraction_bytes)
        .ok_or_else(invalid)?;
    u64::try_from(bytes).map_err(|_| invalid())
}

fn model_key(value: &str) -> String {
    let value = value.rsplit('/').next().unwrap_or(value);
    let value = value.strip_suffix("-GGUF").unwrap_or(value);
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn artifact_repository_key(value: &str) -> Option<String> {
    let url = Url::parse(value).ok()?;
    let segments = url.path_segments()?.collect::<Vec<_>>();
    if let Some(index) = segments.iter().position(|segment| *segment == "models") {
        return segments.get(index + 2).map(|segment| model_key(segment));
    }
    segments
        .iter()
        .position(|segment| *segment == "resolve" || *segment == "tree")
        .and_then(|index| index.checked_sub(1))
        .and_then(|index| segments.get(index))
        .map(|segment| model_key(segment))
}

fn quantization_matches(configured: &str, discovered: &str) -> bool {
    let configured = quantization_families(configured);
    let discovered = quantization_families(discovered);
    configured
        .iter()
        .any(|family| discovered.iter().any(|candidate| family == candidate))
}

fn quantization_families(value: &str) -> Vec<String> {
    value
        .to_ascii_uppercase()
        .split(['_', '-', '/', ' '])
        .filter_map(|token| {
            if token.is_empty() {
                None
            } else if token.chars().all(|character| character.is_ascii_digit()) {
                Some(format!("Q{token}"))
            } else if token.starts_with("Q") || token.starts_with("IQ") || token.starts_with("MXFP")
            {
                Some(token.to_owned())
            } else {
                None
            }
        })
        .collect()
}

async fn discover_modelscope(
    client: &Client,
    url: Url,
) -> Result<Vec<DiscoveredArtifact>, DiscoveryError> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| DiscoveryError::Unavailable(error.to_string()))?;
    if response.status() == StatusCode::NOT_FOUND {
        return Err(DiscoveryError::NotFound);
    }
    if !response.status().is_success() {
        return Err(DiscoveryError::Unavailable(format!(
            "上游返回 HTTP {}",
            response.status()
        )));
    }
    let envelope: ModelScopeEnvelope = response
        .json()
        .await
        .map_err(|error| DiscoveryError::Unavailable(error.to_string()))?;
    if envelope.code == 404 {
        return Err(DiscoveryError::NotFound);
    }
    if envelope.code != 200 {
        return Err(DiscoveryError::Unavailable(format!(
            "ModelScope 返回错误码 {}",
            envelope.code
        )));
    }
    let data = envelope
        .data
        .ok_or_else(|| DiscoveryError::Unavailable("ModelScope 文件清单缺失".into()))?;
    Ok(data
        .files
        .into_iter()
        .filter(|file| file.kind.is_empty() || file.kind == "blob")
        .filter_map(|file| {
            let sha256 = normalize_sha256(&file.sha256)?;
            let path = file.path;
            if !is_gguf(&path) || !valid_artifact_path(&path) || file.size == 0 {
                return None;
            }
            Some(DiscoveredArtifact {
                path,
                size: file.size,
                sha256,
            })
        })
        .collect())
}

async fn discover_huggingface(
    client: &Client,
    url: Url,
) -> Result<Vec<DiscoveredArtifact>, DiscoveryError> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| DiscoveryError::Unavailable(error.to_string()))?;
    if response.status() == StatusCode::NOT_FOUND {
        return Err(DiscoveryError::NotFound);
    }
    if !response.status().is_success() {
        return Err(DiscoveryError::Unavailable(format!(
            "上游返回 HTTP {}",
            response.status()
        )));
    }
    let files: Vec<HuggingFaceFile> = response
        .json()
        .await
        .map_err(|error| DiscoveryError::Unavailable(error.to_string()))?;
    Ok(files
        .into_iter()
        .filter(|file| {
            file.kind == "file" && is_gguf(&file.path) && valid_artifact_path(&file.path)
        })
        .filter_map(|file| {
            let sha256 = normalize_sha256(
                file.lfs
                    .as_ref()
                    .map(|lfs| lfs.oid.as_str())
                    .unwrap_or_default(),
            )?;
            let size = file
                .size
                .or_else(|| file.lfs.as_ref().and_then(|lfs| lfs.size))?;
            (size > 0).then_some(DiscoveredArtifact {
                path: file.path,
                size,
                sha256,
            })
        })
        .collect())
}

fn repository_url(
    source: ModelSource,
    model: &str,
    revision: &str,
    base: &str,
) -> Result<Url, DiscoveryError> {
    if !valid_model_id(model) || !valid_segment(revision) {
        return Err(DiscoveryError::Invalid("模型名称或修订号无效".into()));
    }
    let (owner, name) = model
        .split_once('/')
        .ok_or_else(|| DiscoveryError::Invalid("模型名称必须包含组织和名称".into()))?;
    let mut url =
        Url::parse(base).map_err(|error| DiscoveryError::Unavailable(error.to_string()))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| DiscoveryError::Invalid("模型仓库地址无效".into()))?;
        segments.pop_if_empty();
        match source {
            ModelSource::ModelScope => {
                segments.extend(["api", "v1", "models", owner, name, "repo", "files"]);
            }
            _ => {
                segments.extend(["api", "models", owner, name, "tree", revision]);
            }
        }
    }
    match source {
        ModelSource::ModelScope => {
            url.query_pairs_mut()
                .append_pair("Revision", revision)
                .append_pair("Recursive", "true");
        }
        ModelSource::HfMirror | ModelSource::HuggingFace => {
            url.query_pairs_mut().append_pair("recursive", "true");
        }
    }
    Ok(url)
}

/// 离线目录只还原有完整身份的历史版本；不能把旧凭据赋给占位清单。
pub(crate) fn recorded_variant(
    source: ModelSource,
    model: &str,
    revision: &str,
    record: &burncloud_download::CatalogDownload,
) -> Option<Variant> {
    let sha256 = normalize_sha256(record.checksum.strip_prefix("sha256:")?)?;
    let artifact = DiscoveredArtifact {
        path: record.variant_id.clone(),
        size: record.total_bytes,
        sha256,
    };
    let variant = build_variant(source, model, revision, artifact).ok()?;
    (variant.artifact_uri == record.artifact_uri).then_some(variant)
}

fn build_variant(
    source: ModelSource,
    model: &str,
    revision: &str,
    artifact: DiscoveredArtifact,
) -> Result<Variant, DiscoveryError> {
    let artifact_uri = artifact_url(source, model, revision, &artifact.path)?;
    Ok(Variant {
        id: artifact.path.clone(),
        model_size: None,
        quantization: quantization_from_path(&artifact.path),
        backend: LLAMA_CPP.into(),
        min_ram_bytes: artifact.size,
        min_vram_bytes: 0,
        disk_size_bytes: artifact.size,
        checksum: format!("sha256:{}", artifact.sha256),
        artifact_uri,
    })
}

fn artifact_url(
    source: ModelSource,
    model: &str,
    revision: &str,
    path: &str,
) -> Result<String, DiscoveryError> {
    if path.split('/').any(|segment| !valid_path_segment(segment)) {
        return Err(DiscoveryError::Invalid("模型工件路径无效".into()));
    }
    let (owner, name) = model
        .split_once('/')
        .ok_or_else(|| DiscoveryError::Invalid("模型名称必须包含组织和名称".into()))?;
    let mut url = Url::parse(source.base_url())
        .map_err(|error| DiscoveryError::Unavailable(error.to_string()))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| DiscoveryError::Invalid("模型仓库地址无效".into()))?;
        segments.pop_if_empty();
        match source {
            ModelSource::ModelScope => {
                segments.extend(["models", owner, name, "resolve", revision]);
            }
            ModelSource::HfMirror | ModelSource::HuggingFace => {
                segments.extend([owner, name, "resolve", revision]);
            }
        }
        segments.extend(path.split('/'));
    }
    Ok(url.to_string())
}

fn quantization_from_path(path: &str) -> String {
    const LABELS: &[&str] = &[
        "MXFP4", "IQ1_S", "IQ2_XXS", "IQ2_XS", "IQ2_S", "IQ3_XXS", "IQ3_XS", "IQ3_S", "IQ4_XS",
        "IQ4_NL", "Q2_K", "Q3_K_L", "Q3_K_M", "Q3_K_S", "Q4_0", "Q4_1", "Q4_K_M", "Q4_K_S", "Q5_0",
        "Q5_1", "Q5_K_M", "Q5_K_S", "Q6_K", "Q8_0", "F16", "BF16", "F32",
    ];
    let upper = path.to_ascii_uppercase();
    LABELS
        .iter()
        .find(|label| upper.contains(**label))
        .copied()
        .unwrap_or("GGUF")
        .into()
}

fn normalize_sha256(value: &str) -> Option<String> {
    let value = value.strip_prefix("sha256:").unwrap_or(value);
    (value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| value.to_ascii_lowercase())
}

fn is_gguf(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(".gguf")
}

fn valid_model_id(value: &str) -> bool {
    value
        .split_once('/')
        .is_some_and(|(owner, name)| valid_segment(owner) && valid_segment(name))
}

fn valid_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value != "."
        && value != ".."
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':')
        })
}

fn valid_path_segment(value: &str) -> bool {
    !value.is_empty() && value != "." && value != ".." && !value.contains(['\\', '\0'])
}

fn valid_artifact_path(value: &str) -> bool {
    value.split('/').all(valid_path_segment)
}

fn success_response(data: ResolveModelData) -> Response {
    no_store(
        Json(ResolveModelResponse {
            success: true,
            data,
        })
        .into_response(),
    )
}

fn resolve_reason(reason: &LocalModelUnsupportedReason) -> ResolveReason {
    match reason {
        LocalModelUnsupportedReason::NoCompatibleVariant
        | LocalModelUnsupportedReason::InsufficientAcceleratorMemory { .. } => ResolveReason {
            code: "no_compatible_variant",
            message: "当前模型仓库没有满足当前硬件的 GGUF 版本".into(),
        },
    }
}

fn resolution_error_response(error: anyhow::Error) -> Response {
    if error.downcast_ref::<ModelDemandError>().is_some() {
        return error_response(StatusCode::BAD_REQUEST, "invalid_model", "模型名称不能为空");
    }

    if let Some(error) = error.downcast_ref::<HardwareProbeError>() {
        return match error {
            HardwareProbeError::UnsupportedPlatform(_) => error_response(
                StatusCode::NOT_IMPLEMENTED,
                "unsupported_platform",
                "当前平台暂不支持硬件检测",
            ),
            _ => error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "hardware_probe_failed",
                "硬件检测失败",
            ),
        };
    }

    if let Some(ModelResolutionError::ResolutionFailed(message)) =
        error.downcast_ref::<ModelResolutionError>()
    {
        if message.starts_with("MANIFEST_BACKEND_MISMATCH:") {
            tracing::error!(%error, "动态模型 Manifest 后端声明不一致");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "manifest_invalid",
                "模型解析清单无效",
            );
        }
    }

    tracing::error!(%error, "模型解析失败");
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "model_resolution_failed",
        "模型解析失败",
    )
}

fn error_response(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    no_store(
        (
            status,
            Json(ResolveModelErrorResponse {
                success: false,
                error: ResolveModelErrorBody { code, message },
            }),
        )
            .into_response(),
    )
}

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
#[path = "model_resolution_tests.rs"]
mod tests;
