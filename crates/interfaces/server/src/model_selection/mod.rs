//! 主服务内的模型选择接口；不创建监听器、独立进程或下载状态仓库。

mod presets;
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
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{sync::mpsc, task::JoinSet};

use burncloud_supply_model::{load_default_manifests, ModelManifest, Variant};

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

fn default_manifests() -> Vec<ModelManifest> {
    match load_default_manifests() {
        Ok(manifests) => manifests,
        Err(error) => panic!("内嵌默认模型清单格式错误：{error}"),
    }
}

struct AppState {
    config: Config,
    client: reqwest::Client,
    manifests: Vec<ModelManifest>,
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
            manifests: default_manifests(),
            database,
        }))
}

fn bad_request(message: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "message": message }))).into_response()
}

async fn list_presets() -> impl IntoResponse {
    Json(presets::all())
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
    ram_bytes: u64,
    vram_bytes: u64,
    disk_bytes: u64,
    artifact_status: ArtifactStatus,
    downloaded: bool,
    local_path: Option<String>,
    actual_sha256: Option<String>,
}

impl From<&Variant> for StatusVariant {
    fn from(variant: &Variant) -> Self {
        Self {
            id: variant.id.clone(),
            quantization: variant.quantization.clone(),
            runtime: variant.backend.clone(),
            artifact_source: variant.artifact_uri.clone(),
            expected_sha256: variant.checksum.clone(),
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

fn model_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn variants_for_resource(
    manifests: &[ModelManifest],
    resource: &ModelResource,
) -> Vec<StatusVariant> {
    if resource.source != Source::ModelScope {
        return Vec::new();
    }
    let Some(repository_name) = resource.model.rsplit('/').next() else {
        return Vec::new();
    };
    manifests
        .iter()
        .find(|manifest| model_key(&manifest.model_name) == model_key(repository_name))
        .map(|manifest| manifest.variants.iter().map(StatusVariant::from).collect())
        .unwrap_or_default()
}

async fn catalog_variants(
    state: &AppState,
    resource: &ModelResource,
    records: &[CatalogDownload],
) -> Vec<StatusVariant> {
    if records.is_empty() {
        return variants_for_resource(&state.manifests, resource);
    }
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
    );
    match tokio::time::timeout(state.config.probe_timeout, discovery).await {
        Ok(Ok(manifest)) => manifest.variants.iter().map(StatusVariant::from).collect(),
        error => {
            tracing::debug!(?error, "目录文件清单不可用，仅展示本地记录固定的工件身份");
            let mut seen = std::collections::HashSet::new();
            records
                .iter()
                .filter_map(|r| recorded_variant(source, &resource.model, &resource.revision, r))
                .filter(|v| seen.insert(v.id.clone()))
                .map(|v| StatusVariant::from(&v))
                .collect()
        }
    }
}

fn apply_catalog_record(variant: &mut StatusVariant, records: &[CatalogDownload]) {
    let Some(r) = records.iter().find(|r| {
        r.variant_id == variant.id
            && r.artifact_uri == variant.artifact_source
            && r.checksum == variant.expected_sha256
    }) else {
        return;
    };
    variant.artifact_status = match r.status {
        DownloadTaskStatus::Queued | DownloadTaskStatus::Downloading => ArtifactStatus::Downloading,
        DownloadTaskStatus::Verifying => ArtifactStatus::Verifying,
        DownloadTaskStatus::Succeeded => ArtifactStatus::Downloaded,
        DownloadTaskStatus::Failed | DownloadTaskStatus::Cancelled => ArtifactStatus::Failed,
    };
    variant.downloaded = variant.artifact_status == ArtifactStatus::Downloaded;
    variant.local_path.clone_from(&r.local_path);
    variant.actual_sha256.clone_from(&r.actual_sha256);
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

#[expect(
    clippy::cognitive_complexity,
    reason = "状态接口需要在一次响应中保留远程探测、变体工件查询和模型级聚合语义"
)]
async fn status(State(state): State<Arc<AppState>>, Query(query): Query<StatusQuery>) -> Response {
    let resource = match (query.preset, query.model, query.source) {
        (Some(id), _, _) => match presets::find(&id) {
            Some(preset) => preset.resource,
            None => return bad_request("未知的预置模型"),
        },
        (None, Some(model), Some(source)) => ModelResource {
            model,
            source,
            revision: query
                .revision
                .unwrap_or_else(|| source.default_revision().to_string()),
        },
        _ => return bad_request("模型选择参数不完整"),
    };
    let Some(url) = resource.url(&state.config) else {
        return bad_request("模型仓库或修订号无效");
    };
    let remote_status = match sources::probe(&state.client, &state.config, url.clone()).await {
        Ok(200) => "downloadable",
        Ok(code) => {
            tracing::debug!(code, model = %resource.model, "模型资源页面不可下载");
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
                    Json(json!({ "message": "本地模型目录暂时不可用" })),
                )
                    .into_response();
            }
        },
        None => Vec::new(),
    };
    let mut variants = catalog_variants(&state, &resource, &records).await;
    let mut model_artifact_status = ArtifactStatus::NotDownloaded;
    let mut model_local_path = None;
    let mut model_actual_sha256 = None;
    for variant in &mut variants {
        apply_catalog_record(variant, &records);
        if artifact_rank(variant.artifact_status) > artifact_rank(model_artifact_status) {
            model_artifact_status = variant.artifact_status;
            model_local_path.clone_from(&variant.local_path);
            model_actual_sha256.clone_from(&variant.actual_sha256);
        }
    }
    Json(json!({
        "status": remote_status,
        "model": resource.model,
        "source": resource.source,
        "revision": resource.revision,
        "resource_url": url.as_str(),
        "artifact_status": model_artifact_status,
        "downloaded": matches!(model_artifact_status, ArtifactStatus::Downloaded),
        "local_path": model_local_path,
        "actual_sha256": model_actual_sha256,
        "variants": variants,
    }))
    .into_response()
}

#[cfg(test)]
mod tests;
