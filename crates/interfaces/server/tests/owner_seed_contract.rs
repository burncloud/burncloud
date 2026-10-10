//! Owner-seeded startup contract for #842.
//!
//! #842 moved default-record ownership out of `platform/storage/database` and into the
//! domains: `identity/user` owns the demo account and demo API key, `supply/channel` owns the
//! four default protocol configs, and the application bootstrap (`start_server`) sequences
//! those calls after the schema steps.
//!
//! This test lives in `burncloud-server` rather than next to the database crate on purpose.
//! `deny.toml` bans `burncloud-supply-channel` as a dependency of `burncloud-database` and
//! allows only a fixed wrapper list (which includes `burncloud-server`). Asserting the
//! owner-seeded state here is what keeps Platform free of any dependency edge to the owners —
//! the one-way direction #842 requires. Platform's own half of the contract (the factories
//! create no domain records) is asserted in
//! `crates/platform/storage/database/tests/initialization_boundary.rs`.
//!
//! Everything below is asserted through the owners' public read APIs, so this suite needs no
//! direct SQL dependency.
//!
//! Verified on SQLite and on real PostgreSQL 16:
//! 1. the schema-only factory really does start empty;
//! 2. the owner calls reproduce the historical startup state exactly — one demo account,
//!    one demo key, four protocol configs;
//! 3. both owner calls are idempotent, so a restart over an existing database neither
//!    duplicates nor removes a row.
use burncloud_database::{create_database_with_url, Database};
use burncloud_service_user::UserDatabase;
use burncloud_supply_channel::ChannelProtocolConfigModel;
use std::error::Error;

/// Read the three seeded row counts through the owning crates' public APIs.
///
/// Counting, rather than reading whole rows, is deliberate: it detects duplicates as well as
/// absences, and PostgreSQL returns `user_api_keys.key` as `bpchar`, which the `Any` driver
/// cannot decode — so listing credentials fails outright on the real-PostgreSQL run.
async fn seed_counts(db: &Database) -> Result<(i64, i64, i64), Box<dyn Error>> {
    let (demo_users, demo_tokens) = UserDatabase::count_seeded_demo_defaults(db).await?;
    // `ChannelProtocolConfigModel::list` projects `is_default::INTEGER` on PostgreSQL, so it
    // decodes on both backends.
    let protocols = ChannelProtocolConfigModel::list(db, 1_000, 0).await?.len() as i64;
    Ok((demo_users, demo_tokens, protocols))
}

/// Exactly what `start_server` does after its schema steps, in the same order.
async fn seed_like_startup(db: &Database) -> Result<(), Box<dyn Error>> {
    UserDatabase::seed_demo_defaults(db).await?;
    ChannelProtocolConfigModel::seed_default_protocol_configs(db).await?;
    Ok(())
}

/// Compare and fail with a described error rather than panicking: these helpers are called
/// from `Result`-returning test functions, and `clippy::panic_in_result_fn` is part of this
/// workspace's lint set.
fn check(actual: i64, expected: i64, what: &str) -> Result<(), Box<dyn Error>> {
    if actual == expected {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("{what}: expected {expected}, got {actual}")).into())
    }
}

async fn assert_seed_counts(
    db: &Database,
    users: i64,
    tokens: i64,
    protocols: i64,
) -> Result<(), Box<dyn Error>> {
    let actual = seed_counts(db).await?;
    check(actual.0, users, "demo account count")?;
    check(actual.1, tokens, "demo API key count")?;
    check(actual.2, protocols, "default protocol config count")?;
    Ok(())
}

/// A uniquely-named SQLite file for one test.
///
/// The name must be unique per test: `tempfile::tempdir()` gives each test its own directory,
/// but a *fixed* file name inside it is not enough if two tests were ever pointed at one
/// directory, and cargo runs the tests in one binary concurrently. The first version of this
/// suite used a constant `owner-seed.sqlite`; the two tests then shared one database and one
/// test's seed made the other's "starts empty" assertion fail.
fn sqlite_url_for(tag: &str) -> Result<(tempfile::TempDir, String), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join(format!("owner-seed-{tag}.sqlite"));
    let url = format!(
        "sqlite:///{}?mode=rwc",
        path.to_string_lossy().replace('\\', "/")
    );
    Ok((dir, url))
}

/// The bootstrap order is significant: the demo key references `user_id = 'demo-user'`, so
/// the account must exist first. This asserts the intervening state as well as the final one.
#[tokio::test]
async fn owner_seeds_reproduce_the_startup_records_and_are_idempotent() -> Result<(), Box<dyn Error>>
{
    let (_dir, url) = sqlite_url_for("idempotent")?;

    let db = create_database_with_url(&url).await?;
    // Diagnostic: a freshly created database must hold no account at all. If this fails, report
    // what is actually present and which file was used, so the source of the row is identified
    // from evidence rather than guessed.
    let observed: Vec<(String, String)> =
        burncloud_database::sqlx::query_as("SELECT id, username FROM user_accounts ORDER BY id")
            .fetch_all(db.get_connection()?.pool())
            .await?;
    if !observed.is_empty() {
        return Err(std::io::Error::other(format!(
            "a freshly created database already holds accounts {observed:?} at {url}"
        ))
        .into());
    }
    assert_seed_counts(&db, 0, 0, 0).await?;

    // Identity's half: the account and its key appear, Supply's configs do not yet.
    UserDatabase::seed_demo_defaults(&db).await?;
    assert_seed_counts(&db, 1, 1, 0).await?;

    // Supply's half completes the startup state.
    ChannelProtocolConfigModel::seed_default_protocol_configs(&db).await?;
    assert_seed_counts(&db, 1, 1, 4).await?;

    // Re-running both phases (a restart) must not duplicate anything.
    seed_like_startup(&db).await?;
    assert_seed_counts(&db, 1, 1, 4).await?;
    db.close().await?;

    // A genuinely reopened database starts empty again and reaches the same state from the
    // owner calls alone.
    let reopened = create_database_with_url(&url).await?;
    assert_seed_counts(&reopened, 0, 0, 0).await?;
    seed_like_startup(&reopened).await?;
    assert_seed_counts(&reopened, 1, 1, 4).await?;
    reopened.close().await?;
    Ok(())
}

struct DisposablePostgres(String);

impl Drop for DisposablePostgres {
    fn drop(&mut self) {
        if let Err(error) = std::process::Command::new("docker")
            .args(["rm", "-f", &self.0])
            .status()
        {
            eprintln!("PostgreSQL test container cleanup failed: {error}");
        }
    }
}

/// The same contract on real PostgreSQL 16. The two owners use dialect-specific SQL
/// (`ON CONFLICT DO NOTHING` versus `INSERT OR IGNORE`, `$n` versus `?` placeholders) and this
/// suite needs PostgreSQL server administration, so neither is covered by the SQLite test.
#[tokio::test]
async fn owner_seeds_reproduce_the_startup_records_on_real_postgres() -> Result<(), Box<dyn Error>>
{
    use burncloud_database::sqlx::{self, ConnectOptions, Executor};
    use std::str::FromStr;

    if std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true") {
        println!("Local PostgreSQL test requires GITHUB_ACTIONS=true with Docker");
        return Ok(());
    }
    let started = std::process::Command::new("docker")
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
        .output()?;
    if !started.status.success() {
        return Err(std::io::Error::other(format!(
            "start PostgreSQL: {}",
            String::from_utf8_lossy(&started.stderr)
        ))
        .into());
    }
    let id = String::from_utf8(started.stdout)?.trim().to_owned();
    if id.is_empty() {
        return Err(std::io::Error::other("Docker did not provide container ID").into());
    }
    let _container = DisposablePostgres(id.clone());

    let published = std::process::Command::new("docker")
        .args(["port", &id, "5432/tcp"])
        .output()?;
    if !published.status.success() {
        return Err(std::io::Error::other("read PostgreSQL published port").into());
    }
    let ports = String::from_utf8(published.stdout)?;
    let port = ports
        .lines()
        .next()
        .and_then(|line| line.rsplit_once(':'))
        .map(|(_, number)| number)
        .ok_or("PostgreSQL container did not publish its port")?;
    let admin_url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let options = sqlx::postgres::PgConnectOptions::from_str(&admin_url)?;
    let mut ready = false;
    for _ in 0..40 {
        ready = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            match options.connect().await {
                Ok(mut connection) => connection.execute("SELECT 1").await.is_ok(),
                Err(_) => false,
            }
        })
        .await
        .unwrap_or(false);
        if ready {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    if !ready {
        return Err(
            std::io::Error::other("real PostgreSQL container did not become SQLx-ready").into(),
        );
    }

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let name = format!("bc_ownerseed842_{}_{}", std::process::id(), stamp);
    let mut admin = options.connect().await?;
    admin
        .execute(format!("CREATE DATABASE {name}").as_str())
        .await?;
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/{name}");

    let db = create_database_with_url(&url).await?;
    assert_seed_counts(&db, 0, 0, 0).await?;
    seed_like_startup(&db).await?;
    assert_seed_counts(&db, 1, 1, 4).await?;
    // Idempotence on PostgreSQL exercises the ON CONFLICT DO NOTHING branches.
    seed_like_startup(&db).await?;
    assert_seed_counts(&db, 1, 1, 4).await?;
    db.close().await?;

    let reopened = create_database_with_url(&url).await?;
    seed_like_startup(&reopened).await?;
    assert_seed_counts(&reopened, 1, 1, 4).await?;
    reopened.close().await?;

    admin
        .execute(format!("DROP DATABASE {name} WITH (FORCE)").as_str())
        .await?;
    println!("Real PostgreSQL 16: owner-seeded startup contract PASS");
    Ok(())
}
