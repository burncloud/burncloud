use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use burncloud_database_sys::DownloadDB;
use burncloud_download_aria2::{Aria2Manager, Aria2RpcClient, DownloadOptions};
use tokio::sync::Mutex;
use tokio::time::{interval, MissedTickBehavior};
use tokio_util::sync::CancellationToken;

use crate::error::{DownloadError, Result};
use crate::manager::{DownloadStatus, MonitorState};

pub(super) async fn monitor_loop(
    aria2: Arc<Aria2Manager>,
    db: Arc<DownloadDB>,
    states: Arc<Mutex<std::collections::HashMap<String, MonitorState>>>,
    cancellation: CancellationToken,
) {
    let mut ticker = interval(Duration::from_secs(2));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = cancellation.cancelled() => break,
            _ = ticker.tick() => {
                let gids = states.lock().await.keys().cloned().collect::<Vec<_>>();
                for gid in gids {
                    tokio::select! {
                        _ = cancellation.cancelled() => return,
                        result = monitor_task(&aria2, &db, &states, &gid) => {
                            if let Err(error) = result {
                                tracing::error!(gid = %gid, error = %error, "下载监控操作失败");
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn monitor_task(
    aria2: &Aria2Manager,
    db: &DownloadDB,
    states: &Mutex<std::collections::HashMap<String, MonitorState>>,
    gid: &str,
) -> Result<()> {
    let client = aria2.create_rpc_client().ok_or_else(|| {
        DownloadError::Aria2(burncloud_download_aria2::Aria2Error::RpcError(
            "aria2 RPC 客户端未就绪".to_string(),
        ))
    })?;
    let status = client.tell_status(gid).await?;
    let task_status = match DownloadStatus::from_aria2(&status.status) {
        Some(status) => status,
        None => {
            finish_failed(
                db,
                states,
                gid,
                format!("aria2 返回未知任务状态：{}", status.status),
            )
            .await;
            return Ok(());
        }
    };
    let Some(snapshot) = snapshot(states, gid).await else {
        return Ok(());
    };
    let (total_length, completed_length, download_speed) = match (
        parse_length(&status.total_length, "totalLength"),
        parse_length(&status.completed_length, "completedLength"),
        parse_length(&status.download_speed, "downloadSpeed"),
    ) {
        (Ok(total), Ok(completed), Ok(speed)) => (total, completed, speed),
        (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => {
            finish_failed(db, states, gid, error.to_string()).await;
            return Ok(());
        }
    };

    match task_status {
        DownloadStatus::Completed => {
            complete_task(
                &client,
                db,
                states,
                gid,
                snapshot,
                (total_length, completed_length),
            )
            .await
        }
        DownloadStatus::Failed => handle_failed(aria2, db, states, gid, &status, snapshot).await,
        DownloadStatus::Cancelled => {
            db.mark_cancelled(snapshot.db_id, gid).await?;
            finish_cancelled(states, gid).await;
            Ok(())
        }
        DownloadStatus::Pending
        | DownloadStatus::Active
        | DownloadStatus::Waiting
        | DownloadStatus::Paused => {
            update_progress(
                db,
                states,
                gid,
                &snapshot,
                task_status,
                (total_length, completed_length, download_speed),
            )
            .await
        }
    }
}

struct TaskSnapshot {
    db_id: i64,
    url: String,
    dir: PathBuf,
    filename: String,
    retry_count: u8,
    max_retries: u8,
    status: DownloadStatus,
    total_length: i64,
    completed_length: i64,
    download_speed: i64,
}

async fn snapshot(
    states: &Mutex<std::collections::HashMap<String, MonitorState>>,
    gid: &str,
) -> Option<TaskSnapshot> {
    let states = states.lock().await;
    let state = states.get(gid)?;
    Some(TaskSnapshot {
        db_id: state.db_id,
        url: state.url.clone(),
        dir: state.dir.clone(),
        filename: state.filename.clone(),
        retry_count: state.retry_count,
        max_retries: state.max_retries,
        status: state.status,
        total_length: state.total_length,
        completed_length: state.completed_length,
        download_speed: state.download_speed,
    })
}

async fn complete_task(
    client: &Aria2RpcClient,
    db: &DownloadDB,
    states: &Mutex<std::collections::HashMap<String, MonitorState>>,
    gid: &str,
    snapshot: TaskSnapshot,
    size: (i64, i64),
) -> Result<()> {
    let files = client.get_files(gid).await?;
    let Some(file) = files.first() else {
        finish_failed(db, states, gid, "aria2 完成任务未返回文件信息".to_string()).await;
        return Ok(());
    };
    let final_path = PathBuf::from(&file.path);
    if !final_path.starts_with(&snapshot.dir) {
        finish_failed(
            db,
            states,
            gid,
            format!("aria2 返回的文件路径超出下载目录：{}", final_path.display()),
        )
        .await;
        return Ok(());
    }
    match tokio::fs::metadata(&final_path).await {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => {
            finish_failed(
                db,
                states,
                gid,
                format!("下载目标不是文件：{}", final_path.display()),
            )
            .await;
            return Ok(());
        }
        Err(error) => {
            finish_failed(db, states, gid, format!("读取下载文件信息失败：{error}")).await;
            return Ok(());
        }
    }
    let final_path_text = final_path.to_string_lossy().into_owned();
    db.mark_completed(snapshot.db_id, gid, &final_path_text, size.0, size.1)
        .await?;
    let waiters = take_waiters(states, gid).await;
    notify_waiters(gid, waiters, Ok(final_path)).await;
    Ok(())
}

async fn handle_failed(
    aria2: &Aria2Manager,
    db: &DownloadDB,
    states: &Mutex<std::collections::HashMap<String, MonitorState>>,
    gid: &str,
    status: &burncloud_download_aria2::DownloadStatus,
    snapshot: TaskSnapshot,
) -> Result<()> {
    let error = status.error_message.clone().unwrap_or_else(|| {
        format!(
            "aria2 errorCode={}",
            status.error_code.as_deref().unwrap_or("unknown")
        )
    });
    if snapshot.retry_count < snapshot.max_retries
        && is_retryable_error(status.error_code.as_deref())
    {
        if let Err(error) = retry_task(aria2, db, states, gid, &snapshot).await {
            finish_failed(db, states, gid, error).await;
        }
    } else {
        finish_failed(db, states, gid, error).await;
    }
    Ok(())
}

async fn update_progress(
    db: &DownloadDB,
    states: &Mutex<std::collections::HashMap<String, MonitorState>>,
    gid: &str,
    snapshot: &TaskSnapshot,
    status: DownloadStatus,
    progress: (i64, i64, i64),
) -> Result<()> {
    if snapshot.status == status
        && snapshot.total_length == progress.0
        && snapshot.completed_length == progress.1
        && snapshot.download_speed == progress.2
    {
        return Ok(());
    }
    db.update_task_progress(snapshot.db_id, gid, status.as_str(), progress)
        .await?;
    if let Some(state) = states.lock().await.get_mut(gid) {
        state.update_progress(status, progress.0, progress.1, progress.2);
    }
    Ok(())
}

async fn retry_task(
    aria2: &Aria2Manager,
    db: &DownloadDB,
    states: &Mutex<std::collections::HashMap<String, MonitorState>>,
    old_gid: &str,
    snapshot: &TaskSnapshot,
) -> std::result::Result<(), String> {
    let client = aria2
        .create_rpc_client()
        .ok_or_else(|| "aria2 RPC 客户端未就绪".to_string())?;
    client
        .remove(old_gid)
        .await
        .map_err(|error| format!("移除旧 aria2 任务失败：{error}"))?;
    let directory = snapshot.dir.to_string_lossy().into_owned();
    let outcome = client
        .add_uri(
            vec![snapshot.url.clone()],
            Some(retry_options(&directory, &snapshot.filename)),
        )
        .await
        .map_err(|error| format!("重建 aria2 任务失败：{error}"))?;
    let new_gid = outcome.gid().to_string();
    let next_retry = snapshot.retry_count + 1;
    if let Err(error) = db
        .update_gid_by_id(snapshot.db_id, &new_gid, i64::from(next_retry))
        .await
    {
        if outcome.is_created() {
            return match client.remove(&new_gid).await {
                Ok(_) => Err(format!("更新重试任务记录失败：{error}")),
                Err(cleanup) => Err(format!(
                    "更新重试任务记录失败：{error}；移除新 aria2 任务失败：{cleanup}"
                )),
            };
        }
        return Err(format!("更新重试任务记录失败：{error}"));
    }

    let mut states = states.lock().await;
    let Some(mut state) = states.remove(old_gid) else {
        return Err(format!("重试任务 {old_gid} 已从监控表移除"));
    };
    state.retry_with_gid(new_gid.clone(), next_retry);
    states.insert(new_gid, state);
    Ok(())
}

async fn finish_failed(
    db: &DownloadDB,
    states: &Mutex<std::collections::HashMap<String, MonitorState>>,
    gid: &str,
    error: String,
) {
    let identity = {
        let states = states.lock().await;
        states
            .get(gid)
            .map(|state| (state.db_id, state.retry_count))
    };
    let Some((db_id, retry_count)) = identity else {
        return;
    };

    let persist_error = db
        .mark_failed(db_id, gid, i64::from(retry_count), &error)
        .await
        .err();
    let message = if let Some(persist_error) = persist_error {
        tracing::error!(gid = %gid, error = %persist_error, "持久化下载失败状态失败");
        format!("{error}; 写入失败状态时出错：{persist_error}")
    } else {
        error
    };
    let waiters = take_waiters(states, gid).await;
    notify_waiters(gid, waiters, Err(message)).await;
}

async fn finish_cancelled(
    states: &Mutex<std::collections::HashMap<String, MonitorState>>,
    gid: &str,
) {
    let waiters = take_waiters(states, gid).await;
    notify_waiters(gid, waiters, Err("aria2 任务已取消".to_string())).await;
}

async fn take_waiters(
    states: &Mutex<std::collections::HashMap<String, MonitorState>>,
    gid: &str,
) -> Vec<crate::manager::DownloadWaiter> {
    match states.lock().await.remove(gid) {
        Some(mut state) => state.take_waiters(),
        None => Vec::new(),
    }
}

async fn notify_waiters(
    gid: &str,
    waiters: Vec<crate::manager::DownloadWaiter>,
    result: std::result::Result<PathBuf, String>,
) {
    for waiter in waiters {
        if waiter.send(result.clone()).is_err() {
            tracing::debug!(gid = %gid, "下载调用方已停止等待");
        }
    }
}

fn parse_length(value: &str, field: &str) -> Result<i64> {
    value.parse::<i64>().map_err(|error| {
        DownloadError::Failed(format!("aria2 字段 {field} 不是有效字节数：{error}"))
    })
}

fn is_retryable_error(error_code: Option<&str>) -> bool {
    matches!(
        error_code.and_then(|code| code.parse::<u8>().ok()),
        Some(2 | 6 | 19 | 29)
    )
}

fn retry_options(dir: &str, filename: &str) -> DownloadOptions {
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

#[cfg(test)]
mod tests {
    use super::is_retryable_error;

    #[test]
    fn retry_eligibility_uses_aria2_error_codes() {
        for code in ["2", "6", "19", "29"] {
            assert!(is_retryable_error(Some(code)), "{code}");
        }
        for code in ["1", "3", "5", "24"] {
            assert!(!is_retryable_error(Some(code)), "{code}");
        }
        assert!(!is_retryable_error(None));
    }
}
