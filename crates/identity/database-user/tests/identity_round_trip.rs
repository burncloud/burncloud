#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration-test file: fail-fast setup is the intended behaviour"
)]
//! Regression cover for the Identity database layer (S1-D, #610).
//!
//! The Identity domain had no test at all before this file. S1-D removes the three legacy
//! `common` DTOs and asserts that Identity's own account / credential / recharge types are the
//! single owner, so those types need a floor: a real SQLite database, the real `UserDatabase::init`
//! schema, the real writers and readers, and field-by-field assertions.
//!
//! A file database rather than `:memory:`: the pool opens several connections and an in-memory
//! SQLite database is per-connection, so the schema and the assertions would see different
//! databases. On Windows the URL needs the `sqlite:///C:/...` form (see `Database::new`).

use burncloud_database::{create_database_with_url, Database};
use burncloud_database_user::{
    UserAccount, UserApiKeyInput, UserApiKeyModel, UserApiKeyUpdateInput, UserDatabase,
    UserRecharge,
};

/// A fresh SQLite file database with the Identity schema applied through the production initializer.
async fn fresh_db(tag: &str) -> (Database, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "bc_identity_{}_{}_{}.db",
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
    (db, path)
}

/// Close the pool, wait for the handle to be released, then delete the test database.
///
/// Measured on this platform (Windows): dropping a `Database` leaves the file locked and removal
/// fails with os error 32; `close().await` alone is not enough either, because the pool releases
/// the handle asynchronously. With a short wait after `close()` the removal succeeds. Before this
/// was noticed the suite silently leaked one database per test into the temp directory (78 had
/// accumulated), so the final state is asserted rather than assumed.
async fn cleanup(db: Database, path: &std::path::Path) {
    db.close().await.ok();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    std::fs::remove_file(path).unwrap_or_else(|e| {
        panic!(
            "test database {} was not removed ({e}); the temp directory would fill up",
            path.display()
        )
    });
}

fn account(id: &str, username: &str, balance_usd: i64, balance_cny: i64) -> UserAccount {
    UserAccount {
        id: id.to_string(),
        username: username.to_string(),
        email: Some(format!("{username}@example.com")),
        password_hash: Some("$2b$12$notarealhash".to_string()),
        github_id: None,
        status: 1,
        balance_usd,
        balance_cny,
        preferred_currency: Some("CNY".to_string()),
    }
}

#[tokio::test]
async fn accounts_round_trip_through_the_real_schema() {
    let (db, path) = fresh_db("accounts").await;

    let user = account("u-1", "alice", 5_000_000_000_i64, 36_200_000_000_i64);
    UserDatabase::create_user(&db, &user)
        .await
        .expect("real INSERT into user_accounts");

    let stored = UserDatabase::get_user_by_id(&db, "u-1")
        .await
        .expect("real SELECT from user_accounts")
        .expect("the account that was just created must be found");

    // Identity fields.
    assert_eq!(stored.id, "u-1");
    assert_eq!(stored.username, "alice");
    assert_eq!(stored.email.as_deref(), Some("alice@example.com"));
    assert_eq!(stored.status, 1);
    assert_eq!(stored.preferred_currency.as_deref(), Some("CNY"));

    // Balance fields still round-trip byte-for-byte. S1-D does NOT move their ownership; the
    // migration of that responsibility is tracked separately (see #610). This assertion pins the
    // current behaviour so a later change to the writer is a deliberate, visible act.
    assert_eq!(stored.balance_usd, 5_000_000_000);
    assert_eq!(stored.balance_cny, 36_200_000_000);

    // The password hash is readable in Rust. Its JSON projection is deliberately NOT asserted here:
    // today `UserAccount` has no `skip_serializing`, so the field does reach JSON, and whether that
    // is correct is a design question with its own issue -- not something this test should freeze.
    // Recorded in #610 as out of scope for S1-D.
    assert_eq!(stored.password_hash.as_deref(), Some("$2b$12$notarealhash"));
    let json = serde_json::to_value(&stored).unwrap();
    assert_eq!(json["username"], serde_json::json!("alice"));
    // Balance is projected into JSON under the same names (Commerce-facing projection).
    // `json!` is used rather than naming `serde_json::Value`: the workspace's type gate disallows that
    // type outside protocol boundaries, and the macro produces the right numeric variant from the
    // value itself, so the nanodollar amount stays an i64 without an explicit constructor.
    assert_eq!(json["balance_usd"], serde_json::json!(stored.balance_usd));

    // Lookup paths share the writer's column list.
    let by_name = UserDatabase::get_user_by_username(&db, "alice")
        .await
        .unwrap()
        .expect("lookup by username");
    assert_eq!(by_name.id, "u-1");
    let by_email = UserDatabase::get_user_by_email(&db, "alice@example.com")
        .await
        .unwrap()
        .expect("lookup by email");
    assert_eq!(by_email.id, "u-1");

    // `UserDatabase::init` seeds a `demo-user` row (password hash `no-login`) so the console has a
    // placeholder account. `list_users` returns it, while `count_users`/`has_admin_user` filter it
    // out because it cannot log in. Both behaviours are asserted so the seed stays visible.
    let all = UserDatabase::list_users(&db).await.unwrap();
    assert_eq!(
        all.len(),
        2,
        "the demo-user seed plus the account created here"
    );
    assert!(
        all.iter().any(|u| u.username == "demo-user"),
        "the seeded placeholder account is present"
    );
    let seed = all.iter().find(|u| u.username == "demo-user").unwrap();
    assert_eq!(
        seed.password_hash.as_deref(),
        Some("no-login"),
        "the seed cannot authenticate"
    );
    assert_eq!(
        UserDatabase::count_users(&db).await.unwrap(),
        1,
        "count_users excludes the seed"
    );
    // `create_user` only inserts the account row; role assignment is a separate call, so no admin
    // exists yet. The first-user-is-admin policy lives above this layer.
    assert!(
        !UserDatabase::has_admin_user(&db).await.unwrap(),
        "creating an account does not grant a role"
    );

    // The role path: assign, then observe.
    UserDatabase::assign_role(&db, "u-1", "admin")
        .await
        .unwrap();
    let roles = UserDatabase::get_user_roles(&db, "u-1").await.unwrap();
    assert!(
        roles.iter().any(|r| r == "admin"),
        "assign_role must be observable through get_user_roles, got {roles:?}"
    );
    assert!(
        UserDatabase::has_admin_user(&db).await.unwrap(),
        "an account holding the admin role counts as an admin"
    );

    cleanup(db, &path).await;
}

#[tokio::test]
async fn balance_writers_are_the_only_mutation_path_and_are_delta_based() {
    // Records the current ownership of balance mutation: it lives in Identity today and is reached
    // from the recharge command. The test asserts the *delta* semantics, because a later move to
    // Commerce must preserve them.
    let (db, path) = fresh_db("balance").await;
    UserDatabase::create_user(&db, &account("u-2", "bob", 1_000_000_000, 0))
        .await
        .unwrap();

    let after_credit = UserDatabase::update_balance_usd(&db, "u-2", 2_500_000_000)
        .await
        .expect("real UPDATE ... SET balance_usd = balance_usd + ?");
    assert_eq!(after_credit, 3_500_000_000);

    let after_debit = UserDatabase::update_balance_usd(&db, "u-2", -500_000_000)
        .await
        .unwrap();
    assert_eq!(after_debit, 3_000_000_000, "a negative delta debits");

    // The currency selector routes to the matching column.
    let cny = UserDatabase::update_balance(&db, "u-2", 7_000_000_000, Some("CNY"))
        .await
        .unwrap();
    assert_eq!(cny, 7_000_000_000);
    let usd_untouched = UserDatabase::get_user_by_id(&db, "u-2")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        usd_untouched.balance_usd, 3_000_000_000,
        "the CNY write must not touch the USD column"
    );
    assert_eq!(usd_untouched.balance_cny, 7_000_000_000);

    cleanup(db, &path).await;
}

#[tokio::test]
async fn recharge_rows_round_trip_and_credit_the_balance() {
    let (db, path) = fresh_db("recharge").await;
    UserDatabase::create_user(&db, &account("u-3", "carol", 0, 0))
        .await
        .unwrap();

    let recharge = UserRecharge {
        id: 0,
        user_id: "u-3".to_string(),
        amount: 10_000_000_000,
        currency: Some("USD".to_string()),
        description: Some("first top-up".to_string()),
        created_at: None,
    };
    let id = UserDatabase::create_recharge(&db, &recharge)
        .await
        .expect("real INSERT into user_recharges");
    assert!(id > 0);

    let listed = UserDatabase::list_recharges(&db, "u-3").await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].amount, 10_000_000_000);
    assert_eq!(listed[0].currency.as_deref(), Some("USD"));
    assert_eq!(listed[0].description.as_deref(), Some("first top-up"));
    assert!(
        listed[0].created_at.is_some(),
        "created_at is stamped by the schema default"
    );

    // The recharge writer credits the balance in the same call (current behaviour, recorded here).
    let credited = UserDatabase::get_user_by_id(&db, "u-3")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        credited.balance_usd, 10_000_000_000,
        "create_recharge credits balance_usd for a USD recharge"
    );

    cleanup(db, &path).await;
}

#[tokio::test]
async fn api_keys_round_trip_and_updates_are_partial() {
    let (db, path) = fresh_db("apikeys").await;
    UserDatabase::create_user(&db, &account("u-4", "dave", 0, 0))
        .await
        .unwrap();

    let created = UserApiKeyModel::create(
        &db,
        &UserApiKeyInput {
            user_id: "u-4".to_string(),
            name: Some("default".to_string()),
            remain_quota: Some(1_000_000),
            unlimited_quota: Some(false),
            expired_time: Some(-1),
        },
    )
    .await
    .expect("real INSERT into user_api_keys");

    assert_eq!(created.user_id, "u-4");
    assert_eq!(created.status, 1);
    assert!(!created.key.is_empty(), "a key is generated");
    assert_eq!(created.expired_time, -1, "-1 means never expires");

    let fetched = UserApiKeyModel::get_by_key(&db, &created.key)
        .await
        .unwrap()
        .expect("lookup by key");
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.remain_quota, 1_000_000);

    // A partial update must leave the untouched fields alone.
    let updated = UserApiKeyModel::update(
        &db,
        &created.key,
        &UserApiKeyUpdateInput {
            name: Some("renamed".to_string()),
            remain_quota: None,
            status: Some(2),
            expired_time: None,
        },
    )
    .await
    .unwrap();
    assert!(updated, "update reports the row was touched");
    let after = UserApiKeyModel::get_by_key(&db, &created.key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.name.as_deref(), Some("renamed"));
    assert_eq!(after.status, 2);
    assert_eq!(
        after.remain_quota, 1_000_000,
        "remain_quota was not updated"
    );
    assert_eq!(after.expired_time, -1, "expired_time was not updated");

    // No assertion on how the key is projected into JSON. `UserApiKey.key` is an ordinary
    // serializable field today, and whether a full key may be returned by create/get/list, or must
    // be masked, is a design decision with its own issue (#612). A test is the wrong place to
    // settle it, and asserting the current shape would freeze it as intended behaviour.

    let listed = UserApiKeyModel::list(&db, 10, 0, Some("u-4"))
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert!(UserApiKeyModel::delete(&db, &created.key).await.unwrap());
    assert!(
        UserApiKeyModel::get_by_key(&db, &created.key)
            .await
            .unwrap()
            .is_none(),
        "a deleted key is gone"
    );

    cleanup(db, &path).await;
}
