#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration tests use unwrap/expect for fixture setup and assertions"
)]
//! The Identity contract against a real PostgreSQL server (#633, plan section 5 item 02).
//!
//! `identity_round_trip.rs` pins this contract on SQLite, and the schema exists for both backends: 18
//! migrations per backend, identical numbering, all referenced from the Rust migration list. What has
//! never happened is applying the PostgreSQL half to an actual server -- nothing in this repository
//! connects with a `postgres://` URL.
//!
//! So this file is the smallest thing that can falsify the dual-backend promise: the same round trip the
//! SQLite test performs, against PostgreSQL.
//!
//! ## Isolation
//!
//! Each run creates its **own database** with a unique name and drops it afterwards. Reusing one database
//! would leave rows behind between runs and make the assertions depend on execution order -- and the
//! plan is explicit that a database test must not share state. Creating a database needs a superuser,
//! which the CI service provides; a URL without that privilege skips with a stated reason rather than
//! failing.
//!
//! ## When no server is configured
//!
//! `BURNCLOUD_TEST_POSTGRES_URL` unset means the test **skips with an explicit message**. That is the
//! plan's rule: a missing environment must be reported as not-run, never hidden behind a passing test.
//! The skip line names the variable so a reader knows what to set to make it run.

use burncloud_database::sqlx::{self, ConnectOptions, Executor};
use burncloud_database::{create_database_with_url, Database};
use burncloud_database_user::{UserAccount, UserDatabase};
use std::str::FromStr;

/// The environment variable naming the server, e.g. `postgres://postgres:postgres@localhost:5432/postgres`.
const SERVER_URL_ENV: &str = "BURNCLOUD_TEST_POSTGRES_URL";

/// A unique database name, so parallel runs and repeated runs cannot collide.
fn unique_db_name(tag: &str) -> String {
    format!(
        "bc_test_{}_{}_{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

/// Replace the trailing database name of a connection URL.
///
/// Parsed rather than string-appended so an URL with query parameters (`?sslmode=disable`) keeps them.
fn url_with_database(server_url: &str, database: &str) -> String {
    let (without_query, query) = match server_url.split_once('?') {
        Some((front, back)) => (front, Some(back)),
        None => (server_url, None),
    };
    let scheme_end = without_query.find("://").map(|i| i + 3).unwrap_or(0);
    let (head, tail) = without_query.split_at(scheme_end);
    let base = match tail.rfind('/') {
        Some(i) => &tail[..i],
        None => tail,
    };
    let rebuilt = format!("{head}{base}/{database}");
    match query {
        Some(q) => format!("{rebuilt}?{q}"),
        None => rebuilt,
    }
}

/// Create the throwaway database, returning its URL.
async fn create_database(server_url: &str, name: &str) -> String {
    let mut conn = sqlx::postgres::PgConnectOptions::from_str(server_url)
        .unwrap_or_else(|e| panic!("{SERVER_URL_ENV} is not a usable PostgreSQL URL: {e}"))
        .connect()
        .await
        .unwrap_or_else(|e| panic!("could not connect to {server_url}: {e}"));

    // A leftover database from a crashed run should not block this run. A failed cleanup is visible,
    // while CREATE below remains the authoritative failure if the database still cannot be created.
    if let Err(error) = conn
        .execute(format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)").as_str())
        .await
    {
        eprintln!("warning: could not drop stale PostgreSQL test database {name}: {error}");
    }
    conn.execute(format!("CREATE DATABASE {name}").as_str())
        .await
        .unwrap_or_else(|e| panic!("CREATE DATABASE {name}: {e}"));

    url_with_database(server_url, name)
}

/// Drop the throwaway database, tolerating connection/drop failures while making them visible.
async fn drop_database(server_url: &str, name: &str) {
    if let Ok(mut conn) = sqlx::postgres::PgConnectOptions::from_str(server_url)
        .unwrap_or_else(|e| panic!("{SERVER_URL_ENV} is not a usable PostgreSQL URL: {e}"))
        .connect()
        .await
    {
        if let Err(error) = conn
            .execute(format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)").as_str())
            .await
        {
            eprintln!("warning: could not drop PostgreSQL test database {name}: {error}");
        }
    }
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

/// Run `body` with a freshly created PostgreSQL database, or skip when no server is configured.
async fn with_postgres<F, Fut>(tag: &str, body: F)
where
    F: FnOnce(Database) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let Ok(server_url) = std::env::var(SERVER_URL_ENV) else {
        println!(
            "SKIPPED: {SERVER_URL_ENV} is not set, so the PostgreSQL half of this contract was NOT \
             exercised. Set it to a superuser URL (e.g. \
             postgres://postgres:postgres@localhost:5432/postgres) to run it."
        );
        return;
    };

    let name = unique_db_name(tag);
    let url = create_database(&server_url, &name).await;

    let db = create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("open the PostgreSQL test database: {e}"));
    UserDatabase::init(&db)
        .await
        .expect("UserDatabase::init must create the user_* schema on PostgreSQL");

    body(db).await;

    drop_database(&server_url, &name).await;
}

#[tokio::test]
async fn accounts_round_trip_through_the_real_postgres_schema() {
    with_postgres("accounts", |db| async move {
        let user = account("pg-u-1", "pg_alice", 5_000_000_000_i64, 36_200_000_000_i64);
        UserDatabase::create_user(&db, &user)
            .await
            .expect("real INSERT into user_accounts on PostgreSQL");

        let stored = UserDatabase::get_user_by_id(&db, "pg-u-1")
            .await
            .expect("real SELECT from user_accounts on PostgreSQL")
            .expect("the account that was just created must be found");

        assert_eq!(stored.id, "pg-u-1");
        assert_eq!(stored.username, "pg_alice");
        assert_eq!(stored.email.as_deref(), Some("pg_alice@example.com"));
        assert_eq!(stored.status, 1);
        assert_eq!(stored.preferred_currency.as_deref(), Some("CNY"));
        assert_eq!(stored.balance_usd, 5_000_000_000);
        assert_eq!(stored.balance_cny, 36_200_000_000);
        assert!(
            stored.balance_usd > i32::MAX as i64,
            "the fixture must exceed i32 so an INTEGER column cannot pass this test"
        );
        assert_eq!(stored.password_hash.as_deref(), Some("$2b$12$notarealhash"));

        let by_name = UserDatabase::get_user_by_username(&db, "pg_alice")
            .await
            .expect("lookup by username")
            .expect("the account must be found by username");
        assert_eq!(by_name.id, "pg-u-1");
        assert_eq!(by_name.balance_usd, 5_000_000_000);

        let by_email = UserDatabase::get_user_by_email(&db, "pg_alice@example.com")
            .await
            .expect("lookup by email")
            .expect("the account must be found by email");
        assert_eq!(by_email.id, "pg-u-1");
    })
    .await;
}

#[tokio::test]
async fn a_missing_account_is_none_on_postgres_not_an_error() {
    with_postgres("absent", |db| async move {
        let missing = UserDatabase::get_user_by_id(&db, "pg-does-not-exist")
            .await
            .expect("a lookup for a missing row must not be an error");
        assert!(missing.is_none(), "a missing account must read as None");
    })
    .await;
}
