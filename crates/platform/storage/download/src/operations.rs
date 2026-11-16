use std::path::PathBuf;
use std::sync::Arc;

use burncloud_download_aria2::{
    quick_start, AddUriOutcome, Aria2Error, Aria2RpcClient, DownloadOptions,
};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::error::{DownloadError, Result};
use crate::manager::{
    invalid_record, DownloadManager, DownloadStatus, DownloadTaskHandle, DownloadTaskSnapshot,
    DownloadTaskStatus, DownloadWaiter, MonitorState, MAX_RETRIES,
};
use crate::monitor::monitor_loop;
use crate::utils::{
    existing_database_file, extract_filename_from_url, normalize_download_dir, sanitize_filename,
    validate_url,
};

struct DownloadTarget<'a> {
    url: &'a str,
    directory: &'a str,
    filename: &'a str,
}

impl DownloadManager {
    /// 初始化 aria2 与数据库，恢复未完成任务并启动监控循环。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = burncloud_download::DownloadManager::new().await?;
    /// # drop(manager);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # 注意
    ///
    /// 此方法需要 Tokio 运行时，并会启动 aria2 服务。
    pub async fn new() -> Result<Self> {
        let db = Arc::new(burncloud_database_sys::DownloadDB::new().await?);
        let aria2 = Arc::new(quick_start().await?);
        let monitor_states = Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        let cancellation = CancellationToken::new();
        let mut manager = Self {
            aria2,
            db,
            monitor_states,
            start_lock: Arc::new(tokio::sync::Mutex::new(())),
            task_lock: Arc::new(tokio::sync::Mutex::new(())),
            cancellation,
            monitor: None,
        };
        if let Err(operation) = manager.restore_incomplete().await {
            let cleanup = match Arc::get_mut(&mut manager.aria2) {
                Some(aria2) => aria2.shutdown().await.map_err(|error| error.to_string()),
                None => Err("aria2 管理器仍被其他任务引用".to_string()),
            };
            return match cleanup {
                Ok(()) => Err(operation),
                Err(cleanup) => Err(DownloadError::Compensation {
                    operation: operation.to_string(),
                    cleanup,
                }),
            };
        }

        let aria2 = Arc::clone(&manager.aria2);
        let db = Arc::clone(&manager.db);
        let states = Arc::clone(&manager.monitor_states);
        let cancellation = manager.cancellation.clone();
        manager.monitor = Some(tokio::spawn(async move {
            monitor_loop(aria2, db, states, cancellation).await;
        }));

        Ok(manager)
    }

    /// 创建或复用下载任务，并在任务登记后立即返回。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// # async fn example(
    /// #     manager: &burncloud_download::DownloadManager,
    /// # ) -> Result<(), Box<dyn std::error::Error>> {
    /// let handle = manager
    ///     .start_download("https://example.com/model.gguf", "models")
    ///     .await?;
    /// println!("task {}", handle.task_id);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # 注意
    ///
    /// 此方法只负责校验、登记和监控注册，不等待下载完成；`gid` 只在当前 aria2
    /// 执行期间稳定，调用方应保存 `task_id`。
    pub async fn start_download(&self, url: &str, dir: &str) -> Result<DownloadTaskHandle> {
        validate_url(url).await?;
        let filename = extract_filename_from_url(url)
            .as_deref()
            .map(sanitize_filename)
            .transpose()?
            .ok_or(DownloadError::InvalidFilename)?;
        let directory = normalize_download_dir(dir).await?;
        let directory_text = directory.to_string_lossy().into_owned();

        let _start_guard = self.start_lock.lock().await;
        if let Some(state) = self
            .monitor_states
            .lock()
            .await
            .values()
            .find(|state| state.has_key(url, &directory_text, &filename))
        {
            return Ok(handle_from_monitor_state(state));
        }

        if let Some(record) = self
            .find_reusable_record(url, &directory_text, &filename)
            .await?
        {
            return handle_from_record(&record);
        }

        self.start_or_join(url, directory, filename).await
    }

    /// 读取指定稳定任务 ID 的当前下载快照。
    ///
    /// # 示例
    ///
    /// 查询成功但没有对应记录时返回 `None`，数据库或字段解析失败则返回错误。
    ///
    /// # 注意
    ///
    /// 任务快照只包含 aria2 与数据库可见的下载信息；摘要校验由上层服务维护。
    pub async fn get_task(&self, task_id: i64) -> Result<Option<DownloadTaskSnapshot>> {
        let record = self
            .db
            .list(None)
            .await?
            .into_iter()
            .find(|record| record.id == task_id);
        let Some(record) = record else {
            return Ok(None);
        };

        let state = self
            .monitor_states
            .lock()
            .await
            .values()
            .find(|state| state.db_id == task_id)
            .map(|state| {
                (
                    state.gid.clone(),
                    state.url.clone(),
                    state.dir.clone(),
                    state.filename.clone(),
                    public_status(state.status),
                    state.total_length,
                    state.completed_length,
                    state.download_speed,
                )
            });

        let (gid, url, download_dir, filename, status, total_length, completed_length, speed) =
            match state {
                Some(state) => state,
                None => (
                    record.gid.clone(),
                    record_url(&record)?,
                    record_download_dir(&record)?,
                    record_filename(&record)?,
                    status_from_record(&record.status).ok_or_else(|| {
                        invalid_record(format!(
                            "任务 {} 的数据库状态无效：{}",
                            record.id, record.status
                        ))
                    })?,
                    record.total_length.unwrap_or(0),
                    record.completed_length.unwrap_or(0),
                    record.download_speed.unwrap_or(0),
                ),
            };

        Ok(Some(DownloadTaskSnapshot {
            task_id: record.id,
            gid,
            url,
            download_dir,
            filename,
            status,
            completed_bytes: non_negative_bytes(completed_length, "completed_length")?,
            total_bytes: optional_non_negative_bytes(total_length, "total_length")?,
            download_speed: non_negative_bytes(speed, "download_speed")?,
            error: record.error,
            final_path: record.final_path.map(PathBuf::from),
        }))
    }

    /// 取消仍处于可取消状态的下载任务。
    ///
    /// # 示例
    ///
    /// 已取消任务可以重复调用并返回同一取消快照；成功、失败等终态不会被伪装成取消成功。
    ///
    /// # 注意
    ///
    /// aria2 移除与数据库持久化按固定顺序执行；持久化失败会返回补偿错误，调用方不得
    /// 把它当作已取消处理。
    pub async fn cancel_task(&self, task_id: i64) -> Result<DownloadTaskSnapshot> {
        let _task_guard = self.task_lock.lock().await;
        let Some(snapshot) = self.get_task(task_id).await? else {
            return Err(DownloadError::TaskNotFound);
        };
        match snapshot.status {
            DownloadTaskStatus::Cancelled => return Ok(snapshot),
            DownloadTaskStatus::Succeeded | DownloadTaskStatus::Failed => {
                return Err(DownloadError::TaskNotCancellable(format!(
                    "任务当前状态为 {:?}",
                    snapshot.status
                )))
            }
            DownloadTaskStatus::Queued
            | DownloadTaskStatus::Downloading
            | DownloadTaskStatus::Verifying => {}
        }

        let should_remove_aria2 = matches!(
            snapshot.status,
            DownloadTaskStatus::Queued | DownloadTaskStatus::Downloading
        );
        if should_remove_aria2 {
            let client = self.rpc_client()?;
            if let Err(error) = client.remove(&snapshot.gid).await {
                return Err(DownloadError::Aria2(error));
            }
        }

        if let Err(error) = self
            .db
            .mark_cancelled(snapshot.task_id, &snapshot.gid)
            .await
        {
            return Err(DownloadError::Compensation {
                operation: error.to_string(),
                cleanup: if should_remove_aria2 {
                    "aria2 任务已移除，无法自动恢复原任务".to_string()
                } else {
                    "无需移除 aria2 任务".to_string()
                },
            });
        }

        let waiters = self.remove_monitor_state(task_id).await;
        notify_cancelled_waiters(&snapshot.gid, waiters).await;
        self.get_task(task_id)
            .await?
            .ok_or(DownloadError::TaskNotFound)
    }

    /// 等待指定下载任务完成并返回现存的最终文件路径。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// # async fn example(
    /// #     manager: &burncloud_download::DownloadManager,
    /// # ) -> Result<(), Box<dyn std::error::Error>> {
    /// let handle = manager
    ///     .start_download("https://example.com/model.gguf", "models")
    ///     .await?;
    /// let path = manager.wait_for_task(handle.task_id).await?;
    /// println!("{}", path.display());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # 注意
    ///
    /// 等待复用监控器已有的通知通道；客户端 HTTP 连接断开不会影响后台任务。
    pub async fn wait_for_task(&self, task_id: i64) -> Result<PathBuf> {
        let receiver = {
            let _start_guard = self.start_lock.lock().await;
            let Some(snapshot) = self.get_task(task_id).await? else {
                return Err(DownloadError::TaskNotFound);
            };
            match snapshot.status {
                DownloadTaskStatus::Succeeded | DownloadTaskStatus::Verifying => {
                    return final_path_from_snapshot(&snapshot).await;
                }
                DownloadTaskStatus::Failed => {
                    return Err(DownloadError::Failed(
                        snapshot.error.unwrap_or_else(|| "下载任务失败".to_string()),
                    ));
                }
                DownloadTaskStatus::Cancelled => {
                    return Err(DownloadError::Failed(
                        snapshot
                            .error
                            .unwrap_or_else(|| "下载任务已取消".to_string()),
                    ));
                }
                DownloadTaskStatus::Queued | DownloadTaskStatus::Downloading => {}
            }

            let (sender, receiver) = oneshot::channel();
            let mut states = self.monitor_states.lock().await;
            let Some(state) = states.values_mut().find(|state| state.db_id == task_id) else {
                return Err(DownloadError::NotificationClosed);
            };
            state.waiters.push(sender);
            receiver
        };
        receive_result(receiver).await
    }

    /// 下载 HTTP 或 HTTPS 资源，并在任务完成后返回最终文件路径。
    ///
    /// # 示例
    ///
    /// ```no_run
    /// # async fn example(
    /// #     manager: &burncloud_download::DownloadManager,
    /// # ) -> Result<(), Box<dyn std::error::Error>> {
    /// let path = manager
    ///     .download("https://example.com/model.gguf", "models")
    ///     .await?;
    /// println!("{}", path.display());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # 注意
    ///
    /// 这是兼容旧调用方的等待包装：先调用 `start_download` 登记任务，再调用
    /// `wait_for_task` 等待同一稳定任务 ID。新模型下载接口应直接使用分离的异步生命周期。
    pub async fn download(&self, url: &str, dir: &str) -> Result<PathBuf> {
        let handle = self.start_download(url, dir).await?;
        self.wait_for_task(handle.task_id).await
    }

    async fn start_or_join(
        &self,
        url: &str,
        directory: PathBuf,
        filename: String,
    ) -> Result<DownloadTaskHandle> {
        let client = self.rpc_client()?;
        let directory_text = directory.to_string_lossy().into_owned();
        let outcome = client
            .add_uri(
                vec![url.to_string()],
                Some(download_options(&directory_text, &filename)),
            )
            .await?;
        let gid = outcome.gid().to_string();
        let db_id = self
            .persist_task(
                &client,
                outcome,
                &gid,
                DownloadTarget {
                    url,
                    directory: &directory_text,
                    filename: &filename,
                },
            )
            .await?;

        let mut states = self.monitor_states.lock().await;
        if let Some(state) = states
            .values()
            .find(|state| state.has_key(url, &directory_text, &filename))
        {
            return Ok(handle_from_monitor_state(state));
        }
        if let Some(state) = states.get_mut(&gid) {
            if state.db_id != db_id
                || state.url != url
                || state.dir != directory
                || state.filename != filename
            {
                return Err(invalid_record(format!(
                    "aria2 GID {gid} is already registered to a different download"
                )));
            }
            return Ok(handle_from_monitor_state(state));
        }

        let state = MonitorState {
            db_id,
            gid: gid.clone(),
            url: url.to_string(),
            dir: directory,
            filename,
            status: DownloadStatus::Pending,
            retry_count: 0,
            max_retries: MAX_RETRIES,
            total_length: 0,
            completed_length: 0,
            download_speed: 0,
            waiters: Vec::new(),
        };
        let handle = handle_from_monitor_state(&state);
        states.insert(gid, state);
        Ok(handle)
    }

    async fn persist_task(
        &self,
        client: &Aria2RpcClient,
        outcome: AddUriOutcome,
        gid: &str,
        target: DownloadTarget<'_>,
    ) -> Result<i64> {
        let DownloadTarget {
            url,
            directory,
            filename,
        } = target;
        match outcome {
            AddUriOutcome::Created(_) => {
                match self.db.insert_task(gid, url, directory, filename).await {
                    Ok(id) => Ok(id),
                    Err(error) => match client.remove(gid).await {
                        Ok(_) => Err(error.into()),
                        Err(cleanup) => Err(DownloadError::Compensation {
                            operation: error.to_string(),
                            cleanup: cleanup.to_string(),
                        }),
                    },
                }
            }
            AddUriOutcome::Existing(_) => {
                if let Some(record) = self.db.get_by_gid(gid).await? {
                    return Ok(record.id);
                }
                match self.db.insert_task(gid, url, directory, filename).await {
                    Ok(id) => Ok(id),
                    Err(insert_error) => self
                        .db
                        .get_by_gid(gid)
                        .await?
                        .map(|record| record.id)
                        .ok_or(insert_error.into()),
                }
            }
        }
    }

    async fn find_reusable_record(
        &self,
        url: &str,
        directory: &str,
        filename: &str,
    ) -> Result<Option<burncloud_database_sys::SysDownload>> {
        for record in self.db.list(None).await? {
            if record.url.as_deref() != Some(url)
                || record.download_dir.as_deref() != Some(directory)
                || record.filename.as_deref() != Some(filename)
            {
                continue;
            }
            if matches!(record.status.as_str(), "failed" | "cancelled") {
                continue;
            }
            if matches!(
                record.status.as_str(),
                "completed" | "succeeded" | "verifying"
            ) {
                let Some(path) = record.final_path.as_deref() else {
                    continue;
                };
                if existing_database_file(path).await?.is_none() {
                    continue;
                }
            }
            return Ok(Some(record));
        }
        Ok(None)
    }

    async fn remove_monitor_state(&self, task_id: i64) -> Vec<DownloadWaiter> {
        let mut states = self.monitor_states.lock().await;
        let Some(gid) = states
            .iter()
            .find_map(|(gid, state)| (state.db_id == task_id).then(|| gid.clone()))
        else {
            return Vec::new();
        };
        states
            .remove(&gid)
            .map(|mut state| state.take_waiters())
            .unwrap_or_default()
    }

    async fn restore_incomplete(&mut self) -> Result<()> {
        let records = self.db.list_incomplete().await?;
        let client = self.rpc_client()?;

        for record in records {
            let url = match record.url.clone() {
                Some(url) if !url.is_empty() => url,
                _ => serde_json::from_str::<Vec<String>>(&record.uris)
                    .map_err(|error| {
                        invalid_record(format!("任务 {} 的 URI 数据无效：{error}", record.id))
                    })?
                    .into_iter()
                    .next()
                    .ok_or_else(|| invalid_record(format!("任务 {} 缺少有效 URL", record.id)))?,
            };
            let saved_dir = record
                .download_dir
                .as_deref()
                .filter(|dir| !dir.is_empty())
                .unwrap_or("models");
            let directory = normalize_download_dir(saved_dir).await?;
            let directory_text = directory.to_string_lossy().into_owned();
            let filename = record
                .filename
                .as_deref()
                .filter(|filename| !filename.is_empty())
                .map(sanitize_filename)
                .or_else(|| {
                    extract_filename_from_url(&url).map(|filename| sanitize_filename(&filename))
                })
                .transpose()?
                .ok_or_else(|| invalid_record(format!("任务 {} 缺少有效文件名", record.id)))?;
            let retry_count = u8::try_from(record.retry_count)
                .map_err(|_| invalid_record(format!("任务 {} 的重试次数无效", record.id)))?;
            let mut gid = record.gid.clone();

            if let Err(error) = client.tell_status(&gid).await {
                tracing::warn!(
                    db_id = record.id,
                    old_gid = %gid,
                    error = %error,
                    "aria2 中找不到已保存任务，尝试重建"
                );
                let (rebuilt_gid, rebuilt_task_created) = Self::rebuild_missing_task(
                    &client,
                    &url,
                    &directory_text,
                    &filename,
                    &record.gid,
                    record.id,
                )
                .await?;
                gid = rebuilt_gid;
                if let Err(error) = self
                    .db
                    .update_gid_by_id(record.id, &gid, i64::from(retry_count))
                    .await
                {
                    if rebuilt_task_created {
                        return match client.remove(&gid).await {
                            Ok(_) => Err(error.into()),
                            Err(cleanup) => Err(DownloadError::Compensation {
                                operation: error.to_string(),
                                cleanup: cleanup.to_string(),
                            }),
                        };
                    }
                    return Err(error.into());
                }
            }

            let status = DownloadStatus::from_aria2(&record.status).ok_or_else(|| {
                invalid_record(format!(
                    "任务 {} 的数据库状态无效：{}",
                    record.id, record.status
                ))
            })?;
            let state = MonitorState {
                db_id: record.id,
                gid: gid.clone(),
                url,
                dir: directory,
                filename,
                status,
                retry_count,
                max_retries: MAX_RETRIES,
                total_length: record.total_length.unwrap_or(0),
                completed_length: record.completed_length.unwrap_or(0),
                download_speed: record.download_speed.unwrap_or(0),
                waiters: Vec::new(),
            };
            let previous = self.monitor_states.lock().await.insert(gid.clone(), state);
            if previous.is_some() {
                return Err(invalid_record(format!("数据库中存在重复 aria2 GID：{gid}")));
            }
        }
        Ok(())
    }

    async fn rebuild_missing_task(
        client: &Aria2RpcClient,
        url: &str,
        directory: &str,
        filename: &str,
        old_gid: &str,
        record_id: i64,
    ) -> Result<(String, bool)> {
        let options = download_options(directory, filename);
        let mut outcome = client.add_uri(vec![url.to_string()], Some(options)).await?;
        if outcome.gid() == old_gid {
            // aria2 偶尔会把恢复请求匹配到旧 GID；先移除该匹配任务，再重新创建，避免数据库继续指向旧任务。
            client.remove(outcome.gid()).await?;
            outcome = client
                .add_uri(
                    vec![url.to_string()],
                    Some(download_options(directory, filename)),
                )
                .await?;
            if outcome.gid() == old_gid {
                client.remove(outcome.gid()).await?;
                return Err(invalid_record(format!(
                    "任务 {record_id} 重建后仍复用了旧 aria2 GID：{old_gid}"
                )));
            }
        }
        Ok((outcome.gid().to_string(), outcome.is_created()))
    }

    fn rpc_client(&self) -> Result<Aria2RpcClient> {
        self.aria2
            .create_rpc_client()
            .ok_or_else(|| Aria2Error::RpcError("aria2 RPC 客户端未就绪".to_string()).into())
    }
}

fn public_status(status: DownloadStatus) -> DownloadTaskStatus {
    match status {
        DownloadStatus::Pending => DownloadTaskStatus::Queued,
        DownloadStatus::Active | DownloadStatus::Waiting | DownloadStatus::Paused => {
            DownloadTaskStatus::Downloading
        }
        DownloadStatus::Completed => DownloadTaskStatus::Verifying,
        DownloadStatus::Failed => DownloadTaskStatus::Failed,
        DownloadStatus::Cancelled => DownloadTaskStatus::Cancelled,
    }
}

fn status_from_record(value: &str) -> Option<DownloadTaskStatus> {
    match value {
        "pending" => Some(DownloadTaskStatus::Queued),
        "active" | "waiting" | "paused" => Some(DownloadTaskStatus::Downloading),
        "completed" | "complete" | "verifying" => Some(DownloadTaskStatus::Verifying),
        "succeeded" => Some(DownloadTaskStatus::Succeeded),
        "failed" => Some(DownloadTaskStatus::Failed),
        "cancelled" | "canceled" | "removed" => Some(DownloadTaskStatus::Cancelled),
        _ => None,
    }
}

fn handle_from_monitor_state(state: &MonitorState) -> DownloadTaskHandle {
    DownloadTaskHandle {
        task_id: state.db_id,
        gid: state.gid.clone(),
        status: public_status(state.status),
    }
}

fn handle_from_record(record: &burncloud_database_sys::SysDownload) -> Result<DownloadTaskHandle> {
    let status = status_from_record(&record.status).ok_or_else(|| {
        invalid_record(format!(
            "任务 {} 的数据库状态无效：{}",
            record.id, record.status
        ))
    })?;
    Ok(DownloadTaskHandle {
        task_id: record.id,
        gid: record.gid.clone(),
        status,
    })
}

fn record_url(record: &burncloud_database_sys::SysDownload) -> Result<String> {
    if let Some(url) = record.url.as_deref().filter(|url| !url.is_empty()) {
        return Ok(url.to_string());
    }
    serde_json::from_str::<Vec<String>>(&record.uris)
        .map_err(|error| invalid_record(format!("任务 {} 的 URI 数据无效：{error}", record.id)))?
        .into_iter()
        .next()
        .filter(|url| !url.is_empty())
        .ok_or_else(|| invalid_record(format!("任务 {} 缺少有效 URL", record.id)))
}

fn record_download_dir(record: &burncloud_database_sys::SysDownload) -> Result<PathBuf> {
    record
        .download_dir
        .as_deref()
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| invalid_record(format!("任务 {} 缺少下载目录", record.id)))
}

fn record_filename(record: &burncloud_database_sys::SysDownload) -> Result<String> {
    record
        .filename
        .as_deref()
        .filter(|filename| !filename.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            record_url(record)
                .ok()
                .and_then(|url| extract_filename_from_url(&url))
        })
        .ok_or_else(|| invalid_record(format!("任务 {} 缺少有效文件名", record.id)))
}

fn non_negative_bytes(value: i64, field: &str) -> Result<u64> {
    u64::try_from(value).map_err(|_| invalid_record(format!("任务字段 {field} 不能为负数")))
}

fn optional_non_negative_bytes(value: i64, field: &str) -> Result<Option<u64>> {
    if value == 0 {
        Ok(None)
    } else {
        non_negative_bytes(value, field).map(Some)
    }
}

async fn final_path_from_snapshot(snapshot: &DownloadTaskSnapshot) -> Result<PathBuf> {
    let Some(path) = snapshot.final_path.as_deref() else {
        return Err(DownloadError::Failed(
            "下载任务缺少最终文件路径".to_string(),
        ));
    };
    existing_database_file(&path.to_string_lossy())
        .await?
        .ok_or_else(|| DownloadError::Failed("下载任务的最终文件路径无效".to_string()))
}

async fn notify_cancelled_waiters(gid: &str, waiters: Vec<DownloadWaiter>) {
    for waiter in waiters {
        if waiter.send(Err("下载任务已取消".to_string())).is_err() {
            tracing::debug!(gid = %gid, "下载调用方已停止等待");
        }
    }
}

fn download_options(dir: &str, filename: &str) -> DownloadOptions {
    DownloadOptions {
        dir: Some(dir.to_string()),
        out: Some(filename.to_string()),
        split: None,
        max_connection_per_server: None,
        continue_download: Some(true),
        allow_overwrite: Some(true),
        auto_file_renaming: Some(false),
    }
}

async fn receive_result(
    receiver: oneshot::Receiver<std::result::Result<PathBuf, String>>,
) -> Result<PathBuf> {
    match receiver.await {
        Ok(Ok(path)) => Ok(path),
        Ok(Err(error)) => Err(DownloadError::Failed(error)),
        Err(_) => Err(DownloadError::NotificationClosed),
    }
}
