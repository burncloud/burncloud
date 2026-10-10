//! Contract boundary for #845: infrastructure fixups must not create domain records.
//!
//! #842 moved default-record ownership out of this crate entirely. `identity/user` owns the
//! demo account and demo API key, `supply/channel` owns the default protocol configs, and the
//! application bootstrap sequences those calls. This file therefore asserts only Platform's
//! half of the contract: **creating a database writes no domain default records**, on SQLite
//! and on real PostgreSQL 16, and re-initialization is stable.
//!
//! The complementary half — that the owner capabilities still produce the historical one
//! account, one key and four protocol configs — is asserted by
//! `crates/interfaces/server/tests/owner_seed_contract.rs`. It cannot live here: `deny.toml`
//! bans `burncloud-supply-channel` as a dependency of `burncloud-database` (see the
//! `[[bans.deny]]` entry and its `wrappers` allow-list), which is exactly the one-way
//! dependency direction #842 requires. `burncloud-server` is on that allow-list, so the
//! owner-seeded bootstrap contract is tested there.
use burncloud_database::{create_database_with_url, create_infrastructure_database_with_url, sqlx};
use std::error::Error;

async fn count(
    db: &burncloud_database::Database,
    table: &str,
    predicate: &str,
) -> Result<i64, Box<dyn Error>> {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE {predicate}");
    Ok(sqlx::query_scalar(&sql)
        .fetch_one(db.get_connection()?.pool())
        .await?)
}

/// Assert that no domain default record exists.
async fn assert_no_seeds(db: &burncloud_database::Database) -> Result<(), Box<dyn Error>> {
    let users = count(db, "user_accounts", "id = 'demo-user'").await?;
    let tokens = count(db, "user_api_keys", "key = 'sk-burncloud-demo'").await?;
    let protocols = count(db, "channel_protocol_configs", "1 = 1").await?;
    assert_eq!(users, 0, "Platform must not create the demo account");
    assert_eq!(tokens, 0, "Platform must not create the demo API key");
    assert_eq!(
        protocols, 0,
        "Platform must not create default protocol configs"
    );
    Ok(())
}

#[tokio::test]
async fn database_factories_create_schema_without_domain_default_records(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("init-boundary.sqlite");
    let url = format!(
        "sqlite:///{}?mode=rwc",
        path.to_string_lossy().replace('\\', "/")
    );

    // Both factories are seed-free (#842): they run migrations and historic fixups only.
    let infrastructure = create_infrastructure_database_with_url(&url).await?;
    assert_no_seeds(&infrastructure).await?;
    infrastructure.close().await?;

    let factory = create_database_with_url(&url).await?;
    assert_no_seeds(&factory).await?;
    factory.close().await?;

    // Re-initialization must remain seed-free and must not disturb existing rows. A row is
    // inserted deliberately so "still zero" cannot pass by the table being empty.
    let reused = create_database_with_url(&url).await?;
    sqlx::query(
        "INSERT INTO user_accounts (id, username, password_hash, status) \
         VALUES ('kept-user', 'kept-user', 'no-login', 1)",
    )
    .execute(reused.get_connection()?.pool())
    .await?;
    reused.close().await?;

    let reopened = create_infrastructure_database_with_url(&url).await?;
    assert_no_seeds(&reopened).await?;
    assert_eq!(
        count(&reopened, "user_accounts", "id = 'kept-user'").await?,
        1,
        "re-initialization must not remove an existing row"
    );
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

#[tokio::test]
async fn postgres_fresh_init_creates_schema_without_domain_default_records(
) -> Result<(), Box<dyn Error>> {
    use sqlx::{ConnectOptions, Executor};
    use std::str::FromStr;

    // This contract must execute against an actual PostgreSQL server in CI.
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
    let name = format!("bc_init842_{}_{}", std::process::id(), stamp);
    let mut admin = options.connect().await?;
    admin
        .execute(format!("CREATE DATABASE {name}").as_str())
        .await?;
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/{name}");

    let infrastructure = create_infrastructure_database_with_url(&url).await?;
    assert_no_seeds(&infrastructure).await?;
    infrastructure.close().await?;

    let factory = create_database_with_url(&url).await?;
    assert_no_seeds(&factory).await?;

    // An existing row must survive re-initialization.
    sqlx::query(
        "INSERT INTO user_accounts (id, username, password_hash, status) \
         VALUES ('kept-user', 'kept-user', 'no-login', 1)",
    )
    .execute(factory.get_connection()?.pool())
    .await?;
    factory.close().await?;

    let reopened = create_infrastructure_database_with_url(&url).await?;
    assert_no_seeds(&reopened).await?;
    assert_eq!(
        count(&reopened, "user_accounts", "id = 'kept-user'").await?,
        1,
        "re-initialization must not remove an existing row"
    );
    reopened.close().await?;

    admin
        .execute(format!("DROP DATABASE {name} WITH (FORCE)").as_str())
        .await?;
    println!("Real PostgreSQL 16: seed-free infrastructure contract PASS");
    Ok(())
}
