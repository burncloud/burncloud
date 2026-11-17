use super::{
    adapter::normalize_attachment, LocalAttachmentState, LocalRouteAttachment,
    LocalRouteAttachmentError,
};
use burncloud_database::Database;
use futures::{stream::FuturesUnordered, StreamExt};
use serde::Deserialize;
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::time::{interval, timeout, Instant, MissedTickBehavior};

const PROBE_INTERVAL: Duration = Duration::from_secs(5);
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_MODEL_LIST_BYTES: usize = 1024 * 1024;

/// 在 Server 生命周期内管理启动清单的挂载与运行心跳。
///
/// # Examples
/// ```no_run
/// # async fn example(db: std::sync::Arc<burncloud_database::Database>) -> Result<(), burncloud_router::local_attachment::LocalRouteAttachmentError> {
/// use burncloud_router::local_attachment::{LocalRouteAttachment, LocalRouteManager};
/// let manager = LocalRouteManager::new(db, vec![LocalRouteAttachment {
///     model: "qwen:4b".into(), base_url: "http://127.0.0.1:8080".into(),
/// }])?;
/// manager.run().await?;
/// # Ok(()) }
/// ```
///
/// # Notes
/// 同一 Database 对象只允许一个管理器。丢弃管理器或运行 future 同步禁止本实例
/// 本机挂载的新许可，并回收首检；不删除记录、不取消已获许可请求。
/// 持续探测只恢复临时探测故障，不能解除推理故障锁或显式隔离。
pub struct LocalRouteManager {
    db: Arc<Database>,
    local: Arc<LocalAttachmentState>,
    attachments: Vec<LocalRouteAttachment>,
    epoch: u64,
}

impl LocalRouteManager {
    /// 完整校验并规范化启动清单，然后认领唯一管理会话；不执行网络或数据库 I/O。
    ///
    /// # Examples
    /// ```no_run
    /// # fn example(db: std::sync::Arc<burncloud_database::Database>) -> Result<(), burncloud_router::local_attachment::LocalRouteAttachmentError> {
    /// use burncloud_router::local_attachment::LocalRouteManager;
    /// let manager = LocalRouteManager::new(db, vec![])?;
    /// drop(manager);
    /// # Ok(()) }
    /// ```
    ///
    /// # Notes
    /// 输入合法域与 attach 相同，规范化后重复目标返回 AttachFailed；空清单合法。
    /// 全部校验成功前不会改变既有会话状态；构造成功时先禁派，run 后才启用新会话。
    pub fn new(
        db: Arc<Database>,
        attachments: Vec<LocalRouteAttachment>,
    ) -> Result<Self, LocalRouteAttachmentError> {
        let attachments = attachments
            .into_iter()
            .map(normalize_attachment)
            .collect::<Result<Vec<_>, _>>()?;
        let mut targets = HashSet::new();
        for attachment in &attachments {
            if !targets.insert((&attachment.model, &attachment.base_url)) {
                return Err(LocalRouteAttachmentError::AttachFailed(format!(
                    "duplicate local route target: {} at {}",
                    attachment.model, attachment.base_url,
                )));
            }
        }
        let local = LocalAttachmentState::for_database(&db);
        local.configure(super::configured_concurrency()?)?;
        let epoch = local.claim_manager()?;
        Ok(Self {
            db,
            local,
            attachments,
            epoch,
        })
    }

    /// 激活会话并并发执行每项首检；持续等待直到调用方取消此 future。
    ///
    /// # Examples
    /// ```no_run
    /// # async fn example(db: std::sync::Arc<burncloud_database::Database>) -> Result<(), burncloud_router::local_attachment::LocalRouteAttachmentError> {
    /// use burncloud_router::local_attachment::LocalRouteManager;
    /// let manager = LocalRouteManager::new(db, vec![])?;
    /// let task = tokio::spawn(manager.run());
    /// task.abort();
    /// // abort 后仍需等待回收，不能把发出取消信号当成停止已经完成。
    /// match task.await {
    ///     Err(error) if error.is_cancelled() => {},
    ///     Err(error) => panic!("管理任务异常退出：{error}"),
    ///     Ok(result) => result?,
    /// }
    /// # Ok(()) }
    /// ```
    ///
    /// # Notes
    /// 首检失败每 5 秒重试，单次 2 秒；首次成功后只调用一次 attach，随后继续探测；15 秒心跳过期禁派，临时故障需两次成功恢复。
    /// 单节点 attach 错误记录后停止该项，不重试持久化、不使其他供给停止服务。
    /// 取消时不自动 detach，重启残留须核对后显式清理；低层 create_app 不隐式启动本管理器。
    pub async fn run(self) -> Result<(), LocalRouteAttachmentError> {
        self.local.activate_manager(self.epoch)?;
        let mut pending: FuturesUnordered<_> = self
            .attachments
            .iter()
            .map(|attachment| self.run_startup(attachment))
            .collect();
        while pending.next().await.is_some() {}
        // 即使所有项成功或停止，仍保留会话直到 Server 结束，而非立即禁派。
        std::future::pending().await
    }

    #[allow(
        clippy::cognitive_complexity,
        reason = "两个串行探测阶段及结构化日志增加计数；不为日志增加抽象"
    )]
    async fn run_startup(&self, attachment: &LocalRouteAttachment) {
        let mut ticks = interval(PROBE_INTERVAL);
        ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticks.tick().await;
            let evidence = match probe(&self.local, &attachment.model, &attachment.base_url).await {
                Ok(at) => at,
                Err(error) => {
                    tracing::warn!(model = %attachment.model, %error, "local route startup probe failed; retrying in the next interval");
                    continue;
                }
            };
            match self
                .local
                .attach(&self.db, attachment.clone(), Some(evidence))
                .await
            {
                Ok(id) => {
                    tracing::info!(model = %attachment.model, attachment_id = id.0, "local route startup attachment published")
                }
                Err(error) => {
                    tracing::error!(model = %attachment.model, base_url = %attachment.base_url, %error,
                        "local route automatic attachment stopped; explicit reconciliation required");
                    return;
                }
            }
            break;
        }
        loop {
            ticks.tick().await;
            // 每次捕获实际 ID；显式重挂后跟随新 ID，旧探测只可反馈旧 ID。
            let Some(id) = self.local.managed_id(self.epoch, attachment) else {
                continue;
            };
            let result = probe(&self.local, &attachment.model, &attachment.base_url).await;
            if let Err(error) = &result {
                tracing::warn!(attachment_id = id, %error, "local route runtime probe failed");
            }
            self.local.record_probe(self.epoch, id, result.ok());
        }
    }
}

pub(super) async fn probe(
    local: &LocalAttachmentState,
    model: &str,
    base_url: &str,
) -> Result<Instant, String> {
    timeout(PROBE_TIMEOUT, probe_models(local, model, base_url))
        .await
        .map_err(|_| "model list probe exceeded 2 seconds".to_owned())??;
    Ok(Instant::now())
}

async fn probe_models(
    local: &LocalAttachmentState,
    model: &str,
    base_url: &str,
) -> Result<(), String> {
    let client = local
        .local_client()
        .await
        .map_err(|error| error.to_string())?;
    let mut response = client
        .get(format!("{base_url}/v1/models"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("model list returned HTTP {}", response.status()));
    }
    if response
        .content_length()
        .is_some_and(|len| len > MAX_MODEL_LIST_BYTES as u64)
    {
        return Err("model list exceeds 1 MiB".into());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if chunk.len() > MAX_MODEL_LIST_BYTES - body.len() {
            return Err("model list exceeds 1 MiB".into());
        }
        body.extend_from_slice(&chunk);
    }
    let list: ModelList = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
    if list.data.iter().any(|entry| entry.id == model) {
        Ok(())
    } else {
        Err(format!("model list does not contain exact alias {model}"))
    }
}

impl Drop for LocalRouteManager {
    fn drop(&mut self) {
        self.local.stop_manager(self.epoch);
    }
}

#[derive(Deserialize)]
struct ModelList {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

#[cfg(test)]
#[path = "local_attachment_manager_tests.rs"]
mod tests;
