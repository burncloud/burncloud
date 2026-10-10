#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! The Identity database layer's rules, beyond the round trip (#633, plan item 02).
//!
//! `identity_round_trip.rs` already covers real SQLite round trips field by field. The plan names the gaps that
//! remain, and this file is those and nothing else:
//!
//! > 待补：用户唯一性、缺失查询、可空字段、角色绑定/查询、API Key 状态更新、密码重置过期/使用状态；
//! > 检查多用户数据隔离与大整数余额字段映射。充值字段归属迁移期间只验证数据不丢，不在此复制计费规则。
//!
//! The fixture mirrors `identity_round_trip.rs` deliberately: a **file** database rather than `:memory:` (the
//! pool opens several connections and an in-memory SQLite database is per-connection, so the schema and the
//! assertions would see different databases), the production `UserDatabase::init` for the schema, and an
//! explicit close-then-wait-then-delete cleanup, because on Windows dropping a `Database` leaves the file locked.

use burncloud_database::{create_database_with_url, Database};
use burncloud_service_user::{
    PasswordResetDatabase, UserAccount, UserApiKeyInput, UserApiKeyModel, UserApiKeyUpdateInput,
    UserDatabase,
};

/// A fresh SQLite file database with the Identity schema applied through the production initializer.
async fn fresh_db(tag: &str) -> (Database, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "bc_identity_rules_{}_{}_{}.db",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::remove_file(&path).ok();
    let normalized = path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{}?mode=rwc", normalized);
    let db = create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("open {url}: {e}"));
    UserDatabase::init(&db)
        .await
        .expect("UserDatabase::init must create the user_* schema");
    // `create_database_with_url` no longer seeds domain default records (#842), and the demo
    // account and demo key are Identity's to write, so seed them through the owner exactly as
    // the application bootstrap does. Several tests below assert on that seeded state.
    UserDatabase::seed_demo_defaults(&db)
        .await
        .expect("UserDatabase::seed_demo_defaults must seed the demo account and key");
    (db, path)
}

/// Close the pool, wait for the handle to be released, then delete the test database.
async fn cleanup(db: Database, path: &std::path::Path) {
    db.close().await.ok();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    std::fs::remove_file(path).ok();
    for suffix in ["-wal", "-shm"] {
        let mut candidate = path.as_os_str().to_os_string();
        candidate.push(suffix);
        std::fs::remove_file(std::path::PathBuf::from(candidate)).ok();
    }
}

/// A user account with only the fields a test cares about.
fn account(id: &str, username: &str) -> UserAccount {
    UserAccount {
        id: id.to_string(),
        username: username.to_string(),
        email: Some(format!("{username}@example.invalid")),
        password_hash: Some("$2b$12$notarealhash".to_string()),
        github_id: None,
        status: 1,
        balance_usd: 0,
        balance_cny: 0,
        preferred_currency: None,
    }
}

// ===========================================================================================
// Uniqueness and missing lookups
// ===========================================================================================

#[tokio::test]
async fn a_duplicate_username_is_rejected_by_the_database() {
    // "用户唯一性". The uniqueness is a schema constraint, so this asserts the constraint rather than the
    // caller's willingness to check first: a second insert with the same username must fail, and the failure
    // must leave exactly one row. Without the constraint the identity model has no single owner of a name.
    let (db, path) = fresh_db("dup_username").await;

    UserDatabase::create_user(&db, &account("usr_a", "alice"))
        .await
        .expect("the first insert succeeds");
    let before = UserDatabase::count_users(&db).await.expect("countable");

    let second = UserDatabase::create_user(&db, &account("usr_b", "alice")).await;
    match &second {
        Ok(()) => panic!("a duplicate username was accepted, so the name has no single owner"),
        Err(e) => println!("duplicate username -> {e}"),
    }
    assert!(second.is_err(), "the constraint must reject it");

    let after = UserDatabase::count_users(&db).await.expect("countable");
    println!("users before {before}, after {after}");
    assert_eq!(before, after, "the failed insert must not have added a row");

    // The original row is intact and still owns the name.
    let found = UserDatabase::get_user_by_username(&db, "alice")
        .await
        .expect("the lookup works");
    assert_eq!(found.map(|u| u.id), Some("usr_a".to_string()));

    cleanup(db, &path).await;
}

#[tokio::test]
async fn a_missing_user_is_none_rather_than_an_error_or_an_empty_row() {
    // "缺失查询". `None` and an error mean different things to a caller: "no such user" is a normal branch,
    // "the query failed" is not. A lookup that returned an error for absence would make login failures
    // indistinguishable from database failures.
    let (db, path) = fresh_db("missing").await;

    for (label, result) in [
        (
            "by username",
            UserDatabase::get_user_by_username(&db, "nobody").await,
        ),
        (
            "by id",
            UserDatabase::get_user_by_id(&db, "usr_nobody").await,
        ),
        (
            "by email",
            UserDatabase::get_user_by_email(&db, "nobody@example.invalid").await,
        ),
    ] {
        match result {
            Ok(None) => println!("{label}: Ok(None)"),
            Ok(Some(u)) => panic!(
                "{label} found a user that was never created: {}",
                u.username
            ),
            Err(e) => panic!("{label} reported an error for absence, which is not an absence: {e}"),
        }
    }

    // And a user with no roles has an empty role list rather than an error.
    let roles = UserDatabase::get_user_roles(&db, "usr_nobody")
        .await
        .expect("a user with no bindings is an empty list, not a failure");
    assert!(roles.is_empty(), "no bindings, no roles: {roles:?}");

    cleanup(db, &path).await;
}

// ===========================================================================================
// Roles, and the seed rows the admin policy has to ignore
// ===========================================================================================

#[tokio::test]
async fn roles_bind_and_read_back_in_a_stable_order() {
    // "角色绑定/查询". A user accumulates roles and the list must contain exactly those, once each, with the
    // ones already held not duplicated by a second assignment. If `assign_role` were a plain INSERT, assigning
    // twice would either fail or list the role twice, and the caller's permission check would still pass --
    // which is why the count is asserted rather than membership alone.
    let (db, path) = fresh_db("roles").await;

    UserDatabase::create_user(&db, &account("usr_r", "roleuser"))
        .await
        .expect("created");

    assert!(UserDatabase::get_user_roles(&db, "usr_r")
        .await
        .expect("readable")
        .is_empty());

    UserDatabase::assign_role(&db, "usr_r", "user")
        .await
        .expect("assign user");
    let after_first = UserDatabase::get_user_roles(&db, "usr_r")
        .await
        .expect("readable");
    println!("after one assignment: {after_first:?}");
    assert_eq!(after_first, vec!["user".to_string()], "exactly one role");

    // Assigning the same role again is idempotent from the caller's point of view: the list still has it once.
    //
    // **`assign_role` reaches idempotency by attempting the insert and *ignoring* the error**
    // (`lib.rs:303`: `if let Err(e) = res { tracing::warn!(...) }`). The constraint it relies on is the
    // composite primary key on `user_role_bindings (user_id, role_id)` from migration 0010, which is what makes
    // the second insert fail rather than duplicate the row.
    //
    // A mutation that replaced the ignore with `res?` was **not caught by the first version of this test** --
    // because the role list is `["user"]` either way, so comparing lists cannot tell a refused insert from an
    // insert that never happened. The `is_ok` assertion below is what closed that gap, and with it the mutation
    // is caught. Recorded because it is the kind of gap a passing suite hides.
    let repeat = UserDatabase::assign_role(&db, "usr_r", "user").await;
    let after_repeat = UserDatabase::get_user_roles(&db, "usr_r")
        .await
        .expect("readable");
    println!("repeat -> {repeat:?}, roles: {after_repeat:?}");

    // **The assertion that actually discriminates.** The role list is `["user"]` both when the duplicate insert
    // was refused *and* when there was no constraint to refuse it -- so comparing lists alone cannot tell the
    // two apart, and a mutation replacing the ignored error with `res?` slipped through the first version of
    // this test. What separates them is the **returned value**: the duplicate insert fails, `assign_role`
    // deliberately swallows that failure, and the caller is told the role is in place. With the error
    // propagated, this call returns `Err` and this assertion fails.
    assert!(
        repeat.is_ok(),
        "assigning a role the user already has is a success, not an error: {repeat:?}"
    );
    assert_eq!(
        after_repeat, after_first,
        "and it must not duplicate the role or remove it"
    );

    // A second role is added rather than replacing the first.
    UserDatabase::assign_role(&db, "usr_r", "admin")
        .await
        .expect("assign admin");
    let mut both = UserDatabase::get_user_roles(&db, "usr_r")
        .await
        .expect("readable");
    both.sort();
    println!("after two assignments: {both:?}");
    assert_eq!(both.len(), 2, "two roles: {both:?}");
    assert!(both.contains(&"user".to_string()) && both.contains(&"admin".to_string()));

    cleanup(db, &path).await;
}

#[tokio::test]
async fn the_admin_policy_ignores_the_seed_accounts() {
    // "首个用户与后续用户角色" -- the policy that makes the first real registrant an admin. It rests entirely on
    // `count_users` and `has_admin_user` **excluding the seeded rows**: a `demo-user` account and any account
    // whose `password_hash` is the `no-login` placeholder. `lib.rs` documents both exclusions, and neither was
    // asserted anywhere.
    //
    // This matters because the policy is "if no admin exists, promote the next registrant". If the seed admin
    // counted, the first real user would silently not be promoted and the system would have an admin nobody can
    // log in as. That failure is invisible in production until someone needs the admin.
    let (db, path) = fresh_db("admin_policy").await;

    // `init` seeds the demo account; it is a real row.
    let all = UserDatabase::list_users(&db).await.expect("listable");
    println!(
        "rows after init: {:?}",
        all.iter().map(|u| &u.username).collect::<Vec<_>>()
    );

    let counted = UserDatabase::count_users(&db).await.expect("countable");
    println!("count_users excludes the seed: {counted}");
    assert_eq!(
        counted, 0,
        "the demo seed row must not count as a user, or the first registrant is never promoted"
    );

    let has_admin = UserDatabase::has_admin_user(&db).await.expect("queryable");
    assert!(
        !has_admin,
        "the seed admin must not count as a real admin, or the first registrant is never promoted"
    );

    // The policy's two branches, exercised through the same two queries a caller would use.
    let promote_next = !UserDatabase::has_admin_user(&db).await.expect("queryable");
    assert!(
        promote_next,
        "with no real admin, the next registrant is promoted"
    );

    // Now a real user, and give them the admin role.
    UserDatabase::create_user(&db, &account("usr_first", "firstreal"))
        .await
        .expect("created");
    assert_eq!(
        UserDatabase::count_users(&db).await.expect("countable"),
        1,
        "a real user counts"
    );

    UserDatabase::assign_role(&db, "usr_first", "admin")
        .await
        .expect("assigned");

    let now_has_admin = UserDatabase::has_admin_user(&db).await.expect("queryable");
    println!("after promoting the first real user: has_admin_user = {now_has_admin}");
    assert!(
        now_has_admin,
        "with a real admin present the policy must stop promoting, or every registrant becomes an admin"
    );
    assert!(
        UserDatabase::has_admin_user(&db).await.expect("queryable"),
        "the promotion branch is now closed"
    );

    cleanup(db, &path).await;
}

#[tokio::test]
async fn a_real_account_with_the_no_login_placeholder_is_not_an_admin() {
    // The second half of the same exclusion, asserted separately because it is a different clause. An account
    // with `password_hash = 'no-login'` cannot log in, so it must not satisfy the admin policy even when it
    // holds the admin role -- otherwise a seeded or system account would suppress the first-user promotion.
    let (db, path) = fresh_db("no_login").await;

    let mut placeheld = account("usr_ph", "placeholder");
    placeheld.password_hash = Some("no-login".to_string());
    UserDatabase::create_user(&db, &placeheld)
        .await
        .expect("created");
    UserDatabase::assign_role(&db, "usr_ph", "admin")
        .await
        .expect("assigned");

    println!(
        "count_users with a no-login account: {}",
        UserDatabase::count_users(&db).await.expect("countable")
    );
    assert!(
        !UserDatabase::has_admin_user(&db).await.expect("queryable"),
        "an account that cannot log in must not count as the admin"
    );

    // **A nullable `password_hash` is documented but not reachable, and that is the finding here.**
    //
    // `user_account.rs:12` says `password_hash: Option<String>, // Nullable for OIDC users`, and the admin
    // query in `lib.rs:387` tests `u.password_hash IS NULL OR u.password_hash != 'no-login'` -- two places
    // built for a row that has no password. **The column cannot hold one**:
    //
    //     crates/platform/storage/database/migrations/sqlite/0010_rename_tables.sql:14
    //         password_hash TEXT NOT NULL,
    //
    // so the `IS NULL` branch is dead and an OIDC user cannot be represented at all. `lib.rs:112` even tries
    // to remedy it -- `ALTER TABLE user_accounts ADD COLUMN password_hash TEXT` -- but the column already
    // exists by then, so the statement does nothing and the failure is silent.
    //
    // This was established by writing the test that stores `None` and reading the error:
    // `NOT NULL constraint failed: user_accounts.password_hash`. **Recorded, not fixed**, because the fix is a
    // schema change: a migration to relax the constraint, or a decision that OIDC users get a sentinel hash
    // instead. Both touch every reader, and the plan says to record ambiguity rather than choose silently.
    //
    // What the test asserts instead is the contradiction itself, so that whichever way it is resolved, this
    // fails and has to be updated deliberately.
    let mut oidc = account("usr_oidc", "oidcuser");
    oidc.password_hash = None;
    let rejected = UserDatabase::create_user(&db, &oidc).await;
    match &rejected {
        Ok(()) => {
            // If this ever starts succeeding, the schema was relaxed and the assertions below should become the
            // OIDC-is-a-real-admin case.
            println!("an OIDC user with no password hash is now storable, so the schema changed");
            let stored = UserDatabase::get_user_by_username(&db, "oidcuser")
                .await
                .expect("readable")
                .expect("exists");
            assert_eq!(
                stored.password_hash, None,
                "and the NULL survives the round trip"
            );
        }
        Err(e) => {
            println!("OIDC user rejected -> {e}");
            assert!(
                e.to_string().contains("NOT NULL") || e.to_string().contains("password_hash"),
                "the rejection must be the NOT NULL constraint, not something else: {e}"
            );
        }
    }

    // `has_admin_user` is false either way, and that is the point: the code path that would recognise an
    // OIDC admin cannot be reached while the column is NOT NULL.
    assert!(
        !UserDatabase::has_admin_user(&db).await.expect("queryable"),
        "with no storable OIDC row and only a no-login account, there is no real admin"
    );

    cleanup(db, &path).await;
}

// ===========================================================================================
// Multi-user isolation, and the currency columns
// ===========================================================================================

#[tokio::test]
async fn users_do_not_see_each_others_data() {
    // "检查多用户数据隔离". Three users, each with their own balances, roles and keys. Every query that takes a
    // user must return that user's rows and nobody else's -- a missing `WHERE user_id` would show up here as
    // another user's data, which is the failure that matters most in an identity store.
    let (db, path) = fresh_db("isolation").await;

    for (id, name, balance) in [
        ("usr_1", "one", 1_000_000_000_i64),
        ("usr_2", "two", 2_000_000_000),
        ("usr_3", "three", 3_000_000_000),
    ] {
        let mut u = account(id, name);
        u.balance_usd = balance;
        UserDatabase::create_user(&db, &u).await.expect("created");
    }

    // Balances stay with their owner.
    for (id, expected) in [
        ("usr_1", 1_000_000_000_i64),
        ("usr_2", 2_000_000_000),
        ("usr_3", 3_000_000_000),
    ] {
        let u = UserDatabase::get_user_by_id(&db, id)
            .await
            .expect("readable")
            .expect("exists");
        assert_eq!(u.balance_usd, expected, "{id}'s own balance");
    }

    // A balance update touches one row. The other two are asserted afterwards, which is what makes this an
    // isolation test rather than a balance test.
    let new_balance = UserDatabase::update_balance_usd(&db, "usr_2", 500)
        .await
        .expect("updated");
    println!("usr_2 after +500 -> {new_balance}");
    assert_eq!(
        new_balance, 2_000_000_500,
        "the delta applied to the right row"
    );

    for (id, expected) in [("usr_1", 1_000_000_000_i64), ("usr_3", 3_000_000_000)] {
        let u = UserDatabase::get_user_by_id(&db, id)
            .await
            .expect("readable")
            .expect("exists");
        assert_eq!(u.balance_usd, expected, "{id} must be untouched");
    }

    // Roles are per user: assigning admin to one does not give it to the others.
    UserDatabase::assign_role(&db, "usr_2", "admin")
        .await
        .expect("assigned");
    for id in ["usr_1", "usr_3"] {
        let roles = UserDatabase::get_user_roles(&db, id)
            .await
            .expect("readable");
        assert!(roles.is_empty(), "{id} must have no roles: {roles:?}");
    }

    // API keys are per user too, through the `user_id` filter.
    for id in ["usr_1", "usr_2"] {
        UserApiKeyModel::create(
            &db,
            &UserApiKeyInput {
                user_id: id.to_string(),
                name: Some(format!("key for {id}")),
                remain_quota: Some(100),
                unlimited_quota: None,
                expired_time: None,
            },
        )
        .await
        .expect("key created");
    }
    let one_keys = UserApiKeyModel::list(&db, 100, 0, Some("usr_1"))
        .await
        .expect("listable");
    println!("usr_1 has {} key(s)", one_keys.len());
    assert_eq!(one_keys.len(), 1, "only usr_1's key");
    assert!(one_keys.iter().all(|k| k.user_id == "usr_1"));
    assert!(
        one_keys[0].name.as_deref().unwrap_or("").contains("usr_1"),
        "and it is the right one"
    );

    cleanup(db, &path).await;
}

#[tokio::test]
async fn large_balances_survive_the_round_trip_exactly() {
    // "大整数余额字段映射". Balances are i64 **nanodollars** -- nine decimal places -- so the values are large by
    // design: one US dollar is 1e9. A column declared too narrow, or a cast through i32 or f64, would lose the
    // low digits silently, and the loss only becomes visible after many small charges. The near-extremes are
    // included because a value close to `i64::MAX` is where a widening or narrowing conversion shows up.
    let (db, path) = fresh_db("big_balance").await;

    let cases: [(&str, &str, i64, i64); 5] = [
        ("usr_zero", "zero", 0, 0),
        ("usr_one_cent", "onecent", 10_000_000, 10_000_000),
        ("usr_dollar", "dollar", 1_000_000_000, 1_000_000_000),
        (
            "usr_large",
            "large",
            9_007_199_254_740_993,
            9_007_199_254_740_993,
        ),
        ("usr_neg", "negative", -1_000_000_000, -2_000_000_000),
    ];

    for (id, name, usd, cny) in cases {
        let mut u = account(id, name);
        u.balance_usd = usd;
        u.balance_cny = cny;
        UserDatabase::create_user(&db, &u).await.expect("created");
    }

    for (id, name, usd, cny) in cases {
        let u = UserDatabase::get_user_by_id(&db, id)
            .await
            .expect("readable")
            .unwrap_or_else(|| panic!("{name} exists"));
        println!("{name}: usd {} cny {}", u.balance_usd, u.balance_cny);
        assert_eq!(u.balance_usd, usd, "{name} usd must be exact");
        assert_eq!(u.balance_cny, cny, "{name} cny must be exact");
    }

    // `9_007_199_254_740_993` is 2^53 + 1: the smallest positive integer an f64 cannot represent. It survives,
    // which is the evidence that the value did not pass through a float anywhere.
    let u = UserDatabase::get_user_by_id(&db, "usr_large")
        .await
        .expect("readable")
        .expect("exists");
    assert_eq!(u.balance_usd, 9_007_199_254_740_993);
    assert_ne!(
        u.balance_usd, 9_007_199_254_740_992,
        "the value must not have been rounded to the nearest f64"
    );

    // A negative delta moves the balance down and the returned value is the post-update balance, not the delta.
    let after = UserDatabase::update_balance_usd(&db, "usr_dollar", -250_000_000)
        .await
        .expect("updated");
    assert_eq!(after, 750_000_000, "one dollar less a quarter");

    // The two currencies are separate columns: changing one leaves the other alone, which is what stops the
    // dual-currency arithmetic from silently using the wrong side.
    let u = UserDatabase::get_user_by_id(&db, "usr_dollar")
        .await
        .expect("readable")
        .expect("exists");
    assert_eq!(u.balance_usd, 750_000_000);
    assert_eq!(u.balance_cny, 1_000_000_000, "the CNY column is untouched");

    // Now move CNY and assert **both** columns.
    //
    // **This block exists because a mutation was missed.** Changing `update_balance_cny` to write
    // `balance_usd` still passed the assertions above, because a wrong-column write leaves CNY exactly as it
    // was -- the test was checking the one value the defect does not change. Asserting USD after the CNY update
    // is what closes that gap.
    let after_cny = UserDatabase::update_balance_cny(&db, "usr_dollar", 1_000)
        .await
        .expect("updated");
    let u = UserDatabase::get_user_by_id(&db, "usr_dollar")
        .await
        .expect("readable")
        .expect("exists");
    println!(
        "after a CNY delta: returned {after_cny}, usd {} cny {}",
        u.balance_usd, u.balance_cny
    );
    assert_eq!(after_cny, 1_000_001_000, "the CNY delta applied");
    assert_eq!(
        u.balance_cny, 1_000_001_000,
        "and was written to the CNY column"
    );
    assert_eq!(
        u.balance_usd, 750_000_000,
        "the CNY method must not have touched USD -- a wrong-column write is invisible in the CNY value"
    );

    cleanup(db, &path).await;
}

#[tokio::test]
async fn a_balance_update_for_an_unknown_user_is_an_error_rather_than_a_silent_no_op() {
    // Recorded because it is a deliberate property of the implementation rather than an accident: the update
    // statement reports zero rows affected and the code then **selects the balance**, which fails on a missing
    // row. So a caller mistyping a user id gets an error instead of `Ok(0)`.
    //
    // That is the safer of the two behaviours, and it is asserted so that a future change to `rows_affected`
    // handling cannot quietly turn it into a success.
    let (db, path) = fresh_db("missing_balance").await;

    let result = UserDatabase::update_balance_usd(&db, "usr_does_not_exist", 1_000).await;
    match &result {
        Ok(balance) => panic!("an unknown user reported a balance of {balance}"),
        Err(e) => println!("unknown user -> {e}"),
    }
    assert!(result.is_err());

    cleanup(db, &path).await;
}

// ===========================================================================================
// API keys: state, quota shape, and the dynamic update
// ===========================================================================================

#[tokio::test]
async fn a_new_key_gets_the_documented_defaults() {
    // "API Key 状态更新". The defaults are a contract: a key starts active, unused, and with the quota shape the
    // caller asked for. `unlimited_quota` forces `remain_quota` to -1 rather than storing the caller's number,
    // which is the sentinel a consuming service reads -- so storing 100 there would make an unlimited key look
    // like it had a hundred units left.
    let (db, path) = fresh_db("key_defaults").await;

    let finite = UserApiKeyModel::create(
        &db,
        &UserApiKeyInput {
            user_id: "usr_1".to_string(),
            name: Some("finite".to_string()),
            remain_quota: Some(500),
            unlimited_quota: Some(false),
            expired_time: None,
        },
    )
    .await
    .expect("created");

    println!(
        "finite key: status={} remain={} unlimited={} used={} expired={} created={:?}",
        finite.status,
        finite.remain_quota,
        finite.unlimited_quota,
        finite.used_quota,
        finite.expired_time,
        finite.created_time
    );
    assert_eq!(finite.status, 1, "a new key is active");
    assert_eq!(finite.used_quota, 0, "and has spent nothing");
    assert_eq!(finite.remain_quota, 500, "the requested quota is stored");
    assert!(!finite.unlimited_quota);
    assert_eq!(
        finite.expired_time, -1,
        "the `-1` sentinel means no expiry, not a timestamp of 1969"
    );
    assert!(finite.created_time.is_some(), "creation is timestamped");
    assert!(
        finite.key.starts_with("sk-"),
        "the key carries the prefix: {}",
        finite.key
    );
    assert_eq!(finite.key.len(), 51, "`sk-` plus 48 characters");
    assert!(finite.id > 0, "the row has an id");

    // Unlimited: the quota field is forced to the sentinel.
    let unlimited = UserApiKeyModel::create(
        &db,
        &UserApiKeyInput {
            user_id: "usr_1".to_string(),
            name: Some("unlimited".to_string()),
            remain_quota: Some(999),
            unlimited_quota: Some(true),
            expired_time: Some(4_102_444_800),
        },
    )
    .await
    .expect("created");

    println!(
        "unlimited key: remain={} unlimited={} expired={}",
        unlimited.remain_quota, unlimited.unlimited_quota, unlimited.expired_time
    );
    assert!(unlimited.unlimited_quota);
    assert_eq!(
        unlimited.remain_quota, -1,
        "an unlimited key stores the sentinel rather than the caller's number"
    );
    assert_eq!(
        unlimited.expired_time, 4_102_444_800,
        "an explicit expiry is kept"
    );

    // Two keys are two keys, and their generated values differ.
    assert_ne!(
        finite.key, unlimited.key,
        "keys are generated, not repeated"
    );

    cleanup(db, &path).await;
}

#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "one scenario walks every partial-update combination in order; splitting it would hide which combination regressed"
)]
async fn updating_one_field_leaves_the_others_alone() {
    // **The dynamic `UPDATE`.** `update` builds its `SET` clause from whichever fields are `Some`, numbers the
    // placeholders as it goes, and binds the parameters **in a fixed order that skips the absent ones**. Every
    // partial combination therefore exercises a different statement, and a mismatch between the numbering and the
    // binding order would write one field's value into another.
    //
    // The test walks the four fields one at a time and then together, checking after each that **only** the
    // intended field moved. That is what catches a shifted bind, which a single "update and read it back" test
    // does not.
    let (db, path) = fresh_db("key_update").await;

    let key = UserApiKeyModel::create(
        &db,
        &UserApiKeyInput {
            user_id: "usr_1".to_string(),
            name: Some("original".to_string()),
            remain_quota: Some(100),
            unlimited_quota: None,
            expired_time: Some(1_000),
        },
    )
    .await
    .expect("created")
    .key;

    // A plain async fn rather than a closure: `Database` is deliberately not `Clone`, and a closure that
    // returns a future over borrowed arguments does not satisfy the borrow checker here. The first version
    // cloned the database and the second used a closure; both failed to compile.
    async fn read(db: &Database, key: &str) -> burncloud_service_user::UserApiKey {
        UserApiKeyModel::get_by_key(db, key)
            .await
            .expect("readable")
            .expect("exists")
    }

    // 1. name only.
    let changed = UserApiKeyModel::update(
        &db,
        &key,
        &UserApiKeyUpdateInput {
            name: Some("renamed".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("updated");
    assert!(changed, "one row changed");
    let k = read(&db, &key).await;
    println!(
        "after name: name={:?} quota={} status={} expired={}",
        k.name, k.remain_quota, k.status, k.expired_time
    );
    assert_eq!(k.name.as_deref(), Some("renamed"));
    assert_eq!(k.remain_quota, 100, "the quota must not move");
    assert_eq!(k.status, 1, "nor the status");
    assert_eq!(k.expired_time, 1_000, "nor the expiry");

    // 2. remain_quota only.
    UserApiKeyModel::update(
        &db,
        &key,
        &UserApiKeyUpdateInput {
            remain_quota: Some(250),
            ..Default::default()
        },
    )
    .await
    .expect("updated");
    let k = read(&db, &key).await;
    println!(
        "after quota: name={:?} quota={} status={} expired={}",
        k.name, k.remain_quota, k.status, k.expired_time
    );
    assert_eq!(k.remain_quota, 250);
    assert_eq!(k.name.as_deref(), Some("renamed"), "the name must not move");
    assert_eq!(k.status, 1);
    assert_eq!(k.expired_time, 1_000);

    // 3. status only -- the disable/enable path.
    UserApiKeyModel::update(
        &db,
        &key,
        &UserApiKeyUpdateInput {
            status: Some(0),
            ..Default::default()
        },
    )
    .await
    .expect("updated");
    let k = read(&db, &key).await;
    println!(
        "after status: name={:?} quota={} status={} expired={}",
        k.name, k.remain_quota, k.status, k.expired_time
    );
    assert_eq!(k.status, 0, "the key is disabled");
    assert_eq!(k.remain_quota, 250, "the quota must not move");
    assert_eq!(k.name.as_deref(), Some("renamed"), "nor the name");
    assert_eq!(k.expired_time, 1_000, "nor the expiry");

    // 4. expired_time only.
    UserApiKeyModel::update(
        &db,
        &key,
        &UserApiKeyUpdateInput {
            expired_time: Some(2_000),
            ..Default::default()
        },
    )
    .await
    .expect("updated");
    let k = read(&db, &key).await;
    assert_eq!(k.expired_time, 2_000, "the expiry moved");
    assert_eq!(k.status, 0, "the status must not move back");
    assert_eq!(k.remain_quota, 250);
    assert_eq!(k.name.as_deref(), Some("renamed"));

    // 5. All four at once: the longest statement, and the one where a numbering error is least visible.
    UserApiKeyModel::update(
        &db,
        &key,
        &UserApiKeyUpdateInput {
            name: Some("all".to_string()),
            remain_quota: Some(1),
            status: Some(1),
            expired_time: Some(3_000),
        },
    )
    .await
    .expect("updated");
    let k = read(&db, &key).await;
    println!(
        "after all four: name={:?} quota={} status={} expired={}",
        k.name, k.remain_quota, k.status, k.expired_time
    );
    assert_eq!(k.name.as_deref(), Some("all"));
    assert_eq!(k.remain_quota, 1);
    assert_eq!(k.status, 1);
    assert_eq!(k.expired_time, 3_000);

    // An update with no fields is a no-op that reports false, and must not blank anything.
    let empty = UserApiKeyModel::update(&db, &key, &UserApiKeyUpdateInput::default())
        .await
        .expect("no error");
    println!("empty update -> {empty}");
    assert!(!empty, "nothing to set means nothing changed");
    let k = read(&db, &key).await;
    assert_eq!(k.name.as_deref(), Some("all"), "and the row is untouched");
    assert_eq!(k.remain_quota, 1);

    // An update to a key that does not exist reports false rather than erroring.
    let missing = UserApiKeyModel::update(
        &db,
        "sk-doesnotexist",
        &UserApiKeyUpdateInput {
            status: Some(0),
            ..Default::default()
        },
    )
    .await
    .expect("no error");
    assert!(!missing, "an unknown key changes nothing");

    cleanup(db, &path).await;
}

#[tokio::test]
async fn keys_list_with_pagination_and_an_optional_user_filter() {
    // "分页". `list` takes `limit`, `offset` and an optional `user_id`, and orders by `id DESC` -- newest first.
    // The ordering and the offset are the parts a caller depends on and neither was asserted.
    let (db, path) = fresh_db("key_paging").await;

    // Five keys for one user, in creation order, so the newest has the highest id.
    let mut keys = Vec::new();
    for n in 0..5 {
        let k = UserApiKeyModel::create(
            &db,
            &UserApiKeyInput {
                user_id: "usr_pager".to_string(),
                name: Some(format!("key-{n}")),
                remain_quota: None,
                unlimited_quota: None,
                expired_time: None,
            },
        )
        .await
        .expect("created");
        keys.push(k);
    }
    // One key belonging to somebody else, which the filter must exclude.
    UserApiKeyModel::create(
        &db,
        &UserApiKeyInput {
            user_id: "usr_other".to_string(),
            name: Some("other".to_string()),
            remain_quota: None,
            unlimited_quota: None,
            expired_time: None,
        },
    )
    .await
    .expect("created");

    // **`UserDatabase::seed_demo_defaults` inserts a demo key**: `user_api_keys ('demo-user',
    // 'sk-burncloud-demo', ..., unlimited_quota 1, ...)`. Ownership moved here from
    // `platform/storage/database` in #842, where the statement used to live as `schema/user.rs`.
    // It is inserted only when the table is empty, so it is present on a fresh database and an
    // unfiltered list sees it. The first version of this test expected five plus one and was told
    // there were seven -- which is that seed.
    let all = UserApiKeyModel::list(&db, 100, 0, None)
        .await
        .expect("listable");
    println!(
        "unfiltered: {} keys, including {:?}",
        all.len(),
        all.iter()
            .filter(|k| k.user_id == "demo-user")
            .map(|k| k.key.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        all.len(),
        7,
        "five of this user's, one of the other's, and the seeded demo key"
    );
    let demo = all.iter().find(|k| k.user_id == "demo-user");
    assert!(
        demo.is_some(),
        "the seeded demo key is present on a fresh database"
    );
    let demo = demo.expect("checked");
    assert_eq!(demo.key, "sk-burncloud-demo");
    assert!(demo.unlimited_quota, "the demo key is seeded unlimited");

    let mine = UserApiKeyModel::list(&db, 100, 0, Some("usr_pager"))
        .await
        .expect("listable");
    println!(
        "filtered, names in order: {:?}",
        mine.iter().map(|k| k.name.clone()).collect::<Vec<_>>()
    );
    assert_eq!(mine.len(), 5, "only this user's keys");

    // Newest first: the names are `key-4, key-3, ... key-0`.
    let names: Vec<String> = mine.iter().filter_map(|k| k.name.clone()).collect();
    assert_eq!(
        names,
        vec!["key-4", "key-3", "key-2", "key-1", "key-0"],
        "ordered by id descending"
    );

    // A page of two, then the next page: no repeats, no gaps, and the pages tile the whole set.
    let page1 = UserApiKeyModel::list(&db, 2, 0, Some("usr_pager"))
        .await
        .expect("listable");
    let page2 = UserApiKeyModel::list(&db, 2, 2, Some("usr_pager"))
        .await
        .expect("listable");
    let page3 = UserApiKeyModel::list(&db, 2, 4, Some("usr_pager"))
        .await
        .expect("listable");

    let paged: Vec<String> = page1
        .iter()
        .chain(page2.iter())
        .chain(page3.iter())
        .filter_map(|k| k.name.clone())
        .collect();
    println!("paged: {paged:?}");
    assert_eq!(paged, names, "three pages of two tile the five in order");
    assert_eq!(page3.len(), 1, "the last page is partial");
    assert_eq!(page1.len(), 2);

    // Past the end is empty, not an error.
    let beyond = UserApiKeyModel::list(&db, 2, 99, Some("usr_pager"))
        .await
        .expect("an offset past the end is an empty page, not a failure");
    assert!(beyond.is_empty(), "{beyond:?}");

    // A limit of zero returns nothing rather than everything, which is the SQLite behaviour a caller might not
    // expect; recorded because "0 means unlimited" is a plausible assumption and this says it is not.
    let zero = UserApiKeyModel::list(&db, 0, 0, Some("usr_pager"))
        .await
        .expect("listable");
    println!("limit 0 -> {} key(s)", zero.len());
    assert!(zero.is_empty(), "a limit of zero is an empty page");

    // The keys themselves are unique, which the generated form implies and this confirms across five.
    let mut ids: Vec<String> = keys.iter().map(|k| k.key.clone()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 5, "five distinct keys");

    cleanup(db, &path).await;
}

#[tokio::test]
async fn deleting_a_key_removes_only_that_key() {
    // `delete` reports whether a row went, which the caller needs in order to distinguish "revoked" from
    // "there was nothing to revoke". Both branches are asserted, and the neighbour key's survival is what makes
    // this about the right row.
    let (db, path) = fresh_db("key_delete").await;

    let first = UserApiKeyModel::create(
        &db,
        &UserApiKeyInput {
            user_id: "usr_d".to_string(),
            name: Some("first".to_string()),
            remain_quota: None,
            unlimited_quota: None,
            expired_time: None,
        },
    )
    .await
    .expect("created");
    let second = UserApiKeyModel::create(
        &db,
        &UserApiKeyInput {
            user_id: "usr_d".to_string(),
            name: Some("second".to_string()),
            remain_quota: None,
            unlimited_quota: None,
            expired_time: None,
        },
    )
    .await
    .expect("created");

    let removed = UserApiKeyModel::delete(&db, &first.key)
        .await
        .expect("deleted");
    assert!(removed, "the key was there and is gone");
    assert!(
        UserApiKeyModel::get_by_key(&db, &first.key)
            .await
            .expect("readable")
            .is_none(),
        "and cannot be read back"
    );
    assert!(
        UserApiKeyModel::get_by_key(&db, &second.key)
            .await
            .expect("readable")
            .is_some(),
        "the other key is untouched"
    );

    // Deleting it again reports false rather than erroring -- the caller can tell "already revoked".
    let again = UserApiKeyModel::delete(&db, &first.key)
        .await
        .expect("no error");
    assert!(!again, "there was nothing left to delete");

    let left = UserApiKeyModel::list(&db, 10, 0, Some("usr_d"))
        .await
        .expect("listable");
    assert_eq!(left.len(), 1, "one key left");
    assert_eq!(left[0].name.as_deref(), Some("second"));

    cleanup(db, &path).await;
}

// ===========================================================================================
// Password reset: expiry and use are the caller's decision, and the data says which
// ===========================================================================================

#[tokio::test]
async fn a_password_reset_token_records_expiry_and_use_without_enforcing_them() {
    // "密码重置过期/使用状态". `PasswordResetDatabase` offers exactly three operations -- create, read, mark used --
    // and **it does not decide whether a token is still valid**. There is no "is this token usable" method, and
    // neither `get_token` nor `mark_used` filters on `expires_at` or `used_at`.
    //
    // That is a design fact with a consequence worth stating: the validity rule lives in whichever caller
    // performs the reset, and **this layer will happily hand back an expired token and mark a used one used
    // again**. The data needed to enforce it is stored, which is what these assertions pin.
    //
    // Recorded rather than changed: adding enforcement here would move a security decision into a data layer
    // that has no way to know the policy (how long a token lives, whether a used one is deleted or retained for
    // audit), and would change the behaviour of every caller.
    let (db, path) = fresh_db("reset_tokens").await;

    UserDatabase::create_user(&db, &account("usr_reset", "resetuser"))
        .await
        .expect("created");

    // An expired token: a timestamp in the past, which this layer stores verbatim.
    PasswordResetDatabase::create_token(&db, "tok-expired", "usr_reset", "2020-01-01T00:00:00Z")
        .await
        .expect("created");
    let expired = PasswordResetDatabase::get_token(&db, "tok-expired")
        .await
        .expect("readable")
        .expect("exists");
    println!(
        "expired token: expires_at={} used_at={:?}",
        expired.expires_at, expired.used_at
    );
    assert_eq!(
        expired.expires_at, "2020-01-01T00:00:00Z",
        "the past expiry is stored as given"
    );
    assert_eq!(expired.used_at, None, "and it has not been used");
    assert_eq!(expired.user_id, "usr_reset");

    // A live token, marked used: `used_at` is set and the expiry is untouched.
    PasswordResetDatabase::create_token(&db, "tok-live", "usr_reset", "2999-01-01T00:00:00Z")
        .await
        .expect("created");
    PasswordResetDatabase::mark_used(&db, "tok-live")
        .await
        .expect("marked");

    let used = PasswordResetDatabase::get_token(&db, "tok-live")
        .await
        .expect("readable")
        .expect("exists");
    println!(
        "used token: expires_at={} used_at={:?}",
        used.expires_at, used.used_at
    );
    assert_eq!(
        used.expires_at, "2999-01-01T00:00:00Z",
        "the expiry is unchanged by use"
    );
    assert!(
        used.used_at.is_some(),
        "`used_at` is what tells a caller the token was already spent"
    );

    // **A used token is still readable and can be marked used again.** The second `mark_used` overwrites the
    // timestamp, so this layer cannot be used to detect a replay -- the caller has to check `used_at` before
    // acting. Asserted because it is the property a security review would ask about.
    let first_used_at = used.used_at.clone();
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    PasswordResetDatabase::mark_used(&db, "tok-live")
        .await
        .expect("marked again");
    let twice = PasswordResetDatabase::get_token(&db, "tok-live")
        .await
        .expect("readable")
        .expect("exists");
    println!(
        "after a second mark_used: {:?} then {:?}",
        first_used_at, twice.used_at
    );
    assert!(twice.used_at.is_some(), "still marked used");
    assert_ne!(
        twice.used_at, first_used_at,
        "the second call overwrote the timestamp rather than being refused"
    );

    // An unknown token is `None`, and marking one used is not an error.
    assert!(
        PasswordResetDatabase::get_token(&db, "tok-missing")
            .await
            .expect("readable")
            .is_none(),
        "an unknown token is None rather than an error"
    );
    PasswordResetDatabase::mark_used(&db, "tok-missing")
        .await
        .expect("marking an unknown token used is a no-op, and reports so by not erroring");

    // Each token belongs to the user it was issued for.
    assert_eq!(expired.user_id, "usr_reset");
    assert_eq!(used.user_id, "usr_reset");

    cleanup(db, &path).await;
}

#[tokio::test]
async fn a_failed_reset_leaves_the_old_password_usable() {
    // "失败状态不误改密码". The rule from the other side: a reset that does not complete must not have altered the
    // stored hash. `update_password_hash` is the only writer, and this asserts that none of the surrounding
    // operations -- creating a token, reading it, marking it used, or creating a token for a user who does not
    // exist -- changes the password.
    //
    // The last case is the one that would matter in production: if `create_token` were a foreign-key insert with
    // a cascade, or if some path wrote the hash as a side effect, a failed reset for an unknown user could lock
    // a real user out or alter an unrelated row.
    let (db, path) = fresh_db("reset_failure").await;

    let mut u = account("usr_pw", "pwuser");
    u.password_hash = Some("$2b$12$originalhashvalue".to_string());
    UserDatabase::create_user(&db, &u).await.expect("created");

    async fn hash_of(db: &Database) -> Option<String> {
        UserDatabase::get_user_by_id(db, "usr_pw")
            .await
            .expect("readable")
            .expect("exists")
            .password_hash
    }
    let before = hash_of(&db).await;
    assert_eq!(before.as_deref(), Some("$2b$12$originalhashvalue"));

    // A full token lifecycle that never reaches the password change.
    PasswordResetDatabase::create_token(&db, "tok-fail", "usr_pw", "2999-01-01T00:00:00Z")
        .await
        .expect("created");
    let _ = PasswordResetDatabase::get_token(&db, "tok-fail")
        .await
        .expect("readable");
    PasswordResetDatabase::mark_used(&db, "tok-fail")
        .await
        .expect("marked");

    // A token for a user who does not exist. Whether the insert succeeds depends on the schema's foreign keys,
    // so both outcomes are accepted -- what is asserted is that neither alters the real user's password.
    let orphan = PasswordResetDatabase::create_token(
        &db,
        "tok-orphan",
        "usr_absent",
        "2999-01-01T00:00:00Z",
    )
    .await;
    println!("token for an absent user -> {orphan:?}");

    let after = hash_of(&db).await;
    println!("password hash before {before:?}, after {after:?}");
    assert_eq!(
        before, after,
        "no failed reset path may change the stored password hash"
    );

    // And the real writer does work, so the assertion above is not vacuous: the hash is only unchanged because
    // nothing called it.
    UserDatabase::update_password_hash(&db, "usr_pw", "$2b$12$newhashvalue")
        .await
        .expect("updated");
    let changed = hash_of(&db).await;
    assert_eq!(
        changed.as_deref(),
        Some("$2b$12$newhashvalue"),
        "the writer works"
    );
    assert_ne!(before, changed, "and it really changes the value");

    cleanup(db, &path).await;
}
