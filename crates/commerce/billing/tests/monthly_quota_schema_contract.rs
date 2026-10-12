#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration test: an unwrap/expect failure is the intended failure signal"
)]
//! Schema contract for the monthly quota types (issue #652, AC2/AC3/AC4).
//!
//! The unit surface of `quota` is small, but what it claims is a fact about *production*: that
//! these types map onto the columns `0015_monthly_quota.sql` actually creates. A hand-written
//! `CREATE TABLE` here would prove nothing about the shipped migration, so every test builds its
//! database from the real embedded migrations through [`create_database_with_url`].
//!
//! Coverage map:
//!
//! * `migrations_create_the_quota_tables` — the guard the rest of the file depends on.
//! * AC3: the ordered column lists from `PRAGMA table_info` are compared with the exact column list
//!   each row struct selects, and a full row is written and read back through
//!   `BillingPlanRow`/`BillingSubscriptionRow`, so a renamed or reordered column fails here.
//! * AC2: the value domains are asserted against the migration comments item by item, and a value
//!   outside them is shown to be *storable* by the database yet *unrepresentable* in the types.
//! * AC4: quota exactly exhausted, over quota, the expiry boundary, cancellation with quota left,
//!   and two subscriptions to the same plan.
//!
//! Honest limits: these tests drive SQLite (the dialect this crate's existing row-mapping test
//! uses). The PostgreSQL contract suites run in CI as well, but this file does not drive them, and
//! nothing here exercises the billing admission path, which issue #652 leaves out of scope.

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

/// The foreign-key parents `billing_plans`/`billing_subscriptions` reference.
async fn seed_parents(db: &Database) -> (i64, String) {
    db.execute_query(
        "INSERT INTO channels (key, name) VALUES ('quota-contract', 'quota-contract')",
    )
    .await
    .unwrap();
    let (channel_id,): (i64,) =
        sqlx::query_as("SELECT id FROM channels WHERE key = 'quota-contract'")
            .fetch_one(db.get_connection().unwrap().pool())
            .await
            .unwrap();
    // `users.id` is TEXT in both dialects; the id is deliberately non-numeric (see the module note
    // in `quota.rs` about the SQLite `user_id` declaration).
    db.execute_query(
        "INSERT INTO users (id, username, password_hash) VALUES ('user-alice', 'alice', 'test-hash')",
    )
    .await
    .unwrap();
    (channel_id, "user-alice".to_owned())
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
const PLAN_COLUMNS: &str =
    "id, name, monthly_fee, billing_mode, request_limit, token_limit, channel_id, created_at, updated_at";
/// The columns every `billing_subscriptions` read in this file selects, in order.
const SUBSCRIPTION_COLUMNS: &str = "id, user_id, plan_id, channel_id, status, quota_used, \
     quota_limit, expires_at, created_at, updated_at";

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

#[tokio::test]
async fn migrations_create_the_quota_tables() {
    let (db, path) = fresh_db("guard").await;
    for expected in ["billing_plans", "billing_subscriptions"] {
        let found: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?")
                .bind(expected)
                .fetch_all(db.get_connection().unwrap().pool())
                .await
                .unwrap();
        assert_eq!(found.len(), 1, "migration 0015 did not create {expected}");
    }
    cleanup(db, &path).await;
}

#[tokio::test]
async fn the_plan_types_map_onto_the_billing_plans_columns() {
    let (db, path) = fresh_db("plan_columns").await;
    let (channel_id, _user) = seed_parents(&db).await;

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
    let (channel_id, user_id) = seed_parents(&db).await;
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
    let (channel_id, _user) = seed_parents(&db).await;
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
    let (channel_id, user_id) = seed_parents(&db).await;
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

/// Insert one subscription and return it, for the boundary cases.
async fn subscription_with(db: &Database, seed: &SubSeed) -> BillingSubscription {
    let (channel_id, user_id) = seed_parents(db).await;
    let plan_id = insert_plan(
        db,
        &PlanSeed {
            name: "boundary-plan",
            mode: "per_request",
            fee: 2_000_000_000,
            request_limit: Some(100),
            token_limit: None,
        },
        channel_id,
    )
    .await;
    insert_subscription(db, &user_id, plan_id, channel_id, seed).await;
    load_subscriptions(db, plan_id).await.remove(0)
}

#[tokio::test]
async fn a_subscription_inside_its_quota_and_window_is_usable() {
    let (db, path) = fresh_db("usable").await;
    let subscription = subscription_with(
        &db,
        &SubSeed {
            status: "active",
            used: 99,
            limit: 100,
            expires_at: LATER,
        },
    )
    .await;

    assert_eq!(subscription.remaining_quota(), 1);
    assert!(!subscription.is_exhausted());
    assert!(!subscription.is_expired_at(NOW));
    assert!(subscription.is_usable_at(NOW));

    let view = subscription.status_response_at(NOW);
    assert_eq!(view.remaining_quota, 1);
    assert!(view.usable);

    cleanup(db, &path).await;
}

#[tokio::test]
async fn a_subscription_whose_quota_is_exactly_used_up_is_not_usable() {
    let (db, path) = fresh_db("exhausted").await;
    let subscription = subscription_with(
        &db,
        &SubSeed {
            status: "active",
            used: 100,
            limit: 100,
            expires_at: LATER,
        },
    )
    .await;

    assert_eq!(subscription.remaining_quota(), 0);
    assert!(subscription.is_exhausted(), "used == limit is exhausted");
    assert!(!subscription.is_usable_at(NOW));

    cleanup(db, &path).await;
}

#[tokio::test]
async fn an_over_quota_subscription_reports_no_negative_remainder() {
    let (db, path) = fresh_db("over_quota").await;
    let subscription = subscription_with(
        &db,
        &SubSeed {
            status: "active",
            used: 250,
            limit: 100,
            expires_at: LATER,
        },
    )
    .await;

    assert!(subscription.is_exhausted());
    assert_eq!(
        subscription.remaining_quota(),
        0,
        "an overrun must not produce a negative remainder"
    );
    assert!(!subscription.is_usable_at(NOW));

    cleanup(db, &path).await;
}

#[tokio::test]
async fn the_expiry_boundary_itself_counts_as_expired() {
    let (db, path) = fresh_db("expiry").await;
    let subscription = subscription_with(
        &db,
        &SubSeed {
            status: "active",
            used: 1,
            limit: 100,
            expires_at: NOW,
        },
    )
    .await;

    assert!(!subscription.is_expired_at(EARLIER));
    assert!(
        subscription.is_usable_at(EARLIER),
        "still inside the window"
    );
    assert!(subscription.is_expired_at(NOW), "expires_at is inclusive");
    assert!(!subscription.is_usable_at(NOW));
    assert!(!subscription.is_usable_at(LATER));

    cleanup(db, &path).await;
}

#[tokio::test]
async fn a_cancelled_subscription_is_unusable_even_with_quota_left() {
    let (db, path) = fresh_db("cancelled").await;
    let subscription = subscription_with(
        &db,
        &SubSeed {
            status: "cancelled",
            used: 0,
            limit: 100,
            expires_at: LATER,
        },
    )
    .await;

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

    cleanup(db, &path).await;
}

#[tokio::test]
async fn an_expired_status_row_is_unusable_before_its_timestamp() {
    // The schema keeps `status` and `expires_at` as separate facts; a row already marked expired
    // must not become usable again just because its timestamp is in the future.
    let (db, path) = fresh_db("expired_status").await;
    let subscription = subscription_with(
        &db,
        &SubSeed {
            status: "expired",
            used: 0,
            limit: 100,
            expires_at: LATER,
        },
    )
    .await;

    assert!(!subscription.is_expired_at(NOW));
    assert_eq!(subscription.status, SubscriptionStatus::Expired);
    assert!(!subscription.is_usable_at(NOW));

    cleanup(db, &path).await;
}

#[tokio::test]
async fn two_subscriptions_to_the_same_plan_are_both_representable() {
    // `billing_subscriptions` has indexes on (user_id), (plan_id), (channel_id), (status) and
    // (expires_at) but no unique constraint on (user_id, plan_id), so a repeated subscription is a
    // state the schema permits. The mapping must not collapse the two rows into one.
    let (db, path) = fresh_db("duplicate").await;
    let (channel_id, user_id) = seed_parents(&db).await;
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

    cleanup(db, &path).await;
}
