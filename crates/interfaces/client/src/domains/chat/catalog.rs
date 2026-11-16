//! 主服务模型选择、模型解析和下载 HTTP 边界。

use super::model::{
    valid_digest, ChatError, ErrorKind, Gpu, Hardware, InitializationRequest, ModelResolution,
    ModelVariant, Operation, Platform, PreparationEvent, PreparationStage, PreparedModel,
};
use reqwest::{Client, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;

/// 模块内部 HTTP 客户端；生产请求主服务，测试可注入本地响应。
#[derive(Clone)]
pub(super) struct CatalogClient {
    client: Client,
    download_client: Client,
    base: Url,
}

/// 每行均为累计快照，最后一行必须携带完成类型。
#[derive(Clone, Debug, Deserialize)]
pub(super) struct CatalogPage<T> {
    pub(super) items: Vec<T>,
    #[serde(rename = "type")]
    kind: Option<String>,
    pub(super) done: bool,
}

/// 预置项和搜索结果均直接绑定资源，不再二次解析仓库。
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub(super) struct CatalogModel {
    pub(super) id: String,
    pub(super) name: String,
    #[serde(default)]
    pub(super) preset: bool,
    pub(super) model: String,
    pub(super) source: String,
    pub(super) revision: String,
}

/// 已确认的仓库快照；页面地址不是可部署的模型文件。
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub(super) struct CatalogVersion {
    pub(super) name: String,
    pub(super) model: String,
    pub(super) revision: String,
    pub(super) source: String,
    pub(super) resource_url: String,
}

#[derive(Clone, Debug, Deserialize)]
struct ResolveEnvelope {
    success: bool,
    data: Option<ResolveData>,
    error: Option<RemoteHardwareError>,
}

#[derive(Clone, Debug, Deserialize)]
struct ResolveData {
    status: String,
    model: String,
    hardware: RemoteHardwareSnapshot,
    candidates: Vec<RemoteModelCandidate>,
    reason: Option<RemoteResolveReason>,
}

#[derive(Clone, Debug, Deserialize)]
struct RemoteModelCandidate {
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

#[derive(Clone, Debug, Deserialize)]
struct RemoteResolveReason {
    code: String,
    message: String,
}

#[derive(Clone, Debug, Deserialize)]
struct HardwareEnvelope {
    success: bool,
    data: Option<RemoteHardwareSnapshot>,
    error: Option<RemoteHardwareError>,
}

#[derive(Clone, Debug, Deserialize)]
struct RemoteHardwareSnapshot {
    platform: String,
    #[serde(rename = "arch")]
    _arch: String,
    cpu: String,
    memory_bytes: u64,
    disk_available_bytes: u64,
    accelerators: Vec<RemoteAccelerator>,
}

#[derive(Clone, Debug, Deserialize)]
struct RemoteAccelerator {
    kind: String,
    name: String,
    memory_bytes: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct RemoteHardwareError {
    code: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct DownloadModelRequest {
    model: String,
    source: String,
    revision: String,
    variant_id: String,
    artifact_uri: String,
    checksum: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct DownloadTaskData {
    task_id: String,
    status: String,
    model: String,
    source: String,
    revision: String,
    variant_id: String,
    completed_bytes: u64,
    total_bytes: Option<u64>,
    download_speed: u64,
    error: Option<RemoteHardwareError>,
    local_path: Option<String>,
    actual_sha256: Option<String>,
}

const DOWNLOAD_POLL_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Debug, Deserialize)]
struct DownloadTaskEnvelope {
    success: bool,
    data: Option<DownloadTaskData>,
    error: Option<RemoteHardwareError>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct CatalogSelection {
    pub(super) name: String,
    pub(super) preset: Option<String>,
    pub(super) version: CatalogVersion,
}
impl CatalogSelection {
    /// 来源、仓库和修订号共同决定身份。
    ///
    /// # Examples
    /// 同一仓库的 `main` 与 `master` 是不同选择。
    ///
    /// # Notes
    /// 长度前缀避免分隔符歧义，不使用展示名称作为运行标识。
    pub(super) fn id(&self) -> String {
        let v = &self.version;
        format!(
            "catalog:{}:{}:{}:{}:{}",
            v.source,
            v.model.len(),
            v.model,
            v.revision.len(),
            v.revision
        )
    }
}

/// 目录状态不能替代本客户端经过校验的运行凭据。
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(super) enum VersionStatus {
    Available,
    Preparing,
    Developing,
    Downloadable,
}

impl CatalogClient {
    /// 请求统一主服务，默认跟随 HOST/PORT，可用 BURNCLOUD_SERVER_URL 指向远程主服务。
    ///
    /// # Examples
    /// `BURNCLOUD_SERVER_URL=http://127.0.0.1:3000`。
    ///
    /// # Notes
    /// LiveView 请求发生在 Rust 服务端，桌面请求发生在本机；不管理额外进程。
    pub(super) fn from_env() -> Result<Self, ChatError> {
        let read = |name| match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(error) => Err(failure(
                ErrorKind::InvalidParameter,
                Operation::Search,
                error.to_string(),
            )),
        };
        let base = match read("BURNCLOUD_SERVER_URL")? {
            Some(value) => value,
            None => {
                let host = read("HOST")?.unwrap_or_else(|| "127.0.0.1".into());
                // 主程序对无效 PORT 使用默认端口，这里保持相同规则，避免空端口变成 HTTP 80。
                let port = read("PORT")?
                    .unwrap_or_else(|| burncloud_common::constants::DEFAULT_PORT.to_string())
                    .parse::<u16>()
                    .unwrap_or(burncloud_common::constants::DEFAULT_PORT);
                server_base(&host, port)
            }
        };
        Self::with_timeouts(&base, Duration::from_secs(45), Duration::from_secs(10 * 60))
    }

    /// 构造目录客户端，拒绝非 HTTP 地址、带认证信息的地址和零超时。
    ///
    /// # Examples
    /// 测试可以注入本机随机端口和短超时。
    ///
    /// # Notes
    /// 错误配置明确返回错误，不退回旧版独立服务。
    #[allow(
        dead_code,
        reason = "测试注入短超时客户端；生产环境从 from_env 使用专用下载超时"
    )]
    pub(super) fn new(base: &str, timeout: Duration) -> Result<Self, ChatError> {
        Self::with_timeouts(base, timeout, timeout)
    }

    fn with_timeouts(
        base: &str,
        timeout: Duration,
        download_timeout: Duration,
    ) -> Result<Self, ChatError> {
        let base = Url::parse(base).map_err(|error| {
            failure(
                ErrorKind::InvalidParameter,
                Operation::Search,
                error.to_string(),
            )
        })?;
        if !matches!(base.scheme(), "http" | "https")
            || base.host_str().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || timeout.is_zero()
            || download_timeout.is_zero()
        {
            return Err(failure(
                ErrorKind::InvalidParameter,
                Operation::Search,
                "主服务地址或超时配置无效".into(),
            ));
        }
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|error| transport(error, Operation::Search))?;
        let download_client = Client::builder()
            .timeout(download_timeout)
            .build()
            .map_err(|error| {
                transport(error, Operation::Initialize(PreparationStage::Downloading))
            })?;
        Ok(Self {
            client,
            download_client,
            base: base.clone(),
        })
    }

    /// 空查询加载预置，非空查询逐行发布累计搜索快照。
    ///
    /// # Examples
    /// 查询 `qwen` 时，先到达的站点结果无需等待其他站点。
    ///
    /// # Notes
    /// EOF 不能代替最终完成行；异常及截断响应明确报错。
    pub(super) async fn models(
        &self,
        query: &str,
        mut publish: impl FnMut(CatalogPage<CatalogModel>),
    ) -> Result<(), ChatError> {
        let query = query.trim();
        if query.is_empty() {
            let items: Vec<CatalogModel> = self
                .get("/api/models/presets", &[], Operation::Search)
                .await?;
            for (index, model) in items.iter().enumerate() {
                model.validate()?;
                if !model.preset
                    || items[..index]
                        .iter()
                        .any(|previous| previous.id == model.id)
                {
                    return Err(invalid_search("预置目录包含非预置条目或重复标识"));
                }
            }
            publish(CatalogPage {
                items,
                kind: Some("models".into()),
                done: true,
            });
            return Ok(());
        }
        let mut response = self
            .request(
                "/api/models/search",
                &[("q", query.to_string())],
                Operation::Search,
            )
            .await?;
        let mut pending = Vec::new();
        let mut done = false;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| transport(error, Operation::Search))?
        {
            pending.extend_from_slice(&chunk);
            while let Some(end) = pending.iter().position(|byte| *byte == b'\n') {
                let bytes: Vec<_> = pending.drain(..=end).collect();
                if bytes.iter().all(u8::is_ascii_whitespace) {
                    continue;
                }
                if done {
                    return Err(invalid_search("完成行后仍包含搜索结果"));
                }
                let snapshot = parse_snapshot(&bytes)?;
                done = snapshot.done;
                publish(snapshot);
            }
        }
        if pending.iter().any(|byte| !byte.is_ascii_whitespace()) {
            if done {
                return Err(invalid_search("完成行后仍包含搜索结果"));
            }
            let snapshot = parse_snapshot(&pending)?;
            done = snapshot.done;
            publish(snapshot);
        }
        if !done {
            return Err(invalid_search("模型搜索响应被截断，缺少完成行"));
        }
        Ok(())
    }

    /// 一次状态查询返回资源与状态，预置项只发送其绑定标识。
    ///
    /// # Examples
    /// `preset=qwen3-8b` 不触发客户端搜索或资源页面探测。
    ///
    /// # Notes
    /// 响应资源必须与选中项一致；下载状态仍不代表存在可部署工件。
    pub(super) async fn inspect(
        &self,
        model: &CatalogModel,
    ) -> Result<(Option<CatalogVersion>, VersionStatus), ChatError> {
        model.validate()?;
        #[derive(Deserialize)]
        struct Status {
            status: VersionStatus,
            model: String,
            source: String,
            revision: String,
            resource_url: String,
        }
        let params = if model.preset {
            vec![("preset", model.id.clone())]
        } else {
            vec![
                ("model", model.model.clone()),
                ("source", model.source.clone()),
                ("revision", model.revision.clone()),
            ]
        };
        let result: Status = self
            .get("/api/models/status", &params, Operation::Status)
            .await?;
        let version = CatalogVersion {
            name: model.name.clone(),
            model: result.model,
            source: result.source,
            revision: result.revision,
            resource_url: result.resource_url,
        };
        version.validate()?;
        if version.model != model.model
            || version.source != model.source
            || version.revision != model.revision
        {
            return Err(failure(
                ErrorKind::InvalidResult,
                Operation::Status,
                "状态响应与选中资源不一致".into(),
            ));
        }
        Ok((Some(version), result.status))
    }

    /// 在一次主服务请求中获取硬件快照和全部兼容 Variant。
    pub(super) async fn resolve_model(
        &self,
        selection: &CatalogSelection,
    ) -> Result<ModelResolution, ChatError> {
        selection.version.validate()?;
        let model = selection.version.model.as_str();
        let url = self
            .base
            .join("/api/node/models/resolve")
            .map_err(|error| {
                failure(
                    ErrorKind::InvalidParameter,
                    Operation::Resolve,
                    error.to_string(),
                )
            })?;
        let response = self
            .client
            .post(url)
            .header("Accept", "application/json")
            .json(&ResolveModelRequest {
                model: selection.version.model.clone(),
                source: selection.version.source.clone(),
                revision: selection.version.revision.clone(),
            })
            .send()
            .await
            .map_err(|error| transport(error, Operation::Resolve))?;
        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|error| transport(error, Operation::Resolve))?;
        let envelope: ResolveEnvelope = serde_json::from_slice(&body).map_err(|error| {
            failure(
                ErrorKind::InvalidResult,
                Operation::Resolve,
                format!("模型解析接口响应格式无效：{error}"),
            )
        })?;
        if !status.is_success() || !envelope.success {
            let error = envelope.error.ok_or_else(|| {
                failure(
                    ErrorKind::InvalidResult,
                    Operation::Resolve,
                    "模型解析失败响应缺少错误信息".into(),
                )
            })?;
            return Err(remote_error(error, Operation::Resolve));
        }
        let data = envelope.data.ok_or_else(|| {
            failure(
                ErrorKind::InvalidResult,
                Operation::Resolve,
                "模型解析成功响应缺少数据".into(),
            )
        })?;
        if data.model != model || !matches!(data.status.as_str(), "resolved" | "unsupported") {
            return Err(failure(
                ErrorKind::InvalidResult,
                Operation::Resolve,
                "模型解析响应与请求不一致".into(),
            ));
        }
        let hardware = data.hardware.into_hardware()?;
        let model_name = data.model.clone();
        let variants = data
            .candidates
            .into_iter()
            .map(|candidate| {
                if candidate.model_name != model_name {
                    return Err(failure(
                        ErrorKind::InvalidResult,
                        Operation::Resolve,
                        "模型解析候选版本与请求模型不一致".into(),
                    ));
                }
                candidate.into_variant()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let reason = data
            .reason
            .map(|reason| format!("{}: {}", reason.code, reason.message));
        if data.status == "resolved" && variants.is_empty() {
            return Err(failure(
                ErrorKind::InvalidResult,
                Operation::Resolve,
                "模型解析成功响应没有候选版本".into(),
            ));
        }
        if data.status == "resolved" && reason.is_some() {
            return Err(failure(
                ErrorKind::InvalidResult,
                Operation::Resolve,
                "模型解析成功响应包含不支持原因".into(),
            ));
        }
        if data.status == "unsupported" && !variants.is_empty() {
            return Err(failure(
                ErrorKind::InvalidResult,
                Operation::Resolve,
                "模型解析不支持响应包含候选版本".into(),
            ));
        }
        if data.status == "unsupported" && reason.is_none() {
            return Err(failure(
                ErrorKind::InvalidResult,
                Operation::Resolve,
                "模型解析不支持响应缺少原因".into(),
            ));
        }
        Ok(ModelResolution {
            hardware,
            variants,
            reason,
        })
    }

    /// 获取主服务 Node 的一次性硬件快照。
    pub(super) async fn inspect_hardware(&self) -> Result<Hardware, ChatError> {
        let response = self
            .request_raw("/api/node/hardware", &[], Operation::Hardware)
            .await?;
        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|error| transport(error, Operation::Hardware))?;
        let response: HardwareEnvelope = serde_json::from_slice(&body).map_err(|error| {
            failure(
                ErrorKind::InvalidResult,
                Operation::Hardware,
                format!("硬件接口响应格式无效：{error}"),
            )
        })?;
        if !status.is_success() || !response.success {
            let error = response.error.ok_or_else(|| {
                failure(
                    ErrorKind::InvalidResult,
                    Operation::Hardware,
                    "硬件接口失败响应缺少错误信息".into(),
                )
            })?;
            return Err(failure(
                ErrorKind::ExecutionFailed,
                Operation::Hardware,
                format!("{}: {}", error.code, error.message),
            ));
        }

        let snapshot = response.data.ok_or_else(|| {
            failure(
                ErrorKind::InvalidResult,
                Operation::Hardware,
                "硬件接口成功响应缺少数据".into(),
            )
        })?;
        snapshot.into_hardware()
    }

    async fn create_download_task(
        &self,
        request: &InitializationRequest,
    ) -> Result<DownloadTaskData, ChatError> {
        let operation = Operation::Initialize(PreparationStage::Downloading);
        let payload = download_request_payload(request, operation)?;
        let url = self
            .base
            .join("/api/node/models/download")
            .map_err(|error| failure(ErrorKind::InvalidParameter, operation, error.to_string()))?;
        let response = self
            .download_client
            .post(url)
            .header("Accept", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|error| transport(error, operation))?;
        decode_download_task(response, operation).await
    }

    async fn get_download_task(&self, task_id: &str) -> Result<DownloadTaskData, ChatError> {
        let operation = Operation::Initialize(PreparationStage::Downloading);
        if !valid_task_id(task_id) {
            return Err(failure(
                ErrorKind::InvalidParameter,
                operation,
                "下载任务 ID 无效".into(),
            ));
        }
        let path = format!("/api/node/models/download/{task_id}");
        let url = self
            .base
            .join(&path)
            .map_err(|error| failure(ErrorKind::InvalidParameter, operation, error.to_string()))?;
        let response = self
            .download_client
            .get(url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|error| transport(error, operation))?;
        decode_download_task(response, operation).await
    }

    /// 请求 Node 取消下载任务，并只接受服务端返回的 `cancelled` 快照。
    ///
    /// # Examples
    ///
    /// 页面取消初始化时传入创建任务接口返回的十进制 `task_id`，成功后再停止本地轮询 Future。
    ///
    /// # Notes
    ///
    /// 方法不把 HTTP 成功状态伪装成取消成功；任务仍在运行、已完成或响应快照非法都会返回
    /// `ChatError`。任务 ID 只用于 URL 路径，不接受空值、负数或超出 `i64` 范围的输入。
    pub(super) async fn cancel_download_task(&self, task_id: &str) -> Result<(), ChatError> {
        let operation = Operation::Initialize(PreparationStage::Downloading);
        if !valid_task_id(task_id) {
            return Err(failure(
                ErrorKind::InvalidParameter,
                operation,
                "下载任务 ID 无效".into(),
            ));
        }
        let path = format!("/api/node/models/download/{task_id}");
        let url = self
            .base
            .join(&path)
            .map_err(|error| failure(ErrorKind::InvalidParameter, operation, error.to_string()))?;
        let response = self
            .download_client
            .delete(url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|error| transport(error, operation))?;
        let data = decode_download_task(response, operation).await?;
        validate_download_task_shape(&data)?;
        if data.task_id != task_id {
            return Err(failure(
                ErrorKind::InvalidResult,
                operation,
                "取消接口返回了不匹配的任务快照".into(),
            ));
        }
        if data.status != "cancelled" {
            return Err(failure(
                ErrorKind::InvalidResult,
                operation,
                "取消接口没有返回 cancelled 任务快照".into(),
            ));
        }
        Ok(())
    }

    async fn poll_download_task(
        &self,
        task_id: &str,
        expected: &InitializationRequest,
        callback: &mut impl FnMut(PreparationEvent),
        downloaded_emitted: &mut bool,
    ) -> Result<DownloadTaskData, ChatError> {
        loop {
            let data = self.get_download_task(task_id).await?;
            let data = map_download_task(data, expected)?;
            match data.status.as_str() {
                "queued" | "downloading" => {
                    callback(PreparationEvent::DownloadProgress {
                        completed: data.completed_bytes,
                        total: data.total_bytes,
                    });
                }
                "verifying" => {
                    emit_downloaded(callback, downloaded_emitted);
                }
                "succeeded" => {
                    emit_downloaded(callback, downloaded_emitted);
                    return Ok(data);
                }
                "failed" | "cancelled" => return Ok(data),
                _ => unreachable!("map_download_task 已校验任务状态"),
            }
            tokio::time::sleep(DOWNLOAD_POLL_INTERVAL).await;
        }
    }

    /// 在 Node 主服务上创建并轮询已确认的目录工件下载任务。
    ///
    /// # Examples
    ///
    /// 目录解析成功后，将当前选中的版本快照交给此方法即可复用服务端下载器。
    ///
    /// # Notes
    ///
    /// 方法只在 Node 任务进入 `succeeded` 后交付成功凭据；过程进度和摘要校验阶段通过
    /// `PreparationEvent` 回调交给客户端状态机。返回的路径属于 Node 主机，不是客户端本地路径。
    pub(super) async fn download_model(
        &self,
        request: &InitializationRequest,
        callback: impl FnMut(PreparationEvent),
    ) -> Result<PreparedModel, ChatError> {
        let operation = Operation::Initialize(PreparationStage::Downloading);
        let initial = self.create_download_task(request).await?;
        let initial = map_download_task(initial, request)?;
        let mut callback = callback;
        callback(PreparationEvent::DownloadTaskCreated {
            task_id: initial.task_id.clone(),
        });
        let mut downloaded_emitted = false;
        let data = match initial.status.as_str() {
            "queued" | "downloading" => {
                callback(PreparationEvent::DownloadProgress {
                    completed: initial.completed_bytes,
                    total: initial.total_bytes,
                });
                self.poll_download_task(
                    &initial.task_id,
                    request,
                    &mut callback,
                    &mut downloaded_emitted,
                )
                .await?
            }
            "verifying" => {
                emit_downloaded(&mut callback, &mut downloaded_emitted);
                self.poll_download_task(
                    &initial.task_id,
                    request,
                    &mut callback,
                    &mut downloaded_emitted,
                )
                .await?
            }
            "succeeded" => {
                emit_downloaded(&mut callback, &mut downloaded_emitted);
                initial
            }
            "failed" | "cancelled" => {
                return Err(download_task_terminal_error(&initial, operation))
            }
            _ => unreachable!("map_download_task 已校验任务状态"),
        };
        if data.status != "succeeded" {
            return Err(download_task_terminal_error(&data, operation));
        }
        let actual_sha256 = data.actual_sha256.ok_or_else(|| {
            failure(
                ErrorKind::InvalidResult,
                Operation::Initialize(PreparationStage::Verifying),
                "模型下载成功响应缺少实际摘要".into(),
            )
        })?;
        let local_path = data.local_path.ok_or_else(|| {
            failure(
                ErrorKind::InvalidResult,
                Operation::Initialize(PreparationStage::Verifying),
                "模型下载成功响应缺少 Node 文件路径".into(),
            )
        })?;
        if !actual_sha256.eq_ignore_ascii_case(&request.variant.expected_sha256) {
            return Err(failure(
                ErrorKind::VerificationFailed,
                Operation::Initialize(PreparationStage::Verifying),
                "服务端返回的模型摘要与预期不一致".into(),
            ));
        }

        callback(PreparationEvent::Verified {
            expected: request.variant.expected_sha256.clone(),
            actual: actual_sha256.clone(),
            path: local_path.clone(),
        });
        Ok(PreparedModel {
            model: request.model.clone(),
            variant: request.variant.clone(),
            actual_sha256,
            local_path,
        })
    }

    async fn request(
        &self,
        path: &str,
        query: &[(&str, String)],
        operation: Operation,
    ) -> Result<reqwest::Response, ChatError> {
        self.request_raw(path, query, operation)
            .await?
            .error_for_status()
            .map_err(|error| transport(error, operation))
    }

    async fn request_raw(
        &self,
        path: &str,
        query: &[(&str, String)],
        operation: Operation,
    ) -> Result<reqwest::Response, ChatError> {
        let url = self
            .base
            .join(path)
            .map_err(|error| failure(ErrorKind::InvalidParameter, operation, error.to_string()))?;
        self.client
            .get(url)
            .query(query)
            .header("Accept", "application/json, application/x-ndjson")
            .send()
            .await
            .map_err(|error| transport(error, operation))
    }
    async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
        operation: Operation,
    ) -> Result<T, ChatError> {
        self.request(path, query, operation)
            .await?
            .json()
            .await
            .map_err(|error| transport(error, operation))
    }
}

#[derive(Debug, Serialize)]
struct ResolveModelRequest {
    model: String,
    source: String,
    revision: String,
}

impl RemoteHardwareSnapshot {
    fn into_hardware(self) -> Result<Hardware, ChatError> {
        let platform = match self.platform.as_str() {
            "linux" => Some(Platform::Linux),
            "macos" => Some(Platform::MacOs),
            "windows" => Some(Platform::Windows),
            "other" => Some(Platform::Other),
            _ => {
                return Err(failure(
                    ErrorKind::InvalidResult,
                    Operation::Hardware,
                    "硬件接口返回了未知平台".into(),
                ));
            }
        };
        if self.cpu.trim().is_empty() || self._arch.trim().is_empty() {
            return Err(failure(
                ErrorKind::InvalidResult,
                Operation::Hardware,
                "硬件接口返回了空的平台或 CPU 信息".into(),
            ));
        }

        let gpus = self
            .accelerators
            .into_iter()
            .map(RemoteAccelerator::into_gpu)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Hardware {
            platform,
            cpu: Some(self.cpu),
            gpus: Some(gpus),
            ram_bytes: Some(self.memory_bytes),
            disk_bytes: Some(self.disk_available_bytes),
        })
    }
}

impl RemoteModelCandidate {
    fn into_variant(self) -> Result<ModelVariant, ChatError> {
        if self.model_name.trim().is_empty()
            || self.variant_id.trim().is_empty()
            || self.quantization.trim().is_empty()
            || self.runtime.trim().is_empty()
            || self.artifact_uri.trim().is_empty()
            || !valid_digest(&self.checksum)
        {
            return Err(failure(
                ErrorKind::InvalidResult,
                Operation::Resolve,
                "模型解析返回了无效候选版本".into(),
            ));
        }
        Ok(ModelVariant {
            id: self.variant_id,
            quantization: self.quantization,
            runtime: self.runtime,
            artifact_source: self.artifact_uri,
            expected_sha256: self.checksum,
            ram_bytes: self.min_ram_bytes,
            vram_bytes: self.min_vram_bytes,
            disk_bytes: self.disk_size_bytes,
        })
    }
}

impl RemoteAccelerator {
    fn into_gpu(self) -> Result<Gpu, ChatError> {
        let kind = match self.kind.as_str() {
            "nvidia" | "amd" | "intel" | "apple" | "other" => self.kind,
            _ => {
                return Err(failure(
                    ErrorKind::InvalidResult,
                    Operation::Hardware,
                    "硬件接口返回了未知加速器类型".into(),
                ));
            }
        };
        if self.name.trim().is_empty() {
            return Err(failure(
                ErrorKind::InvalidResult,
                Operation::Hardware,
                "硬件接口返回了空加速器名称".into(),
            ));
        }
        Ok(Gpu {
            name: format!("{kind}: {}", self.name),
            available_bytes: self.memory_bytes,
        })
    }
}

fn server_base(host: &str, port: u16) -> String {
    let host = match host {
        "0.0.0.0" => "127.0.0.1".to_string(),
        "::" | "[::]" => "[::1]".to_string(),
        value if value.contains(':') && !value.starts_with('[') => format!("[{value}]"),
        value => value.to_string(),
    };
    format!("http://{host}:{port}")
}

fn remote_error(error: RemoteHardwareError, operation: Operation) -> ChatError {
    let kind = match error.code.as_str() {
        "invalid_model" => ErrorKind::InvalidParameter,
        "model_repository_not_found" => ErrorKind::NotFound,
        "model_repository_unavailable" => ErrorKind::Timeout,
        "unsupported_platform" | "hardware_probe_failed" => ErrorKind::ExecutionFailed,
        "manifest_invalid" | "model_resolution_failed" => ErrorKind::InvalidResult,
        _ => ErrorKind::InvalidResult,
    };
    failure(
        kind,
        operation,
        format!("{}: {}", error.code, error.message),
    )
}

fn invalid_search(reason: &str) -> ChatError {
    failure(ErrorKind::InvalidResult, Operation::Search, reason.into())
}

fn parse_snapshot(bytes: &[u8]) -> Result<CatalogPage<CatalogModel>, ChatError> {
    let snapshot: CatalogPage<CatalogModel> =
        serde_json::from_slice(bytes).map_err(|error| invalid_search(&error.to_string()))?;
    let valid = match (snapshot.done, snapshot.kind.as_deref()) {
        (false, None) => true,
        (true, Some("models")) => !snapshot.items.is_empty(),
        (true, Some("failed" | "unavailable")) => snapshot.items.is_empty(),
        _ => false,
    };
    if !valid {
        return Err(invalid_search("模型搜索快照类型无效"));
    }
    for (index, model) in snapshot.items.iter().enumerate() {
        model.validate()?;
        if model.preset
            || snapshot.items[..index]
                .iter()
                .any(|previous| previous.id == model.id)
        {
            return Err(invalid_search("模型搜索快照含预置项或重复标识"));
        }
    }
    Ok(snapshot)
}

fn valid_resource(model: &str, source: &str, revision: &str) -> bool {
    let safe = |part: &str| {
        !part.is_empty()
            && part.len() <= 128
            && !matches!(part, "." | "..")
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    matches!(source, "modelscope" | "hf-mirror" | "huggingface")
        && safe(revision)
        && model
            .split_once('/')
            .is_some_and(|(org, name)| safe(org) && safe(name))
}

impl CatalogModel {
    /// 在 HTTP 响应边界验证展示项及其资源绑定。
    ///
    /// # Examples
    /// 无来源或含路径跳转的仓库必须拒收。
    ///
    /// # Notes
    /// 不把缺失字段补成虚构仓库。
    pub(super) fn validate(&self) -> Result<(), ChatError> {
        let id_matches_model = self
            .model
            .rsplit('/')
            .next()
            .is_some_and(|name| name.to_ascii_lowercase() == self.id);
        if self.name.trim().is_empty()
            || !valid_resource(&self.model, &self.source, &self.revision)
            || (!self.preset && !id_matches_model)
        {
            return Err(invalid_search("模型目录条目无效或资源不完整"));
        }
        Ok(())
    }
}

impl CatalogVersion {
    /// 验证仓库快照可交接，且资源页面为 HTTP(S) 地址。
    ///
    /// # Examples
    /// 无效来源、空修订号或非 HTTP 页面均拒绝交接。
    ///
    /// # Notes
    /// 校验页面并不意味着它是模型文件。
    pub(super) fn validate(&self) -> Result<(), ChatError> {
        let url_ok = Url::parse(&self.resource_url).is_ok_and(|url| {
            matches!(url.scheme(), "http" | "https")
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
        });
        if self.name.trim().is_empty()
            || !valid_resource(&self.model, &self.source, &self.revision)
            || !url_ok
        {
            return Err(failure(
                ErrorKind::InvalidResult,
                Operation::Status,
                "目录资源信息无效或不完整".into(),
            ));
        }
        Ok(())
    }
}

impl<T> CatalogPage<T> {
    /// 仅完成且没有结果时显示空列表或失败说明。
    ///
    /// # Examples
    /// `unavailable` 显示没有匹配模型，而非模拟预置。
    ///
    /// # Notes
    /// 部分来源失败不掩盖已成功返回的结果。
    pub(super) fn feedback(&self) -> Option<String> {
        if !self.done || !self.items.is_empty() {
            return None;
        }
        match self.kind.as_deref() {
            Some("failed") => Some("模型搜索失败，请重试".into()),
            Some("unavailable") => Some("没有找到匹配模型".into()),
            _ => None,
        }
    }
}

fn download_request_payload(
    request: &InitializationRequest,
    operation: Operation,
) -> Result<DownloadModelRequest, ChatError> {
    let selection = request.model.catalog_selection.as_ref().ok_or_else(|| {
        failure(
            ErrorKind::InvalidParameter,
            operation,
            "目录模型缺少仓库选择快照".into(),
        )
    })?;
    selection.version.validate().map_err(|error| ChatError {
        kind: error.kind,
        operation,
        reason: error.reason,
    })?;
    if !valid_digest(&request.variant.expected_sha256)
        || request.variant.artifact_source.trim().is_empty()
    {
        return Err(failure(
            ErrorKind::InvalidParameter,
            operation,
            "目录模型工件凭据无效".into(),
        ));
    }
    Ok(DownloadModelRequest {
        model: selection.version.model.clone(),
        source: selection.version.source.clone(),
        revision: selection.version.revision.clone(),
        variant_id: request.variant.id.clone(),
        artifact_uri: request.variant.artifact_source.clone(),
        checksum: request.variant.expected_sha256.clone(),
    })
}

async fn decode_download_task(
    response: reqwest::Response,
    operation: Operation,
) -> Result<DownloadTaskData, ChatError> {
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|error| transport(error, operation))?;
    let envelope: DownloadTaskEnvelope = serde_json::from_slice(&body).map_err(|error| {
        failure(
            ErrorKind::InvalidResult,
            operation,
            format!("模型下载任务响应格式无效：{error}"),
        )
    })?;
    if !status.is_success() || !envelope.success {
        let error = envelope.error.ok_or_else(|| {
            failure(
                ErrorKind::InvalidResult,
                operation,
                "模型下载失败响应缺少错误信息".into(),
            )
        })?;
        return Err(download_error(error));
    }
    envelope.data.ok_or_else(|| {
        failure(
            ErrorKind::InvalidResult,
            operation,
            "模型下载成功响应缺少任务数据".into(),
        )
    })
}

fn valid_task_id(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<i64>().is_ok()
}

fn validate_download_task_shape(data: &DownloadTaskData) -> Result<(), ChatError> {
    let operation = if data.status == "verifying" || data.status == "succeeded" {
        Operation::Initialize(PreparationStage::Verifying)
    } else {
        Operation::Initialize(PreparationStage::Downloading)
    };
    if !valid_task_id(&data.task_id)
        || !matches!(
            data.status.as_str(),
            "queued" | "downloading" | "verifying" | "succeeded" | "failed" | "cancelled"
        )
        || data.model.trim().is_empty()
        || data.source.trim().is_empty()
        || data.revision.trim().is_empty()
        || data.variant_id.trim().is_empty()
        || data
            .total_bytes
            .is_some_and(|total| total == 0 || data.completed_bytes > total)
        || data
            .local_path
            .as_deref()
            .is_some_and(|path| path.trim().is_empty() || path.contains('\0'))
        || data
            .actual_sha256
            .as_deref()
            .is_some_and(|digest| !valid_digest(digest))
        || (data.status == "succeeded"
            && (data.local_path.is_none() || data.actual_sha256.is_none()))
        || (data.status == "failed"
            && data.error.as_ref().is_none_or(|error| {
                error.code.trim().is_empty() || error.message.trim().is_empty()
            }))
    {
        return Err(failure(
            ErrorKind::InvalidResult,
            operation,
            "模型下载任务响应字段无效".into(),
        ));
    }
    Ok(())
}

fn map_download_task(
    data: DownloadTaskData,
    request: &InitializationRequest,
) -> Result<DownloadTaskData, ChatError> {
    validate_download_task_shape(&data)?;
    let operation = if data.status == "verifying" || data.status == "succeeded" {
        Operation::Initialize(PreparationStage::Verifying)
    } else {
        Operation::Initialize(PreparationStage::Downloading)
    };
    let selection = request.model.catalog_selection.as_ref().ok_or_else(|| {
        failure(
            ErrorKind::InvalidParameter,
            operation,
            "目录模型缺少仓库选择快照".into(),
        )
    })?;
    if data.model != selection.version.model
        || data.source != selection.version.source
        || data.revision != selection.version.revision
        || data.variant_id != request.variant.id
    {
        return Err(failure(
            ErrorKind::InvalidResult,
            operation,
            "模型下载任务身份与当前选择不一致".into(),
        ));
    }
    Ok(data)
}

fn download_task_terminal_error(data: &DownloadTaskData, operation: Operation) -> ChatError {
    let error = data.error.clone().unwrap_or_else(|| RemoteHardwareError {
        code: "download_failed".into(),
        message: if data.status == "cancelled" {
            "模型下载任务已取消".into()
        } else {
            "模型文件下载失败".into()
        },
    });
    let mut mapped = download_error(error);
    if matches!(mapped.operation, Operation::Initialize(_)) {
        mapped.operation = operation;
    }
    mapped
}

fn emit_downloaded(callback: &mut impl FnMut(PreparationEvent), emitted: &mut bool) {
    if !*emitted {
        callback(PreparationEvent::Downloaded);
        *emitted = true;
    }
}

fn download_error(error: RemoteHardwareError) -> ChatError {
    let operation = if error.code == "checksum_mismatch" || error.code == "download_storage_failed"
    {
        Operation::Initialize(PreparationStage::Verifying)
    } else {
        Operation::Initialize(PreparationStage::Downloading)
    };
    let kind = match error.code.as_str() {
        "invalid_download_request" => ErrorKind::InvalidParameter,
        "checksum_mismatch" => ErrorKind::VerificationFailed,
        "download_service_unavailable"
        | "download_failed"
        | "download_task_not_found"
        | "download_task_not_cancellable" => ErrorKind::ExecutionFailed,
        "download_storage_failed" => ErrorKind::VerificationFailed,
        _ => ErrorKind::InvalidResult,
    };
    failure(
        kind,
        operation,
        format!("{}: {}", error.code, error.message),
    )
}

fn failure(kind: ErrorKind, operation: Operation, reason: String) -> ChatError {
    ChatError {
        kind,
        operation,
        reason,
    }
}
fn transport(error: reqwest::Error, operation: Operation) -> ChatError {
    let kind = if error.is_timeout() {
        ErrorKind::Timeout
    } else if error.is_decode() {
        ErrorKind::InvalidResult
    } else {
        ErrorKind::NotConnected
    };
    failure(
        kind,
        operation,
        format!("模型服务请求失败，请确认服务已启动后重试：{error}"),
    )
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
