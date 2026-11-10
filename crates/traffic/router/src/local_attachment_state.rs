use super::{LocalRouteAttachment, LocalRouteAttachmentError, LocalRouteAttachmentId};
use burncloud_database::Database;
use burncloud_supply_channel::ChannelService;
use burncloud_supply_contracts::{Channel, ChannelType};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex, OnceLock, Weak,
    },
};
use tokio::sync::{Mutex as AsyncMutex, OnceCell, OwnedMutexGuard};
use tokio::time::Instant;

#[derive(Clone, PartialEq, Eq, Hash)]
struct AttachmentTarget {
    model: String,
    base_url: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lifecycle {
    Absent,
    Attached,
    Quarantined,
    Detached,
}

struct AttachmentSlot {
    id: Option<i32>,
    lifecycle: Lifecycle,
    unresolved: bool,
    epoch: u64,
    management: Arc<AsyncMutex<()>>,
    heartbeat: Option<Instant>,
    probe_healthy: bool,
    recovery_successes: u8,
    faulted: bool,
    in_flight: Arc<AtomicU32>,
}

#[derive(Default)]
struct AttachmentIndex {
    epoch: u64,
    manager_claimed: bool,
    manager_active: bool,
    max_concurrency: u32,
    configured: bool,
    targets: HashMap<AttachmentTarget, AttachmentSlot>,
    ids: HashMap<i32, AttachmentTarget>,
}

// 仅协调实例内管理与派发许可；供给及能力仍只由 Supply 保存。
pub(crate) struct LocalAttachmentState {
    index: Mutex<AttachmentIndex>,
    client: OnceCell<reqwest::Client>,
}

impl LocalAttachmentState {
    pub(crate) fn for_database(db: &Arc<Database>) -> Arc<Self> {
        type Registry = Mutex<Vec<(Weak<Database>, Arc<LocalAttachmentState>)>>;
        static STATES: OnceLock<Registry> = OnceLock::new();
        let mut states = STATES
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .unwrap_or_else(|_| panic!("local attachment registry poisoned"));
        states.retain(|(db, _)| db.strong_count() > 0);
        for (weak, state) in states.iter() {
            if weak
                .upgrade()
                .is_some_and(|existing| Arc::ptr_eq(&existing, db))
            {
                return Arc::clone(state);
            }
        }
        let state = Arc::new(Self {
            index: Mutex::new(AttachmentIndex::default()),
            client: OnceCell::new(),
        });
        states.push((Arc::downgrade(db), Arc::clone(&state)));
        state
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "共享索引中毒表示内部不变量损坏，不是可恢复业务错误"
    )]
    pub(super) fn claim_manager(&self) -> Result<u64, LocalRouteAttachmentError> {
        let mut index = self
            .index
            .lock()
            .unwrap_or_else(|_| panic!("local attachment index poisoned"));
        if index.manager_claimed {
            return Err(LocalRouteAttachmentError::AttachFailed(
                "a local route manager already owns this database instance".into(),
            ));
        }
        index.epoch = index.epoch.checked_add(1).ok_or_else(|| {
            LocalRouteAttachmentError::AttachFailed(
                "local route manager generation exhausted".into(),
            )
        })?;
        index.manager_claimed = true;
        index.manager_active = false;
        Ok(index.epoch)
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "共享索引中毒表示内部不变量损坏，不是可恢复业务错误"
    )]
    pub(super) fn activate_manager(&self, epoch: u64) -> Result<(), LocalRouteAttachmentError> {
        debug_assert!(epoch > 0);
        let mut index = self
            .index
            .lock()
            .unwrap_or_else(|_| panic!("local attachment index poisoned"));
        if index.epoch != epoch || !index.manager_claimed {
            return Err(LocalRouteAttachmentError::AttachFailed(
                "local route manager session is closed".into(),
            ));
        }
        index.manager_active = true;
        Ok(())
    }

    pub(super) fn stop_manager(&self, epoch: u64) {
        debug_assert!(epoch > 0);
        let mut index = self
            .index
            .lock()
            .unwrap_or_else(|_| panic!("local attachment index poisoned"));
        if index.epoch == epoch {
            // 保留代数，关闭后不得退回无管理器模式，也不能由旧 Drop 关闭新会话。
            index.manager_active = false;
            index.manager_claimed = false;
        }
    }

    fn attach_epoch(index: &AttachmentIndex) -> Result<u64, LocalRouteAttachmentError> {
        Self::ensure_epoch(index, index.epoch)?;
        Ok(index.epoch)
    }

    fn ensure_epoch(index: &AttachmentIndex, epoch: u64) -> Result<(), LocalRouteAttachmentError> {
        if index.epoch != epoch || (epoch != 0 && !index.manager_active) {
            return Err(LocalRouteAttachmentError::AttachFailed(
                "local route manager session stopped or changed; attachment remains unpublished"
                    .into(),
            ));
        }
        Ok(())
    }

    pub(super) async fn attach(
        &self,
        db: &Database,
        attachment: LocalRouteAttachment,
        evidence: Option<Instant>,
    ) -> Result<LocalRouteAttachmentId, LocalRouteAttachmentError> {
        debug_assert!(!attachment.model.is_empty() && attachment.model.chars().count() <= 34);
        debug_assert!(!attachment.base_url.ends_with('/'));
        let target = AttachmentTarget {
            model: attachment.model,
            base_url: attachment.base_url,
        };
        // 在第一次 await 前捕获会话，不能把等待旧操作的请求归入后来创建的新会话。
        let epoch = Self::attach_epoch(
            &self
                .index
                .lock()
                .unwrap_or_else(|_| panic!("local attachment index poisoned")),
        )?;
        let _management = self.lock_target(&target).await;
        let (id, unresolved) = {
            let index = self
                .index
                .lock()
                .unwrap_or_else(|_| panic!("local attachment index poisoned"));
            Self::ensure_epoch(&index, epoch)?;
            let slot = &index.targets[&target];
            (slot.id, slot.unresolved)
        };
        if let Some(id) = id {
            let index = self
                .index
                .lock()
                .unwrap_or_else(|_| panic!("local attachment index poisoned"));
            if slot_ready(&index.targets[&target], &index, Instant::now()) {
                return Ok(LocalRouteAttachmentId(id));
            }
            return Err(LocalRouteAttachmentError::AttachFailed(format!(
                "attachment {id} has no valid health/heartbeat evidence or is locked"
            )));
        }
        let existing = match Self::find_existing_target(db, &target).await {
            Ok(existing) => existing,
            Err(error) => {
                if !unresolved {
                    self.index
                        .lock()
                        .unwrap_or_else(|_| panic!("local attachment index poisoned"))
                        .targets
                        .remove(&target);
                }
                return Err(error);
            }
        };
        if let Some(channel) = existing {
            let mut index = self
                .index
                .lock()
                .unwrap_or_else(|_| panic!("local attachment index poisoned"));
            let slot = index
                .targets
                .get_mut(&target)
                .unwrap_or_else(|| panic!("managed target missing"));
            slot.id = Some(channel.id);
            slot.unresolved = true;
            index.ids.insert(channel.id, target);
            return Err(LocalRouteAttachmentError::AttachFailed(format!(
                "unpublished attachment {} exists; explicitly detach it before attaching again",
                channel.id
            )));
        }
        if unresolved {
            return Err(LocalRouteAttachmentError::AttachFailed("previous attachment outcome is unknown; absence does not prove it cannot commit later".into()));
        }
        let evidence = match evidence {
            Some(evidence) => evidence,
            None => super::manager::probe(self, &target.model, &target.base_url)
                .await
                .map_err(LocalRouteAttachmentError::AttachFailed)?,
        };
        {
            let mut index = self
                .index
                .lock()
                .unwrap_or_else(|_| panic!("local attachment index poisoned"));
            Self::ensure_epoch(&index, epoch)?;
            index
                .targets
                .get_mut(&target)
                .unwrap_or_else(|| panic!("managed target missing"))
                .unresolved = true;
        }
        // 先禁派再 await；取消或错误都保留未决状态，不在 Drop 中执行 SQL。
        let mut channel = build_channel(&target);
        let id = ChannelService::create(db, &mut channel)
            .await
            .map_err(|e| LocalRouteAttachmentError::AttachFailed(e.to_string()))?;
        let mut index = self
            .index
            .lock()
            .unwrap_or_else(|_| panic!("local attachment index poisoned"));
        // 即使写入返回时已经停止，也保留实际 ID 供显式清理，绝不发布迟到提交。
        index.ids.insert(id, target.clone());
        index
            .targets
            .get_mut(&target)
            .unwrap_or_else(|| panic!("managed target missing"))
            .id = Some(id);
        Self::ensure_epoch(&index, epoch)?;
        let slot = index
            .targets
            .get_mut(&target)
            .unwrap_or_else(|| panic!("managed target missing"));
        if evidence.elapsed() >= HEARTBEAT_TTL {
            return Err(LocalRouteAttachmentError::AttachFailed(
                "probe evidence expired before publication; explicitly reconcile attachment".into(),
            ));
        }
        slot.heartbeat = Some(evidence);
        slot.probe_healthy = true;
        slot.recovery_successes = 0;
        slot.faulted = false;
        slot.epoch = epoch;
        slot.lifecycle = Lifecycle::Attached;
        slot.unresolved = false;
        // 发布与返回间没有 await，取消不能留下已发布但未交还的中间步骤。
        Ok(LocalRouteAttachmentId(id))
    }

    pub(super) async fn quarantine(
        &self,
        db: &Database,
        id: i32,
    ) -> Result<(), LocalRouteAttachmentError> {
        debug_assert!(id > 0);
        let (target, _) = self.resolve_id(db, id).await?;
        let Some(target) = target else {
            // 从未发布的非法残留也只能按 ID 和标签处理，不为它建立路由状态。
            let changed = ChannelService::quarantine_if_tag(db, id, "burncloud-node-local")
                .await
                .map_err(|e| LocalRouteAttachmentError::DetachFailed(e.to_string()))?;
            if !changed {
                return Err(LocalRouteAttachmentError::DetachFailed(format!(
                    "local route attachment {id} no longer exists"
                )));
            }
            return Ok(());
        };
        let _management = self.lock_target(&target).await;
        // await 之后重读归属；旧 ID 只能清理自身，不能隔离同目标的新 ID。
        let (_, record) = self.resolve_id(db, id).await?;
        let owned = {
            let mut index = self
                .index
                .lock()
                .unwrap_or_else(|_| panic!("local attachment index poisoned"));
            let slot = index
                .targets
                .get_mut(&target)
                .unwrap_or_else(|| panic!("managed target missing"));
            let owned = slot.id == Some(id) || (slot.id.is_none() && record.is_some());
            if owned {
                slot.id = Some(id);
                slot.unresolved = true;
                index.ids.insert(id, target.clone());
            }
            owned
        };
        let changed = ChannelService::quarantine_if_tag(db, id, "burncloud-node-local")
            .await
            .map_err(|e| LocalRouteAttachmentError::DetachFailed(e.to_string()))?;
        if !changed {
            return Err(LocalRouteAttachmentError::DetachFailed(format!(
                "local route attachment {id} no longer exists"
            )));
        }
        let mut index = self
            .index
            .lock()
            .unwrap_or_else(|_| panic!("local attachment index poisoned"));
        if owned {
            let slot = index
                .targets
                .get_mut(&target)
                .unwrap_or_else(|| panic!("managed target missing"));
            slot.lifecycle = Lifecycle::Quarantined;
            slot.unresolved = false;
        }
        Ok(())
    }

    pub(super) async fn detach(
        &self,
        db: &Database,
        id: i32,
    ) -> Result<(), LocalRouteAttachmentError> {
        debug_assert!(id > 0);
        let (target, _) = self.resolve_id(db, id).await?;
        let Some(target) = target else {
            // 从未发布的非法残留也只能按 ID 和标签处理，不为它建立路由状态。
            ChannelService::delete_if_tag(db, id, "burncloud-node-local")
                .await
                .map_err(|e| LocalRouteAttachmentError::DetachFailed(e.to_string()))?;
            // 无关联未决目标且事务确认缺失时，摘除是幂等成功。
            return Ok(());
        };
        let _management = self.lock_target(&target).await;
        // await 之后重读归属；旧 ID 只能清理自身，不能隔离同目标的新 ID。
        let (_, record) = self.resolve_id(db, id).await?;
        let (owned, was_unresolved) = {
            let mut index = self
                .index
                .lock()
                .unwrap_or_else(|_| panic!("local attachment index poisoned"));
            let slot = index
                .targets
                .get_mut(&target)
                .unwrap_or_else(|| panic!("managed target missing"));
            let owned = slot.id == Some(id) || (slot.id.is_none() && record.is_some());
            let was_unresolved = slot.unresolved;
            if owned {
                slot.id = Some(id);
                slot.unresolved = true;
                index.ids.insert(id, target.clone());
            }
            (owned, was_unresolved)
        };
        let changed = ChannelService::delete_if_tag(db, id, "burncloud-node-local")
            .await
            .map_err(|e| LocalRouteAttachmentError::DetachFailed(e.to_string()))?;
        if !changed && owned && was_unresolved {
            return Err(LocalRouteAttachmentError::DetachFailed(
                "missing record cannot resolve a previous unknown transaction".into(),
            ));
        }
        let mut index = self
            .index
            .lock()
            .unwrap_or_else(|_| panic!("local attachment index poisoned"));
        if owned {
            let slot = index
                .targets
                .get_mut(&target)
                .unwrap_or_else(|| panic!("managed target missing"));
            slot.lifecycle = Lifecycle::Detached;
            index.ids.remove(&id);
            index.targets.remove(&target);
        } else if index
            .targets
            .get(&target)
            .is_some_and(|slot| slot.id.is_none() && !slot.unresolved)
        {
            index.targets.remove(&target);
        }
        Ok(())
    }

    async fn lock_target(&self, target: &AttachmentTarget) -> OwnedMutexGuard<()> {
        loop {
            let management = {
                let mut index = self
                    .index
                    .lock()
                    .unwrap_or_else(|_| panic!("local attachment index poisoned"));
                Arc::clone(
                    &index
                        .targets
                        .entry(target.clone())
                        .or_insert_with(|| AttachmentSlot {
                            id: None,
                            lifecycle: Lifecycle::Absent,
                            unresolved: false,
                            epoch: 0,
                            management: Arc::new(AsyncMutex::new(())),
                            heartbeat: None,
                            probe_healthy: false,
                            recovery_successes: 0,
                            faulted: false,
                            in_flight: Arc::new(AtomicU32::new(0)),
                        })
                        .management,
                )
            };
            let guard = Arc::clone(&management).lock_owned().await;
            let current = self
                .index
                .lock()
                .unwrap_or_else(|_| panic!("local attachment index poisoned"))
                .targets
                .get(target)
                .is_some_and(|slot| Arc::ptr_eq(&slot.management, &management));
            if current {
                return guard;
            }
            // 旧对象已被摘除；只重取协调锁，不重试数据库写入。
        }
    }

    async fn find_existing_target(
        db: &Database,
        target: &AttachmentTarget,
    ) -> Result<Option<Channel>, LocalRouteAttachmentError> {
        let channels = ChannelService::list(db, i32::MAX, 0)
            .await
            .map_err(|e| LocalRouteAttachmentError::AttachFailed(e.to_string()))?;
        let mut found = None;
        for channel in channels {
            if channel.tag.as_deref() != Some("burncloud-node-local")
                || channel.models != target.model
            {
                continue;
            }
            let matches = channel
                .base_url
                .as_deref()
                .and_then(|raw| reqwest::Url::parse(raw).ok())
                .is_some_and(|url| url.as_str().trim_end_matches('/') == target.base_url);
            if matches {
                if found.is_some() {
                    return Err(LocalRouteAttachmentError::AttachFailed(
                        "multiple stored attachments own the same target".into(),
                    ));
                }
                found = Some(channel);
            }
        }
        Ok(found)
    }

    async fn resolve_id(
        &self,
        db: &Database,
        id: i32,
    ) -> Result<(Option<AttachmentTarget>, Option<Channel>), LocalRouteAttachmentError> {
        let indexed = self
            .index
            .lock()
            .unwrap_or_else(|_| panic!("local attachment index poisoned"))
            .ids
            .get(&id)
            .cloned();
        let channel = ChannelService::get_by_id(db, id)
            .await
            .map_err(|e| LocalRouteAttachmentError::DetachFailed(e.to_string()))?;
        if let Some(channel) = channel.as_ref() {
            if channel.tag.as_deref() != Some("burncloud-node-local") {
                return Err(LocalRouteAttachmentError::DetachFailed(
                    "channel is not a local attachment".into(),
                ));
            }
        }
        let target = indexed.or_else(|| {
            channel.as_ref().and_then(|channel| {
                let url = reqwest::Url::parse(channel.base_url.as_deref()?).ok()?;
                // 非根地址残留只允许按 ID 清理，不能变成可派发目标。
                if !matches!(url.scheme(), "http" | "https")
                    || url.path() != "/"
                    || !url.username().is_empty()
                    || url.password().is_some()
                    || url.query().is_some()
                    || url.fragment().is_some()
                    || !url
                        .host_str()?
                        .trim_matches(['[', ']'])
                        .parse::<std::net::IpAddr>()
                        .ok()?
                        .is_loopback()
                {
                    return None;
                }
                Some(AttachmentTarget {
                    model: channel.models.clone(),
                    base_url: url.as_str().trim_end_matches('/').to_owned(),
                })
            })
        });
        Ok((target, channel))
    }

    pub(crate) fn lifecycle_allows(&self, id: i32, model: &str, base_url: &str) -> bool {
        debug_assert!(id > 0);
        let index = self
            .index
            .lock()
            .unwrap_or_else(|_| panic!("local attachment index poisoned"));
        let Some(target) = index.ids.get(&id) else {
            return false;
        };
        target.model == model
            && target.base_url == base_url
            && index.targets.get(target).is_some_and(|slot| {
                slot.id == Some(id)
                    && slot_ready(slot, &index, Instant::now())
                    && slot.in_flight.load(Ordering::Acquire) < index.max_concurrency.max(1)
            })
    }

    #[expect(clippy::panic_in_result_fn, reason = "共享索引中毒是内部不变量损坏")]
    pub(crate) fn configure(&self, limit: u32) -> Result<(), LocalRouteAttachmentError> {
        debug_assert!(limit > 0);
        let mut index = self
            .index
            .lock()
            .unwrap_or_else(|_| panic!("local attachment index poisoned"));
        if (index.configured || index.epoch != 0 || index.targets.values().any(|s| s.id.is_some()))
            && index.max_concurrency.max(1) != limit
        {
            return Err(LocalRouteAttachmentError::AttachFailed(
                "local concurrency already initialized with a different limit".into(),
            ));
        }
        index.max_concurrency = limit;
        index.configured = true;
        Ok(())
    }

    pub(crate) fn try_acquire(
        self: &Arc<Self>,
        id: i32,
        model: &str,
        base_url: &str,
    ) -> Option<LocalDispatchPermit> {
        let index = self
            .index
            .lock()
            .unwrap_or_else(|_| panic!("local attachment index poisoned"));
        let target = index.ids.get(&id)?;
        let slot = index.targets.get(target)?;
        if target.model != model
            || target.base_url != base_url
            || slot.id != Some(id)
            || !slot_ready(slot, &index, Instant::now())
            || slot.in_flight.load(Ordering::Acquire) >= index.max_concurrency.max(1)
        {
            return None;
        }
        // 检查与加一共用索引锁；Drop 只减自身计数，旧 ID 不会污染新挂载。
        slot.in_flight.fetch_add(1, Ordering::AcqRel);
        Some(LocalDispatchPermit {
            local: Arc::clone(self),
            id,
            epoch: slot.epoch,
            in_flight: Arc::clone(&slot.in_flight),
        })
    }

    pub(super) fn managed_id(&self, epoch: u64, attachment: &LocalRouteAttachment) -> Option<i32> {
        let index = self
            .index
            .lock()
            .unwrap_or_else(|_| panic!("local attachment index poisoned"));
        if index.epoch != epoch || !index.manager_active {
            return None;
        }
        let target = AttachmentTarget {
            model: attachment.model.clone(),
            base_url: attachment.base_url.clone(),
        };
        index
            .targets
            .get(&target)
            .filter(|s| s.epoch == epoch && s.lifecycle == Lifecycle::Attached && !s.unresolved)
            .and_then(|s| s.id)
    }

    pub(super) fn record_probe(&self, epoch: u64, id: i32, evidence: Option<Instant>) {
        let mut index = self
            .index
            .lock()
            .unwrap_or_else(|_| panic!("local attachment index poisoned"));
        if index.epoch != epoch || !index.manager_active {
            return;
        }
        let Some(target) = index.ids.get(&id).cloned() else {
            return;
        };
        let Some(slot) = index.targets.get_mut(&target) else {
            return;
        };
        if slot.id != Some(id)
            || slot.epoch != epoch
            || slot.lifecycle != Lifecycle::Attached
            || slot.unresolved
        {
            return;
        }
        // 先判断旧证据是否过期，避免刷新时间戳绕过连续两次恢复门槛。
        if slot.heartbeat.is_none_or(|t| t.elapsed() >= HEARTBEAT_TTL) {
            slot.probe_healthy = false;
            slot.recovery_successes = 0;
        }
        match evidence.filter(|t| t.elapsed() < HEARTBEAT_TTL) {
            Some(at) => {
                slot.heartbeat = Some(at);
                if !slot.probe_healthy {
                    slot.recovery_successes += 1;
                    slot.probe_healthy = slot.recovery_successes >= 2;
                }
            }
            None => {
                slot.probe_healthy = false;
                slot.recovery_successes = 0;
            }
        }
    }

    pub(crate) async fn local_client(&self) -> Result<&reqwest::Client, reqwest::Error> {
        self.client
            .get_or_try_init(|| async {
                reqwest::Client::builder()
                    .connect_timeout(std::time::Duration::from_secs(
                        crate::HTTP_CONNECT_TIMEOUT_SECS,
                    ))
                    .timeout(std::time::Duration::from_secs(
                        crate::HTTP_REQUEST_TIMEOUT_SECS,
                    ))
                    .pool_idle_timeout(std::time::Duration::from_secs(
                        crate::HTTP_POOL_IDLE_TIMEOUT_SECS,
                    ))
                    .tcp_keepalive(std::time::Duration::from_secs(
                        crate::HTTP_TCP_KEEPALIVE_SECS,
                    ))
                    .redirect(reqwest::redirect::Policy::none())
                    .no_proxy()
                    .build()
            })
            .await
    }
}

fn build_channel(target: &AttachmentTarget) -> Channel {
    Channel {
        id: 0,
        type_: ChannelType::OpenAI as i32,
        key: String::new(),
        status: 1,
        name: format!("BurnCloud Node: {}", target.model),
        weight: 1,
        created_time: None,
        test_time: None,
        response_time: None,
        base_url: Some(target.base_url.clone()),
        models: target.model.clone(),
        group: "default".to_string(),
        used_quota: 0,
        model_mapping: None,
        priority: 0,
        auto_ban: 1,
        other_info: None,
        tag: Some("burncloud-node-local".to_string()),
        setting: None,
        param_override: None,
        header_override: None,
        remark: Some("Managed by BurnCloud Node runtime".to_string()),
        api_version: Some("default".to_string()),
        pricing_region: None,
        rpm_cap: None,
        tpm_cap: None,
        reservation_green: None,
        reservation_yellow: None,
        reservation_red: None,
    }
}

#[cfg(test)]
#[path = "local_attachment_state_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "local_attachment_dispatch_tests.rs"]
mod dispatch_tests;

const HEARTBEAT_TTL: std::time::Duration = std::time::Duration::from_secs(15);

fn slot_ready(slot: &AttachmentSlot, index: &AttachmentIndex, now: Instant) -> bool {
    slot.lifecycle == Lifecycle::Attached
        && !slot.unresolved
        && !slot.faulted
        && slot.probe_healthy
        && slot
            .heartbeat
            .is_some_and(|t| now.duration_since(t) < HEARTBEAT_TTL)
        && slot.epoch == index.epoch
        && (index.epoch == 0 || index.manager_active)
}

pub(crate) struct LocalDispatchPermit {
    local: Arc<LocalAttachmentState>,
    id: i32,
    epoch: u64,
    in_flight: Arc<AtomicU32>,
}

impl LocalDispatchPermit {
    pub(crate) fn feedback(&self, fault: bool) -> bool {
        let mut index = self
            .local
            .index
            .lock()
            .unwrap_or_else(|_| panic!("local attachment index poisoned"));
        if index.epoch != self.epoch || (self.epoch != 0 && !index.manager_active) {
            return false;
        }
        let Some(target) = index.ids.get(&self.id).cloned() else {
            return false;
        };
        let Some(slot) = index.targets.get_mut(&target) else {
            return false;
        };
        if slot.id != Some(self.id)
            || slot.epoch != self.epoch
            || slot.unresolved
            || slot.lifecycle != Lifecycle::Attached
            || slot.faulted
        {
            return false;
        }
        if fault {
            slot.faulted = true;
        }
        true
    }
}

impl Drop for LocalDispatchPermit {
    fn drop(&mut self) {
        let previous = self.in_flight.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}

pub(crate) fn configured_concurrency() -> Result<u32, LocalRouteAttachmentError> {
    match std::env::var("BURNCLOUD_LOCAL_ROUTE_MAX_CONCURRENCY") {
        Ok(raw) => parse_concurrency(&raw),
        Err(std::env::VarError::NotPresent) => Ok(1),
        Err(error) => Err(LocalRouteAttachmentError::AttachFailed(format!(
            "BURNCLOUD_LOCAL_ROUTE_MAX_CONCURRENCY: {error}"
        ))),
    }
}

fn parse_concurrency(raw: &str) -> Result<u32, LocalRouteAttachmentError> {
    if !raw.is_empty() && raw.bytes().all(|c| c.is_ascii_digit()) {
        if let Ok(value @ 1..=u32::MAX) = raw.parse() {
            return Ok(value);
        }
    }
    Err(LocalRouteAttachmentError::AttachFailed(
        "BURNCLOUD_LOCAL_ROUTE_MAX_CONCURRENCY must be a decimal positive u32".into(),
    ))
}
