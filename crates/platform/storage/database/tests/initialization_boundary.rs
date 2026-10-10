//! Contract boundary for #845: infrastructure fixups and legacy seed behavior.
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

async fn assert_seeds(
    db: &burncloud_database::Database,
    expected_users: i64,
    expected_tokens: i64,
    expected_protocols: i64,
) -> Result<(), Box<dyn Error>> {
    let users = count(db, "user_accounts", "id = 'demo-user'").await?;
    let tokens = count(db, "user_api_keys", "key = 'sk-burncloud-demo'").await?;
    let protocols = count(db, "channel_protocol_configs", "1 = 1").await?;
    assert_eq!(users, expected_users);
    assert_eq!(tokens, expected_tokens);
    assert_eq!(protocols, expected_protocols);
    Ok(())
}

#[tokio::test]
async fn infrastructure_bootstrap_is_seed_free_and_legacy_factory_is_compatible(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("init-boundary.sqlite");
    let url = format!(
        "sqlite:///{}?mode=rwc",
        path.to_string_lossy().replace('\\', "/")
    );

    let infrastructure = create_infrastructure_database_with_url(&url).await?;
    assert_seeds(&infrastructure, 0, 0, 0).await?;
    infrastructure.close().await?;

    let legacy = create_database_with_url(&url).await?;
    assert_seeds(&legacy, 1, 1, 4).await?;
    legacy.close().await?;

    // Reinitialization must not remove, duplicate or overwrite seeded data.
    let reopened = create_infrastructure_database_with_url(&url).await?;
    assert_seeds(&reopened, 1, 1, 4).await?;
    reopened.close().await?;

    let legacy_again = create_database_with_url(&url).await?;
    assert_seeds(&legacy_again, 1, 1, 4).await?;
    legacy_again.close().await?;
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
async fn postgres_infrastructure_bootstrap_retains_legacy_seed_contract(
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
        return Err(std::io::Error::other("real PostgreSQL container did not become SQLx-ready").into());
    }

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let name = format!("bc_init845_{}_{}", std::process::id(), stamp);
    let mut admin = options.connect().await?;
    admin
        .execute(format!("CREATE DATABASE {name}").as_str())
        .await?;
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/{name}");

    let infrastructure = create_infrastructure_database_with_url(&url).await?;
    assert_seeds(&infrastructure, 0, 0, 0).await?;
    infrastructure.close().await?;

    let legacy = create_database_with_url(&url).await?;
    assert_seeds(&legacy, 1, 1, 4).await?;
    legacy.close().await?;

    let reopened = create_infrastructure_database_with_url(&url).await?;
    assert_seeds(&reopened, 1, 1, 4).await?;
    reopened.close().await?;

    let legacy_again = create_database_with_url(&url).await?;
    assert_seeds(&legacy_again, 1, 1, 4).await?;
    legacy_again.close().await?;
    admin
        .execute(format!("DROP DATABASE {name} WITH (FORCE)").as_str())
        .await?;
    println!("Real PostgreSQL 16: infrastructure and legacy seed contracts PASS");
    Ok(())
}
