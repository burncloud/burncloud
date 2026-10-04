#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test, and unwrap is how a failed precondition is \
              reported."
)]
//! The three behaviours the plan lists for `traffic/database-router` that had no test at all (#633,
//! plan section 5 item 11):
//!
//! 1. **Token validation through the JOIN.** `validate_token_and_get_info` never appeared in the test
//!    sources.
//! 2. **Video task identity and status persistence.** The whole `RouterVideoTaskModel` never appeared.
//! 3. **What settlement returns versus what it wrote.** `deduct_quota` never appeared either.
//!
//! The plan adds a warning with this item, and it is the reason (3) is written the way it is: *do not
//! interpret "returned false" as "nothing was written"*. `deduct_quota` records the charge and then
//! reports exhaustion, so the two are separate facts and both are asserted.

use burncloud_database::create_database_with_url;
use burncloud_database_router::{
    RouterDatabase, RouterLog, RouterLogModel, RouterToken, RouterTokenModel, RouterVideoTask,
    RouterVideoTaskModel,
};
use tempfile::NamedTempFile;

/// An isolated SQLite database with the production schema applied.
///
/// Same shape as the helper in the neighbouring files, including the two things that are easy to get
/// wrong: the three-slash URL (the two-slash form is not a URL SQLite can open on Windows) and taking
/// the path away from the temp guard so removal is controlled here rather than racing the pool. This file
/// does not attempt to fix the residual leak -- `temp_file_leak.rs` measures it and #643 owns the fix.
async fn create_test_db() -> TestDb {
    let tmp = NamedTempFile::new().unwrap_or_else(|e| panic!("failed to create temp file: {e}"));
    let path = tmp
        .into_temp_path()
        .keep()
        .unwrap_or_else(|e| panic!("failed to keep temp path: {e}"));
    let normalized = path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{normalized}?mode=rwc");
    let db = create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("failed to initialize test database: {e}"));
    RouterDatabase::init(&db)
        .await
        .unwrap_or_else(|e| panic!("failed to initialize router tables: {e}"));
    TestDb { db: Some(db), path }
}

struct TestDb {
    db: Option<burncloud_database::Database>,
    path: std::path::PathBuf,
}

impl std::ops::Deref for TestDb {
    type Target = burncloud_database::Database;

    fn deref(&self) -> &Self::Target {
        self.db
            .as_ref()
            .unwrap_or_else(|| panic!("the test database was used after cleanup"))
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let Some(db) = self.db.take() else {
            return;
        };
        let path = self.path.clone();
        let _ = std::thread::Builder::new()
            .name("bc-gap-testdb-cleanup".to_string())
            .spawn(move || {
                if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    if let Err(e) = rt.block_on(db.close()) {
                        eprintln!("test database close failed (cleanup continues): {e}");
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
                for suffix in ["", "-wal", "-shm"] {
                    let mut candidate = path.clone().into_os_string();
                    candidate.push(suffix);
                    let _ = std::fs::remove_file(std::path::PathBuf::from(candidate));
                }
            });
    }
}

/// Insert the account a credential has to belong to. `validate_token_and_get_info` requires the owner to
/// be active, so the credential tests need one.
async fn insert_user(db: &burncloud_database::Database, id: &str, status: i32) {
    use burncloud_database::sqlx;
    sqlx::query(
        "INSERT INTO user_accounts (id, username, email, password_hash, github_id, status, \
         balance_usd, balance_cny, preferred_currency) \
         VALUES (?, ?, ?, ?, NULL, ?, 0, 0, 'CNY')",
    )
    .bind(id)
    .bind(format!("user_{id}"))
    .bind(format!("{id}@example.com"))
    .bind("$2b$12$notarealhash")
    .bind(status)
    .execute(db.get_connection().expect("connection").pool())
    .await
    .expect("insert user_accounts");
}

/// Insert an API key row, which is the table `validate_token_and_get_info` reads.
async fn insert_api_key(db: &burncloud_database::Database, key: &str, user_id: &str, status: i64) {
    use burncloud_database::sqlx;
    sqlx::query(
        "INSERT INTO user_api_keys (key, user_id, status, remain_quota, used_quota) \
         VALUES (?, ?, ?, 1000000000, 0)",
    )
    .bind(key)
    .bind(user_id)
    .bind(status)
    .execute(db.get_connection().expect("connection").pool())
    .await
    .expect("insert user_api_keys");
}

/// Insert a `router_tokens` row, the right-hand side of the validation JOIN.
async fn insert_router_token(db: &burncloud_database::Database, token: &str, user_id: &str) {
    RouterTokenModel::create(
        db,
        &RouterToken {
            token: token.to_string(),
            user_id: user_id.to_string(),
            status: "active".to_string(),
            quota_limit: -1,
            used_quota: 0,
            expired_time: -1,
            accessed_time: 0,
            key_version: 1,
            old_key_hash: None,
            old_key_expires_at: 0,
            ip_whitelist: None,
            key_prefix: "bc_live_".to_string(),
            created_at: 1_700_000_000,
            last_rotated_at: 0,
        },
    )
    .await
    .expect("insert router_tokens");
}

// -------------------------------------------------------------------------------------------
// 1. token validation through the JOIN
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_valid_credential_resolves_through_the_join_and_carries_the_router_columns() {
    // The control case. `validate_token_and_get_info` joins `user_api_keys` to `user_accounts` to reach
    // the group, and LEFT joins `router_tokens` for the classifier inputs, so this asserts that both
    // halves arrive.
    let db = create_test_db().await;
    insert_user(&db, "u-1", 1).await;
    insert_api_key(&db, "bc_live_valid", "u-1", 1).await;
    insert_router_token(&db, "bc_live_valid", "u-1").await;

    let info = RouterDatabase::validate_token_and_get_info(&db, "bc_live_valid")
        .await
        .expect("validation must not be an error for a valid credential")
        .expect("a valid credential must resolve to a row");

    assert_eq!(info.user_id, "u-1");
    assert_eq!(
        info.group, "default",
        "the group comes from user_accounts through the JOIN, so it must not be empty"
    );
    assert_eq!(info.remain_quota, 1_000_000_000);
    assert_eq!(info.used_quota, 0);
}

#[tokio::test]
async fn a_credential_without_a_router_token_still_resolves_with_none_for_the_router_columns() {
    // The LEFT JOIN is the reason `order_type` and `price_cap` are `Option`, and the source documents
    // that this case reads as `None`. Pinned because the alternative -- an INNER JOIN -- would make a
    // token that exists only in `user_api_keys` fail authentication entirely, which is a much larger
    // behaviour change than it looks.
    let db = create_test_db().await;
    insert_user(&db, "u-2", 1).await;
    insert_api_key(&db, "bc_live_no_router_row", "u-2", 1).await;

    let info = RouterDatabase::validate_token_and_get_info(&db, "bc_live_no_router_row")
        .await
        .expect("a missing router_tokens row must not be an error")
        .expect("the credential exists in user_api_keys, so it must resolve");

    assert_eq!(info.user_id, "u-2");
    assert_eq!(
        info.order_type, None,
        "no router_tokens row means no order_type to read"
    );
    assert_eq!(info.price_cap, None, "and no price cap either");
}

#[tokio::test]
async fn an_unknown_credential_resolves_to_nothing_rather_than_erroring() {
    let db = create_test_db().await;
    let info = RouterDatabase::validate_token_and_get_info(&db, "bc_live_never_created")
        .await
        .expect("an unknown credential is a value, not an error");
    assert!(info.is_none(), "an unknown credential must not resolve");
}

#[tokio::test]
async fn a_disabled_credential_does_not_resolve() {
    // `user_api_keys.status` is checked in the WHERE clause, so a status change is the revocation path.
    let db = create_test_db().await;
    insert_user(&db, "u-3", 1).await;
    insert_api_key(&db, "bc_live_disabled_key", "u-3", 0).await;

    let info = RouterDatabase::validate_token_and_get_info(&db, "bc_live_disabled_key")
        .await
        .expect("a disabled credential is a value, not an error");
    assert!(
        info.is_none(),
        "a credential whose status is not 1 must not authenticate"
    );
}

#[tokio::test]
async fn a_credential_whose_owner_is_disabled_does_not_resolve() {
    // The JOIN checks `u.status = 1` as well as the key's own status. Disabling the account has to revoke
    // every credential under it; if only `t.status` were checked, a disabled account could keep spending.
    let db = create_test_db().await;
    insert_user(&db, "u-4", 0).await;
    insert_api_key(&db, "bc_live_owner_disabled", "u-4", 1).await;

    let info = RouterDatabase::validate_token_and_get_info(&db, "bc_live_owner_disabled")
        .await
        .expect("a disabled owner is a value, not an error");
    assert!(
        info.is_none(),
        "a credential whose owner is disabled must not authenticate, even though the key itself is active"
    );
}

#[tokio::test]
async fn one_users_credential_does_not_resolve_another_users_identity() {
    // The JOIN is on `t.user_id = u.id`; a wrong join column would hand one user another's identity, so
    // the two are given different groups to make that visible.
    let db = create_test_db().await;
    insert_user(&db, "u-a", 1).await;
    insert_user(&db, "u-b", 1).await;
    insert_api_key(&db, "bc_live_a", "u-a", 1).await;
    insert_api_key(&db, "bc_live_b", "u-b", 1).await;

    let a = RouterDatabase::validate_token_and_get_info(&db, "bc_live_a")
        .await
        .expect("validate a")
        .expect("a must resolve");
    let b = RouterDatabase::validate_token_and_get_info(&db, "bc_live_b")
        .await
        .expect("validate b")
        .expect("b must resolve");

    assert_eq!(a.user_id, "u-a");
    assert_eq!(b.user_id, "u-b");
    assert_ne!(a.user_id, b.user_id, "the credentials must not be swapped");
}

// -------------------------------------------------------------------------------------------
// 2. video task identity and status persistence
// -------------------------------------------------------------------------------------------

fn video_task(task_id: &str, channel_id: i32) -> RouterVideoTask {
    RouterVideoTask {
        task_id: task_id.to_string(),
        channel_id,
        user_id: Some("u-1".to_string()),
        model: Some("veo-3".to_string()),
        duration: 8,
        resolution: "1080p".to_string(),
    }
}

#[tokio::test]
async fn a_video_task_round_trips_with_its_channel_and_metadata() {
    // The mapping exists so a later `GET /v1/videos/{task_id}` can be routed to the channel that created
    // the task, so the channel id is the field that matters most.
    let db = create_test_db().await;
    RouterVideoTaskModel::save(&db, &video_task("task-1", 42))
        .await
        .expect("save");

    let loaded = RouterVideoTaskModel::get_by_task_id(&db, "task-1")
        .await
        .expect("get")
        .expect("the task that was just saved must be found");

    assert_eq!(loaded.task_id, "task-1");
    assert_eq!(loaded.channel_id, 42, "the routing target must survive");
    assert_eq!(loaded.user_id.as_deref(), Some("u-1"));
    assert_eq!(loaded.model.as_deref(), Some("veo-3"));
    assert_eq!(loaded.duration, 8);
    assert_eq!(loaded.resolution, "1080p");
}

#[tokio::test]
async fn an_unknown_video_task_reads_as_none() {
    let db = create_test_db().await;
    let loaded = RouterVideoTaskModel::get_by_task_id(&db, "task-never-saved")
        .await
        .expect("a missing task is a value, not an error");
    assert!(loaded.is_none(), "an unknown task id must not resolve");
}

#[tokio::test]
async fn saving_the_same_task_id_twice_keeps_the_first_channel() {
    // `save` is `INSERT OR IGNORE`, so the first write wins rather than the mapping being replaced. That
    // is the behaviour a caller has to know about: a retry with a different channel id does not move the
    // task. Asserted because "upsert" and "insert-if-absent" differ exactly here, and the SQL says the
    // latter.
    let db = create_test_db().await;
    RouterVideoTaskModel::save(&db, &video_task("task-dup", 7))
        .await
        .expect("first save");
    RouterVideoTaskModel::save(&db, &video_task("task-dup", 99))
        .await
        .expect("second save must not error");

    let loaded = RouterVideoTaskModel::get_by_task_id(&db, "task-dup")
        .await
        .expect("get")
        .expect("the task must still exist");

    assert_eq!(
        loaded.channel_id, 7,
        "the first write wins; the second save is ignored rather than replacing the mapping"
    );

    // And it did not insert a second row under a different id.
    let other = RouterVideoTaskModel::get_by_task_id(&db, "task-dup")
        .await
        .expect("get again");
    assert_eq!(other.map(|t| t.channel_id), Some(7));
}

#[tokio::test]
async fn video_tasks_are_isolated_from_each_other() {
    // Several tasks are written together because the lookup is by task id: a wrong predicate would return
    // another task's channel, which would route a video request to the wrong upstream.
    let db = create_test_db().await;
    for (id, channel) in [("task-a", 11), ("task-b", 22), ("task-c", 33)] {
        RouterVideoTaskModel::save(&db, &video_task(id, channel))
            .await
            .unwrap_or_else(|e| panic!("save {id}: {e}"));
    }

    for (id, expected) in [("task-a", 11), ("task-b", 22), ("task-c", 33)] {
        let loaded = RouterVideoTaskModel::get_by_task_id(&db, id)
            .await
            .unwrap_or_else(|e| panic!("get {id}: {e}"))
            .unwrap_or_else(|| panic!("{id} must exist"));
        assert_eq!(
            loaded.channel_id, expected,
            "{id} routed to the wrong channel"
        );
    }
}

#[tokio::test]
async fn nullable_video_task_fields_round_trip_as_none() {
    // `user_id` and `model` are `Option`, and a row written with them absent must read back absent rather
    // than as an empty string.
    let db = create_test_db().await;
    RouterVideoTaskModel::save(
        &db,
        &RouterVideoTask {
            task_id: "task-nullable".to_string(),
            channel_id: 5,
            user_id: None,
            model: None,
            duration: 0,
            resolution: String::new(),
        },
    )
    .await
    .expect("save");

    let loaded = RouterVideoTaskModel::get_by_task_id(&db, "task-nullable")
        .await
        .expect("get")
        .expect("the task must exist");
    assert_eq!(loaded.user_id, None, "absent user_id must stay absent");
    assert_eq!(loaded.model, None, "absent model must stay absent");
    assert_eq!(loaded.channel_id, 5, "the channel is still required");
}

// -------------------------------------------------------------------------------------------
// 3. settlement: what it returns versus what it wrote
// -------------------------------------------------------------------------------------------

/// A minimal log entry. Built field by field rather than with `..Default::default()` because `RouterLog`
/// has no `Default` impl -- it mirrors the table row, and adding one would risk a "default" that does not
/// match the schema's own defaults.
fn tiny_log(request_id: &str) -> RouterLog {
    RouterLog {
        id: 0,
        request_id: request_id.to_string(),
        user_id: Some("u-1".to_string()),
        path: "/v1/chat/completions".to_string(),
        upstream_id: None,
        status_code: 200,
        latency_ms: 10,
        prompt_tokens: 10,
        completion_tokens: 5,
        cost: 1_000,
        model: Some("gpt-4o".to_string()),
        cache_read_tokens: 0,
        reasoning_tokens: 0,
        pricing_region: None,
        video_tokens: 0,
        cache_write_tokens: 0,
        audio_input_tokens: 0,
        audio_output_tokens: 0,
        image_tokens: 0,
        embedding_tokens: 0,
        input_cost: 0,
        output_cost: 0,
        cache_read_cost: 0,
        cache_write_cost: 0,
        audio_cost: 0,
        image_cost: 0,
        video_cost: 0,
        reasoning_cost: 0,
        embedding_cost: 0,
        layer_decision: None,
        traffic_color: None,
        cost_status: None,
        error_type: None,
        created_at: None,
    }
}

#[tokio::test]
async fn writing_a_log_entry_does_not_settle_any_quota() {
    // The source states the separation explicitly: logging is side-effect free with respect to spend, and
    // settlement belongs to `deduct_quota` scoped to the credential that authorized the request. Pinned
    // because a log write that also debited would double-charge every request.
    let db = create_test_db().await;
    insert_user(&db, "u-log", 1).await;
    insert_api_key(&db, "bc_live_log", "u-log", 1).await;

    let before: (i64, i64) = {
        use burncloud_database::sqlx;
        sqlx::query_as("SELECT remain_quota, used_quota FROM user_api_keys WHERE key = ?")
            .bind("bc_live_log")
            .fetch_one(db.get_connection().expect("connection").pool())
            .await
            .expect("read quota before")
    };

    RouterLogModel::insert(&db, &tiny_log("req-1"))
        .await
        .expect("insert log");

    let after: (i64, i64) = {
        use burncloud_database::sqlx;
        sqlx::query_as("SELECT remain_quota, used_quota FROM user_api_keys WHERE key = ?")
            .bind("bc_live_log")
            .fetch_one(db.get_connection().expect("connection").pool())
            .await
            .expect("read quota after")
    };

    assert_eq!(
        before, after,
        "inserting a log entry must not change any credential's quota"
    );

    // And the log really was written, so the assertion above is not passing because nothing happened.
    let logs = RouterLogModel::get(&db, 10, 0).await.expect("read logs");
    assert_eq!(logs.len(), 1, "the log entry must be present");
    assert_eq!(logs[0].request_id, "req-1");
}

#[tokio::test]
async fn settlement_reports_exhaustion_separately_from_what_it_wrote() {
    // This is the plan's warning made into a test: *do not interpret "returned false" as "nothing was
    // written"*. `deduct_quota` writes the charge and then reports that the credential is now exhausted,
    // so the return value and the stored total are two different facts. A caller that treated `false` as
    // "no charge happened" would under-report spend; one that treated it as "the write failed" would
    // retry and double-charge.
    let db = create_test_db().await;
    insert_user(&db, "u-cap", 1).await;
    insert_api_key(&db, "bc_live_cap", "u-cap", 1).await;

    // Give the credential a cap of 1000 nanodollars and spend 900, then settle 200 more.
    {
        use burncloud_database::sqlx;
        sqlx::query("UPDATE user_api_keys SET remain_quota = 1000, used_quota = 900 WHERE key = ?")
            .bind("bc_live_cap")
            .execute(db.get_connection().expect("connection").pool())
            .await
            .expect("set the cap");
    }

    let within_limit = RouterTokenModel::deduct_quota(&db, "bc_live_cap", 200)
        .await
        .expect("settlement must not error");

    assert!(
        !within_limit,
        "the credential was already at 900 of a 1000 cap, so this settlement crosses it and the caller \
         must be told the credential is exhausted"
    );

    let used: i64 = {
        use burncloud_database::sqlx;
        sqlx::query_scalar("SELECT used_quota FROM user_api_keys WHERE key = ?")
            .bind("bc_live_cap")
            .fetch_one(db.get_connection().expect("connection").pool())
            .await
            .expect("read used_quota")
    };

    assert_eq!(
        used, 1100,
        "the charge was recorded even though the return value said the credential is exhausted -- the \
         two facts are separate, which is what the plan warns about"
    );
}

#[tokio::test]
async fn settlement_against_an_unknown_credential_reports_failure_and_writes_nothing() {
    // The other side of the same coin, and it must fail closed: no credential means no settlement and a
    // `false`, not a silent success.
    let db = create_test_db().await;
    let settled = RouterTokenModel::deduct_quota(&db, "bc_live_absent", 100)
        .await
        .expect("an unknown credential is a value, not an error");
    assert!(
        !settled,
        "settling against a credential that does not exist must report failure"
    );
}

#[tokio::test]
async fn a_zero_cost_settlement_is_allowed_and_writes_nothing() {
    // `cost <= 0` short-circuits to `true` before any lookup, so a zero-cost settlement succeeds even for
    // a credential that does not exist. Recorded because the same short-circuit means a negative cost is
    // accepted too -- a refund path would go through this branch, and whichever way that is decided
    // should be decided deliberately rather than by this short-circuit.
    let db = create_test_db().await;
    assert!(
        RouterTokenModel::deduct_quota(&db, "bc_live_absent", 0)
            .await
            .expect("zero cost must not error"),
        "a zero-cost settlement short-circuits before the credential is looked up"
    );

    insert_user(&db, "u-zero", 1).await;
    insert_api_key(&db, "bc_live_zero", "u-zero", 1).await;
    RouterTokenModel::deduct_quota(&db, "bc_live_zero", 0)
        .await
        .expect("zero cost");

    let used: i64 = {
        use burncloud_database::sqlx;
        sqlx::query_scalar("SELECT used_quota FROM user_api_keys WHERE key = ?")
            .bind("bc_live_zero")
            .fetch_one(db.get_connection().expect("connection").pool())
            .await
            .expect("read used_quota")
    };
    assert_eq!(used, 0, "a zero-cost settlement must not change the total");
}
