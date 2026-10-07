#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration test: an unwrap/expect failure is the intended failure signal"
)]
//! Real database mapping for the channel rows (S1-C), mirroring the Commerce billing test.
//!
//! The Supply contract no longer carries `FromRow`: the row structs live in this crate as
//! `pub(crate)`. Parity between the Rust types is checked inside the crate; what can only be checked
//! here is that sqlx actually decodes the real columns of `channel_providers` and
//! `channel_abilities`, including the dialect-specific quoting (`type as type_`, `` `group` ``) and
//! the nullable columns added by later migrations.

use burncloud_database::{create_database_with_url, Database};
use burncloud_database_channel::ChannelProviderModel;
use burncloud_supply_contracts::{Channel, ChannelType};

/// A fresh SQLite file database with the real migrations applied.
///
/// A file rather than `:memory:`: the pool opens several connections and an in-memory SQLite
/// database is per-connection, so migrations and assertions would see different databases. On
/// Windows the URL needs the `sqlite:///C:/...` form (see `Database::new`).
async fn fresh_db(tag: &str) -> (Database, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "bc_channel_{}_{}_{}.db",
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

fn sample_channel(name: &str, group: &str) -> Channel {
    Channel {
        id: 0,
        type_: ChannelType::Anthropic as i32,
        key: "sk-secret".to_string(),
        status: 1,
        name: name.to_string(),
        weight: 10,
        created_time: None,
        test_time: Some(1_700_000_000),
        response_time: Some(120),
        base_url: Some("https://api.anthropic.com/v1".to_string()),
        models: "claude-3-5-sonnet,claude-3-haiku".to_string(),
        group: group.to_string(),
        used_quota: 42_000,
        // Mapping keys and values are both routable aliases. The target here is distinct so this
        // round-trip test observes both rows independently; overlap is covered in ability_sync.rs.
        model_mapping: Some(r#"{"claude-3":"claude-3-5-sonnet-2024"}"#.to_string()),
        priority: 7,
        auto_ban: 0,
        other_info: Some(r#"{"owner":"supply"}"#.to_string()),
        tag: Some("prod".to_string()),
        setting: Some(r#"{"retry":2}"#.to_string()),
        param_override: None,
        header_override: None,
        remark: Some("primary".to_string()),
        api_version: Some("2024-02-01".to_string()),
        pricing_region: Some("international".to_string()),
        rpm_cap: Some(600),
        tpm_cap: Some(200_000),
        reservation_green: Some(0.5),
        reservation_yellow: Some(0.3),
        reservation_red: Some(0.2),
    }
}

#[tokio::test]
async fn channel_survives_a_real_insert_and_select() {
    let (db, path) = fresh_db("channel").await;
    let mut channel = sample_channel("claude-prod", "default");

    let id = ChannelProviderModel::create(&db, &mut channel)
        .await
        .expect("real INSERT into channel_providers");
    assert!(id > 0, "SQLite assigned a row id");

    let stored = ChannelProviderModel::get_by_id(&db, id)
        .await
        .expect("real SELECT from channel_providers")
        .expect("the row that was just inserted must be found");

    // The domain type comes back whole: every column decodes, including the `type as type_` alias.
    assert_eq!(stored.id, id);
    assert_eq!(stored.type_, ChannelType::Anthropic as i32);
    assert_eq!(stored.key, "sk-secret");
    assert_eq!(stored.name, "claude-prod");
    assert_eq!(stored.status, 1);
    assert_eq!(stored.weight, 10);
    assert_eq!(stored.models, "claude-3-5-sonnet,claude-3-haiku");
    assert_eq!(stored.group, "default");
    assert_eq!(stored.priority, 7);
    // Runtime/accounting columns keep their existing ownership: Supply creation does not overwrite
    // probe state or Commerce-owned usage. Supply-owned configuration, however, must round-trip.
    assert_eq!(
        stored.test_time, None,
        "probe-owned test_time is not written by Supply create"
    );
    assert_eq!(
        stored.response_time, None,
        "probe latency is not written by create"
    );
    assert_eq!(
        stored.used_quota, 0,
        "Commerce-owned quota ignores the supplied 42_000 and keeps DEFAULT 0"
    );
    assert_eq!(
        stored.model_mapping.as_deref(),
        Some(r#"{"claude-3":"claude-3-5-sonnet-2024"}"#),
        "model_mapping is Supply configuration and must be persisted"
    );
    assert_eq!(
        stored.auto_ban, 1,
        "runtime auto-ban ignores the supplied 0 and keeps DEFAULT 1"
    );
    assert_eq!(
        stored.other_info.as_deref(),
        Some(r#"{"owner":"supply"}"#),
        "other_info must round-trip"
    );
    assert_eq!(stored.tag.as_deref(), Some("prod"), "tag must round-trip");
    assert_eq!(
        stored.setting.as_deref(),
        Some(r#"{"retry":2}"#),
        "setting must round-trip"
    );
    assert_eq!(
        stored.remark.as_deref(),
        Some("primary"),
        "remark must round-trip"
    );

    // Columns that ARE persisted come back intact.
    assert_eq!(
        stored.base_url.as_deref(),
        Some("https://api.anthropic.com/v1")
    );
    assert_eq!(stored.api_version.as_deref(), Some("2024-02-01"));
    assert_eq!(stored.pricing_region.as_deref(), Some("international"));
    assert_eq!(stored.rpm_cap, Some(600));
    assert_eq!(stored.tpm_cap, Some(200_000));
    assert_eq!(stored.reservation_green, Some(0.5));
    assert_eq!(stored.reservation_yellow, Some(0.3));
    assert_eq!(stored.reservation_red, Some(0.2));
    assert!(
        stored.created_time.is_some(),
        "created_time is stamped by the writer"
    );

    cleanup(db, &path).await;
}

#[tokio::test]
async fn list_decodes_the_group_column_and_list_by_ids_matches_get_by_id() {
    let (db, path) = fresh_db("list").await;

    let mut first = sample_channel("first", "default");
    let first_id = ChannelProviderModel::create(&db, &mut first).await.unwrap();
    let mut second = sample_channel("second", "vip");
    let second_id = ChannelProviderModel::create(&db, &mut second)
        .await
        .unwrap();

    let listed = ChannelProviderModel::list(&db, 100, 0)
        .await
        .expect("real SELECT with the dialect-specific `group` quoting");
    assert_eq!(listed.len(), 2, "both channels come back");

    // `list_by_ids` is the batch path the routing layer uses instead of its own SELECT (#607).
    let by_ids = ChannelProviderModel::list_by_ids(&db, &[first_id, second_id])
        .await
        .expect("real SELECT ... WHERE id IN (...)");
    assert_eq!(by_ids.len(), 2);
    let mut names: Vec<&str> = by_ids.iter().map(|c| c.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, vec!["first", "second"]);
    // The batch path must return the same values as the single-row path.
    let single = ChannelProviderModel::get_by_id(&db, second_id)
        .await
        .unwrap()
        .unwrap();
    let batched = by_ids.iter().find(|c| c.id == second_id).unwrap();
    assert_eq!(batched.group, single.group);
    assert_eq!(batched.type_, single.type_);
    assert_eq!(batched.weight, single.weight);
    assert_eq!(batched.tpm_cap, single.tpm_cap);

    // An empty id set must not issue a malformed `WHERE id IN ()` statement.
    let none = ChannelProviderModel::list_by_ids(&db, &[]).await.unwrap();
    assert!(none.is_empty());

    cleanup(db, &path).await;
}

#[tokio::test]
async fn ability_rows_round_trip_through_the_real_table() {
    let (db, path) = fresh_db("ability").await;

    let mut channel = sample_channel("with-abilities", "default");
    let channel_id = ChannelProviderModel::create(&db, &mut channel)
        .await
        .unwrap();

    // sync_abilities derives the rows from `channel.models`; this is the production writer.
    let stored = ChannelProviderModel::get_by_id(&db, channel_id)
        .await
        .unwrap()
        .unwrap();
    ChannelProviderModel::sync_abilities(&db, &stored)
        .await
        .expect("real INSERT into channel_abilities");

    let abilities =
        burncloud_database_channel::ChannelAbilityModel::list_by_channel(&db, channel_id)
            .await
            .expect("real SELECT with the dialect-specific `group` alias");
    // The stored row carries `model_mapping`, so re-syncing from the database must reproduce both
    // mapping names as well as the explicit model list.
    let mut models: Vec<&str> = abilities.iter().map(|a| a.model.as_str()).collect();
    models.sort_unstable();
    assert_eq!(
        models,
        vec![
            "claude-3",
            "claude-3-5-sonnet",
            "claude-3-5-sonnet-2024",
            "claude-3-haiku"
        ],
        "abilities include explicit models plus model_mapping key/value"
    );
    for ability in &abilities {
        assert_eq!(ability.channel_id, channel_id);
        assert_eq!(ability.group, "default");
        assert!(ability.enabled_bool(), "synced abilities are enabled");
    }

    // A disabled channel loses its abilities entirely: `sync_abilities` deletes the existing rows
    // and returns before inserting, so the channel is not routable.
    let mut disabled = stored.clone();
    disabled.status = 2;
    ChannelProviderModel::sync_abilities(&db, &disabled)
        .await
        .unwrap();
    let after = burncloud_database_channel::ChannelAbilityModel::list_by_channel(&db, channel_id)
        .await
        .unwrap();
    assert!(
        after.is_empty(),
        "a disabled channel keeps no ability rows, so it cannot be routed"
    );

    cleanup(db, &path).await;
}
