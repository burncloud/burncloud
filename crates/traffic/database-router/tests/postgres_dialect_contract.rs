#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration-test file: fail-fast setup is the intended behaviour"
)]
//! PostgreSQL half of the `database-router` dialect contract (#658).
//!
//! The matching SQLite contracts already live in `cost_roundtrip.rs`, `usage_stats_tests.rs`,
//! `token_paths.rs`, and `settlement_and_routing_gaps.rs`. These tests deliberately drive the same
//! production APIs against a real PostgreSQL database so backend-only SQL cannot remain unexecuted.
//!
//! Timestamp convention used here: `router_logs.created_at` and `router_video_tasks.created_at` are
//! `TIMESTAMP` on PostgreSQL and `TEXT` on SQLite (migration audit in
//! the #658 PostgreSQL dialect contract). Therefore the production router-log window query
//! must use `EXTRACT(EPOCH FROM created_at)` on PostgreSQL, not the millisecond-integer convention used by
//! tables such as `channel_protocol_configs`.

use burncloud_database::sqlx::{self, ConnectOptions, Executor};
use burncloud_database::{create_database_with_url, Database};
use burncloud_database_router::{
    get_usage_stats, RouterDatabase, RouterLog, RouterLogModel, RouterVideoTask,
    RouterVideoTaskModel,
};
use std::str::FromStr;

const SERVER_URL_ENV: &str = "BURNCLOUD_TEST_POSTGRES_URL";

fn unique_db_name(tag: &str) -> String {
    format!(
        "bc_dbr_pg_{}_{}_{}",
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
            "SKIPPED: {SERVER_URL_ENV} is not set, so the database-router PostgreSQL contract was NOT \
             exercised. Set it to a PostgreSQL superuser URL to run it."
        );
        return;
    };

    let name = unique_db_name(tag);
    let url = create_database(&server_url, &name).await;
    let db = create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("open PostgreSQL test database {name}: {e}"));
    RouterDatabase::init(&db)
        .await
        .expect("RouterDatabase::init must create the router schema on PostgreSQL");

    body(db).await;
    drop_database(&server_url, &name).await;
}

#[tokio::test]
async fn router_log_round_trip_and_time_window_use_the_postgres_timestamp_branch() {
    with_postgres("log", |db| async move {
        let log = RouterLog {
            request_id: "pg-router-log-1".to_string(),
            user_id: Some("pg-router-user".to_string()),
            path: "/v1/chat/completions".to_string(),
            upstream_id: Some("pg-channel-1".to_string()),
            status_code: 200,
            latency_ms: 37,
            prompt_tokens: 1_234,
            completion_tokens: 567,
            cost: 9_876_543_210,
            model: Some("pg-contract-model".to_string()),
            layer_decision: Some("affinity".to_string()),
            traffic_color: Some("g".to_string()),
            cache_read_tokens: 10,
            reasoning_tokens: 20,
            input_cost: 3_000_000_000,
            output_cost: 6_876_543_210,
            ..RouterLog::default()
        };

        RouterLogModel::insert(&db, &log)
            .await
            .expect("32-value PostgreSQL INSERT with $n placeholders must succeed");

        let stored = RouterLogModel::get(&db, 10, 0)
            .await
            .expect("router_logs must decode on PostgreSQL");
        let stored = stored
            .into_iter()
            .find(|row| row.request_id == "pg-router-log-1")
            .expect("the inserted router log must round-trip");
        assert_eq!(stored.prompt_tokens, 1_234);
        assert_eq!(stored.completion_tokens, 567);
        assert_eq!(stored.cost, 9_876_543_210);
        assert_eq!(stored.model.as_deref(), Some("pg-contract-model"));
        assert_eq!(stored.traffic_color.as_deref(), Some("g"));
        assert_eq!(stored.layer_decision.as_deref(), Some("affinity"));
        assert!(
            stored.created_at.is_some(),
            "PostgreSQL TIMESTAMP default must be visible through the production row mapping"
        );

        let filtered = RouterLogModel::get_filtered(
            &db,
            Some("pg-router-user"),
            Some("pg-channel-1"),
            Some("pg-contract-model"),
            10,
            0,
        )
        .await
        .expect("filtered router logs must decode on PostgreSQL");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].traffic_color.as_deref(), Some("g"));
        assert_eq!(filtered[0].layer_decision.as_deref(), Some("affinity"));
        assert!(filtered[0].created_at.is_some());

        let stats = get_usage_stats(&db, "pg-router-user", "day")
            .await
            .expect("EXTRACT(EPOCH FROM created_at) PostgreSQL branch must execute");
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.total_prompt_tokens, 1_234);
        assert_eq!(stats.total_completion_tokens, 567);
        assert_eq!(stats.total_cost_nano, 9_876_543_210);
    })
    .await;
}

#[tokio::test]
async fn legacy_log_insert_does_not_write_identity_credential_spend_on_postgres() {
    with_postgres("log_quota", |db| async move {
        let conn = db.get_connection().expect("PostgreSQL connection");
        sqlx::query(
            "INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota) \
             VALUES ('pg-credential-one', 'pg-user', 'active', -1, 1000000), \
                    ('pg-credential-two', 'pg-user', 'active', -1, 2222222)",
        )
        .execute(conn.pool())
        .await
        .expect("create two Identity credentials for same user");

        let log = RouterLog {
            request_id: "pg-duplicate-log".to_string(),
            user_id: Some("pg-user".to_string()),
            path: "/v1/chat/completions".to_string(),
            prompt_tokens: 123,
            completion_tokens: 456,
            cost: 99_000_000,
            ..RouterLog::default()
        };

        for _ in 0..2 {
            RouterLogModel::insert(&db, &log)
                .await
                .expect("insert duplicate log with PostgreSQL placeholders");
        }

        let spent: Vec<(String, i64)> = sqlx::query_as(
            "SELECT token, used_quota FROM router_tokens WHERE user_id = $1 ORDER BY token",
        )
        .bind("pg-user")
        .fetch_all(conn.pool())
        .await
        .expect("read authoritative Identity spend after logging");

        assert_eq!(
            spent,
            vec![
                ("pg-credential-one".to_string(), 1_000_000),
                ("pg-credential-two".to_string(), 2_222_222)
            ],
            "PostgreSQL log insertion must not charge or cross-charge credentials"
        );

        let saved = RouterLogModel::get(&db, 10, 0)
            .await
            .expect("read duplicate logs");
        assert_eq!(saved.len(), 2, "both log rows should persist");
        assert!(saved.iter().all(|row| row.traffic_color.is_none()));
        assert!(saved.iter().all(|row| row.created_at.is_some()));

        let filtered = RouterLogModel::get_filtered(&db, Some("pg-user"), None, None, 10, 0)
            .await
            .expect("nullable PostgreSQL color must decode in filtered logs");
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|row| row.traffic_color.is_none()));
    })
    .await;
}

#[tokio::test]
async fn token_info_join_quotes_group_and_preserves_the_left_join_on_postgres() {
    with_postgres("token_join", |db| async move {
        let conn = db.get_connection().expect("database connection");
        sqlx::query(
            r#"INSERT INTO user_accounts
               (id, username, password_hash, status, "group", balance_usd, balance_cny)
               VALUES ($1, $2, $3, 1, $4, 0, 0)"#,
        )
        .bind("pg-join-user")
        .bind("pg_join_user")
        .bind("$2b$12$placeholder")
        .bind("vip")
        .execute(conn.pool())
        .await
        .expect("insert PostgreSQL user account");

        sqlx::query(
            r#"INSERT INTO user_api_keys
               (user_id, key, status, name, remain_quota, unlimited_quota, used_quota,
                created_time, accessed_time, expired_time)
               VALUES ($1, $2, 1, 'pg-contract', 123456789, 0, 7, 0, 0, -1)"#,
        )
        .bind("pg-join-user")
        .bind("pg-live-key")
        .execute(conn.pool())
        .await
        .expect("insert PostgreSQL API key");

        // Deliberately do not insert router_tokens. The production query LEFT JOINs that table; the
        // credential must still resolve, with classifier fields absent rather than the whole row lost.
        let info = RouterDatabase::validate_token_and_get_info(&db, "pg-live-key")
            .await
            .expect("$1 placeholder and quoted group JOIN must execute")
            .expect("the active user_api_keys row must resolve without a router_tokens row");
        assert_eq!(info.user_id, "pg-join-user");
        assert_eq!(info.group, "vip");
        assert_eq!(info.remain_quota, 123_456_789);
        assert_eq!(info.used_quota, 7);
        assert_eq!(info.order_type, None, "LEFT JOIN absence stays None");
        assert_eq!(info.price_cap, None, "LEFT JOIN absence stays None");
    })
    .await;
}

#[tokio::test]
async fn video_task_conflict_keeps_the_first_mapping_on_postgres() {
    with_postgres("video", |db| async move {
        let first = RouterVideoTask {
            task_id: "pg-video-task".to_string(),
            channel_id: 11,
            user_id: Some("pg-user".to_string()),
            model: Some("video-model".to_string()),
            duration: 8,
            resolution: "1080p".to_string(),
        };
        let duplicate = RouterVideoTask {
            channel_id: 99,
            ..first.clone()
        };

        RouterVideoTaskModel::save(&db, &first)
            .await
            .expect("first PostgreSQL video-task INSERT");
        RouterVideoTaskModel::save(&db, &duplicate)
            .await
            .expect("ON CONFLICT (task_id) DO NOTHING must accept a duplicate");

        let stored = RouterVideoTaskModel::get_by_task_id(&db, "pg-video-task")
            .await
            .expect("PostgreSQL video-task SELECT")
            .expect("task must remain present");
        assert_eq!(
            stored.channel_id, 11,
            "the PostgreSQL conflict branch must keep the first routing target"
        );
        assert_eq!(stored.model.as_deref(), Some("video-model"));
        assert_eq!(stored.duration, 8);
    })
    .await;
}
