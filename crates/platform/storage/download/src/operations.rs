use std::path::PathBuf;
use std::sync::Arc;

use burncloud_download_aria2::{
    quick_start, AddUriOutcome, Aria2Error, Aria2RpcClient, DownloadOptions,
};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::error::{DownloadError, Result};
use crate::manager::{
    invalid_record, DownloadManager, DownloadStatus, DownloadWaiter, MonitorState, MAX_RETRIES,
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
    /// 成功时返回数据库记录中的绝对文件路径，并确认该文件仍存在。
    /// 下载目录必须位于可执行文件同级的 `models` 子目录内；回环、私有和链路本地目标地址会被拒绝。
    pub async fn download(&self, url: &str, dir: &str) -> Result<PathBuf> {
        validate_url(url).await?;
        let filename = extract_filename_from_url(url)
            .as_deref()
            .map(sanitize_filename)
            .transpose()?
            .ok_or(DownloadError::InvalidFilename)?;
        let directory = normalize_download_dir(dir).await?;
        let directory_text = directory.to_string_lossy().into_owned();

        let receiver = {
            let _start_guard = self.start_lock.lock().await;
            let (sender, receiver) = oneshot::channel();
            let (receiver, sender) = match self
                .attach_waiter(url, &directory_text, &filename, sender)
                .await
            {
                Ok(()) => (receiver, None),
                Err(sender) => (receiver, Some(sender)),
            };

            if let Some(sender) = sender {
                if let Some(record) = self
                    .db
                    .find_completed(url, &directory_text, &filename)
                    .await?
                {
                    if let Some(path) = record.final_path.as_deref() {
                        if let Some(path) = existing_database_file(path).await? {
                            return Ok(path);
                        }
                    }
                }

                self.start_or_join(url, directory, filename, sender).await?;
            }
            receiver
        };
        receive_result(receiver).await
    }

    async fn start_or_join(
        &self,
        url: &str,
        directory: PathBuf,
        filename: String,
        sender: DownloadWaiter,
    ) -> Result<()> {
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
            .values_mut()
            .find(|state| state.has_key(url, &directory_text, &filename))
        {
            state.waiters.push(sender);
            return Ok(());
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
            state.waiters.push(sender);
            return Ok(());
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
            waiters: vec![sender],
        };
        states.insert(gid, state);
        Ok(())
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

    async fn attach_waiter(
        &self,
        url: &str,
        dir: &str,
        filename: &str,
        sender: DownloadWaiter,
    ) -> std::result::Result<(), DownloadWaiter> {
        let mut states = self.monitor_states.lock().await;
        if let Some(state) = states
            .values_mut()
            .find(|state| state.has_key(url, dir, filename))
        {
            state.waiters.push(sender);
            Ok(())
        } else {
            Err(sender)
        }
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
                let outcome = client
                    .add_uri(
                        vec![url.clone()],
                        Some(download_options(&directory_text, &filename)),
                    )
                    .await?;
                gid = outcome.gid().to_string();
                if let Err(error) = self
                    .db
                    .update_gid_by_id(record.id, &gid, i64::from(retry_count))
                    .await
                {
                    if outcome.is_created() {
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

    fn rpc_client(&self) -> Result<Aria2RpcClient> {
        self.aria2
            .create_rpc_client()
            .ok_or_else(|| Aria2Error::RpcError("aria2 RPC 客户端未就绪".to_string()).into())
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
