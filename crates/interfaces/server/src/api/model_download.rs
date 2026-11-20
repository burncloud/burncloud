//! Node 模型下载与摘要校验接口。

use axum::{
    extract::{rejection::JsonRejection, Json, Path as AxumPath, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use burncloud_download::{
    DownloadError, DownloadManager, DownloadTaskSnapshot as ManagerTaskSnapshot,
    DownloadTaskStatus as ManagerTaskStatus,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::Path as FilePath,
    sync::{Arc, OnceLock},
};
use tokio::io::AsyncReadExt;
use tokio::sync::{Mutex, OnceCell};

use crate::AppState;

/// 构建公开的 Node 模型下载接口。
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/node/models/download", post(create_download_task))
        .route(
            "/api/node/models/download/{task_id}",
            get(get_download_task).delete(cancel_download_task),
        )
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum DownloadSource {
    #[serde(rename = "modelscope")]
    ModelScope,
    #[serde(rename = "hf-mirror")]
    HfMirror,
    #[serde(rename = "huggingface")]
    HuggingFace,
}

impl DownloadSource {
    fn as_str(&self) -> &'static str {
        match self {
            Self::ModelScope => "modelscope",
            Self::HfMirror => "hf-mirror",
            Self::HuggingFace => "huggingface",
        }
    }

    fn host(&self) -> &'static str {
        match self {
            Self::ModelScope => "modelscope.cn",
            Self::HfMirror => "hf-mirror.com",
            Self::HuggingFace => "huggingface.co",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DownloadModelRequest {
    model: String,
    source: DownloadSource,
    revision: String,
    variant_id: String,
    artifact_uri: String,
    checksum: String,
}

#[derive(Clone, Debug)]
struct DownloadRequestIdentity {
    model: String,
    source: DownloadSource,
    revision: String,
    variant_id: String,
    artifact_uri: String,
    checksum: String,
    storage_directory: String,
    filename: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum DownloadTaskState {
    Queued,
    Downloading,
    Verifying,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct TaskError {
    code: String,
    message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct DownloadTaskData {
    task_id: String,
    status: DownloadTaskState,
    model: String,
    source: String,
    revision: String,
    variant_id: String,
    completed_bytes: u64,
    total_bytes: Option<u64>,
    download_speed: u64,
    error: Option<TaskError>,
    local_path: Option<String>,
    actual_sha256: Option<String>,
}

type ValidatedDownload = DownloadRequestIdentity;

#[derive(Clone, Debug)]
struct DownloadTaskSnapshot {
    task_id: i64,
    identity: DownloadRequestIdentity,
    status: DownloadTaskState,
    completed_bytes: u64,
    total_bytes: Option<u64>,
    download_speed: u64,
    error: Option<TaskError>,
    final_path: Option<String>,
    actual_sha256: Option<String>,
}

#[derive(Clone, Debug)]
struct NodeTaskState {
    identity: DownloadRequestIdentity,
    status: DownloadTaskState,
    error: Option<TaskError>,
    final_path: Option<String>,
    actual_sha256: Option<String>,
    cancel_requested: bool,
}

type TaskRegistry = Arc<Mutex<HashMap<i64, NodeTaskState>>>;

static TASK_REGISTRY: OnceLock<TaskRegistry> = OnceLock::new();

#[derive(Debug, Serialize)]
struct DownloadTaskResponse {
    success: bool,
    data: DownloadTaskData,
}

#[derive(Debug, Serialize)]
struct DownloadModelErrorResponse {
    success: bool,
    error: DownloadModelErrorBody,
}

#[derive(Debug, Serialize)]
struct DownloadModelErrorBody {
    code: &'static str,
    message: &'static str,
}

#[expect(
    clippy::cognitive_complexity,
    reason = "创建下载任务需要在同一边界完成请求校验、任务登记和初始快照构造"
)]
async fn create_download_task(
    State(state): State<AppState>,
    payload: Result<Json<DownloadModelRequest>, JsonRejection>,
) -> Response {
    let request = match payload {
        Ok(Json(request)) => match validate_request(request) {
            Ok(request) => request,
            Err(message) => {
                return error_response(StatusCode::BAD_REQUEST, "invalid_download_request", message)
            }
        },
        Err(error) => {
            tracing::debug!(%error, "模型下载请求体无效");
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid_download_request",
                "模型下载请求无效",
            );
        }
    };

    let manager = match state
        .download_manager
        .get_or_try_init(|| async { DownloadManager::new().await })
        .await
    {
        Ok(manager) => manager,
        Err(error) => {
            tracing::error!(%error, "初始化模型下载管理器失败");
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "download_service_unavailable",
                "模型下载服务暂时不可用",
            );
        }
    };

    let handle = match manager
        .start_download(&request.artifact_uri, &request.storage_directory)
        .await
    {
        Ok(handle) => handle,
        Err(error) => return download_error_response(error),
    };

    let registry = task_registry();
    register_task(registry, handle.task_id, request.clone(), handle.status).await;
    spawn_task_verification(
        Arc::clone(&state.download_manager),
        Arc::clone(registry),
        handle.task_id,
        request,
    );

    match task_snapshot(manager, registry, handle.task_id).await {
        Ok(Some(snapshot)) => task_response(StatusCode::ACCEPTED, snapshot),
        Ok(None) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "download_storage_failed",
            "下载任务登记后无法读取任务记录",
        ),
        Err(error) => task_storage_error_response(error),
    }
}

async fn get_download_task(
    State(state): State<AppState>,
    AxumPath(task_id): AxumPath<String>,
) -> Response {
    let task_id = match parse_task_id(&task_id) {
        Ok(task_id) => task_id,
        Err(message) => {
            return error_response(StatusCode::BAD_REQUEST, "invalid_download_task_id", message)
        }
    };
    let manager = match state
        .download_manager
        .get_or_try_init(|| async { DownloadManager::new().await })
        .await
    {
        Ok(manager) => manager,
        Err(error) => {
            tracing::error!(%error, "初始化模型下载管理器失败");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "download_storage_failed",
                "读取模型下载任务失败",
            );
        }
    };
    match task_snapshot(manager, task_registry(), task_id).await {
        Ok(Some(snapshot)) => task_response(StatusCode::OK, snapshot),
        Ok(None) => error_response(
            StatusCode::NOT_FOUND,
            "download_task_not_found",
            "模型下载任务不存在",
        ),
        Err(error) => task_storage_error_response(error),
    }
}

async fn cancel_download_task(
    State(state): State<AppState>,
    AxumPath(task_id): AxumPath<String>,
) -> Response {
    let task_id = match parse_task_id(&task_id) {
        Ok(task_id) => task_id,
        Err(message) => {
            return error_response(StatusCode::BAD_REQUEST, "invalid_download_task_id", message)
        }
    };
    let manager = match state
        .download_manager
        .get_or_try_init(|| async { DownloadManager::new().await })
        .await
    {
        Ok(manager) => manager,
        Err(error) => {
            tracing::error!(%error, "初始化模型下载管理器失败");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "download_storage_failed",
                "取消模型下载任务失败",
            );
        }
    };

    let registry = task_registry();
    if let Err(error) = request_task_cancellation(registry, task_id).await {
        return cancellation_error_response(error);
    }

    match manager.cancel_task(task_id).await {
        Ok(snapshot) => {
            mark_task_cancelled(registry, task_id, &snapshot).await;
            match task_snapshot(manager, registry, task_id).await {
                Ok(Some(snapshot)) => task_response(StatusCode::OK, snapshot),
                Ok(None) => error_response(
                    StatusCode::NOT_FOUND,
                    "download_task_not_found",
                    "模型下载任务不存在",
                ),
                Err(error) => task_storage_error_response(error),
            }
        }
        Err(error) => {
            clear_task_cancellation(registry, task_id).await;
            cancellation_error_response(error)
        }
    }
}

fn task_registry() -> &'static TaskRegistry {
    TASK_REGISTRY.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

async fn register_task(
    registry: &TaskRegistry,
    task_id: i64,
    identity: DownloadRequestIdentity,
    status: ManagerTaskStatus,
) {
    let mut tasks = registry.lock().await;
    tasks.entry(task_id).or_insert_with(|| NodeTaskState {
        identity,
        status: task_state_from_manager(status),
        error: None,
        final_path: None,
        actual_sha256: None,
        cancel_requested: false,
    });
}

fn task_state_from_manager(status: ManagerTaskStatus) -> DownloadTaskState {
    match status {
        ManagerTaskStatus::Queued => DownloadTaskState::Queued,
        ManagerTaskStatus::Downloading => DownloadTaskState::Downloading,
        ManagerTaskStatus::Verifying => DownloadTaskState::Verifying,
        ManagerTaskStatus::Succeeded => DownloadTaskState::Succeeded,
        ManagerTaskStatus::Failed => DownloadTaskState::Failed,
        ManagerTaskStatus::Cancelled => DownloadTaskState::Cancelled,
    }
}

async fn task_snapshot(
    manager: &DownloadManager,
    registry: &TaskRegistry,
    task_id: i64,
) -> Result<Option<DownloadTaskSnapshot>, DownloadError> {
    let Some(manager_snapshot) = manager.get_task(task_id).await? else {
        return Ok(None);
    };
    let Some(node_state) = registry.lock().await.get(&task_id).cloned() else {
        return Err(DownloadError::InvalidRecord(format!(
            "任务 {task_id} 缺少 Node 下载任务状态"
        )));
    };
    if node_state.identity.filename != manager_snapshot.filename {
        tracing::debug!(
            task_id,
            requested = %node_state.identity.filename,
            actual = %manager_snapshot.filename,
            "下载器已规范化任务文件名"
        );
    }
    let status = effective_task_state(&node_state, manager_snapshot.status);
    let error = node_state.error.clone().or_else(|| {
        (status == DownloadTaskState::Failed)
            .then(|| manager_error(&manager_snapshot))
            .flatten()
    });
    Ok(Some(DownloadTaskSnapshot {
        task_id: manager_snapshot.task_id,
        identity: node_state.identity,
        status,
        completed_bytes: manager_snapshot.completed_bytes,
        total_bytes: manager_snapshot.total_bytes,
        download_speed: manager_snapshot.download_speed,
        error,
        final_path: node_state.final_path.or_else(|| {
            manager_snapshot
                .final_path
                .map(|path| path.to_string_lossy().into_owned())
        }),
        actual_sha256: node_state.actual_sha256,
    }))
}

fn effective_task_state(
    node_state: &NodeTaskState,
    manager_status: ManagerTaskStatus,
) -> DownloadTaskState {
    match node_state.status {
        DownloadTaskState::Succeeded | DownloadTaskState::Failed | DownloadTaskState::Cancelled => {
            node_state.status
        }
        DownloadTaskState::Queued
        | DownloadTaskState::Downloading
        | DownloadTaskState::Verifying => task_state_from_manager(manager_status),
    }
}

fn manager_error(snapshot: &ManagerTaskSnapshot) -> Option<TaskError> {
    snapshot.error.as_ref().map(|message| TaskError {
        code: "download_failed".to_string(),
        message: if message.is_empty() {
            "模型文件下载失败".to_string()
        } else {
            message.clone()
        },
    })
}

fn task_response(status: StatusCode, snapshot: DownloadTaskSnapshot) -> Response {
    no_store(
        (
            status,
            Json(DownloadTaskResponse {
                success: true,
                data: task_data(snapshot),
            }),
        )
            .into_response(),
    )
}

fn task_data(snapshot: DownloadTaskSnapshot) -> DownloadTaskData {
    DownloadTaskData {
        task_id: snapshot.task_id.to_string(),
        status: snapshot.status,
        model: snapshot.identity.model,
        source: snapshot.identity.source.as_str().to_string(),
        revision: snapshot.identity.revision,
        variant_id: snapshot.identity.variant_id,
        completed_bytes: snapshot.completed_bytes,
        total_bytes: snapshot.total_bytes,
        download_speed: snapshot.download_speed,
        error: snapshot.error,
        local_path: snapshot.final_path,
        actual_sha256: snapshot.actual_sha256,
    }
}

#[expect(
    clippy::large_futures,
    reason = "后台校验任务必须持有下载管理器直到生命周期结束"
)]
fn spawn_task_verification(
    manager_cell: Arc<OnceCell<DownloadManager>>,
    registry: TaskRegistry,
    task_id: i64,
    identity: DownloadRequestIdentity,
) {
    tokio::spawn(async move {
        let Some(manager) = manager_cell.get() else {
            finish_task_error(
                &registry,
                task_id,
                TaskError {
                    code: "download_storage_failed".to_string(),
                    message: "模型下载管理器不可用".to_string(),
                },
            )
            .await;
            return;
        };
        verify_task(manager, &registry, task_id, &identity).await;
    });
}

#[expect(
    clippy::cognitive_complexity,
    clippy::large_futures,
    reason = "后台校验需要串联下载等待、文件摘要计算和可取消状态提交"
)]
async fn verify_task(
    manager: &DownloadManager,
    registry: &TaskRegistry,
    task_id: i64,
    identity: &DownloadRequestIdentity,
) {
    let path = match manager.wait_for_task(task_id).await {
        Ok(path) => path,
        Err(error) => {
            finish_task_error(registry, task_id, task_error_from_download(&error)).await;
            return;
        }
    };
    if !begin_verification(registry, task_id).await {
        return;
    }

    let actual_sha256 = match sha256_file(&path).await {
        Ok(checksum) => checksum,
        Err(error) => {
            tracing::error!(task_id, path = %path.display(), %error, "读取下载文件并计算摘要失败");
            finish_task_error(
                registry,
                task_id,
                TaskError {
                    code: "download_storage_failed".to_string(),
                    message: "下载文件校验失败".to_string(),
                },
            )
            .await;
            return;
        }
    };

    let mut tasks = registry.lock().await;
    let Some(task) = tasks.get_mut(&task_id) else {
        return;
    };
    if task.cancel_requested || task.status == DownloadTaskState::Cancelled {
        return;
    }
    task.final_path = Some(path.to_string_lossy().into_owned());
    task.actual_sha256 = Some(actual_sha256.clone());
    if actual_sha256.eq_ignore_ascii_case(&identity.checksum) {
        task.status = DownloadTaskState::Succeeded;
        task.error = None;
    } else {
        tracing::warn!(
            task_id,
            path = %path.display(),
            expected = %identity.checksum,
            actual = %actual_sha256,
            "下载文件摘要不匹配"
        );
        task.status = DownloadTaskState::Failed;
        task.error = Some(TaskError {
            code: "checksum_mismatch".to_string(),
            message: "下载文件摘要与请求摘要不一致".to_string(),
        });
    }
}

async fn begin_verification(registry: &TaskRegistry, task_id: i64) -> bool {
    let mut tasks = registry.lock().await;
    let Some(task) = tasks.get_mut(&task_id) else {
        return false;
    };
    if task.cancel_requested
        || matches!(
            task.status,
            DownloadTaskState::Succeeded | DownloadTaskState::Failed | DownloadTaskState::Cancelled
        )
    {
        return false;
    }
    task.status = DownloadTaskState::Verifying;
    true
}

async fn finish_task_error(registry: &TaskRegistry, task_id: i64, error: TaskError) {
    let mut tasks = registry.lock().await;
    let Some(task) = tasks.get_mut(&task_id) else {
        return;
    };
    if task.cancel_requested
        || matches!(
            task.status,
            DownloadTaskState::Cancelled | DownloadTaskState::Succeeded
        )
    {
        return;
    }
    task.status = DownloadTaskState::Failed;
    task.error = Some(error);
}

fn task_error_from_download(error: &DownloadError) -> TaskError {
    let (code, message) = match error {
        DownloadError::Failed(message) if !message.is_empty() => {
            ("download_failed", message.clone())
        }
        DownloadError::Aria2(_) | DownloadError::Failed(_) => {
            ("download_failed", "模型文件下载失败".to_string())
        }
        DownloadError::TaskNotFound
        | DownloadError::NotificationClosed
        | DownloadError::Database(_)
        | DownloadError::InvalidRecord(_)
        | DownloadError::Compensation { .. }
        | DownloadError::InvalidUrl(_)
        | DownloadError::InvalidDirectory(_)
        | DownloadError::InvalidFilename
        | DownloadError::TaskNotCancellable(_) => (
            "download_storage_failed",
            "下载任务状态读取失败".to_string(),
        ),
    };
    TaskError {
        code: code.to_string(),
        message,
    }
}

enum CancellationDecision {
    Proceed,
    AlreadyCancelled,
}

async fn request_task_cancellation(
    registry: &TaskRegistry,
    task_id: i64,
) -> Result<CancellationDecision, DownloadError> {
    let mut tasks = registry.lock().await;
    let task = tasks.get_mut(&task_id).ok_or(DownloadError::TaskNotFound)?;
    match task.status {
        DownloadTaskState::Cancelled => Ok(CancellationDecision::AlreadyCancelled),
        DownloadTaskState::Succeeded | DownloadTaskState::Failed => Err(
            DownloadError::TaskNotCancellable(format!("任务当前状态为 {:?}", task.status)),
        ),
        DownloadTaskState::Queued
        | DownloadTaskState::Downloading
        | DownloadTaskState::Verifying => {
            if task.cancel_requested {
                return Err(DownloadError::TaskNotCancellable(
                    "取消操作正在进行".to_string(),
                ));
            }
            task.cancel_requested = true;
            Ok(CancellationDecision::Proceed)
        }
    }
}

async fn clear_task_cancellation(registry: &TaskRegistry, task_id: i64) {
    if let Some(task) = registry.lock().await.get_mut(&task_id) {
        task.cancel_requested = false;
    }
}

async fn mark_task_cancelled(
    registry: &TaskRegistry,
    task_id: i64,
    manager_snapshot: &ManagerTaskSnapshot,
) {
    if let Some(task) = registry.lock().await.get_mut(&task_id) {
        task.status = DownloadTaskState::Cancelled;
        task.cancel_requested = false;
        if task.final_path.is_none() {
            task.final_path = manager_snapshot
                .final_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned());
        }
    }
}

fn task_storage_error_response(error: DownloadError) -> Response {
    match error {
        DownloadError::TaskNotFound => error_response(
            StatusCode::NOT_FOUND,
            "download_task_not_found",
            "模型下载任务不存在",
        ),
        error => {
            tracing::error!(%error, "读取模型下载任务失败");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "download_storage_failed",
                "读取模型下载任务失败",
            )
        }
    }
}

fn cancellation_error_response(error: DownloadError) -> Response {
    match error {
        DownloadError::TaskNotFound => error_response(
            StatusCode::NOT_FOUND,
            "download_task_not_found",
            "模型下载任务不存在",
        ),
        DownloadError::TaskNotCancellable(_) => error_response(
            StatusCode::CONFLICT,
            "download_task_not_cancellable",
            "模型下载任务当前不可取消",
        ),
        error => {
            tracing::error!(%error, "取消模型下载任务失败");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "download_storage_failed",
                "取消模型下载任务失败",
            )
        }
    }
}

fn validate_request(request: DownloadModelRequest) -> Result<ValidatedDownload, &'static str> {
    let model = request.model.trim().to_owned();
    let revision = request.revision.trim().to_owned();
    let variant_id = request.variant_id.trim().to_owned();
    let artifact_uri = request.artifact_uri.trim().to_owned();
    let checksum = normalize_checksum(&request.checksum).ok_or("模型摘要格式无效")?;

    if !valid_model_id(&model) || !valid_segment(&revision) {
        return Err("模型名称或修订号无效");
    }
    if !valid_variant_id(&variant_id) {
        return Err("量化版本标识无效");
    }

    let url = Url::parse(&artifact_uri).map_err(|_| "工件地址无效")?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !url
            .host_str()
            .is_some_and(|host| host.eq_ignore_ascii_case(request.source.host()))
        || !artifact_path_matches(&url, &request.source, &model, &revision, &variant_id)
    {
        return Err("工件地址必须是支持仓库的 HTTPS GGUF resolve 地址");
    }

    let filename = variant_id
        .rsplit('/')
        .next()
        .map(safe_path_segment)
        .ok_or("量化版本文件名无效")?;
    let identity = DownloadRequestIdentity {
        model,
        source: request.source,
        revision,
        variant_id,
        artifact_uri,
        checksum,
        storage_directory: String::new(),
        filename,
    };
    let storage_directory = storage_directory(&identity);
    Ok(DownloadRequestIdentity {
        storage_directory,
        ..identity
    })
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

fn valid_variant_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.to_ascii_lowercase().ends_with(".gguf")
        && value.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && !segment.contains('\\')
                && !segment.chars().any(char::is_control)
        })
}

fn artifact_path_matches(
    url: &Url,
    source: &DownloadSource,
    model: &str,
    revision: &str,
    variant_id: &str,
) -> bool {
    let Some((owner, name)) = model.split_once('/') else {
        return false;
    };
    let mut expected = Vec::new();
    if matches!(source, DownloadSource::ModelScope) {
        expected.push("models");
    }
    expected.extend([owner, name, "resolve", revision]);
    expected.extend(variant_id.split('/'));
    url.path_segments()
        .is_some_and(|segments| segments.eq(expected))
}

fn normalize_checksum(value: &str) -> Option<String> {
    let digest = value.trim().strip_prefix("sha256:")?;
    (digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| format!("sha256:{}", digest.to_ascii_lowercase()))
}

fn storage_directory(request: &DownloadRequestIdentity) -> String {
    let checksum = request
        .checksum
        .strip_prefix("sha256:")
        .unwrap_or(&request.checksum);
    format!(
        "models/{}/{}/{}/{}",
        request.source.as_str(),
        safe_path_segment(&request.model.replace('/', "_")),
        safe_path_segment(&request.revision),
        checksum
    )
}

fn parse_task_id(value: &str) -> Result<i64, &'static str> {
    let value = value.trim();
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("任务 ID 必须是十进制数字");
    }
    value.parse::<i64>().map_err(|_| "任务 ID 超出有效范围")
}

fn safe_path_segment(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() || cleaned.chars().all(|character| character == '.') {
        "_".into()
    } else {
        cleaned
    }
}

async fn sha256_file(path: &FilePath) -> std::io::Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

#[expect(
    clippy::cognitive_complexity,
    reason = "下载器错误需要按稳定公开错误码分组，避免向客户端泄露内部错误"
)]
fn download_error_response(error: DownloadError) -> Response {
    match error {
        DownloadError::InvalidUrl(_) | DownloadError::InvalidDirectory(_) => {
            tracing::debug!(%error, "模型下载请求未通过下载器校验");
            error_response(
                StatusCode::BAD_REQUEST,
                "invalid_download_request",
                "下载地址或目录无效",
            )
        }
        DownloadError::InvalidFilename => {
            tracing::debug!(%error, "模型下载文件名无效");
            error_response(
                StatusCode::BAD_REQUEST,
                "invalid_download_request",
                "下载文件名无效",
            )
        }
        error => {
            tracing::warn!(%error, "模型文件下载失败");
            error_response(
                StatusCode::BAD_GATEWAY,
                "download_failed",
                "模型文件下载失败",
            )
        }
    }
}

fn error_response(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    no_store(
        (
            status,
            Json(DownloadModelErrorResponse {
                success: false,
                error: DownloadModelErrorBody { code, message },
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
#[path = "model_download_tests.rs"]
mod tests;
