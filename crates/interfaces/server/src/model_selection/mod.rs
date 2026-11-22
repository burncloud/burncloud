//! 主服务内的模型选择接口；不创建监听器、独立进程或下载状态仓库。

pub(crate) mod presets;
mod sources;

use std::{sync::Arc, time::Duration};

use crate::api::model_resolution::{discover_manifest_at, recorded_variant, ModelSource};
use burncloud_database::Database;
use burncloud_download::{catalog_downloads, CatalogDownload, DownloadTaskStatus};

use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{sync::mpsc, task::JoinSet};

use burncloud_supply_model::Variant;

use sources::{FetchError, Merged, ModelResource, SearchItem, Source};

struct Config {
    modelscope: String,
    hf_mirror: String,
    huggingface: String,
    search_timeout: Duration,
    probe_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            modelscope: "https://modelscope.cn".into(),
            hf_mirror: "https://hf-mirror.com".into(),
            huggingface: "https://huggingface.co".into(),
            search_timeout: Duration::from_secs(8),
            probe_timeout: Duration::from_secs(10),
        }
    }
}

struct AppState {
    config: Config,
    client: reqwest::Client,
    database: Option<Arc<Database>>,
}

/// 构建由主服务统一管理的三个模型选择接口。
///
/// # Examples
/// 主服务通过 `Router::merge` 挂载路由，不需要额外监听端口。
///
/// # Notes
/// HTTP 客户端初始化失败会阻止本次应用构建，不静默降级为模拟目录。
pub(super) fn routes(database: Arc<Database>) -> anyhow::Result<Router> {
    Ok(app(
        Config::default(),
        reqwest::Client::builder().build()?,
        Some(database),
    ))
}

fn app(config: Config, client: reqwest::Client, database: Option<Arc<Database>>) -> Router {
    Router::new()
        .route("/api/models/presets", get(list_presets))
        .route("/api/models/search", get(search))
        .route("/api/models/status", get(status))
        .with_state(Arc::new(AppState {
            config,
            client,
            database,
        }))
}

fn bad_request(message: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "message": message }))).into_response()
}

async fn list_presets() -> Response {
    match presets::all() {
        Ok(items) => Json(items).into_response(),
        Err(error) => {
            tracing::error!(%error, "预置模型目录校验失败");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "code": "preset_catalog_invalid",
                    "message": "预置模型目录无效"
                })),
            )
                .into_response()
        }
    }
}

#[derive(Deserialize)]
struct SearchQuery {
    #[serde(default)]
    q: String,
}

#[derive(Serialize)]
struct SearchLine<'a> {
    done: bool,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    kind: Option<&'static str>,
    items: &'a [SearchItem],
}

fn line(
    done: bool,
    kind: Option<&'static str>,
    items: &[SearchItem],
) -> Result<String, serde_json::Error> {
    serde_json::to_string(&SearchLine { done, kind, items })
        .inspect_err(|error| tracing::error!(%error, "模型搜索快照序列化失败"))
        .map(|mut text| {
            text.push('\n');
            text
        })
}

async fn search(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Response {
    let query = query.q.trim().to_string();
    if query.is_empty() {
        return bad_request("请输入模型名称");
    }
    // 最多三个上游快照及一个最终快照，使用有界队列避免慢客户端累积响应。
    let (tx, rx) = mpsc::channel(4);
    tokio::spawn(run_search(state, query, tx));
    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    });
    (
        [
            (header::CONTENT_TYPE, "application/x-ndjson; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache, no-transform"),
            (header::HeaderName::from_static("x-accel-buffering"), "no"),
        ],
        Body::from_stream(stream),
    )
        .into_response()
}

type SearchTask = (Source, Result<Vec<SearchItem>, FetchError>);

fn spawn_search(
    tasks: &mut JoinSet<SearchTask>,
    state: &Arc<AppState>,
    query: &str,
    source: Source,
) {
    let state = state.clone();
    let query = query.to_string();
    tasks.spawn(async move {
        let result = sources::search_source(&state.client, &state.config, source, &query).await;
        (source, result)
    });
}

async fn next_result(
    tasks: &mut JoinSet<SearchTask>,
    tx: &mpsc::Sender<Result<String, serde_json::Error>>,
) -> Option<SearchTask> {
    loop {
        let joined = tokio::select! {
            // 响应关闭后立即结束编排，JoinSet 析构会取消所有上游请求。
            biased;
            () = tx.closed() => return None,
            joined = tasks.join_next() => joined,
        };
        match joined? {
            Ok(value) => return Some(value),
            Err(error) => tracing::warn!(%error, "模型搜索任务异常结束"),
        }
    }
}

async fn run_search(
    state: Arc<AppState>,
    query: String,
    tx: mpsc::Sender<Result<String, serde_json::Error>>,
) {
    let mut tasks = JoinSet::new();
    spawn_search(&mut tasks, &state, &query, Source::ModelScope);
    spawn_search(&mut tasks, &state, &query, Source::HfMirror);
    let mut merged = Merged::default();
    let mut any_success = false;
    let mut fallback_started = false;
    while let Some((source, result)) = next_result(&mut tasks, &tx).await {
        if result == Err(FetchError::Timeout) && source != Source::HuggingFace && !fallback_started
        {
            tracing::warn!(?source, "国内模型站点超时，启动一次回退查询");
            fallback_started = true;
            spawn_search(&mut tasks, &state, &query, Source::HuggingFace);
        }
        if !publish_result(source, result, &mut merged, &mut any_success, &tx).await {
            return;
        }
    }
    finish_search(tx, merged, any_success).await;
}

async fn finish_search(
    tx: mpsc::Sender<Result<String, serde_json::Error>>,
    merged: Merged,
    any_success: bool,
) {
    let kind = match (merged.items().is_empty(), any_success) {
        (false, _) => "models",
        (true, true) => "unavailable",
        (true, false) => "failed",
    };
    if tx
        .send(line(true, Some(kind), merged.items()))
        .await
        .is_err()
    {
        // 最后一帧前离开的客户端无需再接收错误响应。
        tracing::debug!("模型搜索客户端已关闭");
    }
}

async fn publish_result(
    source: Source,
    result: Result<Vec<SearchItem>, FetchError>,
    merged: &mut Merged,
    any_success: &mut bool,
    tx: &mpsc::Sender<Result<String, serde_json::Error>>,
) -> bool {
    match result {
        Ok(items) => {
            *any_success = true;
            if merged.add(items) {
                // 接收端关闭表示用户取消，不属于上游故障。
                return tx.send(line(false, None, merged.items())).await.is_ok();
            }
        }
        Err(error) => tracing::warn!(?source, ?error, "模型搜索来源不可用"),
    }
    true
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusQuery {
    preset: Option<String>,
    model: Option<String>,
    source: Option<Source>,
    revision: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ArtifactStatus {
    NotDownloaded,
    Downloading,
    Verifying,
    Downloaded,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
struct StatusVariant {
    id: String,
    quantization: String,
    runtime: String,
    artifact_source: String,
    expected_sha256: String,
    file_size_bytes: u64,
    ram_bytes: u64,
    vram_bytes: u64,
    disk_bytes: u64,
    artifact_status: ArtifactStatus,
    downloaded: bool,
    local_path: Option<String>,
    actual_sha256: Option<String>,
}

impl StatusVariant {
    fn from_variant(variant: &Variant, file_size_bytes: u64) -> Self {
        Self {
            id: variant.id.clone(),
            quantization: variant.quantization.clone(),
            runtime: variant.backend.clone(),
            artifact_source: variant.artifact_uri.clone(),
            expected_sha256: variant.checksum.clone(),
            file_size_bytes,
            ram_bytes: variant.min_ram_bytes,
            vram_bytes: variant.min_vram_bytes,
            disk_bytes: variant.disk_size_bytes,
            artifact_status: ArtifactStatus::NotDownloaded,
            downloaded: false,
            local_path: None,
            actual_sha256: None,
        }
    }
}

#[derive(Serialize)]
struct CatalogVersion {
    model: String,
    source: Source,
    revision: String,
    resource_url: String,
}

#[derive(Serialize)]
struct UnavailableReason {
    code: &'static str,
    message: &'static str,
}

#[derive(Serialize)]
struct CatalogInspection {
    #[serde(flatten)]
    version: CatalogVersion,
    #[serde(rename = "status")]
    remote_status: &'static str,
    downloadable: bool,
    unavailable_reason: Option<UnavailableReason>,
    identity_status: Option<presets::IdentityStatus>,
    artifact_status: ArtifactStatus,
    downloaded: bool,
    local_path: Option<String>,
    actual_sha256: Option<String>,
    variants: Vec<StatusVariant>,
}

struct CatalogVariants {
    variants: Vec<StatusVariant>,
    downloadable: bool,
    unavailable_reason: Option<UnavailableReason>,
}

fn apply_catalog_record(variant: &mut StatusVariant, records: &[CatalogDownload]) {
    let Some(record) = records.iter().find(|record| {
        record.variant_id == variant.id
            && record.artifact_uri == variant.artifact_source
            && record.checksum == variant.expected_sha256
    }) else {
        return;
    };

    let verified_local_file = record.status == DownloadTaskStatus::Succeeded
        && record
            .local_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty())
        && record.actual_sha256.as_deref() == Some(record.checksum.as_str());
    variant.artifact_status = match record.status {
        DownloadTaskStatus::Queued | DownloadTaskStatus::Downloading => ArtifactStatus::Downloading,
        DownloadTaskStatus::Verifying => ArtifactStatus::Verifying,
        DownloadTaskStatus::Succeeded if verified_local_file => ArtifactStatus::Downloaded,
        DownloadTaskStatus::Succeeded => ArtifactStatus::Failed,
        DownloadTaskStatus::Failed | DownloadTaskStatus::Cancelled => ArtifactStatus::Failed,
    };
    variant.downloaded = variant.artifact_status == ArtifactStatus::Downloaded;
    if variant.downloaded {
        variant.local_path.clone_from(&record.local_path);
        variant.actual_sha256.clone_from(&record.actual_sha256);
    }
}

fn artifact_rank(status: ArtifactStatus) -> u8 {
    match status {
        ArtifactStatus::NotDownloaded => 0,
        ArtifactStatus::Failed => 1,
        ArtifactStatus::Downloading => 2,
        ArtifactStatus::Verifying => 3,
        ArtifactStatus::Downloaded => 4,
    }
}

async fn catalog_variants(
    state: &AppState,
    resource: &ModelResource,
    records: &[CatalogDownload],
    preset: Option<&'static presets::Preset>,
) -> Result<CatalogVariants, crate::api::model_resolution::DiscoveryError> {
    let source = match resource.source {
        Source::ModelScope => ModelSource::ModelScope,
        Source::HfMirror => ModelSource::HfMirror,
        Source::HuggingFace => ModelSource::HuggingFace,
    };
    let discovery = discover_manifest_at(
        &state.client,
        source,
        resource.source.base(&state.config),
        &resource.model,
        &resource.revision,
        preset,
    );
    let result = tokio::time::timeout(state.config.probe_timeout, discovery).await;
    match result {
        Ok(Ok((manifest, sizes))) => {
            let mut variants = manifest
                .variants
                .iter()
                .filter_map(|variant| {
                    let file_size = sizes
                        .iter()
                        .find(|(id, _)| id == &variant.id)
                        .map(|(_, size)| *size)?;
                    let mut status_variant = StatusVariant::from_variant(variant, file_size);
                    apply_catalog_record(&mut status_variant, records);
                    Some(status_variant)
                })
                .collect::<Vec<_>>();
            variants.sort_by(|left, right| left.id.cmp(&right.id));
            let downloadable = !variants.is_empty();
            Ok(CatalogVariants {
                variants,
                downloadable,
                unavailable_reason: (!downloadable).then_some(UnavailableReason {
                    code: "no_gguf_artifact",
                    message: "该仓库修订没有可验证的 GGUF 工件",
                }),
            })
        }
        Ok(Err(crate::api::model_resolution::DiscoveryError::NotFound)) => Ok(CatalogVariants {
            variants: Vec::new(),
            downloadable: false,
            unavailable_reason: Some(UnavailableReason {
                code: "repository_not_found",
                message: "模型仓库或指定修订不存在",
            }),
        }),
        Ok(Err(crate::api::model_resolution::DiscoveryError::Unavailable(error))) => {
            tracing::debug!(%error, "上游不可用，尝试精确匹配已验证的本地工件");
            Ok(CatalogVariants {
                variants: local_fallback_variants(source, resource, records, preset)?,
                downloadable: false,
                unavailable_reason: Some(UnavailableReason {
                    code: "repository_unavailable",
                    message: "模型仓库暂时不可访问",
                }),
            })
        }
        Err(error) => {
            tracing::debug!(?error, "上游文件发现超时，尝试精确匹配已验证的本地工件");
            Ok(CatalogVariants {
                variants: local_fallback_variants(source, resource, records, preset)?,
                downloadable: false,
                unavailable_reason: Some(UnavailableReason {
                    code: "repository_unavailable",
                    message: "模型仓库暂时不可访问",
                }),
            })
        }
        Ok(Err(error @ crate::api::model_resolution::DiscoveryError::Invalid(_))) => Err(error),
    }
}

fn local_fallback_variants(
    source: ModelSource,
    resource: &ModelResource,
    records: &[CatalogDownload],
    preset: Option<&'static presets::Preset>,
) -> Result<Vec<StatusVariant>, crate::api::model_resolution::DiscoveryError> {
    let mut seen = std::collections::HashSet::new();
    let mut variants = Vec::new();
    for record in records {
        if record.status != DownloadTaskStatus::Succeeded
            || record.total_bytes == 0
            || !record
                .local_path
                .as_deref()
                .is_some_and(|path| !path.trim().is_empty())
            || record.actual_sha256.as_deref() != Some(record.checksum.as_str())
        {
            continue;
        }
        let Some(variant) =
            recorded_variant(source, &resource.model, &resource.revision, preset, record)?
        else {
            continue;
        };
        let identity = (
            variant.id.clone(),
            variant.artifact_uri.clone(),
            variant.checksum.clone(),
        );
        if !seen.insert(identity) {
            continue;
        }
        let mut status_variant = StatusVariant::from_variant(&variant, record.total_bytes);
        status_variant.artifact_status = ArtifactStatus::Downloaded;
        status_variant.downloaded = true;
        status_variant.local_path.clone_from(&record.local_path);
        status_variant
            .actual_sha256
            .clone_from(&record.actual_sha256);
        variants.push(status_variant);
    }
    variants.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(variants)
}

fn status_response(
    resource: &ModelResource,
    resource_url: &Url,
    remote_status: &'static str,
    identity_status: Option<presets::IdentityStatus>,
    catalog: CatalogVariants,
) -> Response {
    let mut artifact_status = ArtifactStatus::NotDownloaded;
    let mut local_path = None;
    let mut actual_sha256 = None;
    for variant in &catalog.variants {
        if artifact_rank(variant.artifact_status) > artifact_rank(artifact_status) {
            artifact_status = variant.artifact_status;
            local_path.clone_from(&variant.local_path);
            actual_sha256.clone_from(&variant.actual_sha256);
        }
    }
    let downloaded = artifact_status == ArtifactStatus::Downloaded
        && local_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty())
        && actual_sha256.is_some();
    Json(CatalogInspection {
        version: CatalogVersion {
            model: resource.model.clone(),
            source: resource.source,
            revision: resource.revision.clone(),
            resource_url: resource_url.to_string(),
        },
        remote_status,
        downloadable: catalog.downloadable,
        unavailable_reason: catalog.unavailable_reason,
        identity_status,
        artifact_status,
        downloaded,
        local_path,
        actual_sha256,
        variants: catalog.variants,
    })
    .into_response()
}

#[expect(
    clippy::cognitive_complexity,
    reason = "状态查询需协调身份校验、远端页面兼容状态、本地凭据与实时工件发现"
)]
async fn status(State(state): State<Arc<AppState>>, Query(query): Query<StatusQuery>) -> Response {
    let (resource, preset) = match (query.preset, query.model, query.source, query.revision) {
        (Some(id), None, None, None) => {
            let preset = match presets::find(&id) {
                Ok(Some(preset)) => preset,
                Ok(None) => return bad_request("未知的预置模型"),
                Err(error) => {
                    tracing::error!(%error, "预置模型目录校验失败");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "code": "preset_catalog_invalid",
                            "message": "预置模型目录无效"
                        })),
                    )
                        .into_response();
                }
            };
            (preset.resource.clone(), Some(preset))
        }
        (None, Some(model), Some(source), revision) => (
            ModelResource {
                model,
                source,
                revision: revision.unwrap_or_else(|| source.default_revision().to_string()),
            },
            None,
        ),
        _ => return bad_request("模型选择参数不完整或互斥参数冲突"),
    };
    let Some(url) = resource.url(&state.config) else {
        return bad_request("模型仓库或修订号无效");
    };
    let identity_status = preset.map(|item| item.identity_status);
    if identity_status == Some(presets::IdentityStatus::PendingValidation) {
        return status_response(
            &resource,
            &url,
            "developing",
            identity_status,
            CatalogVariants {
                variants: Vec::new(),
                downloadable: false,
                unavailable_reason: Some(UnavailableReason {
                    code: "preset_identity_pending",
                    message: "该预置模型身份待校验，暂不可下载",
                }),
            },
        );
    }

    let remote_status = match sources::probe(&state.client, &state.config, url.clone()).await {
        Ok(200) => "downloadable",
        Ok(code) => {
            tracing::debug!(code, model = %resource.model, "模型资源页面不可访问");
            "developing"
        }
        Err(error) => {
            tracing::warn!(%error, model = %resource.model, "模型资源页面探测失败");
            "developing"
        }
    };
    let records = match &state.database {
        Some(database) => match catalog_downloads(
            database.clone(),
            &resource.model,
            resource.source.as_str(),
            &resource.revision,
        )
        .await
        {
            Ok(records) => records,
            Err(error) => {
                tracing::error!(%error, "读取本地模型目录失败");
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({
                        "code": "catalog_unavailable",
                        "message": "本地模型目录暂时不可用"
                    })),
                )
                    .into_response();
            }
        },
        None => Vec::new(),
    };
    let catalog = match catalog_variants(&state, &resource, &records, preset).await {
        Ok(catalog) => catalog,
        Err(crate::api::model_resolution::DiscoveryError::Invalid(_)) => {
            return bad_request("模型仓库或修订号无效");
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "code": "catalog_unavailable",
                    "message": "模型目录暂时不可用"
                })),
            )
                .into_response();
        }
    };
    status_response(&resource, &url, remote_status, identity_status, catalog)
}

#[cfg(test)]
mod tests;
