#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test-only assertions and fixture setup intentionally use unwrap/expect"
)]
//! Identity database rules beyond the basic round-trip tests.

use burncloud_database::{create_database_with_url, Database};
use burncloud_database_user::{
    PasswordResetDatabase, UserAccount, UserApiKey, UserApiKeyInput, UserApiKeyModel,
    UserApiKeyUpdateInput, UserDatabase,
};

fn remove_file_if_present(path: &std::path::Path) {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("failed to remove test file {}: {error}", path.display()),
    }
}

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
    remove_file_if_present(&path);
    let normalized = path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{}?mode=rwc", normalized);
    let db = create_database_with_url(&url)
        .await
        .unwrap_or_else(|error| panic!("open {url}: {error}"));
    UserDatabase::init(&db)
        .await
        .expect("UserDatabase::init must create the user_* schema");
    (db, path)
}

async fn cleanup(db: Database, path: &std::path::Path) {
    db.close().await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    remove_file_if_present(path);
    for suffix in ["-wal", "-shm"] {
        let mut candidate = path.as_os_str().to_os_string();
        candidate.push(suffix);
        remove_file_if_present(&std::path::PathBuf::from(candidate));
    }
}

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

async fn read_api_key(db: &Database, key: &str) -> UserApiKey {
    UserApiKeyModel::get_by_key(db, key)
        .await
        .expect("key query succeeds")
        .expect("key exists")
}

async fn exercise_single_field_key_updates(db: &Database, key: &str) {
    let changed = UserApiKeyModel::update(
        db,
        key,
        &UserApiKeyUpdateInput {
            name: Some("renamed".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("name update succeeds");
    assert!(changed, "one row changed");
    let current = read_api_key(db, key).await;
    assert_eq!(current.name.as_deref(), Some("renamed"));
    assert_eq!(current.remain_quota, 100);
    assert_eq!(current.status, 1);
    assert_eq!(current.expired_time, 1_000);

    UserApiKeyModel::update(
        db,
        key,
        &UserApiKeyUpdateInput {
            remain_quota: Some(250),
            ..Default::default()
        },
    )
    .await
    .expect("quota update succeeds");
    let current = read_api_key(db, key).await;
    assert_eq!(current.remain_quota, 250);
    assert_eq!(current.name.as_deref(), Some("renamed"));
    assert_eq!(current.status, 1);
    assert_eq!(current.expired_time, 1_000);

    UserApiKeyModel::update(
        db,
        key,
        &UserApiKeyUpdateInput {
            status: Some(0),
            ..Default::default()
        },
    )
    .await
    .expect("status update succeeds");
    let current = read_api_key(db, key).await;
    assert_eq!(current.status, 0);
    assert_eq!(current.remain_quota, 250);
    assert_eq!(current.name.as_deref(), Some("renamed"));
    assert_eq!(current.expired_time, 1_000);

    UserApiKeyModel::update(
        db,
        key,
        &UserApiKeyUpdateInput {
            expired_time: Some(2_000),
            ..Default::default()
        },
    )
    .await
    .expect("expiry update succeeds");
    let current = read_api_key(db, key).await;
    assert_eq!(current.expired_time, 2_000);
    assert_eq!(current.status, 0);
    assert_eq!(current.remain_quota, 250);
    assert_eq!(current.name.as_deref(), Some("renamed"));
}

#[tokio::test]
async fn a_duplicate_username_is_rejected_by_the_database() {
    let (db, path) = fresh_db("dup_username").await;
    UserDatabase::create_user(&db, &account("usr_a", "alice"))
        .await
        .expect("first insert succeeds");
    let before = UserDatabase::count_users(&db).await.expect("countable");

    let second = UserDatabase::create_user(&db, &account("usr_b", "alice")).await;
    assert!(second.is_err(), "the unique constraint must reject a duplicate username");
    let after = UserDatabase::count_users(&db).await.expect("countable");
    assert_eq!(before, after, "the failed insert must not add a row");
    let found = UserDatabase::get_user_by_username(&db, "alice")
        .await
        .expect("lookup succeeds");
    assert_eq!(found.map(|user| user.id), Some("usr_a".to_string()));
    cleanup(db, &path).await;
}

#[tokio::test]
async fn a_missing_user_is_none_rather_than_an_error_or_an_empty_row() {
    let (db, path) = fresh_db("missing").await;
    for result in [
        UserDatabase::get_user_by_username(&db, "nobody").await,
        UserDatabase::get_user_by_id(&db, "usr_nobody").await,
        UserDatabase::get_user_by_email(&db, "nobody@example.invalid").await,
    ] {
        assert!(matches!(result, Ok(None)), "absence must be Ok(None)");
    }
    let roles = UserDatabase::get_user_roles(&db, "usr_nobody")
        .await
        .expect("role query succeeds");
    assert!(roles.is_empty());
    cleanup(db, &path).await;
}

#[tokio::test]
async fn roles_bind_and_read_back_in_a_stable_order() {
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
    let first = UserDatabase::get_user_roles(&db, "usr_r")
        .await
        .expect("readable");
    assert_eq!(first, vec!["user".to_string()]);

    let repeat = UserDatabase::assign_role(&db, "usr_r", "user").await;
    assert!(repeat.is_ok(), "re-assigning the same role is idempotent");
    let repeated = UserDatabase::get_user_roles(&db, "usr_r")
        .await
        .expect("readable");
    assert_eq!(repeated, first, "the role is not duplicated");

    UserDatabase::assign_role(&db, "usr_r", "admin")
        .await
        .expect("assign admin");
    let mut both = UserDatabase::get_user_roles(&db, "usr_r")
        .await
        .expect("readable");
    both.sort();
    assert_eq!(both.len(), 2);
    assert!(both.contains(&"admin".to_string()));
    assert!(both.contains(&"user".to_string()));
    cleanup(db, &path).await;
}

#[tokio::test]
async fn the_admin_policy_ignores_the_seed_accounts() {
    let (db, path) = fresh_db("admin_policy").await;
    assert_eq!(UserDatabase::count_users(&db).await.expect("countable"), 0);
    assert!(!UserDatabase::has_admin_user(&db).await.expect("queryable"));

    UserDatabase::create_user(&db, &account("usr_first", "firstreal"))
        .await
        .expect("created");
    assert_eq!(UserDatabase::count_users(&db).await.expect("countable"), 1);
    UserDatabase::assign_role(&db, "usr_first", "admin")
        .await
        .expect("assigned");
    assert!(UserDatabase::has_admin_user(&db).await.expect("queryable"));
    cleanup(db, &path).await;
}

#[tokio::test]
async fn a_real_account_with_the_no_login_placeholder_is_not_an_admin() {
    let (db, path) = fresh_db("no_login").await;
    let mut placeholder = account("usr_ph", "placeholder");
    placeholder.password_hash = Some("no-login".to_string());
    UserDatabase::create_user(&db, &placeholder)
        .await
        .expect("created");
    UserDatabase::assign_role(&db, "usr_ph", "admin")
        .await
        .expect("assigned");
    assert!(!UserDatabase::has_admin_user(&db).await.expect("queryable"));

    let mut oidc = account("usr_oidc", "oidcuser");
    oidc.password_hash = None;
    let stored = UserDatabase::create_user(&db, &oidc).await;
    match stored {
        Ok(()) => {
            let oidc = UserDatabase::get_user_by_username(&db, "oidcuser")
                .await
                .expect("readable")
                .expect("exists");
            assert_eq!(oidc.password_hash, None);
        }
        Err(error) => assert!(
            error.to_string().contains("NOT NULL") || error.to_string().contains("password_hash"),
            "unexpected OIDC insert failure: {error}"
        ),
    }
    assert!(!UserDatabase::has_admin_user(&db).await.expect("queryable"));
    cleanup(db, &path).await;
}

#[tokio::test]
async fn users_do_not_see_each_others_data() {
    let (db, path) = fresh_db("isolation").await;
    for (id, name, balance) in [
        ("usr_1", "one", 1_000_000_000_i64),
        ("usr_2", "two", 2_000_000_000),
        ("usr_3", "three", 3_000_000_000),
    ] {
        let mut user = account(id, name);
        user.balance_usd = balance;
        UserDatabase::create_user(&db, &user).await.expect("created");
    }

    for (id, expected) in [
        ("usr_1", 1_000_000_000_i64),
        ("usr_2", 2_000_000_000),
        ("usr_3", 3_000_000_000),
    ] {
        let user = UserDatabase::get_user_by_id(&db, id)
            .await
            .expect("readable")
            .expect("exists");
        assert_eq!(user.balance_usd, expected);
    }

    assert_eq!(
        UserDatabase::update_balance_usd(&db, "usr_2", 500)
            .await
            .expect("updated"),
        2_000_000_500
    );
    for (id, expected) in [("usr_1", 1_000_000_000_i64), ("usr_3", 3_000_000_000)] {
        let user = UserDatabase::get_user_by_id(&db, id)
            .await
            .expect("readable")
            .expect("exists");
        assert_eq!(user.balance_usd, expected);
    }

    UserDatabase::assign_role(&db, "usr_2", "admin")
        .await
        .expect("assigned");
    for id in ["usr_1", "usr_3"] {
        assert!(UserDatabase::get_user_roles(&db, id)
            .await
            .expect("readable")
            .is_empty());
    }

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
    assert_eq!(one_keys.len(), 1);
    assert!(one_keys.iter().all(|key| key.user_id == "usr_1"));
    cleanup(db, &path).await;
}

#[tokio::test]
async fn large_balances_survive_the_round_trip_exactly() {
    let (db, path) = fresh_db("big_balance").await;
    let cases = [
        ("usr_zero", "zero", 0, 0),
        ("usr_one_cent", "onecent", 10_000_000, 10_000_000),
        ("usr_dollar", "dollar", 1_000_000_000, 1_000_000_000),
        ("usr_large", "large", 9_007_199_254_740_993, 9_007_199_254_740_993),
        ("usr_neg", "negative", -1_000_000_000, -2_000_000_000),
    ];
    for (id, name, usd, cny) in cases {
        let mut user = account(id, name);
        user.balance_usd = usd;
        user.balance_cny = cny;
        UserDatabase::create_user(&db, &user).await.expect("created");
    }
    for (id, _, usd, cny) in cases {
        let user = UserDatabase::get_user_by_id(&db, id)
            .await
            .expect("readable")
            .expect("exists");
        assert_eq!(user.balance_usd, usd);
        assert_eq!(user.balance_cny, cny);
    }

    let large = UserDatabase::get_user_by_id(&db, "usr_large")
        .await
        .expect("readable")
        .expect("exists");
    assert_eq!(large.balance_usd, 9_007_199_254_740_993);
    assert_ne!(large.balance_usd, 9_007_199_254_740_992);

    assert_eq!(
        UserDatabase::update_balance_usd(&db, "usr_dollar", -250_000_000)
            .await
            .expect("updated"),
        750_000_000
    );
    let after_cny = UserDatabase::update_balance_cny(&db, "usr_dollar", 1_000)
        .await
        .expect("updated");
    let user = UserDatabase::get_user_by_id(&db, "usr_dollar")
        .await
        .expect("readable")
        .expect("exists");
    assert_eq!(after_cny, 1_000_001_000);
    assert_eq!(user.balance_cny, 1_000_001_000);
    assert_eq!(user.balance_usd, 750_000_000);
    cleanup(db, &path).await;
}

#[tokio::test]
async fn a_balance_update_for_an_unknown_user_is_an_error_rather_than_a_silent_no_op() {
    let (db, path) = fresh_db("missing_balance").await;
    let result = UserDatabase::update_balance_usd(&db, "usr_does_not_exist", 1_000).await;
    assert!(result.is_err());
    cleanup(db, &path).await;
}

#[tokio::test]
async fn a_new_key_gets_the_documented_defaults() {
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
    assert_eq!(finite.status, 1);
    assert_eq!(finite.used_quota, 0);
    assert_eq!(finite.remain_quota, 500);
    assert!(!finite.unlimited_quota);
    assert_eq!(finite.expired_time, -1);
    assert!(finite.created_time.is_some());
    assert!(finite.key.starts_with("sk-"));
    assert_eq!(finite.key.len(), 51);
    assert!(finite.id > 0, "the row has an id");

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
    assert!(unlimited.unlimited_quota);
    assert_eq!(unlimited.remain_quota, -1);
    assert_eq!(unlimited.expired_time, 4_102_444_800);
    assert_ne!(finite.key, unlimited.key);
    cleanup(db, &path).await;
}

#[tokio::test]
async fn updating_one_field_leaves_the_others_alone() {
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

    exercise_single_field_key_updates(&db, &key).await;
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
    .expect("combined update succeeds");
    let current = read_api_key(&db, &key).await;
    assert_eq!(current.name.as_deref(), Some("all"));
    assert_eq!(current.remain_quota, 1);
    assert_eq!(current.status, 1);
    assert_eq!(current.expired_time, 3_000);

    let empty = UserApiKeyModel::update(&db, &key, &UserApiKeyUpdateInput::default())
        .await
        .expect("empty update is not an error");
    assert!(!empty);
    let current = read_api_key(&db, &key).await;
    assert_eq!(current.name.as_deref(), Some("all"));
    assert_eq!(current.remain_quota, 1);

    let missing = UserApiKeyModel::update(
        &db,
        "sk-doesnotexist",
        &UserApiKeyUpdateInput {
            status: Some(0),
            ..Default::default()
        },
    )
    .await
    .expect("missing update is not an error");
    assert!(!missing);
    cleanup(db, &path).await;
}

#[tokio::test]
async fn keys_list_with_pagination_and_an_optional_user_filter() {
    let (db, path) = fresh_db("key_paging").await;
    let mut keys = Vec::new();
    for index in 0..5 {
        keys.push(
            UserApiKeyModel::create(
                &db,
                &UserApiKeyInput {
                    user_id: "usr_pager".to_string(),
                    name: Some(format!("key-{index}")),
                    remain_quota: None,
                    unlimited_quota: None,
                    expired_time: None,
                },
            )
            .await
            .expect("created"),
        );
    }
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

    let all = UserApiKeyModel::list(&db, 100, 0, None)
        .await
        .expect("listable");
    assert_eq!(all.len(), 7);
    let demo = all
        .iter()
        .find(|key| key.user_id == "demo-user")
        .expect("seeded demo key exists");
    assert_eq!(demo.key, "sk-burncloud-demo");
    assert!(demo.unlimited_quota);

    let mine = UserApiKeyModel::list(&db, 100, 0, Some("usr_pager"))
        .await
        .expect("listable");
    let names: Vec<String> = mine.iter().filter_map(|key| key.name.clone()).collect();
    assert_eq!(
        names,
        vec!["key-4", "key-3", "key-2", "key-1", "key-0"]
    );

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
        .filter_map(|key| key.name.clone())
        .collect();
    assert_eq!(paged, names);
    assert_eq!(page3.len(), 1);
    assert!(UserApiKeyModel::list(&db, 2, 99, Some("usr_pager"))
        .await
        .expect("listable")
        .is_empty());
    assert!(UserApiKeyModel::list(&db, 0, 0, Some("usr_pager"))
        .await
        .expect("listable")
        .is_empty());

    let mut ids: Vec<String> = keys.iter().map(|key| key.key.clone()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 5);
    cleanup(db, &path).await;
}

#[tokio::test]
async fn deleting_a_key_removes_only_that_key() {
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

    assert!(UserApiKeyModel::delete(&db, &first.key).await.expect("deleted"));
    assert!(UserApiKeyModel::get_by_key(&db, &first.key)
        .await
        .expect("readable")
        .is_none());
    assert!(UserApiKeyModel::get_by_key(&db, &second.key)
        .await
        .expect("readable")
        .is_some());
    assert!(!UserApiKeyModel::delete(&db, &first.key)
        .await
        .expect("second delete is not an error"));
    let remaining = UserApiKeyModel::list(&db, 10, 0, Some("usr_d"))
        .await
        .expect("listable");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].name.as_deref(), Some("second"));
    cleanup(db, &path).await;
}

#[tokio::test]
async fn a_password_reset_token_records_expiry_and_use_without_enforcing_them() {
    let (db, path) = fresh_db("reset_tokens").await;
    UserDatabase::create_user(&db, &account("usr_reset", "resetuser"))
        .await
        .expect("created");

    PasswordResetDatabase::create_token(&db, "tok-expired", "usr_reset", "2020-01-01T00:00:00Z")
        .await
        .expect("created");
    let expired = PasswordResetDatabase::get_token(&db, "tok-expired")
        .await
        .expect("readable")
        .expect("exists");
    assert_eq!(expired.expires_at, "2020-01-01T00:00:00Z");
    assert_eq!(expired.used_at, None);
    assert_eq!(expired.user_id, "usr_reset");

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
    assert_eq!(used.expires_at, "2999-01-01T00:00:00Z");
    assert!(used.used_at.is_some());

    let first_used_at = used.used_at.clone();
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    PasswordResetDatabase::mark_used(&db, "tok-live")
        .await
        .expect("marked again");
    let twice = PasswordResetDatabase::get_token(&db, "tok-live")
        .await
        .expect("readable")
        .expect("exists");
    assert_ne!(twice.used_at, first_used_at);
    assert!(PasswordResetDatabase::get_token(&db, "tok-missing")
        .await
        .expect("readable")
        .is_none());
    PasswordResetDatabase::mark_used(&db, "tok-missing")
        .await
        .expect("unknown token is a no-op");
    cleanup(db, &path).await;
}

#[tokio::test]
async fn a_failed_reset_leaves_the_old_password_usable() {
    let (db, path) = fresh_db("reset_failure").await;
    let mut user = account("usr_pw", "pwuser");
    user.password_hash = Some("$2b$12$originalhashvalue".to_string());
    UserDatabase::create_user(&db, &user).await.expect("created");

    async fn hash_of(db: &Database) -> Option<String> {
        UserDatabase::get_user_by_id(db, "usr_pw")
            .await
            .expect("readable")
            .expect("exists")
            .password_hash
    }

    let before = hash_of(&db).await;
    PasswordResetDatabase::create_token(&db, "tok-fail", "usr_pw", "2999-01-01T00:00:00Z")
        .await
        .expect("created");
    drop(
        PasswordResetDatabase::get_token(&db, "tok-fail")
            .await
            .expect("readable"),
    );
    PasswordResetDatabase::mark_used(&db, "tok-fail")
        .await
        .expect("marked");
    let orphan = PasswordResetDatabase::create_token(
        &db,
        "tok-orphan",
        "usr_absent",
        "2999-01-01T00:00:00Z",
    )
    .await;
    println!("token for an absent user -> {orphan:?}");

    let after = hash_of(&db).await;
    assert_eq!(before, after, "failed reset paths must not alter the password");
    UserDatabase::update_password_hash(&db, "usr_pw", "$2b$12$newhashvalue")
        .await
        .expect("updated");
    let changed = hash_of(&db).await;
    assert_eq!(changed.as_deref(), Some("$2b$12$newhashvalue"));
    assert_ne!(before, changed);
    cleanup(db, &path).await;
}
