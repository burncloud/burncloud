#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration test: an unwrap/expect failure is the intended failure signal"
)]
//! Schema contract for the monthly quota types (issue #652, AC2/AC3/AC4).
//!
//! The unit surface of `quota` is small, but what it claims is a fact about *production*: that
//! these types map onto the columns `0015_monthly_quota.sql` actually creates. A hand-written
//! `CREATE TABLE` for the tables under test would prove nothing about the shipped migration, so
//! every database-backed test here builds its database from the real embedded migrations through
//! [`create_database_with_url`].
//!
//! Coverage map:
//!
//! * `migrations_create_the_quota_tables` — the guard the database-backed tests depend on.
//! * AC3: the ordered column lists from `PRAGMA table_info` are compared with the exact column list
//!   each row struct selects, and a full row is written and read back through
//!   `BillingPlanRow`/`BillingSubscriptionRow`, so a renamed or reordered column fails here.
//! * AC2: the value domains are asserted against the migration comments item by item, and a value
//!   outside them is shown to be *storable* by the database yet *unrepresentable* in the types.
//! * AC4: quota exactly exhausted, over quota, the expiry boundary, cancellation with quota left,
//!   and two subscriptions to the same plan. The boundary predicates are pure functions of a row,
//!   so those tests build the rows directly instead of round-tripping them through the database;
//!   that keeps them independent of the schema mismatch described below.
//!
//! ## Why `scaffold_legacy_fk_parents` exists (finding F2)
//!
//! `0015` declares `billing_plans.channel_id -> channels(id)`,
//! `billing_subscriptions.user_id -> users(id)` and
//! `billing_subscriptions.channel_id -> channels(id)`. But migration
//! `0010_rename_tables.sql` renames `channels` to `channel_providers` and `users` to
//! `user_accounts`, and `burncloud-database`'s rename fixup (`schema/rename.rs`, `copy_and_drop`)
//! then **drops the legacy names**. On SQLite a foreign key may name a table that does not exist
//! (creation succeeds), but with foreign-key enforcement on — sqlx enables it by default — the first
//! *insert* into either table fails with `no such table: channels`.
//!
//! That mismatch is a schema defect, and the fix is a migration change, which this issue forbids
//! (`0015` is shipped; no migration may be edited). It is registered in the PR's Findings for a
//! follow-up. To keep this file's round-trip assertions meaningful in the meantime, the two legacy
//! parent tables are created here as an explicit, documented scaffold, and
//! `the_canonical_parent_tables_exist_but_0015_names_the_legacy_ones` pins the real schema state so
//! the scaffold can never quietly be mistaken for the production schema.

use burncloud_commerce_billing::{
    BillingMode, BillingPlan, BillingPlanRow, BillingSubscription, BillingSubscriptionRow,
    SubscriptionStatus,
};
use burncloud_database::{create_database_with_url, Database};

/// A fixed "now" so the expiry boundary is asserted rather than computed.
const NOW: i64 = 1_800_000_000_000;
/// One day later, in milliseconds.
const LATER: i64 = NOW + 86_400_000;
/// One day earlier, in milliseconds.
const EARLIER: i64 = NOW - 86_400_000;

/// Build a fresh SQLite database in a temporary file and run the real migrations.
async fn fresh_db(tag: &str) -> (Database, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "bc_quota_{}_{}_{}.db",
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
    (db, path)
}

/// Close the pool, wait for the handle to be released, then delete the test database.
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

/// Render an optional integer for the literal test inserts below.
fn sql_opt(value: Option<i64>) -> String {
    value.map_or_else(|| "NULL".to_owned(), |v| v.to_string())
}

/// Whether `table` exists in the migrated database.
async fn table_exists(db: &Database, table: &str) -> bool {
    let found: Vec<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_all(db.get_connection().unwrap().pool())
            .await
            .unwrap();
    !found.is_empty()
}

/// The columns of `table`, in declaration order.
async fn table_columns(db: &Database, table: &str) -> Vec<String> {
    let rows: Vec<(i64, String)> = sqlx::query_as(&format!("PRAGMA table_info({table})"))
        .fetch_all(db.get_connection().unwrap().pool())
        .await
        .unwrap();
    rows.into_iter().map(|(_, name)| name).collect()
}

/// The columns every `billing_plans` read in this file selects, in order.
const PLAN_COLUMNS: &str = "id, name, monthly_fee, billing_mode, request_limit, token_limit, \
     channel_id, created_at, updated_at";
/// The columns every `billing_subscriptions` read in this file selects, in order.
const SUBSCRIPTION_COLUMNS: &str = "id, user_id, plan_id, channel_id, status, quota_used, \
     quota_limit, expires_at, created_at, updated_at";

/// Create the two **legacy** foreign-key parents the shipped `0015` still names, and return a
/// channel id plus a non-numeric user id.
///
/// See the module header: migration `0010` renamed these tables and the rename fixup dropped them,
/// so they do not exist in the migrated database. Using the legacy names (rather than
/// `channel_providers`/`user_accounts`) is deliberate — it is the only scaffold that makes `0015`'s
/// own foreign-key declarations resolve, so the child inserts below exercise a real constraint check
/// instead of disabling enforcement. The user id is non-numeric on purpose: on SQLite the column is
/// declared `INTEGER`, and a digit-only id would be coerced by column affinity, which is the
/// divergence recorded as finding F1.
async fn scaffold_legacy_fk_parents(db: &Database) -> (i64, String) {
    db.execute_query(
        "CREATE TABLE channels (id INTEGER PRIMARY KEY AUTOINCREMENT, key TEXT NOT NULL)",
    )
    .await
    .unwrap();
    db.execute_query("CREATE TABLE users (id TEXT PRIMARY KEY)")
        .await
        .unwrap();
    db.execute_query("INSERT INTO channels (key) VALUES ('quota-contract')")
        .await
        .unwrap();
    db.execute_query("INSERT INTO users (id) VALUES ('user-alice')")
        .await
        .unwrap();
    let (channel_id,): (i64,) =
        sqlx::query_as("SELECT id FROM channels WHERE key = 'quota-contract'")
            .fetch_one(db.get_connection().unwrap().pool())
            .await
            .unwrap();
    (channel_id, "user-alice".to_owned())
}

/// A plan row to insert.
struct PlanSeed<'a> {
    name: &'a str,
    mode: &'a str,
    fee: i64,
    request_limit: Option<i64>,
    token_limit: Option<i64>,
}

/// Insert a plan and return its assigned id.
async fn insert_plan(db: &Database, seed: &PlanSeed<'_>, channel_id: i64) -> i64 {
    db.execute_query(&format!(
        "INSERT INTO billing_plans (name, monthly_fee, billing_mode, request_limit, token_limit, \
         channel_id, created_at, updated_at) VALUES ('{}', {}, '{}', {}, {}, {}, 1700000000000, \
         1700000000001)",
        seed.name,
        seed.fee,
        seed.mode,
        sql_opt(seed.request_limit),
        sql_opt(seed.token_limit),
        channel_id
    ))
    .await
    .unwrap();
    let (id,): (i64,) = sqlx::query_as("SELECT id FROM billing_plans WHERE name = ?")
        .bind(seed.name)
        .fetch_one(db.get_connection().unwrap().pool())
        .await
        .unwrap();
    id
}

/// A subscription row to insert.
struct SubSeed {
    status: &'static str,
    used: i64,
    limit: i64,
    expires_at: i64,
}

/// Insert a subscription for `user_id` on `plan_id`.
async fn insert_subscription(
    db: &Database,
    user_id: &str,
    plan_id: i64,
    channel_id: i64,
    seed: &SubSeed,
) {
    db.execute_query(&format!(
        "INSERT INTO billing_subscriptions (user_id, plan_id, channel_id, status, quota_used, \
         quota_limit, expires_at, created_at, updated_at) VALUES ('{}', {}, {}, '{}', {}, {}, {}, \
         1700000000000, 1700000000001)",
        user_id, plan_id, channel_id, seed.status, seed.used, seed.limit, seed.expires_at
    ))
    .await
    .unwrap();
}

/// Read one plan through the real `BillingPlanRow` decoding path.
async fn load_plan_row(db: &Database, name: &str) -> BillingPlanRow {
    sqlx::query_as::<_, BillingPlanRow>(&format!(
        "SELECT {PLAN_COLUMNS} FROM billing_plans WHERE name = ?"
    ))
    .bind(name)
    .fetch_one(db.get_connection().unwrap().pool())
    .await
    .unwrap()
}

/// Read every subscription of a plan, in insertion order, through the real row decoding path.
async fn load_subscriptions(db: &Database, plan_id: i64) -> Vec<BillingSubscription> {
    let rows: Vec<BillingSubscriptionRow> = sqlx::query_as::<_, BillingSubscriptionRow>(&format!(
        "SELECT {SUBSCRIPTION_COLUMNS} FROM billing_subscriptions WHERE plan_id = ? ORDER BY id"
    ))
    .bind(plan_id)
    .fetch_all(db.get_connection().unwrap().pool())
    .await
    .unwrap();
    rows.into_iter()
        .map(|row| BillingSubscription::try_from(row).expect("row maps onto the types"))
        .collect()
}

/// A subscription value in memory, for the boundary predicates.
///
/// The boundary methods are pure functions of a row, so these tests do not need a database at all;
/// constructing the row directly also keeps them independent of finding F2.
fn in_memory_subscription(
    status: SubscriptionStatus,
    used: i64,
    limit: i64,
    expires_at: i64,
) -> BillingSubscription {
    BillingSubscription {
        id: 1,
        user_id: "user-alice".to_owned(),
        plan_id: 7,
        channel_id: 3,
        status,
        quota_used: used,
        quota_limit: limit,
        expires_at,
        created_at: 1_700_000_000_000,
        updated_at: 1_700_000_000_001,
    }
}

#[tokio::test]
async fn migrations_create_the_quota_tables() {
    let (db, path) = fresh_db("guard").await;
    for expected in ["billing_plans", "billing_subscriptions"] {
        assert!(
            table_exists(&db, expected).await,
            "migration 0015 did not create {expected}"
        );
    }
    cleanup(db, &path).await;
}

#[tokio::test]
async fn the_canonical_parent_tables_exist_but_0015_names_the_legacy_ones() {
    // Pins the real schema state that `scaffold_legacy_fk_parents` works around: `0010` created the
    // canonical parents, and the rename fixup dropped the legacy names that `0015`'s foreign keys
    // still reference. Without this test the scaffold could be mistaken for the production schema.
    let (db, path) = fresh_db("parents").await;
    for canonical in ["user_accounts", "channel_providers"] {
        assert!(
            table_exists(&db, canonical).await,
            "0010 should have created {canonical}"
        );
    }
    for legacy in ["users", "channels"] {
        assert!(
            !table_exists(&db, legacy).await,
            "{legacy} should have been renamed away and dropped; finding F2 assumes this"
        );
    }
    cleanup(db, &path).await;
}

#[tokio::test]
async fn the_plan_types_map_onto_the_billing_plans_columns() {
    let (db, path) = fresh_db("plan_columns").await;
    let (channel_id, _user) = scaffold_legacy_fk_parents(&db).await;

    // AC3: the migration's declared columns are, one by one and in order, the ones the row struct
    // selects. A renamed, added or reordered column breaks this before any decoding can hide it.
    assert_eq!(
        table_columns(&db, "billing_plans").await,
        vec![
            "id",
            "name",
            "monthly_fee",
            "billing_mode",
            "request_limit",
            "token_limit",
            "channel_id",
            "created_at",
            "updated_at"
        ]
    );

    // A per-token plan: `token_limit` set, `request_limit` NULL.
    insert_plan(
        &db,
        &PlanSeed {
            name: "pro-monthly",
            mode: "per_token",
            fee: 9_900_000_000,
            request_limit: None,
            token_limit: Some(5_000_000),
        },
        channel_id,
    )
    .await;

    let plan = BillingPlan::try_from(load_plan_row(&db, "pro-monthly").await).unwrap();
    assert!(plan.id > 0, "SQLite assigned a row id");
    assert_eq!(plan.name, "pro-monthly");
    assert_eq!(plan.monthly_fee, 9_900_000_000, "nanodollar fee survives");
    assert_eq!(plan.billing_mode, BillingMode::PerToken);
    assert_eq!(plan.request_limit, None, "the unused limit stays NULL");
    assert_eq!(plan.token_limit, Some(5_000_000));
    assert_eq!(plan.channel_id, channel_id);
    assert_eq!(plan.created_at, 1_700_000_000_000);
    assert_eq!(plan.updated_at, 1_700_000_000_001);

    // A per-request plan inverts which limit column is populated.
    insert_plan(
        &db,
        &PlanSeed {
            name: "basic-monthly",
            mode: "per_request",
            fee: 1_500_000_000,
            request_limit: Some(1_000),
            token_limit: None,
        },
        channel_id,
    )
    .await;
    let basic = BillingPlan::try_from(load_plan_row(&db, "basic-monthly").await).unwrap();
    assert_eq!(basic.billing_mode, BillingMode::PerRequest);
    assert_eq!(basic.request_limit, Some(1_000));
    assert_eq!(basic.token_limit, None);

    cleanup(db, &path).await;
}

#[tokio::test]
async fn the_subscription_types_map_onto_the_billing_subscriptions_columns() {
    let (db, path) = fresh_db("sub_columns").await;
    let (channel_id, user_id) = scaffold_legacy_fk_parents(&db).await;
    let plan_id = insert_plan(
        &db,
        &PlanSeed {
            name: "pro-monthly",
            mode: "per_token",
            fee: 9_900_000_000,
            request_limit: None,
            token_limit: Some(5_000_000),
        },
        channel_id,
    )
    .await;

    assert_eq!(
        table_columns(&db, "billing_subscriptions").await,
        vec![
            "id",
            "user_id",
            "plan_id",
            "channel_id",
            "status",
            "quota_used",
            "quota_limit",
            "expires_at",
            "created_at",
            "updated_at"
        ]
    );

    insert_subscription(
        &db,
        &user_id,
        plan_id,
        channel_id,
        &SubSeed {
            status: "active",
            used: 1_200,
            limit: 5_000_000,
            expires_at: LATER,
        },
    )
    .await;

    let stored = load_subscriptions(&db, plan_id).await;
    assert_eq!(stored.len(), 1);
    let subscription = &stored[0];
    assert!(subscription.id > 0);
    // The foreign key targets the text `users.id`, so the value round-trips as text.
    assert_eq!(subscription.user_id, "user-alice");
    assert_eq!(subscription.plan_id, plan_id);
    assert_eq!(subscription.channel_id, channel_id);
    assert_eq!(subscription.status, SubscriptionStatus::Active);
    assert_eq!(subscription.quota_used, 1_200);
    assert_eq!(subscription.quota_limit, 5_000_000);
    assert_eq!(subscription.expires_at, LATER);
    assert_eq!(subscription.created_at, 1_700_000_000_000);
    assert_eq!(subscription.updated_at, 1_700_000_000_001);

    cleanup(db, &path).await;
}

#[tokio::test]
async fn the_value_domains_match_the_migration_comments() {
    // AC2, item by item against the two comments in 0015_monthly_quota.sql.
    assert_eq!(BillingMode::PerRequest.as_str(), "per_request");
    assert_eq!(BillingMode::PerToken.as_str(), "per_token");
    for mode in [BillingMode::PerRequest, BillingMode::PerToken] {
        assert_eq!(mode.as_str().parse::<BillingMode>().unwrap(), mode);
    }
    assert_eq!(SubscriptionStatus::Active.as_str(), "active");
    assert_eq!(SubscriptionStatus::Expired.as_str(), "expired");
    assert_eq!(SubscriptionStatus::Cancelled.as_str(), "cancelled");
    for status in [
        SubscriptionStatus::Active,
        SubscriptionStatus::Expired,
        SubscriptionStatus::Cancelled,
    ] {
        assert_eq!(
            status.as_str().parse::<SubscriptionStatus>().unwrap(),
            status
        );
    }

    // Values the migration does not permit are refused, and the error names the column.
    let mode_error = "weekly".parse::<BillingMode>().unwrap_err();
    assert_eq!(mode_error.field(), "billing_mode");
    assert_eq!(mode_error.value(), "weekly");
    let status_error = "paused".parse::<SubscriptionStatus>().unwrap_err();
    assert_eq!(status_error.field(), "status");
    assert_eq!(status_error.value(), "paused");
}

#[tokio::test]
async fn a_billing_mode_the_schema_permits_but_the_comment_does_not_is_unrepresentable() {
    // The columns carry no CHECK constraint, so the database itself stores 'weekly'. AC2 asks that
    // an invalid value be unrepresentable in the types: decoding must refuse it rather than let a
    // plan exist whose mode no code can interpret.
    let (db, path) = fresh_db("bad_mode").await;
    let (channel_id, _user) = scaffold_legacy_fk_parents(&db).await;
    insert_plan(
        &db,
        &PlanSeed {
            name: "legacy-plan",
            mode: "weekly",
            fee: 0,
            request_limit: None,
            token_limit: None,
        },
        channel_id,
    )
    .await;

    let row = load_plan_row(&db, "legacy-plan").await;
    assert_eq!(
        row.billing_mode, "weekly",
        "the column stored the text as-is"
    );
    let error = BillingPlan::try_from(row).unwrap_err();
    assert_eq!(error.field(), "billing_mode");
    assert_eq!(error.value(), "weekly");

    cleanup(db, &path).await;
}

#[tokio::test]
async fn a_subscription_status_the_schema_permits_but_the_comment_does_not_is_unrepresentable() {
    let (db, path) = fresh_db("bad_status").await;
    let (channel_id, user_id) = scaffold_legacy_fk_parents(&db).await;
    let plan_id = insert_plan(
        &db,
        &PlanSeed {
            name: "pro-monthly",
            mode: "per_token",
            fee: 1,
            request_limit: None,
            token_limit: Some(10),
        },
        channel_id,
    )
    .await;
    insert_subscription(
        &db,
        &user_id,
        plan_id,
        channel_id,
        &SubSeed {
            status: "suspended",
            used: 0,
            limit: 10,
            expires_at: LATER,
        },
    )
    .await;

    let rows: Vec<BillingSubscriptionRow> = sqlx::query_as::<_, BillingSubscriptionRow>(&format!(
        "SELECT {SUBSCRIPTION_COLUMNS} FROM billing_subscriptions WHERE plan_id = ?"
    ))
    .bind(plan_id)
    .fetch_all(db.get_connection().unwrap().pool())
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    let error = BillingSubscription::try_from(rows.into_iter().next().unwrap()).unwrap_err();
    assert_eq!(error.field(), "status");
    assert_eq!(error.value(), "suspended");

    cleanup(db, &path).await;
}

#[tokio::test]
async fn a_subscription_inside_its_quota_and_window_is_usable() {
    let subscription = in_memory_subscription(SubscriptionStatus::Active, 99, 100, LATER);

    assert_eq!(subscription.remaining_quota(), 1);
    assert!(!subscription.is_exhausted());
    assert!(!subscription.is_expired_at(NOW));
    assert!(subscription.is_usable_at(NOW));

    let view = subscription.status_response_at(NOW);
    assert_eq!(view.remaining_quota, 1);
    assert!(view.usable);
    assert_eq!(view.status, SubscriptionStatus::Active);
    assert_eq!(view.quota_limit, 100);
    assert_eq!(view.expires_at, LATER);
}

#[tokio::test]
async fn a_subscription_whose_quota_is_exactly_used_up_is_not_usable() {
    let subscription = in_memory_subscription(SubscriptionStatus::Active, 100, 100, LATER);

    assert_eq!(subscription.remaining_quota(), 0);
    assert!(subscription.is_exhausted(), "used == limit is exhausted");
    assert!(!subscription.is_usable_at(NOW));
    assert!(!subscription.status_response_at(NOW).usable);
}

#[tokio::test]
async fn an_over_quota_subscription_reports_no_negative_remainder() {
    let subscription = in_memory_subscription(SubscriptionStatus::Active, 250, 100, LATER);

    assert!(subscription.is_exhausted());
    assert_eq!(
        subscription.remaining_quota(),
        0,
        "an overrun must not produce a negative remainder"
    );
    assert!(!subscription.is_usable_at(NOW));
}

#[tokio::test]
async fn an_extreme_overrun_still_clamps_to_zero() {
    // The two quota columns are independent i64 values; an extreme pair must clamp, not overflow.
    let subscription =
        in_memory_subscription(SubscriptionStatus::Active, i64::MAX, i64::MIN, LATER);

    assert_eq!(subscription.remaining_quota(), 0);
    assert!(subscription.is_exhausted());
    assert!(!subscription.is_usable_at(NOW));
}

#[tokio::test]
async fn the_expiry_boundary_itself_counts_as_expired() {
    let subscription = in_memory_subscription(SubscriptionStatus::Active, 1, 100, NOW);

    assert!(!subscription.is_expired_at(EARLIER));
    assert!(
        subscription.is_usable_at(EARLIER),
        "still inside the window"
    );
    assert!(subscription.is_expired_at(NOW), "expires_at is inclusive");
    assert!(!subscription.is_usable_at(NOW));
    assert!(!subscription.is_usable_at(LATER));
}

#[tokio::test]
async fn a_cancelled_subscription_is_unusable_even_with_quota_left() {
    let subscription = in_memory_subscription(SubscriptionStatus::Cancelled, 0, 100, LATER);

    assert_eq!(subscription.status, SubscriptionStatus::Cancelled);
    assert_eq!(
        subscription.remaining_quota(),
        100,
        "quota is still recorded"
    );
    assert!(!subscription.is_exhausted());
    assert!(
        !subscription.is_usable_at(NOW),
        "cancellation outranks remaining quota"
    );
}

#[tokio::test]
async fn an_expired_status_row_is_unusable_before_its_timestamp() {
    // The schema keeps `status` and `expires_at` as separate facts; a row already marked expired
    // must not become usable again just because its timestamp is in the future.
    let subscription = in_memory_subscription(SubscriptionStatus::Expired, 0, 100, LATER);

    assert!(!subscription.is_expired_at(NOW));
    assert_eq!(subscription.status, SubscriptionStatus::Expired);
    assert!(!subscription.is_usable_at(NOW));
}

#[tokio::test]
async fn two_subscriptions_to_the_same_plan_are_both_representable() {
    // `billing_subscriptions` has indexes on (user_id), (plan_id), (channel_id), (status) and
    // (expires_at) but no unique constraint on (user_id, plan_id), so a repeated subscription is a
    // state the schema permits. The mapping must not collapse the two rows into one.
    let (db, path) = fresh_db("duplicate").await;
    let (channel_id, user_id) = scaffold_legacy_fk_parents(&db).await;
    let plan_id = insert_plan(
        &db,
        &PlanSeed {
            name: "pro-monthly",
            mode: "per_request",
            fee: 1_000_000_000,
            request_limit: Some(50),
            token_limit: None,
        },
        channel_id,
    )
    .await;
    for used in [10, 20] {
        insert_subscription(
            &db,
            &user_id,
            plan_id,
            channel_id,
            &SubSeed {
                status: "active",
                used,
                limit: 50,
                expires_at: LATER,
            },
        )
        .await;
    }

    let stored = load_subscriptions(&db, plan_id).await;
    assert_eq!(stored.len(), 2, "both rows must survive the mapping");
    assert_ne!(
        stored[0].id, stored[1].id,
        "distinct rows keep distinct ids"
    );
    assert_eq!(stored[0].quota_used, 10);
    assert_eq!(stored[1].quota_used, 20);
    assert_eq!(stored[0].user_id, stored[1].user_id);
    assert_eq!(stored[0].plan_id, stored[1].plan_id);
    assert!(stored.iter().all(|s| s.is_usable_at(NOW)));

    cleanup(db, &path).await;
}
