#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration test: an unwrap/expect failure is the intended failure signal"
)]
//! PostgreSQL half of the `database-channel` dialect contract (#658).
//!
//! `sqlite_row_mapping.rs` and `protocol_config.rs` already pin the same contracts on SQLite. This
//! file executes the backend-only SQL that SQLite can never reach: `RETURNING id`, PostgreSQL quoted
//! identifiers, `ON CONFLICT ... DO UPDATE`, and `is_default = FALSE`.
//!
//! Timestamp convention: `channel_protocol_configs.created_at` / `updated_at` use the millisecond
//! integer convention (`BIGINT` on PostgreSQL, `INTEGER` on SQLite), not the `TIMESTAMP` convention
//! used by `router_logs`. `ChannelProtocolConfigModel::upsert` writes millisecond integers into those
//! columns, and the assertions below make that cross-backend column contract explicit.

use burncloud_database::sqlx::{self, ConnectOptions, Executor};
use burncloud_database::{create_database_with_url, Database};
use burncloud_supply_channel::{
    ChannelProtocolConfigInput, ChannelProtocolConfigModel, ChannelProviderModel,
};
use burncloud_supply_contracts::{Channel, ChannelType};
use std::str::FromStr;

const SERVER_URL_ENV: &str = "BURNCLOUD_TEST_POSTGRES_URL";

fn unique_db_name(tag: &str) -> String {
    format!(
        "bc_dbc_pg_{}_{}_{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

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

async fn create_database(server_url: &str, name: &str) -> String {
    let mut conn = sqlx::postgres::PgConnectOptions::from_str(server_url)
        .unwrap_or_else(|e| panic!("{SERVER_URL_ENV} is not a usable PostgreSQL URL: {e}"))
        .connect()
        .await
        .unwrap_or_else(|e| panic!("could not connect to {server_url}: {e}"));

    conn.execute(format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)").as_str())
        .await
        .ok();
    conn.execute(format!("CREATE DATABASE {name}").as_str())
        .await
        .unwrap_or_else(|e| panic!("CREATE DATABASE {name}: {e}"));
    url_with_database(server_url, name)
}

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

async fn with_postgres<F, Fut>(tag: &str, body: F)
where
    F: FnOnce(Database) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let Ok(server_url) = std::env::var(SERVER_URL_ENV) else {
        println!(
            "SKIPPED: {SERVER_URL_ENV} is not set, so the database-channel PostgreSQL contract was NOT \
             exercised. Set it to a PostgreSQL superuser URL to run it."
        );
        return;
    };

    let name = unique_db_name(tag);
    let url = create_database(&server_url, &name).await;
    let db = create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("open PostgreSQL test database {name}: {e}"));

    body(db).await;
    drop_database(&server_url, &name).await;
}

fn sample_channel() -> Channel {
    Channel {
        id: 0,
        type_: ChannelType::Anthropic as i32,
        key: "pg-secret".to_string(),
        status: 1,
        name: "pg-anthropic".to_string(),
        weight: 17,
        created_time: None,
        test_time: None,
        response_time: None,
        base_url: Some("https://api.anthropic.com/v1".to_string()),
        models: "claude-pg-contract".to_string(),
        group: "pg-vip".to_string(),
        used_quota: 0,
        model_mapping: None,
        priority: 9,
        auto_ban: 1,
        other_info: None,
        tag: None,
        setting: None,
        param_override: None,
        header_override: None,
        remark: None,
        api_version: Some("2024-02-01".to_string()),
        pricing_region: Some("international".to_string()),
        rpm_cap: Some(600),
        tpm_cap: Some(200_000),
        reservation_green: Some(0.5),
        reservation_yellow: Some(0.3),
        reservation_red: Some(0.2),
    }
}

fn protocol_input(
    channel_type: i32,
    api_version: &str,
    is_default: bool,
) -> ChannelProtocolConfigInput {
    ChannelProtocolConfigInput {
        channel_type,
        api_version: api_version.to_string(),
        is_default: Some(is_default),
        chat_endpoint: Some(format!("/{api_version}/chat")),
        embed_endpoint: Some(format!("/{api_version}/embed")),
        models_endpoint: None,
        request_mapping: Some("{\"provider\":\"postgres-contract\"}".to_string()),
        response_mapping: None,
        detection_rules: None,
    }
}

/// GitHub-hosted Code Test does not set BURNCLOUD_TEST_POSTGRES_URL. Run the
/// dialect contracts against a disposable real PostgreSQL 16 container rather than
/// silently counting their opt-in SKIPPED results as acceptance.
struct TestPostgresContainer(String);

impl Drop for TestPostgresContainer {
    fn drop(&mut self) {
        if let Err(error) = std::process::Command::new("docker")
            .args(["rm", "-f", &self.0])
            .status()
        {
            eprintln!("PostgreSQL container cleanup failed: {error}");
        }
    }
}

#[test]
fn github_actions_executes_real_supply_owner_postgres_contracts() {
    if std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true") {
        println!("Local opt-in: set {SERVER_URL_ENV} to run real PostgreSQL contracts.");
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
        .expect("GitHub-hosted CI must provide Docker for real PostgreSQL tests");
    assert!(
        run.status.success(),
        "start PostgreSQL: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let id = String::from_utf8_lossy(&run.stdout).trim().to_owned();
    assert!(
        !id.is_empty(),
        "Docker did not return a PostgreSQL container ID"
    );
    let _cleanup = TestPostgresContainer(id.clone());
    let output = std::process::Command::new("docker")
        .args(["port", &id, "5432/tcp"])
        .output()
        .expect("read PostgreSQL port");
    assert!(
        output.status.success(),
        "published PostgreSQL port query failed"
    );
    let ports = String::from_utf8_lossy(&output.stdout);
    let port = ports
        .lines()
        .next()
        .and_then(|line| line.rsplit_once(':'))
        .map(|(_, port)| port)
        .expect("PostgreSQL must publish a TCP port");
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let opts = sqlx::postgres::PgConnectOptions::from_str(&url).expect("PostgreSQL URL must parse");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("SQLx readiness runtime");
    let mut ready = false;
    for _ in 0..40 {
        ready = runtime.block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(3), async {
                match opts.connect().await {
                    Ok(mut conn) => conn.execute("SELECT 1").await.is_ok(),
                    Err(_) => false,
                }
            })
            .await
            .unwrap_or(false)
        });
        if ready {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    assert!(ready, "real PostgreSQL 16 did not become SQLx-ready");
    let binary = std::env::current_exe().expect("current integration-test binary");
    for test in [
        "channel_create_returns_the_postgres_id_and_round_trips_quoted_columns",
        "failed_channel_writes_roll_back_atomically_on_postgres",
        "protocol_upsert_uses_postgres_conflict_and_default_demote_branches",
    ] {
        let output = std::process::Command::new(&binary)
            .args(["--exact", test, "--nocapture"])
            .env(SERVER_URL_ENV, &url)
            .output()
            .expect("spawn isolated real PostgreSQL test");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("1 passed; 0 failed"),
            "real PostgreSQL contract {test} failed:\nstdout:\n{stdout}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        println!("Real PostgreSQL 16: {test}: PASS");
    }
}

#[tokio::test]
async fn channel_create_returns_the_postgres_id_and_round_trips_quoted_columns() {
    with_postgres("provider", |db| async move {
        let mut channel = sample_channel();
        let id = ChannelProviderModel::create(&db, &mut channel)
            .await
            .expect("PostgreSQL INSERT ... RETURNING id must succeed");
        assert!(id > 0, "PostgreSQL must return the generated channel id");
        assert_eq!(channel.id, id, "create also updates the domain object's id");

        let stored = ChannelProviderModel::get_by_id(&db, id)
            .await
            .expect("PostgreSQL SELECT with quoted type/group columns")
            .expect("the inserted channel must be found");
        assert_eq!(stored.id, id);
        assert_eq!(stored.type_, ChannelType::Anthropic as i32);
        assert_eq!(stored.group, "pg-vip");
        assert_eq!(stored.name, "pg-anthropic");
        assert_eq!(stored.priority, 9);
        assert_eq!(stored.tpm_cap, Some(200_000));
        assert_eq!(stored.api_version.as_deref(), Some("2024-02-01"));
        assert_eq!(stored.reservation_green, Some(0.5));
        assert!(
            (stored.reservation_yellow.unwrap_or_default() - 0.3).abs() < 0.000001,
            "PostgreSQL REAL values must decode to f64 without loss beyond float4 precision"
        );
        let page = ChannelProviderModel::list(&db, 100, 0)
            .await
            .expect("PostgreSQL list must decode REAL columns");
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].id, id);
        let by_ids = ChannelProviderModel::list_by_ids(&db, &[id])
            .await
            .expect("PostgreSQL list_by_ids must decode REAL columns");
        assert_eq!(by_ids.len(), 1);
        assert_eq!(by_ids[0].id, id);

        let abilities = burncloud_supply_channel::ChannelAbilityModel::list_by_channel(&db, id)
            .await
            .expect(
                "ability rows created by ChannelProviderModel::create must decode on PostgreSQL",
            );
        assert_eq!(abilities.len(), 1);
        assert_eq!(abilities[0].group, "pg-vip");
        assert_eq!(abilities[0].model, "claude-pg-contract");
        let enabled_models =
            burncloud_supply_channel::ChannelAbilityModel::list_distinct_models(&db)
                .await
                .expect("PostgreSQL BOOLEAN enabled predicate must execute");
        assert!(
            enabled_models
                .iter()
                .any(|model| model == "claude-pg-contract"),
            "enabled channel model must be discoverable on PostgreSQL"
        );
    })
    .await;
}

#[tokio::test]
async fn failed_channel_writes_roll_back_atomically_on_postgres() {
    with_postgres("provider_atomicity", |db| async move {
        let mut failed = sample_channel();
        failed.name = "pg-failed-create".to_string();
        failed.models = "same,same".to_string();

        assert!(
            ChannelProviderModel::create(&db, &mut failed)
                .await
                .is_err(),
            "duplicate abilities must keep returning an error"
        );
        assert_eq!(
            failed.id, 0,
            "failed creation must not publish the rolled-back PostgreSQL id"
        );
        assert_eq!(
            failed.created_time, None,
            "failed creation must leave the caller timestamp unchanged"
        );
        assert!(
            ChannelProviderModel::list(&db, 100, 0)
                .await
                .expect("list providers after failed create")
                .is_empty(),
            "provider INSERT must roll back with the failed ability INSERT"
        );

        let conn = db.get_connection().expect("PostgreSQL connection");
        let ability_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM channel_abilities")
            .fetch_one(conn.pool())
            .await
            .expect("count abilities after failed create");
        assert_eq!(
            ability_count.0, 0,
            "the first ability INSERT must roll back with the duplicate failure"
        );

        let mut existing = sample_channel();
        existing.name = "pg-before".to_string();
        existing.models = "old".to_string();
        let id = ChannelProviderModel::create(&db, &mut existing)
            .await
            .expect("control channel create");

        existing.name = "pg-after".to_string();
        existing.models = "same,same".to_string();
        assert!(
            ChannelProviderModel::update(&db, &existing).await.is_err(),
            "duplicate abilities must fail the update"
        );

        let stored = ChannelProviderModel::get_by_id(&db, id)
            .await
            .expect("read provider after failed update")
            .expect("provider remains");
        assert_eq!(
            stored.name, "pg-before",
            "provider UPDATE must roll back with ability synchronization"
        );
        assert_eq!(stored.models, "old");

        let abilities = burncloud_supply_channel::ChannelAbilityModel::list_by_channel(&db, id)
            .await
            .expect("read abilities after failed update");
        assert_eq!(abilities.len(), 1);
        assert_eq!(abilities[0].group, "pg-vip");
        assert_eq!(
            abilities[0].model, "old",
            "ability DELETE and partial rebuild must roll back too"
        );
    })
    .await;
}

#[tokio::test]
async fn protocol_upsert_uses_postgres_conflict_and_default_demote_branches() {
    with_postgres("protocol", |db| async move {
        let channel_type = 77;

        ChannelProtocolConfigModel::upsert(&db, &protocol_input(channel_type, "v1", true))
            .await
            .expect("first PostgreSQL protocol upsert");

        let first = ChannelProtocolConfigModel::get_by_type_version(&db, channel_type, "v1")
            .await
            .expect("read first protocol config")
            .expect("v1 must exist");
        assert!(first.is_default_bool());
        assert!(
            first.created_at.unwrap_or_default() > 0,
            "PostgreSQL BIGINT created_at must carry the millisecond timestamp written by upsert"
        );
        assert!(
            first.updated_at.unwrap_or_default() > 0,
            "PostgreSQL BIGINT updated_at must carry the millisecond timestamp written by upsert"
        );

        // Same (type, version) must execute ON CONFLICT ... DO UPDATE rather than insert a second row.
        let mut rewritten = protocol_input(channel_type, "v1", true);
        rewritten.chat_endpoint = Some("/v1/changed".to_string());
        ChannelProtocolConfigModel::upsert(&db, &rewritten)
            .await
            .expect("PostgreSQL ON CONFLICT DO UPDATE branch");
        let rewritten = ChannelProtocolConfigModel::get_by_type_version(&db, channel_type, "v1")
            .await
            .expect("read rewritten config")
            .expect("v1 must still exist");
        assert_eq!(rewritten.id, first.id, "upsert keeps the same row identity");
        assert_eq!(rewritten.chat_endpoint.as_deref(), Some("/v1/changed"));

        // Making v2 default executes the PostgreSQL-only `is_default = FALSE` demotion statement.
        ChannelProtocolConfigModel::upsert(&db, &protocol_input(channel_type, "v2", true))
            .await
            .expect("v2 default upsert");
        let old = ChannelProtocolConfigModel::get_by_type_version(&db, channel_type, "v1")
            .await
            .expect("read demoted v1")
            .expect("v1 remains present");
        assert!(
            !old.is_default_bool(),
            "PostgreSQL `is_default = FALSE` must demote the previous default"
        );
        let current = ChannelProtocolConfigModel::get_default(&db, channel_type)
            .await
            .expect("read current default")
            .expect("v2 must be the new default");
        assert_eq!(current.api_version, "v2");

        let rows: Vec<_> = ChannelProtocolConfigModel::list(&db, 200, 0)
            .await
            .expect("list protocol configs")
            .into_iter()
            .filter(|row| row.channel_type == channel_type)
            .collect();
        assert_eq!(rows.len(), 2, "v1 and v2 are distinct rows");
        assert_eq!(
            rows.iter().filter(|row| row.is_default_bool()).count(),
            1,
            "exactly one PostgreSQL row remains default"
        );
    })
    .await;
}
