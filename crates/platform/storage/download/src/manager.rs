use std::collections::HashMap;
use std::sync::Arc;

use burncloud_database::DatabaseError;
use burncloud_database_sys::DownloadDB;
use burncloud_download_aria2::{
    quick_start, Aria2Error, Aria2Manager, Aria2RpcClient, DownloadOptions, DownloadStatus,
};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use crate::error::{DownloadError, Result};
use crate::utils::extract_filename_from_url;

const MONITOR_INTERVAL: Duration = Duration::from_secs(4);
const MAX_ZERO_SPEED_COUNT: u8 = 3;
const MAX_STATUS_FAILURE_COUNT: u8 = 3;

/// 下载任务的监控状态。
///
/// `zero_speed_count` 用于记录连续轮询中下载速度为零的次数，`restarting` 用于避免同一个任务并发
/// 重试，`can_restart` 用于限制一个进程运行期间最多自动重试一次。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MonitorState {
    zero_speed_count: u8, // 检查时下载速度为 0 的次数，初始为 0。
    restarting: bool,     // 是否正在重试，初始为 false。
    can_restart: bool,    // 是否允许重试，初始为 true；每次进程运行期间最多重试一次。
}

impl MonitorState {
    /// 创建指定重试权限的初始监控状态。
    ///
    /// 新注册任务从零次零速、未重试状态开始；重试得到的新 GID 传入 `false` 后不可再次自动重试。
    fn new(can_restart: bool) -> Self {
        Self {
            zero_speed_count: 0,
            restarting: false,
            can_restart,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MonitorAction {
    Continue,
    Retry,
    Fail,
    Stop,
}

/// 下载管理器。
pub struct DownloadManager {
    aria2: Arc<Aria2Manager>,
    db: Arc<DownloadDB>,
    monitor_states: Arc<Mutex<HashMap<String, MonitorState>>>,
}

impl DownloadManager {
    /// 创建下载管理器，启动 aria2、初始化数据库并恢复未完成的下载。
    ///
    /// 失败：aria2 启动、数据库初始化或恢复任一环节失败时返回对应错误。
    pub async fn new() -> Result<Self> {
        let aria2 = Arc::new(quick_start().await?);
        let db = Arc::new(DownloadDB::new().await?);
        let manager = Self {
            aria2,
            db,
            monitor_states: Arc::new(Mutex::new(HashMap::new())),
        };
        manager.restore_incomplete_downloads().await?;
        Ok(manager)
    }

    /// 添加一个下载任务并注册唯一的进度监控任务。
    ///
    /// 参数：`url` 是下载地址，`download_dir` 是可选的保存目录。
    /// 失败：aria2 或数据库操作失败时返回错误，错误不会被静默忽略。
    pub async fn add_download(&self, url: &str, download_dir: Option<&str>) -> Result<String> {
        let client = self.rpc_client()?;
        let dir = download_dir.unwrap_or("./downloads").to_string();
        let filename = extract_filename_from_url(url);
        let options = Self::download_options(Some(dir.clone()), filename.clone());
        let uris = vec![url.to_string()];
        let outcome = client.add_uri(uris.clone(), Some(options)).await?;
        let gid = outcome.gid().to_string();

        if outcome.is_created() {
            self.db
                .add(&gid, uris, Some(&dir), filename.as_deref())
                .await?;
        }

        self.start_progress_monitor(&gid).await;
        Ok(gid)
    }

    /// 查询下载状态并同步状态与进度到数据库。
    ///
    /// 失败：RPC 查询、数值解析或数据库更新失败时返回错误。
    pub async fn get_status(&self, gid: &str) -> Result<DownloadStatus> {
        let client = self.rpc_client()?;
        let status = client.tell_status(gid).await?;
        let (total, completed, speed) = Self::parse_progress(&status)?;

        self.db.update_status(gid, &status.status).await?;
        self.db
            .update_progress(gid, total, completed, speed)
            .await?;
        Ok(status)
    }

    /// 暂停下载任务并更新数据库状态。
    ///
    /// 失败：aria2 RPC 或数据库更新失败时返回错误。
    pub async fn pause(&self, gid: &str) -> Result<()> {
        let client = self.rpc_client()?;
        client.pause(gid).await?;
        self.db.update_status(gid, "paused").await?;
        Ok(())
    }

    /// 恢复下载任务并更新数据库状态。
    ///
    /// 失败：aria2 RPC 或数据库更新失败时返回错误。
    pub async fn resume(&self, gid: &str) -> Result<()> {
        let client = self.rpc_client()?;
        client.unpause(gid).await?;
        self.db.update_status(gid, "active").await?;
        Ok(())
    }

    /// 删除 aria2 任务、数据库记录和对应的内存监控状态。
    ///
    /// 失败：aria2 删除或数据库删除失败时返回错误；无论 aria2 删除结果如何，都会清理监控状态。
    pub async fn remove(&self, gid: &str) -> Result<()> {
        let remove_result = match self.rpc_client() {
            Ok(client) => client
                .remove(gid)
                .await
                .map(|_| ())
                .map_err(DownloadError::from),
            Err(error) => Err(error),
        };
        self.clear_monitor_state(gid).await;
        remove_result?;
        self.db.delete(gid).await?;
        Ok(())
    }

    /// 注册并启动一个可自动重试一次的进度监控任务。
    ///
    /// GID 已注册时不重复启动 tokio 任务；注册检查和插入在同一把锁内完成，释放锁后才创建任务。
    pub async fn start_progress_monitor(&self, gid: &str) {
        if self.register_monitor(gid).await {
            self.spawn_progress_monitor(gid.to_string());
        }
    }

    /// 恢复数据库中仍为 active 的任务，并为恢复后的 GID 启动进度监控。
    ///
    /// 失败：URI JSON 解析、aria2 添加任务或数据库 GID 更新失败时返回错误，保留数据库记录供下次恢复。
    pub async fn restore_incomplete_downloads(&self) -> Result<Vec<String>> {
        let client = self.rpc_client()?;
        let incomplete = self.db.list(Some("active")).await?;
        let mut restored = Vec::with_capacity(incomplete.len());

        for download in incomplete {
            let uris = Self::parse_uris(&download.uris)?;
            if uris.is_empty() {
                return Err(DownloadError::Database(DatabaseError::InvalidData {
                    message: format!("下载记录 {} 的 URI 列表为空", download.gid),
                }));
            }

            let dir = download
                .download_dir
                .clone()
                .filter(|value| !value.is_empty())
                .or_else(|| Some("./downloads".to_string()));
            let options = Self::download_options(dir, download.filename.clone());
            let outcome = client.add_uri(uris, Some(options)).await?;
            let new_gid = outcome.gid().to_string();

            self.db.update_gid(&download.gid, &new_gid).await?;
            self.start_progress_monitor(&new_gid).await;
            restored.push(new_gid);
        }

        Ok(restored)
    }

    /// 按指定权限注册并启动监控，用于重试生成的新 GID。
    ///
    /// 注册完成后才创建 tokio 任务；如果 GID 已存在，则保留已有的唯一监控任务。
    async fn start_monitor_with_state(&self, gid: &str, can_restart: bool) {
        if self.register_monitor_with_state(gid, can_restart).await {
            self.spawn_progress_monitor(gid.to_string());
        }
    }

    /// 以默认可重试状态幂等注册 GID。
    ///
    /// 检查与插入必须在锁内一次完成，返回值为 `true` 表示调用方应启动监控任务。
    async fn register_monitor(&self, gid: &str) -> bool {
        self.register_monitor_with_state(gid, true).await
    }

    /// 在监控状态锁内完成 GID 检查和完整状态插入。
    ///
    /// 返回 `false` 表示 GID 已存在，调用方不得再次创建监控任务。
    async fn register_monitor_with_state(&self, gid: &str, can_restart: bool) -> bool {
        let mut states = self.monitor_states.lock().await;
        insert_monitor_state(&mut states, gid, can_restart)
    }

    /// 创建后台监控任务，并将所需共享状态移动到任务中。
    fn spawn_progress_monitor(&self, gid: String) {
        let aria2 = Arc::clone(&self.aria2);
        let db = Arc::clone(&self.db);
        let monitor_states = Arc::clone(&self.monitor_states);

        tokio::spawn(async move {
            Self::run_progress_monitor(aria2, db, monitor_states, gid).await;
        });
    }

    /// 每四秒查询一次任务状态，并根据速度、失败次数和下载状态维护任务生命周期。
    ///
    /// 查询失败连续达到三次时只清理内存监控状态，保留数据库记录等待下次启动恢复。
    async fn run_progress_monitor(
        aria2: Arc<Aria2Manager>,
        db: Arc<DownloadDB>,
        monitor_states: Arc<Mutex<HashMap<String, MonitorState>>>,
        gid: String,
    ) {
        let mut status_failure_count = 0_u8;

        loop {
            let should_stop = Self::poll_monitor_once(
                &aria2,
                &db,
                &monitor_states,
                &gid,
                &mut status_failure_count,
            )
            .await;
            if should_stop {
                break;
            }

            sleep(MONITOR_INTERVAL).await;
        }
    }

    /// 执行一次监控轮询，并返回是否应退出监控任务。
    async fn poll_monitor_once(
        aria2: &Arc<Aria2Manager>,
        db: &Arc<DownloadDB>,
        monitor_states: &Arc<Mutex<HashMap<String, MonitorState>>>,
        gid: &str,
        status_failure_count: &mut u8,
    ) -> bool {
        let status_result = match aria2.create_rpc_client() {
            Some(client) => client.tell_status(gid).await,
            None => Err(Aria2Error::RpcError("aria2 客户端未就绪".to_string())),
        };

        match status_result {
            Ok(status) => {
                *status_failure_count = 0;
                Self::handle_successful_status(aria2, db, monitor_states, gid, status).await
            }
            Err(error) => {
                Self::handle_status_failure(monitor_states, gid, status_failure_count, &error).await
            }
        }
    }

    /// 处理成功的状态查询，保存进度并执行监控状态机动作。
    async fn handle_successful_status(
        aria2: &Arc<Aria2Manager>,
        db: &Arc<DownloadDB>,
        monitor_states: &Arc<Mutex<HashMap<String, MonitorState>>>,
        gid: &str,
        status: DownloadStatus,
    ) -> bool {
        let speed = match Self::persist_status_and_progress(db, gid, &status).await {
            Ok(speed) => speed,
            Err(error) => {
                tracing::error!(gid = %gid, error = %error, "下载状态中的进度字段无效");
                clear_monitor_state(monitor_states, gid).await;
                return true;
            }
        };

        if status.status == "complete" || status.status == "error" {
            clear_monitor_state(monitor_states, gid).await;
            return true;
        }

        let action = next_monitor_action(monitor_states, gid, &status.status, speed).await;
        Self::execute_monitor_action(aria2, db, monitor_states, gid, action).await
    }

    /// 持久化状态和进度，并返回本次查询到的下载速度。
    async fn persist_status_and_progress(
        db: &Arc<DownloadDB>,
        gid: &str,
        status: &DownloadStatus,
    ) -> Result<i64> {
        if let Err(error) = db.update_status(gid, &status.status).await {
            tracing::warn!(gid = %gid, error = %error, "保存下载状态失败");
        }

        let (total, completed, speed) = Self::parse_progress(status)?;
        if let Err(error) = db.update_progress(gid, total, completed, speed).await {
            tracing::warn!(gid = %gid, error = %error, "保存下载进度失败");
        }
        Ok(speed)
    }

    /// 执行状态机决定的继续、停止、重试或错误处理动作。
    async fn execute_monitor_action(
        aria2: &Arc<Aria2Manager>,
        db: &Arc<DownloadDB>,
        monitor_states: &Arc<Mutex<HashMap<String, MonitorState>>>,
        gid: &str,
        action: MonitorAction,
    ) -> bool {
        match action {
            MonitorAction::Continue => false,
            MonitorAction::Stop => true,
            MonitorAction::Retry => {
                Self::execute_retry_action(aria2, db, monitor_states, gid).await
            }
            MonitorAction::Fail => Self::execute_fail_action(aria2, db, monitor_states, gid).await,
        }
    }

    /// 执行自动重试动作；失败时保留数据库记录并标记为 error。
    async fn execute_retry_action(
        aria2: &Arc<Aria2Manager>,
        db: &Arc<DownloadDB>,
        monitor_states: &Arc<Mutex<HashMap<String, MonitorState>>>,
        gid: &str,
    ) -> bool {
        let manager = Self {
            aria2: Arc::clone(aria2),
            db: Arc::clone(db),
            monitor_states: Arc::clone(monitor_states),
        };
        if let Err(error) = manager.retry_stalled_download(gid).await {
            tracing::error!(gid = %gid, error = %error, "自动重试下载失败");
            clear_monitor_state(monitor_states, gid).await;
            if let Err(update_error) = db.update_status(gid, "error").await {
                tracing::error!(
                    gid = %gid,
                    error = %update_error,
                    "自动重试失败后更新数据库状态失败"
                );
            }
        }
        true
    }

    /// 执行不可重试任务的失败动作。
    async fn execute_fail_action(
        aria2: &Arc<Aria2Manager>,
        db: &Arc<DownloadDB>,
        monitor_states: &Arc<Mutex<HashMap<String, MonitorState>>>,
        gid: &str,
    ) -> bool {
        let manager = Self {
            aria2: Arc::clone(aria2),
            db: Arc::clone(db),
            monitor_states: Arc::clone(monitor_states),
        };
        manager.mark_stalled_download_as_error(gid).await;
        true
    }

    /// 处理一次状态查询失败，并在达到上限时清理监控状态。
    async fn handle_status_failure(
        monitor_states: &Arc<Mutex<HashMap<String, MonitorState>>>,
        gid: &str,
        status_failure_count: &mut u8,
        error: &Aria2Error,
    ) -> bool {
        *status_failure_count = status_failure_count.saturating_add(1);
        if !reset_zero_speed_count(monitor_states, gid).await {
            return true;
        }

        if *status_failure_count >= MAX_STATUS_FAILURE_COUNT {
            tracing::error!(
                gid = %gid,
                failures = *status_failure_count,
                error = %error,
                "连续三次查询下载状态失败，停止监控并保留数据库记录"
            );
            clear_monitor_state(monitor_states, gid).await;
            return true;
        }

        tracing::warn!(
            gid = %gid,
            failures = *status_failure_count,
            error = %error,
            "查询下载状态失败，将继续重试"
        );
        false
    }

    /// 重试卡住的下载任务，并把旧数据库记录迁移到新 GID。
    ///
    /// 流程：读取旧记录、标记旧状态正在重试并释放锁、仅删除旧 aria2 任务、用原下载参数添加任务、迁移
    /// 数据库 GID、清理旧状态并为新 GID 启动不可再次重试的唯一监控。
    async fn retry_stalled_download(&self, old_gid: &str) -> Result<()> {
        let download = self.db.get(old_gid).await?.ok_or_else(|| {
            DownloadError::Database(DatabaseError::InvalidData {
                message: format!("找不到待重试下载记录: {old_gid}"),
            })
        })?;

        {
            let mut states = self.monitor_states.lock().await;
            let Some(state) = states.get_mut(old_gid) else {
                return Ok(());
            };
            if state.restarting {
                return Ok(());
            }
            state.restarting = true;
        }

        let client = self.rpc_client()?;
        client.remove(old_gid).await?;

        let uris = Self::parse_uris(&download.uris)?;
        if uris.is_empty() {
            return Err(DownloadError::Database(DatabaseError::InvalidData {
                message: format!("下载记录 {old_gid} 的 URI 列表为空"),
            }));
        }
        let dir = download
            .download_dir
            .clone()
            .filter(|value| !value.is_empty())
            .or_else(|| Some("./downloads".to_string()));
        let options = Self::download_options(dir, download.filename.clone());
        let outcome = client.add_uri(uris, Some(options)).await?;
        let new_gid = outcome.gid().to_string();

        self.db.update_gid(old_gid, &new_gid).await?;
        self.clear_monitor_state(old_gid).await;
        self.start_monitor_with_state(&new_gid, false).await;
        Ok(())
    }

    /// 删除无法重试的卡住任务，清理内存状态并把数据库状态更新为 error。
    ///
    /// aria2 删除失败仍会记录错误并继续执行状态清理，避免失败任务永久占用内存监控表。
    async fn mark_stalled_download_as_error(&self, gid: &str) {
        self.remove_aria2_task_for_stalled_download(gid).await;
        self.clear_monitor_state(gid).await;
        if let Err(error) = self.db.update_status(gid, "error").await {
            tracing::error!(gid = %gid, error = %error, "更新卡住任务数据库状态失败");
        }
    }

    /// 尝试删除无法重试的 aria2 任务，并记录删除失败原因。
    async fn remove_aria2_task_for_stalled_download(&self, gid: &str) {
        let client = match self.rpc_client() {
            Ok(client) => client,
            Err(error) => {
                tracing::error!(gid = %gid, error = %error, "无法创建 aria2 客户端以删除卡住任务");
                return;
            }
        };

        if let Err(error) = client.remove(gid).await {
            tracing::error!(gid = %gid, error = %error, "删除卡住的 aria2 任务失败");
        }
    }

    /// 清理指定 GID 的内存监控状态。
    async fn clear_monitor_state(&self, gid: &str) {
        clear_monitor_state(&self.monitor_states, gid).await;
    }

    /// 创建当前 aria2 RPC 客户端。
    ///
    /// 失败：aria2 尚未就绪时返回明确的 RPC 错误。
    fn rpc_client(&self) -> Result<Aria2RpcClient> {
        self.aria2.create_rpc_client().ok_or_else(|| {
            DownloadError::Aria2(Aria2Error::RpcError("aria2 客户端未就绪".to_string()))
        })
    }

    /// 根据目录和文件名生成 aria2 下载选项。
    ///
    /// 默认开启断点续传和覆盖下载，关闭自动重命名，保证新任务沿用旧任务的文件策略。
    fn download_options(dir: Option<String>, filename: Option<String>) -> DownloadOptions {
        DownloadOptions {
            dir,
            out: filename,
            split: None,
            max_connection_per_server: None,
            continue_download: Some(true),
            allow_overwrite: Some(true),
            auto_file_renaming: Some(false),
        }
    }

    /// 解析数据库中的 URI JSON 数组。
    ///
    /// 失败：JSON 格式错误转换为数据库序列化错误，不使用空数组掩盖损坏记录。
    fn parse_uris(value: &str) -> Result<Vec<String>> {
        serde_json::from_str(value)
            .map_err(|error| DownloadError::Database(DatabaseError::Serialization(error)))
    }

    /// 解析 aria2 返回的长度和速度字段。
    ///
    /// 失败：任何字段不是合法有符号整数时返回明确的 RPC 错误。
    fn parse_progress(status: &DownloadStatus) -> Result<(i64, i64, i64)> {
        Ok((
            Self::parse_number("totalLength", &status.total_length)?,
            Self::parse_number("completedLength", &status.completed_length)?,
            Self::parse_number("downloadSpeed", &status.download_speed)?,
        ))
    }

    /// 解析一个 aria2 数值字段。
    ///
    /// 失败：字段内容不是整数时返回带字段名和值的错误。
    fn parse_number(field: &str, value: &str) -> Result<i64> {
        value.parse::<i64>().map_err(|error| {
            DownloadError::Aria2(Aria2Error::RpcError(format!(
                "aria2 字段 {field} 的值 {value:?} 不是有效整数: {error}"
            )))
        })
    }
}

/// 在锁保护的监控状态表中幂等插入 GID。
///
/// 调用方必须在进入本函数前持有状态锁；函数不执行异步操作，返回值用于决定是否启动监控任务。
fn insert_monitor_state(
    states: &mut HashMap<String, MonitorState>,
    gid: &str,
    can_restart: bool,
) -> bool {
    if states.contains_key(gid) {
        return false;
    }
    states.insert(gid.to_string(), MonitorState::new(can_restart));
    true
}

/// 根据一次成功的状态查询更新监控计数，并决定下一步动作。
///
/// active 且速度为零时累计连续次数；速度恢复或任务非 active 时清零。达到三次后按重试权限选择重试、
/// 标记错误或停止。
fn update_monitor_state(state: &mut MonitorState, status: &str, speed: i64) -> MonitorAction {
    if status == "active" && speed == 0 {
        state.zero_speed_count = state.zero_speed_count.saturating_add(1);
    } else {
        state.zero_speed_count = 0;
    }

    if state.zero_speed_count < MAX_ZERO_SPEED_COUNT {
        return MonitorAction::Continue;
    }
    if state.restarting {
        return MonitorAction::Continue;
    }
    if state.can_restart {
        MonitorAction::Retry
    } else {
        MonitorAction::Fail
    }
}

/// 清理指定 GID 的监控状态，并返回该状态是否曾存在。
async fn clear_monitor_state(
    monitor_states: &Arc<Mutex<HashMap<String, MonitorState>>>,
    gid: &str,
) -> bool {
    monitor_states.lock().await.remove(gid).is_some()
}

/// 根据一次成功查询更新状态表，并返回要执行的监控动作。
async fn next_monitor_action(
    monitor_states: &Arc<Mutex<HashMap<String, MonitorState>>>,
    gid: &str,
    status: &str,
    speed: i64,
) -> MonitorAction {
    let mut states = monitor_states.lock().await;
    match states.get_mut(gid) {
        Some(state) => update_monitor_state(state, status, speed),
        None => MonitorAction::Stop,
    }
}

/// 查询失败时清零零速计数，并返回 GID 是否仍在监控状态表中。
async fn reset_zero_speed_count(
    monitor_states: &Arc<Mutex<HashMap<String, MonitorState>>>,
    gid: &str,
) -> bool {
    let mut states = monitor_states.lock().await;
    let Some(state) = states.get_mut(gid) else {
        return false;
    };
    state.zero_speed_count = 0;
    true
}

#[cfg(test)]
mod tests {
    use super::{
        insert_monitor_state, update_monitor_state, Aria2Error, DownloadManager, MonitorAction,
        MonitorState,
    };
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    /// 验证同一个 GID 只能注册一次，且首次注册时完整写入初始状态。
    #[test]
    fn monitor_registration_is_idempotent() {
        let mut states = HashMap::new();

        assert!(insert_monitor_state(&mut states, "gid-1", true));
        assert!(!insert_monitor_state(&mut states, "gid-1", false));
        assert_eq!(
            states.get("gid-1"),
            Some(&MonitorState {
                zero_speed_count: 0,
                restarting: false,
                can_restart: true,
            })
        );
    }

    /// 验证 active 任务连续三次零速后只触发一次可重试动作，速度恢复会清零计数。
    #[test]
    fn active_zero_speed_reaches_retry_threshold() {
        let mut state = MonitorState::new(true);

        assert_eq!(
            update_monitor_state(&mut state, "active", 0),
            MonitorAction::Continue
        );
        assert_eq!(state.zero_speed_count, 1);
        assert_eq!(
            update_monitor_state(&mut state, "active", 0),
            MonitorAction::Continue
        );
        assert_eq!(
            update_monitor_state(&mut state, "active", 0),
            MonitorAction::Retry
        );
        assert_eq!(state.zero_speed_count, 3);

        state.restarting = true;
        assert_eq!(
            update_monitor_state(&mut state, "active", 0),
            MonitorAction::Continue
        );

        state.restarting = false;
        assert_eq!(
            update_monitor_state(&mut state, "active", 128),
            MonitorAction::Continue
        );
        assert_eq!(state.zero_speed_count, 0);
    }

    /// 验证不可重试的新 GID 连续三次零速后进入失败流程。
    #[test]
    fn non_restartable_task_enters_failure_flow() {
        let mut state = MonitorState::new(false);

        assert_eq!(
            update_monitor_state(&mut state, "active", 0),
            MonitorAction::Continue
        );
        assert_eq!(
            update_monitor_state(&mut state, "active", 0),
            MonitorAction::Continue
        );
        assert_eq!(
            update_monitor_state(&mut state, "active", 0),
            MonitorAction::Fail
        );
    }

    /// 验证非 active 状态不会累计零速重试次数。
    #[test]
    fn non_active_status_resets_zero_speed_count() {
        let mut state = MonitorState::new(true);
        state.zero_speed_count = 2;

        assert_eq!(
            update_monitor_state(&mut state, "paused", 0),
            MonitorAction::Continue
        );
        assert_eq!(state.zero_speed_count, 0);
    }

    /// 验证状态查询连续三次失败后清理 GID，失败期间同时清零零速计数。
    #[tokio::test]
    async fn repeated_status_failures_clear_the_monitor_state() {
        let states = Arc::new(Mutex::new(HashMap::new()));
        {
            let mut guard = states.lock().await;
            assert!(insert_monitor_state(&mut guard, "gid-1", true));
        }
        let error = Aria2Error::RpcError("status unavailable".to_string());
        let mut failure_count = 0;

        assert!(
            !DownloadManager::handle_status_failure(&states, "gid-1", &mut failure_count, &error)
                .await
        );
        assert!(
            !DownloadManager::handle_status_failure(&states, "gid-1", &mut failure_count, &error)
                .await
        );
        assert!(
            DownloadManager::handle_status_failure(&states, "gid-1", &mut failure_count, &error)
                .await
        );
        assert_eq!(failure_count, 3);
        assert!(!states.lock().await.contains_key("gid-1"));
    }
}
