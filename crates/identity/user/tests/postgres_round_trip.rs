#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration-test file: fail-fast setup is the intended behaviour"
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
use burncloud_service_user::{BalanceModel, UserAccount, UserDatabase};
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
    // The database is the last path segment: scheme://[user[:pass]@]host[:port]/dbname
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

    // A leftover database from a crashed run would make CREATE fail; drop first, ignoring the outcome.
    conn.execute(format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)").as_str())
        .await
        .ok();
    conn.execute(format!("CREATE DATABASE {name}").as_str())
        .await
        .unwrap_or_else(|e| panic!("CREATE DATABASE {name}: {e}"));

    url_with_database(server_url, name)
}

/// Drop the throwaway database, tolerating an already-dropped one.
async fn drop_database(server_url: &str, name: &str) {
    if let Ok(mut conn) = sqlx::postgres::PgConnectOptions::from_str(server_url)
        .unwrap_or_else(|e| panic!("{SERVER_URL_ENV} is not a usable PostgreSQL URL: {e}"))
        .connect()
        .await
    {
        conn.execute(format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)").as_str())
            .await
            .ok();
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
        // Explicit skip: the plan requires a missing environment to read as "not run".
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
    // The production initializer, so this exercises the real migrations rather than a hand-made schema.
    UserDatabase::init(&db)
        .await
        .expect("UserDatabase::init must create the user_* schema on PostgreSQL");

    body(db).await;

    drop_database(&server_url, &name).await;
}

/// Exercise the Identity wallet contract on an actual disposable PostgreSQL 16
/// instance using the existing GitHub-hosted Test gate (no workflow edit).
/// Missing PostgreSQL must not masquerade as a passing integration contract.
struct TestPostgresContainer(String);

impl Drop for TestPostgresContainer {
    fn drop(&mut self) {
        match std::process::Command::new("docker")
            .args(["rm", "-f", &self.0])
            .status()
        {
            Ok(status) if !status.success() => {
                eprintln!("Docker PostgreSQL cleanup failed: {status}");
            }
            Err(error) => eprintln!("Docker PostgreSQL cleanup failed: {error}"),
            _ => {}
        }
    }
}

#[test]
fn github_actions_runs_real_identity_wallet_contracts() {
    if std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true") {
        println!("Real PostgreSQL CI runner is GitHub Actions only. Set {SERVER_URL_ENV} to run the individual PostgreSQL contracts locally.");
        return;
    }

    let run = std::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "-d",
            "-e",
            "POSTGRES_PASSWORD=postgres",
            "-e",
            "POSTGRES_USER=postgres",
            "-e",
            "POSTGRES_DB=postgres",
            "-p",
            "127.0.0.1::5432",
            "postgres:16",
        ])
        .output()
        .expect("GitHub-hosted runner must provide Docker for the real PostgreSQL contract");
    assert!(
        run.status.success(),
        "could not start PostgreSQL 16: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let id = String::from_utf8_lossy(&run.stdout).trim().to_string();
    assert!(!id.is_empty(), "Docker must return a container ID");
    let _cleanup = TestPostgresContainer(id.clone());

    let port_output = std::process::Command::new("docker")
        .args(["port", &id, "5432/tcp"])
        .output()
        .expect("read the published PostgreSQL port");
    assert!(port_output.status.success(), "Docker port lookup failed");
    let port_line = String::from_utf8_lossy(&port_output.stdout);
    let port = port_line
        .lines()
        .next()
        .and_then(|line| line.rsplit_once(':'))
        .map(|(_, port)| port)
        .expect("expected a published PostgreSQL port");
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    // pg_isready inside the container can succeed before Docker's published
    // host port accepts connections. Probe the actual SQLx connection path.
    let opts = sqlx::postgres::PgConnectOptions::from_str(&url)
        .expect("published PostgreSQL port must yield a valid connection URL");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("create SQLx readiness runtime");
    let mut ready = false;
    for _ in 0..40 {
        let connected = runtime.block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(3), async {
                match opts.connect().await {
                    Ok(mut conn) => conn.execute("SELECT 1").await.is_ok(),
                    Err(_) => false,
                }
            })
            .await
            .unwrap_or(false)
        });
        if connected {
            ready = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    assert!(
        ready,
        "PostgreSQL 16 did not accept a SQLx connection from the GitHub-hosted runner"
    );
    let runner = std::env::current_exe().expect("find the current integration test binary");

    for test in [
        "accounts_round_trip_through_the_real_postgres_schema",
        "a_missing_account_is_none_on_postgres_not_an_error",
        "identity_wallet_preserves_debit_variants_on_real_postgres",
    ] {
        let result = std::process::Command::new(&runner)
            .args(["--exact", test, "--nocapture"])
            .env(SERVER_URL_ENV, &url)
            .output()
            .expect("launch isolated PostgreSQL integration test");
        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            result.status.success() && stdout.contains("1 passed; 0 failed"),
            "real PostgreSQL contract {test} did not pass:\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        println!("Real PostgreSQL 16: {test}: PASS");
    }
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

        // 64-bit balance columns. This is the assertion most likely to differ between backends: SQLite
        // stores an 8-byte INTEGER natively, while PostgreSQL needs BIGINT, and an INTEGER column would
        // silently truncate. The value is above i32::MAX so truncation cannot pass unnoticed.
        assert_eq!(stored.balance_usd, 5_000_000_000);
        assert_eq!(stored.balance_cny, 36_200_000_000);
        assert!(
            stored.balance_usd > i32::MAX as i64,
            "the fixture must exceed i32 so an INTEGER column cannot pass this test"
        );

        assert_eq!(stored.password_hash.as_deref(), Some("$2b$12$notarealhash"));

        // The lookup paths must use the same column list as the writer on this backend too.
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

#[tokio::test]
async fn identity_wallet_preserves_debit_variants_on_real_postgres() {
    with_postgres("wallet", |db| async move {
        UserDatabase::create_user(
            &db,
            &account(
                "pg-wallet-1",
                "pg_wallet_user",
                10_000_000_000,
                20_000_000_000,
            ),
        )
        .await
        .expect("PostgreSQL wallet row");

        assert!(BalanceModel::deduct_usd(&db, "pg-wallet-1", 1_000_000_000)
            .await
            .expect("Postgres USD debit"));
        assert!(BalanceModel::deduct_cny(&db, "pg-wallet-1", 2_000_000_000)
            .await
            .expect("Postgres CNY debit"));
        assert!(BalanceModel::deduct_dual_currency(
            &db,
            "pg-wallet-1",
            20_000_000_000,
            "CNY",
            2_000_000_000,
        )
        .await
        .expect("Postgres mixed-currency debit"));

        let charged = UserDatabase::get_user_by_id(&db, "pg-wallet-1")
            .await
            .expect("query wallet")
            .expect("wallet exists");
        assert_eq!(charged.balance_usd, 8_000_000_000);
        assert_eq!(charged.balance_cny, 0);

        assert!(BalanceModel::deduct_dual_currency(
            &db,
            "pg-wallet-missing",
            1,
            "USD",
            2_000_000_000,
        )
        .await
        .is_err());
        assert!(!BalanceModel::deduct_dual_currency_router_legacy(
            &db,
            "pg-wallet-missing",
            1,
            "USD",
            2_000_000_000,
        )
        .await
        .expect("legacy missing-wallet behavior"));

        UserDatabase::create_user(
            &db,
            &account(
                "pg-wallet-2",
                "pg_wallet_legacy",
                3_000_000_000,
                8_000_000_000,
            ),
        )
        .await
        .expect("create legacy wallet");
        assert!(BalanceModel::deduct_dual_currency_router_legacy(
            &db,
            "pg-wallet-2",
            5_000_000_000,
            "USD",
            2_000_000_000,
        )
        .await
        .expect("Postgres legacy mixed-currency debit"));

        let legacy = UserDatabase::get_user_by_id(&db, "pg-wallet-2")
            .await
            .expect("query legacy wallet")
            .expect("legacy wallet exists");
        assert_eq!(legacy.balance_usd, 0);
        assert_eq!(legacy.balance_cny, 4_000_000_000);
    })
    .await;
}
