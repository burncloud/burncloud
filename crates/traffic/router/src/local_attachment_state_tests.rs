#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "独立白盒测试的断言前提失败必须中止"
)]
use super::*;
use crate::local_attachment::{ExistingRouterLocalAttacher, LocalRouteAttacher};
use crate::model_router::ModelRouter;

async fn database() -> (tempfile::NamedTempFile, Arc<Database>) {
    let file = tempfile::NamedTempFile::new().unwrap();
    let url = burncloud_database::sqlite_url(&file.path().to_string_lossy(), cfg!(windows), true);
    let db = Arc::new(
        burncloud_database::create_database_with_url(&url)
            .await
            .unwrap(),
    );
    (file, db)
}

fn attachment(model: &str) -> LocalRouteAttachment {
    LocalRouteAttachment {
        model: model.into(),
        base_url: "http://127.0.0.1:18080".into(),
    }
}

#[tokio::test]
async fn same_database_shares_state_but_independent_objects_do_not() {
    let (_file, db) = database().await;
    let state = LocalAttachmentState::for_database(&db);
    assert!(Arc::ptr_eq(
        &state,
        &LocalAttachmentState::for_database(&db.clone())
    ));
    let router = ModelRouter::new(db.clone());
    assert!(std::ptr::eq(
        router.local_attachment_state().as_ref(),
        state.as_ref()
    ));
    let (_other_file, other_db) = database().await;
    assert!(!Arc::ptr_eq(
        &state,
        &LocalAttachmentState::for_database(&other_db)
    ));
    let url = burncloud_database::sqlite_url(&_file.path().to_string_lossy(), cfg!(windows), true);
    let same_url_db = Arc::new(
        burncloud_database::create_database_with_url(&url)
            .await
            .unwrap(),
    );
    assert!(!Arc::ptr_eq(
        &state,
        &LocalAttachmentState::for_database(&same_url_db)
    ));
    same_url_db.get_connection().unwrap().pool().close().await;
    other_db.get_connection().unwrap().pool().close().await;
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn lifecycle_duplicate_identity_old_permit_and_replacement() {
    let (_file, db) = database().await;
    let state = LocalAttachmentState::for_database(&db);
    let first = EvidenceAttacher::new(db.clone());
    let second = EvidenceAttacher::new(db.clone());
    let target = attachment("Qwen:中文 4b");
    let id = first.attach(target.clone()).await.unwrap();
    assert!(id.0 > 0);
    assert!(state.lifecycle_allows(id.0, &target.model, &target.base_url));
    assert!(!state.lifecycle_allows(id.0, "Qwen", &target.base_url));
    assert!(!state.lifecycle_allows(id.0, &target.model, "http://127.0.0.2:18080"));
    assert_eq!(second.attach(target.clone()).await.unwrap(), id);
    assert_eq!(
        ChannelService::list(&db, i32::MAX, 0).await.unwrap().len(),
        1
    );
    // 该值代表隔离前已经授予的一次许可，不因为隔离而撤销。
    let old_permit = state.lifecycle_allows(id.0, &target.model, &target.base_url);
    second.quarantine(id).await.unwrap();
    assert!(old_permit);
    assert!(!state.lifecycle_allows(id.0, &target.model, &target.base_url));
    first.quarantine(id).await.unwrap();
    assert!(first.attach(target.clone()).await.is_err());
    second.detach(id).await.unwrap();
    first.detach(id).await.unwrap();
    assert!(first.quarantine(id).await.is_err());
    let new_id = first.attach(target.clone()).await.unwrap();
    assert_ne!(id, new_id);
    second.detach(id).await.unwrap();
    assert!(second.quarantine(id).await.is_err());
    assert!(!state.lifecycle_allows(id.0, &target.model, &target.base_url));
    assert!(state.lifecycle_allows(new_id.0, &target.model, &target.base_url));
    first.detach(new_id).await.unwrap();
    assert!(state.index.lock().unwrap().targets.is_empty());
    assert!(state.index.lock().unwrap().ids.is_empty());
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn concurrent_same_target_creates_once_and_normalizes_default_port() {
    let (_file, db) = database().await;
    let first = EvidenceAttacher::new(db.clone());
    let second = EvidenceAttacher::new(db.clone());
    let a = LocalRouteAttachment {
        model: "test".into(),
        base_url: "http://127.0.0.1:80/".into(),
    };
    let b = LocalRouteAttachment {
        model: "test".into(),
        base_url: "http://127.0.0.1".into(),
    };
    let (a, b) = tokio::join!(first.attach(a), second.attach(b));
    assert_eq!(a.as_ref().unwrap(), b.as_ref().unwrap());
    let id = a.unwrap();
    let records = ChannelService::list(&db, i32::MAX, 0).await.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].base_url.as_deref(), Some("http://127.0.0.1"));
    first.detach(id).await.unwrap();
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn unpublished_record_is_not_adopted_and_requires_explicit_cleanup() {
    let (_file, db) = database().await;
    let state = LocalAttachmentState::for_database(&db);
    let target = AttachmentTarget {
        model: "test".into(),
        base_url: "http://127.0.0.1:18080".into(),
    };
    let mut channel = build_channel(&target);
    let id = ChannelService::create(&db, &mut channel).await.unwrap();
    let attacher = EvidenceAttacher::new(db.clone());
    assert!(attacher.attach(attachment("test")).await.is_err());
    assert!(!state.lifecycle_allows(id, &target.model, &target.base_url));
    attacher.detach(LocalRouteAttachmentId(id)).await.unwrap();
    let fresh = attacher.attach(attachment("test")).await.unwrap();
    assert_ne!(fresh.0, id);
    attacher.detach(fresh).await.unwrap();
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn cancelled_write_keeps_target_unresolved_and_does_not_disable_other_targets() {
    let (_file, db) = database().await;
    let state = LocalAttachmentState::for_database(&db);
    let attacher = EvidenceAttacher::new(db.clone());
    let stable = attacher.attach(attachment("stable")).await.unwrap();
    let pool = db.get_connection().unwrap().pool();
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("UPDATE channel_providers SET weight = weight WHERE id = ?")
        .bind(stable.0)
        .execute(&mut *blocker)
        .await
        .unwrap();
    let other_db = db.clone();
    let job = tokio::spawn(async move {
        EvidenceAttacher::new(other_db)
            .attach(attachment("pending"))
            .await
    });
    let target = AttachmentTarget {
        model: "pending".into(),
        base_url: "http://127.0.0.1:18080".into(),
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if state
                .index
                .lock()
                .unwrap()
                .targets
                .get(&target)
                .is_some_and(|slot| slot.unresolved)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(state.lifecycle_allows(stable.0, "stable", "http://127.0.0.1:18080"));
    job.abort();
    assert!(job.await.unwrap_err().is_cancelled());
    blocker.rollback().await.unwrap();
    assert!(state.index.lock().unwrap().targets[&target].unresolved);
    assert!(attacher.attach(attachment("pending")).await.is_err());
    assert!(state.lifecycle_allows(stable.0, "stable", "http://127.0.0.1:18080"));
    attacher.detach(stable).await.unwrap();
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn waiter_rechecks_management_lock_identity_after_detach() {
    let (_file, db) = database().await;
    let state = LocalAttachmentState::for_database(&db);
    let target = AttachmentTarget {
        model: "test".into(),
        base_url: "http://127.0.0.1:18080".into(),
    };
    let old_guard = state.lock_target(&target).await;
    let mut waiter = Box::pin(state.lock_target(&target));
    assert!(futures::poll!(waiter.as_mut()).is_pending());
    state.index.lock().unwrap().targets.remove(&target);
    let new_guard = state.lock_target(&target).await;
    drop(old_guard);
    assert!(futures::poll!(waiter.as_mut()).is_pending());
    drop(new_guard);
    drop(waiter.await);
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn storage_failures_keep_admission_closed_until_explicit_cleanup() {
    let (_file, db) = database().await;
    let manager = EvidenceAttacher::new(db.clone());
    let pool = db.get_connection().unwrap().pool();
    sqlx::raw_sql("CREATE TRIGGER fail_attach BEFORE INSERT ON channel_providers BEGIN SELECT RAISE(ABORT, 'injected'); END").execute(pool).await.unwrap();
    assert!(manager.attach(attachment("failed-create")).await.is_err());
    sqlx::raw_sql("DROP TRIGGER fail_attach")
        .execute(pool)
        .await
        .unwrap();
    // 所有适配器句柄释放后，未知结果仍由同数据库实例保留，且不能反向保活数据库。
    drop(manager);
    assert_eq!(Arc::strong_count(&db), 1);
    let manager = EvidenceAttacher::new(db.clone());
    assert!(manager.attach(attachment("failed-create")).await.is_err());
    assert!(ChannelService::list(&db, i32::MAX, 0)
        .await
        .unwrap()
        .is_empty());
    let state = LocalAttachmentState::for_database(&db);
    let id = manager.attach(attachment("working")).await.unwrap();
    for operation in ["UPDATE", "DELETE"] {
        let sql = format!("CREATE TRIGGER fail_local_{operation} BEFORE {operation} ON channel_providers BEGIN SELECT RAISE(ABORT, 'injected'); END");
        sqlx::raw_sql(&sql).execute(pool).await.unwrap();
        let result = if operation == "UPDATE" {
            manager.quarantine(id).await
        } else {
            manager.detach(id).await
        };
        assert!(result.is_err());
        assert!(!state.lifecycle_allows(id.0, "working", "http://127.0.0.1:18080"));
        assert_eq!(
            ChannelService::get_by_id(&db, id.0)
                .await
                .unwrap()
                .unwrap()
                .status,
            1
        );
        assert_eq!(
            ChannelService::list_enabled_at_highest_priority(&db, "default", "working")
                .await
                .unwrap()
                .len(),
            1
        );
        sqlx::raw_sql(&format!("DROP TRIGGER fail_local_{operation}"))
            .execute(pool)
            .await
            .unwrap();
    }
    manager.detach(id).await.unwrap();
    let next = manager.attach(attachment("working")).await.unwrap();
    assert_ne!(id, next);
    manager.detach(next).await.unwrap();
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn old_attach_waiter_cannot_join_a_new_manager_session() {
    let (_file, db) = database().await;
    let state = LocalAttachmentState::for_database(&db);
    let epoch = state.claim_manager().unwrap();
    state.activate_manager(epoch).unwrap();
    let target = AttachmentTarget {
        model: "waiting".into(),
        base_url: "http://127.0.0.1:18080".into(),
    };
    let guard = state.lock_target(&target).await;
    let attacher = EvidenceAttacher::new(db.clone());
    let mut attempt = Box::pin(attacher.attach(attachment("waiting")));
    assert!(futures::poll!(attempt.as_mut()).is_pending());
    state.stop_manager(epoch);
    let next = state.claim_manager().unwrap();
    state.activate_manager(next).unwrap();
    state.stop_manager(epoch);
    assert!(state.index.lock().unwrap().manager_active);
    drop(guard);
    assert!(attempt.await.is_err());
    assert!(ChannelService::list(&db, 100, 0).await.unwrap().is_empty());
    state.stop_manager(next);
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn committed_write_after_session_stop_stays_unpublished_in_new_session() {
    let (_file, db) = database().await;
    let state = LocalAttachmentState::for_database(&db);
    let epoch = state.claim_manager().unwrap();
    state.activate_manager(epoch).unwrap();
    let attacher = EvidenceAttacher::new(db.clone());
    let stable = attacher.attach(attachment("stable")).await.unwrap();
    let pool = db.get_connection().unwrap().pool();
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("UPDATE channel_providers SET weight = weight WHERE id = ?")
        .bind(stable.0)
        .execute(&mut *blocker)
        .await
        .unwrap();
    let other_db = db.clone();
    let job = tokio::spawn(async move {
        EvidenceAttacher::new(other_db)
            .attach(attachment("late"))
            .await
    });
    let target = AttachmentTarget {
        model: "late".into(),
        base_url: "http://127.0.0.1:18080".into(),
    };
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if state
                .index
                .lock()
                .unwrap()
                .targets
                .get(&target)
                .is_some_and(|slot| slot.unresolved)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    state.stop_manager(epoch);
    let next = state.claim_manager().unwrap();
    state.activate_manager(next).unwrap();
    blocker.rollback().await.unwrap();
    assert!(job.await.unwrap().is_err());
    let id = state.index.lock().unwrap().targets[&target].id.unwrap();
    assert!(ChannelService::get_by_id(&db, id).await.unwrap().is_some());
    assert!(!state.lifecycle_allows(id, &target.model, &target.base_url));
    assert!(attacher.attach(attachment("late")).await.is_err());
    attacher.detach(LocalRouteAttachmentId(id)).await.unwrap();
    let replacement = attacher.attach(attachment("late")).await.unwrap();
    assert_ne!(id, replacement.0);
    assert!(state.lifecycle_allows(replacement.0, &target.model, &target.base_url));
    state.stop_manager(next);
    pool.close().await;
}

// 生命周期测试直接注入已经通过探测的内部证据；公开网络边界另由管理器和集成测试覆盖。
struct EvidenceAttacher {
    db: Arc<Database>,
}
impl EvidenceAttacher {
    fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
    async fn attach(
        &self,
        target: LocalRouteAttachment,
    ) -> Result<LocalRouteAttachmentId, LocalRouteAttachmentError> {
        let target = crate::local_attachment::adapter::normalize_attachment(target)?;
        LocalAttachmentState::for_database(&self.db)
            .attach(&self.db, target, Some(Instant::now()))
            .await
    }
    async fn quarantine(
        &self,
        id: LocalRouteAttachmentId,
    ) -> Result<(), LocalRouteAttachmentError> {
        ExistingRouterLocalAttacher::new(self.db.clone())
            .quarantine(id)
            .await
    }
    async fn detach(&self, id: LocalRouteAttachmentId) -> Result<(), LocalRouteAttachmentError> {
        ExistingRouterLocalAttacher::new(self.db.clone())
            .detach(id)
            .await
    }
}

#[tokio::test]
async fn heartbeat_expiry_requires_two_successes_and_never_unlocks_inference_fault() {
    let (_file, db) = database().await;
    let state = LocalAttachmentState::for_database(&db);
    let epoch = state.claim_manager().unwrap();
    state.activate_manager(epoch).unwrap();
    let target = attachment("heartbeat");
    let id = EvidenceAttacher::new(db.clone())
        .attach(target.clone())
        .await
        .unwrap();
    tokio::time::pause();
    state.record_probe(epoch, id.0, Some(Instant::now()));
    tokio::time::advance(std::time::Duration::from_millis(14999)).await;
    assert!(state.lifecycle_allows(id.0, &target.model, &target.base_url));
    tokio::time::advance(std::time::Duration::from_millis(1)).await;
    assert!(!state.lifecycle_allows(id.0, &target.model, &target.base_url));
    state.record_probe(epoch, id.0, Some(Instant::now()));
    assert!(!state.lifecycle_allows(id.0, &target.model, &target.base_url));
    state.record_probe(epoch, id.0, None);
    state.record_probe(epoch, id.0, Some(Instant::now()));
    assert!(!state.lifecycle_allows(id.0, &target.model, &target.base_url));
    state.record_probe(epoch, id.0, Some(Instant::now()));
    let permit = state
        .try_acquire(id.0, &target.model, &target.base_url)
        .unwrap();
    assert!(permit.feedback(true));
    assert!(!permit.feedback(false));
    drop(permit);
    state.record_probe(epoch, id.0, Some(Instant::now()));
    state.record_probe(epoch, id.0, Some(Instant::now()));
    assert!(!state.lifecycle_allows(id.0, &target.model, &target.base_url));
    tokio::time::resume();
    db.get_connection().unwrap().pool().close().await;
}

#[tokio::test]
async fn permits_are_atomic_per_id_and_old_drop_does_not_release_replacement() {
    let (_file, db) = database().await;
    let state = LocalAttachmentState::for_database(&db);
    state.configure(1).unwrap();
    let attacher = EvidenceAttacher::new(db.clone());
    let target = attachment("slots");
    let id = attacher.attach(target.clone()).await.unwrap();
    let old = state
        .try_acquire(id.0, &target.model, &target.base_url)
        .unwrap();
    assert!(state
        .try_acquire(id.0, &target.model, &target.base_url)
        .is_none());
    assert_eq!(attacher.attach(target.clone()).await.unwrap(), id);
    assert!(state.configure(2).is_err());
    attacher.detach(id).await.unwrap();
    let fresh = attacher.attach(target.clone()).await.unwrap();
    let new_permit = state
        .try_acquire(fresh.0, &target.model, &target.base_url)
        .unwrap();
    assert!(!old.feedback(true));
    drop(old);
    assert!(state
        .try_acquire(fresh.0, &target.model, &target.base_url)
        .is_none());
    drop(new_permit);
    assert!(state
        .try_acquire(fresh.0, &target.model, &target.base_url)
        .is_some());
    db.get_connection().unwrap().pool().close().await;
}

#[test]
fn concurrency_configuration_is_strict_and_does_not_preallocate() {
    for raw in ["", "0", "-1", "+1", " 1", "1 ", "1.0", "4294967296", "一"] {
        assert!(parse_concurrency(raw).is_err(), "{raw}");
    }
    assert_eq!(parse_concurrency("1").unwrap(), 1);
    assert_eq!(parse_concurrency("4294967295").unwrap(), u32::MAX);
}
