#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic_in_result_fn,
    reason = "Test-only file: the assertions are the test, and clippy.toml's allow-panic-in-tests does not recognise #[tokio::test]."
)]
//! Token validation's two paths, and what each of them checks (#633, plan item 11).
//!
//! `cost_roundtrip.rs` covers the cost columns and `usage_stats_tests.rs` covers the statistics windows and the
//! balance deductions, so neither is repeated here. The plan's remaining items are reached from the credential
//! side:
//!
//! | Plan item | Here |
//! | --- | --- |
//! | Token 验证 JOIN、过期/状态 | `the_join_path_checks_the_account_and_the_credential` |
//! | **the two paths disagree about a disabled account** | `the_two_validation_paths_disagree_about_a_disabled_account` |
//! | 日志写入不扣额度 | `inserting_a_log_does_not_change_any_spend_column` |
//! | 同用户多 Token 隔离 | `two_credentials_for_one_user_do_not_share_a_quota` |
//! | video task 唯一标识和状态持久化 | `a_video_task_is_unique_by_id_and_its_status_persists` |
//!
//! ## What the probe established before any assertion was written
//!
//! A first version of this file asserted that `RouterTokenModel::validate` rejected a token whose `user_id`
//! matched no account, because the plan lists "Token 验证 JOIN" and a JOIN implies the account is checked. The
//! probe showed it returns `Valid` -- and reading the source explained why: there are **two** paths.
//!
//! * `RouterDatabase::validate_token_and_get_info` (`lib.rs:185`) reads `user_api_keys`, **joins**
//!   `user_accounts`, and filters `t.status = 1 AND u.status = 1`.
//! * `RouterTokenModel::validate` / `validate_detailed` (`token.rs:271`, `:322`) read `router_tokens` and filter
//!   on the token's own `status = 'active'` only.
//!
//! Measured, for one credential present in both tables:

use burncloud_database::{create_database_with_url, Database};
use burncloud_router::{
    RouterDatabase, RouterLog, RouterToken, RouterTokenModel, RouterVideoTask, RouterVideoTaskModel,
};

/// A temporary database with the router schema, deleted at the end of the test.
struct TempDb {
    db: Database,
    path: std::path::PathBuf,
}

impl TempDb {
    async fn new(tag: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // A process-wide counter as well as the clock: `as_nanos` has nanosecond precision but the platform
        // clock does not have nanosecond resolution, so two tests starting within one tick would otherwise
        // compute the same path and share a database.
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "bc_dbr_rules_{}_{}_{}_{}.db",
            tag,
            std::process::id(),
            serial,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::remove_file(&path).ok();
        let normalized = path.to_string_lossy().replace('\\', "/");
        // Three slashes: `sqlite:///C:/...` is the absolute form.
        let url = format!("sqlite:///{}?mode=rwc", normalized);
        let db = create_database_with_url(&url).await?;
        RouterDatabase::init(&db).await?;
        Ok(Self { db, path })
    }

    async fn cleanup(self) {
        let path = self.path.clone();
        self.db.close().await.ok();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        std::fs::remove_file(&path).ok();
        for suffix in ["-wal", "-shm"] {
            let mut candidate = path.as_os_str().to_os_string();
            candidate.push(suffix);
            std::fs::remove_file(std::path::PathBuf::from(candidate)).ok();
        }
    }
}

/// A credential row for `router_tokens`.
fn token(key: &str, user_id: &str, status: &str, expired_time: i64) -> RouterToken {
    RouterToken {
        token: key.to_string(),
        user_id: user_id.to_string(),
        status: status.to_string(),
        quota_limit: 1_000_000_000,
        used_quota: 0,
        expired_time,
        accessed_time: 0,
        key_version: 1,
        old_key_hash: None,
        old_key_expires_at: 0,
        ip_whitelist: None,
        key_prefix: "test_".to_string(),
        created_at: 0,
        last_rotated_at: 0,
    }
}

/// Insert an account, returning nothing -- the id is the argument so tests can name it.
async fn account(db: &Database, id: &str, status: i32) -> Result<(), Box<dyn std::error::Error>> {
    db.execute_query(&format!(
        "INSERT INTO user_accounts (id, username, password_hash, status, balance_usd, balance_cny) \
         VALUES ('{id}', '{id}', '$2b$12$placeholder', {status}, 1000000000, 0)"
    ))
    .await?;
    Ok(())
}

/// Insert a `user_api_keys` row -- the table the JOIN path reads.
async fn api_key(
    db: &Database,
    user_id: &str,
    key: &str,
    status: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    db.execute_query(&format!(
        "INSERT INTO user_api_keys (user_id, key, status, name, remain_quota, unlimited_quota, used_quota, \
         created_time, accessed_time, expired_time) \
         VALUES ('{user_id}', '{key}', {status}, 'probe', 100, 0, 0, 0, 0, -1)"
    ))
    .await?;
    Ok(())
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is after the epoch")
        .as_secs() as i64
}

// ===========================================================================================
// The JOIN path
// ===========================================================================================

#[tokio::test]
async fn the_join_path_checks_the_account_and_the_credential(
) -> Result<(), Box<dyn std::error::Error>> {
    // `validate_token_and_get_info` is the path that carries the account check, and the plan's "Token 验证 JOIN"
    // names it. Four refusals are asserted, because each is a separate clause in one `WHERE`: the account's
    // status, the credential's status, an unknown key, and the account being absent entirely.
    let temp = TempDb::new("join").await?;

    account(&temp.db, "usr_join", 1).await?;
    api_key(&temp.db, "usr_join", "sk-join", 1).await?;

    let info = RouterDatabase::validate_token_and_get_info(&temp.db, "sk-join")
        .await?
        .expect("an active account with an active key validates");
    println!("valid: {info:?}");
    assert_eq!(info.user_id, "usr_join");
    assert_eq!(info.remain_quota, 100, "the quota comes from user_api_keys");

    // 1. An unknown key.
    assert!(
        RouterDatabase::validate_token_and_get_info(&temp.db, "sk-absent")
            .await?
            .is_none(),
        "an unknown key reads as None"
    );

    // 2. A disabled credential.
    temp.db
        .execute_query("UPDATE user_api_keys SET status = 0 WHERE key = 'sk-join'")
        .await?;
    assert!(
        RouterDatabase::validate_token_and_get_info(&temp.db, "sk-join")
            .await?
            .is_none(),
        "a disabled credential must not validate"
    );
    temp.db
        .execute_query("UPDATE user_api_keys SET status = 1 WHERE key = 'sk-join'")
        .await?;

    // 3. A disabled account -- the clause the other path is missing.
    temp.db
        .execute_query("UPDATE user_accounts SET status = 0 WHERE id = 'usr_join'")
        .await?;
    assert!(
        RouterDatabase::validate_token_and_get_info(&temp.db, "sk-join")
            .await?
            .is_none(),
        "**a disabled account must not be able to use its credential**"
    );

    // And re-enabling restores it, so the refusal is the status and not a side effect of the update.
    temp.db
        .execute_query("UPDATE user_accounts SET status = 1 WHERE id = 'usr_join'")
        .await?;
    assert!(
        RouterDatabase::validate_token_and_get_info(&temp.db, "sk-join")
            .await?
            .is_some(),
        "re-enabling the account restores access"
    );

    temp.cleanup().await;
    Ok(())
}

#[tokio::test]
async fn the_two_validation_paths_disagree_about_a_disabled_account(
) -> Result<(), Box<dyn std::error::Error>> {
    // **The finding, and it is a security question rather than a tidiness one.**
    //
    // One credential is written to both tables -- `user_api_keys` (read by the JOIN path) and `router_tokens`
    // (read by `RouterTokenModel`). Disabling the **account** then produces opposite answers:
    //
    //     validate_token_and_get_info -> None           (refused)
    //     RouterTokenModel::validate  -> Some(user)     (accepted)
    //     validate_detailed           -> Valid          (accepted)
    //
    // So which answer a caller gets depends on which function it happened to call. A caller using
    // `RouterTokenModel` keeps honouring a credential whose owner has been disabled -- which is precisely the
    // action an operator takes to cut off a compromised account.
    //
    // **Measured, not inferred.** The first version of this file asserted that `RouterTokenModel::validate`
    // rejected a token whose `user_id` matched no account, on the reasoning that "Token 验证 JOIN" implies an
    // account check; the probe showed it returns `Valid`, and reading the source showed why -- `token.rs` filters
    // on `router_tokens.status` alone.
    //
    // **Recorded, not fixed.** The fix is a contract decision: either `RouterTokenModel` gains the same account
    // check, which changes what its callers see and touches the rotate and quota paths that share the helper, or
    // `validate_token_and_get_info` is declared the only entry point callers may use. Both are decisions for
    // whoever owns the credential path, and the plan says to record ambiguity rather than settle it silently.
    let temp = TempDb::new("paths_disagree").await?;

    account(&temp.db, "usr_both", 1).await?;
    api_key(&temp.db, "usr_both", "sk-both", 1).await?;
    RouterTokenModel::create(&temp.db, &token("sk-both", "usr_both", "active", -1)).await?;

    // While the account is active the two agree, which is what makes the disagreement below attributable to the
    // status change rather than to the two reading different rows.
    let via_join = RouterDatabase::validate_token_and_get_info(&temp.db, "sk-both").await?;
    let via_model = RouterTokenModel::validate(&temp.db, "sk-both").await?;
    println!(
        "active:   join={:?} model={:?}",
        via_join.is_some(),
        via_model.is_some()
    );
    assert!(
        via_join.is_some() && via_model.is_some(),
        "both accept an active account"
    );

    temp.db
        .execute_query("UPDATE user_accounts SET status = 0 WHERE id = 'usr_both'")
        .await?;

    let via_join = RouterDatabase::validate_token_and_get_info(&temp.db, "sk-both").await?;
    let via_model = RouterTokenModel::validate(&temp.db, "sk-both").await?;
    let detailed = RouterTokenModel::validate_detailed(&temp.db, "sk-both").await?;
    println!(
        "disabled: join={:?} model={:?} detailed_is_valid={:?}",
        via_join.is_some(),
        via_model.is_some(),
        matches!(
            detailed,
            burncloud_router::RouterTokenValidationResult::Valid(_)
        )
    );

    assert!(
        via_join.is_none(),
        "the JOIN path refuses a disabled account"
    );
    assert!(
        via_model.is_some(),
        "**recorded: the model path still accepts it** -- a caller using `RouterTokenModel` keeps honouring a \
         credential whose owner has been disabled"
    );
    assert!(
        matches!(
            detailed,
            burncloud_router::RouterTokenValidationResult::Valid(_)
        ),
        "and so does `validate_detailed`, which is the entry point callers are likelier to reach for"
    );

    // The account check is the only difference: the credential itself is still active in both tables.
    let credential_status = temp
        .db
        .fetch_all::<(String,)>("SELECT status FROM router_tokens WHERE token = 'sk-both'")
        .await?;
    println!("the credential's own status: {:?}", credential_status[0].0);
    assert_eq!(
        credential_status[0].0, "active",
        "so the refusal is the account's status and the acceptance is the missing check, not a credential change"
    );

    temp.cleanup().await;
    Ok(())
}

#[tokio::test]
async fn the_model_path_still_honours_expiry_and_credential_status(
) -> Result<(), Box<dyn std::error::Error>> {
    // The boundary of the finding: `RouterTokenModel` **does** check the two things it owns -- the credential's
    // own status and its expiry -- and distinguishes expiry from invalidity. So the gap is specifically the
    // account, not "the model path checks nothing", and a reader should not conclude more than was measured.
    let temp = TempDb::new("model_checks").await?;
    account(&temp.db, "usr_m", 1).await?;

    RouterTokenModel::create(&temp.db, &token("sk-ok", "usr_m", "active", -1)).await?;
    RouterTokenModel::create(&temp.db, &token("sk-exp", "usr_m", "active", now() - 3600)).await?;
    RouterTokenModel::create(&temp.db, &token("sk-off", "usr_m", "disabled", -1)).await?;

    assert!(
        RouterTokenModel::validate(&temp.db, "sk-ok")
            .await?
            .is_some(),
        "an active, unexpired credential validates"
    );
    assert!(
        RouterTokenModel::validate(&temp.db, "sk-exp")
            .await?
            .is_none(),
        "an expired credential does not"
    );
    assert!(
        RouterTokenModel::validate(&temp.db, "sk-off")
            .await?
            .is_none(),
        "a disabled credential does not"
    );

    // Expiry is reported as `Expired` and a disabled credential as `Invalid`, so a caller can tell "renew" from
    // "revoked". That distinction is why `validate_detailed` exists.
    let expired = RouterTokenModel::validate_detailed(&temp.db, "sk-exp").await?;
    let disabled = RouterTokenModel::validate_detailed(&temp.db, "sk-off").await?;
    println!("expired: {expired:?}");
    println!("disabled: {disabled:?}");
    assert!(matches!(
        expired,
        burncloud_router::RouterTokenValidationResult::Expired
    ));
    assert!(matches!(
        disabled,
        burncloud_router::RouterTokenValidationResult::Invalid
    ));

    // Never-expiring sentinels: `-1` and `0` both mean "no expiry", because the check is `> 0 && now > it`.
    RouterTokenModel::create(&temp.db, &token("sk-neg", "usr_m", "active", -1)).await?;
    RouterTokenModel::create(&temp.db, &token("sk-zero", "usr_m", "active", 0)).await?;
    for key in ["sk-neg", "sk-zero"] {
        assert!(
            RouterTokenModel::validate(&temp.db, key).await?.is_some(),
            "{key} must never expire"
        );
    }

    temp.cleanup().await;
    Ok(())
}

// ===========================================================================================
// Logging does not settle spend
// ===========================================================================================

#[tokio::test]
async fn inserting_a_log_does_not_change_any_spend_column() -> Result<(), Box<dyn std::error::Error>>
{
    // "日志写入不扣额度". `insert_log` documents itself as "intentionally side-effect free with respect to
    // credential spend quota" (`lib.rs:233`), and settlement is `deduct_quota`'s job. The assertion is on the
    // **numbers before and after**, not on the documentation: a log writer that also settled would double-charge
    // every request, and the second charge would be invisible in a test that only checked the log was written.
    let temp = TempDb::new("logging").await?;
    account(&temp.db, "usr_log", 1).await?;
    RouterTokenModel::create(&temp.db, &token("sk-log", "usr_log", "active", -1)).await?;

    let before = RouterTokenModel::validate(&temp.db, "sk-log")
        .await?
        .expect("exists");
    println!(
        "before: used_quota={} balance_usd={}",
        before.used_quota,
        temp.db
            .fetch_all::<(i64,)>("SELECT balance_usd FROM user_accounts WHERE id = 'usr_log'")
            .await?[0]
            .0
    );

    // A log with a substantial cost -- the field the router records the real charge in. Built in full because
    // `RouterLog` has no `Default`; the shape is the one `cost_roundtrip.rs` uses.
    let log = RouterLog {
        id: 0,
        request_id: "req-logging".to_string(),
        user_id: Some("usr_log".to_string()),
        path: "/v1/chat/completions".to_string(),
        upstream_id: None,
        status_code: 200,
        latency_ms: 10,
        prompt_tokens: 100,
        completion_tokens: 50,
        cost: 5_000_000_000,
        model: Some("model".to_string()),
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
    };
    RouterDatabase::insert_log(&temp.db, &log).await?;

    let written = temp
        .db
        .fetch_all::<(i64, i64, i64)>(
            "SELECT COALESCE(SUM(cost),0), COUNT(*), COALESCE(SUM(prompt_tokens),0) FROM router_logs",
        )
        .await?;
    println!("log rows written with cost {}", written[0].0);
    assert_eq!(written[0].1, 1, "the log was written");
    assert_eq!(
        written[0].0, 5_000_000_000,
        "and carries the cost it was given"
    );
    assert_eq!(written[0].2, 100, "and the token counts");

    let after = RouterTokenModel::validate(&temp.db, "sk-log")
        .await?
        .expect("exists");
    let balance_after = temp
        .db
        .fetch_all::<(i64,)>("SELECT balance_usd FROM user_accounts WHERE id = 'usr_log'")
        .await?[0]
        .0;

    println!(
        "after: used_quota={} balance_usd={}",
        after.used_quota, balance_after
    );
    assert_eq!(
        after.used_quota, before.used_quota,
        "**writing a log must not change the credential's settled spend**"
    );
    assert_eq!(
        balance_after, 1_000_000_000,
        "nor the user's balance: the charge is `deduct_quota`'s to make, exactly once"
    );

    // The contrast that makes this a test of an interface rather than of a no-op: `deduct_quota` **does** move
    // the same column.
    let settled = RouterTokenModel::deduct_quota(&temp.db, "sk-log", 1_000_000_000).await?;
    println!("deduct_quota -> {settled}");
    let deducted = RouterTokenModel::validate(&temp.db, "sk-log")
        .await?
        .expect("exists");
    assert_eq!(
        deducted.used_quota, 1_000_000_000,
        "settlement moves `used_quota`; logging does not"
    );

    temp.cleanup().await;
    Ok(())
}

// ===========================================================================================
// Isolation between credentials of one user
// ===========================================================================================

#[tokio::test]
async fn two_credentials_for_one_user_do_not_share_a_quota(
) -> Result<(), Box<dyn std::error::Error>> {
    // "同用户多 Token 隔离". Quota lives on the credential, not the account, so settling against one must leave
    // the other untouched. The failure this catches is a settlement that updates by `user_id` -- which would look
    // correct in every single-credential test and would charge the wrong key the moment a user had two.
    let temp = TempDb::new("multi_token").await?;
    account(&temp.db, "usr_multi", 1).await?;

    for key in ["sk-one", "sk-two"] {
        RouterTokenModel::create(&temp.db, &token(key, "usr_multi", "active", -1)).await?;
    }

    RouterTokenModel::deduct_quota(&temp.db, "sk-one", 400_000_000).await?;

    let one = RouterTokenModel::validate(&temp.db, "sk-one")
        .await?
        .expect("exists");
    let two = RouterTokenModel::validate(&temp.db, "sk-two")
        .await?
        .expect("exists");
    println!(
        "after settling sk-one: one={} two={}",
        one.used_quota, two.used_quota
    );

    assert_eq!(
        one.used_quota, 400_000_000,
        "the credential that was charged"
    );
    assert_eq!(
        two.used_quota, 0,
        "**and the other credential of the same user is untouched**"
    );
    assert_eq!(one.user_id, two.user_id, "both belong to the same account");

    // The balance is the account's, so it **is** shared -- which is the other half of the distinction and worth
    // asserting so the two axes are not confused.
    let balance = temp
        .db
        .fetch_all::<(i64,)>("SELECT balance_usd FROM user_accounts WHERE id = 'usr_multi'")
        .await?[0]
        .0;
    println!("account balance after the credential settlement: {balance}");
    assert_eq!(
        balance, 1_000_000_000,
        "settling a credential's quota does not move the account balance -- those are separate ledgers"
    );

    // Disabling one credential leaves the other usable.
    RouterTokenModel::update_status(&temp.db, "sk-one", "disabled").await?;
    assert!(RouterTokenModel::validate(&temp.db, "sk-one")
        .await?
        .is_none());
    assert!(
        RouterTokenModel::validate(&temp.db, "sk-two")
            .await?
            .is_some(),
        "disabling one credential must not disable the account's others"
    );

    temp.cleanup().await;
    Ok(())
}

#[tokio::test]
async fn a_settlement_past_the_limit_is_recorded_and_reported(
) -> Result<(), Box<dyn std::error::Error>> {
    // The plan's "实际费用跨限额后仍按现有结算契约记录" and its warning: **do not read `false` as "nothing was
    // written"**. `deduct_quota` reports `false` once the total passes the limit, and the row still moves -- so a
    // caller that treated `false` as a refused write would lose the charge from its accounting.
    let temp = TempDb::new("over_limit").await?;
    account(&temp.db, "usr_over", 1).await?;

    let mut t = token("sk-over", "usr_over", "active", -1);
    t.quota_limit = 1_000_000_000;
    RouterTokenModel::create(&temp.db, &t).await?;

    // A charge exactly at the limit is accepted.
    let at_limit = RouterTokenModel::deduct_quota(&temp.db, "sk-over", 1_000_000_000).await?;
    let after = RouterTokenModel::validate(&temp.db, "sk-over")
        .await?
        .expect("exists");
    println!("at the limit -> {at_limit}, used={}", after.used_quota);
    assert!(
        at_limit,
        "a charge bringing the total to the limit is settled"
    );
    assert_eq!(after.used_quota, 1_000_000_000);

    // The next one crosses it.
    let over = RouterTokenModel::deduct_quota(&temp.db, "sk-over", 1).await?;
    let after = RouterTokenModel::validate(&temp.db, "sk-over")
        .await?
        .expect("exists");
    println!("one past the limit -> {over}, used={}", after.used_quota);
    assert!(!over, "the caller is told the total has passed the limit");
    assert_eq!(
        after.used_quota, 1_000_000_001,
        "**and the charge is recorded anyway** -- `false` means \"now over\", not \"not written\""
    );

    // `check_quota` is the pre-check, and it agrees once the total is past the limit.
    let can_spend = RouterTokenModel::check_quota(&temp.db, "sk-over", 1).await?;
    println!("check_quota for one more -> {can_spend}");
    assert!(
        !can_spend,
        "the pre-check refuses once the total is past the limit, which is what stops the next request"
    );

    temp.cleanup().await;
    Ok(())
}

// ===========================================================================================
// Video tasks
// ===========================================================================================

#[tokio::test]
async fn a_video_task_mapping_persists_and_ignores_a_repeat(
) -> Result<(), Box<dyn std::error::Error>> {
    // "video task 唯一标识和状态持久化". **The row is a routing mapping, not a status record**: the table is
    // `task_id -> channel_id`, with the model and resolution alongside, so that a later `GET /v1/videos/{id}` can
    // be routed to the channel that created the task. There is no status column and no `create` / `update_status`
    // -- my first version of this test invented both.
    //
    // The probe showed `router_video_tasks` starts empty, so every row here is this test's.
    let temp = TempDb::new("video").await?;

    let first = RouterVideoTask {
        task_id: "task-one".to_string(),
        channel_id: 7,
        user_id: Some("usr_video".to_string()),
        model: Some("video-model".to_string()),
        duration: 5,
        resolution: "1280x720".to_string(),
    };
    let second = RouterVideoTask {
        task_id: "task-two".to_string(),
        channel_id: 9,
        user_id: None,
        model: None,
        duration: 0,
        resolution: "640x480".to_string(),
    };

    RouterVideoTaskModel::save(&temp.db, &first).await?;
    RouterVideoTaskModel::save(&temp.db, &second).await?;

    let read_first = RouterVideoTaskModel::get_by_task_id(&temp.db, "task-one")
        .await?
        .expect("task-one exists");
    println!("task-one: {read_first:?}");
    assert_eq!(
        read_first.channel_id, 7,
        "the mapping points at the channel"
    );
    assert_eq!(read_first.user_id.as_deref(), Some("usr_video"));
    assert_eq!(read_first.model.as_deref(), Some("video-model"));
    assert_eq!(read_first.duration, 5);
    assert_eq!(read_first.resolution, "1280x720");

    // Two tasks are two rows, and reading one does not disturb the other.
    let read_second = RouterVideoTaskModel::get_by_task_id(&temp.db, "task-two")
        .await?
        .expect("task-two exists");
    assert_eq!(read_second.channel_id, 9);
    assert_eq!(read_second.user_id, None, "an absent user stays absent");
    assert_eq!(read_second.model, None);
    assert_ne!(
        read_first.channel_id, read_second.channel_id,
        "the two tasks route to different channels"
    );

    // An unknown task id is `None` rather than an error, which is what a GET for a task that never existed
    // depends on.
    assert!(
        RouterVideoTaskModel::get_by_task_id(&temp.db, "task-absent")
            .await?
            .is_none(),
        "an unknown task reads as None"
    );

    // **A repeat of the same id is ignored rather than overwriting.** `save` uses `INSERT OR IGNORE`
    // (`router_video_task.rs:29`) and `ON CONFLICT DO NOTHING` for PostgreSQL, so the **first** mapping wins.
    //
    // Recorded because it is the behaviour a caller would not predict from the name `save`: a second save of the
    // same task with a different channel leaves the original in place. That is the correct choice for a routing
    // mapping -- a task was created on the channel it was created on -- but a caller updating a mapping would
    // find the update silently discarded.
    RouterVideoTaskModel::save(
        &temp.db,
        &RouterVideoTask {
            task_id: "task-one".to_string(),
            channel_id: 99,
            user_id: Some("usr_someone_else".to_string()),
            model: Some("other-model".to_string()),
            duration: 30,
            resolution: "1920x1080".to_string(),
        },
    )
    .await?;

    let after_repeat = RouterVideoTaskModel::get_by_task_id(&temp.db, "task-one")
        .await?
        .expect("still there");
    println!("after a repeat save with channel 99: {after_repeat:?}");
    assert_eq!(
        after_repeat.channel_id, 7,
        "**the first mapping wins; the repeat is ignored rather than overwriting**"
    );
    assert_eq!(after_repeat.user_id.as_deref(), Some("usr_video"));
    assert_eq!(
        after_repeat.duration, 5,
        "and none of the other columns moved"
    );

    let count = temp
        .db
        .fetch_all::<(i64,)>("SELECT COUNT(*) FROM router_video_tasks")
        .await?[0]
        .0;
    assert_eq!(count, 2, "a repeat must not add a row");

    temp.cleanup().await;
    Ok(())
}
