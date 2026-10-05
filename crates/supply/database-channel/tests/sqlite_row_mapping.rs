#![allow(clippy::unwrap_used, clippy::expect_used)]
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
    let _ = std::fs::remove_file(&path);
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
        test_time: None,
        response_time: Some(120),
        base_url: Some("https://api.anthropic.com/v1".to_string()),
        models: "claude-3-5-sonnet,claude-3-haiku".to_string(),
        group: group.to_string(),
        used_quota: 0,
        model_mapping: Some(r#"{"claude-3":"claude-3-5-sonnet-2024"}"#.to_string()),
        priority: 7,
        auto_ban: 1,
        other_info: Some(r#"{"owner":"supply"}"#.to_string()),
        tag: Some("prod".to_string()),
        setting: Some(r#"{"region":"global"}"#.to_string()),
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

    // Supply-owned declarative configuration must survive create/read exactly.
    assert_eq!(
        stored.model_mapping.as_deref(),
        Some(r#"{"claude-3":"claude-3-5-sonnet-2024"}"#)
    );
    assert_eq!(stored.other_info.as_deref(), Some(r#"{"owner":"supply"}"#));
    assert_eq!(stored.tag.as_deref(), Some("prod"));
    assert_eq!(stored.setting.as_deref(), Some(r#"{"region":"global"}"#));
    assert_eq!(stored.remark.as_deref(), Some("primary"));

    // Runtime/accounting fields deliberately remain outside the Supply writer.
    assert_eq!(stored.test_time, None, "runtime test_time is not Supply-owned");
    assert_eq!(
        stored.response_time, None,
        "runtime response_time is not Supply-owned"
    );
    assert_eq!(
        stored.used_quota, 0,
        "Commerce-owned used_quota keeps its database default"
    );
    assert_eq!(
        stored.auto_ban, 1,
        "runtime auto_ban keeps its database default"
    );

    // Existing persisted columns keep their previous semantics.
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
    assert_eq!(batched.model_mapping, single.model_mapping);
    assert_eq!(batched.tag, single.tag);
    assert_eq!(batched.remark, single.remark);

    // An empty id set must not issue a malformed `WHERE id IN ()` statement.
    let none = ChannelProviderModel::list_by_ids(&db, &[]).await.unwrap();
    assert!(none.is_empty());

    cleanup(db, &path).await;
}

#[tokio::test]
async fn ability_rows_include_models_and_persisted_mapping_entries() {
    let (db, path) = fresh_db("ability").await;

    let mut channel = sample_channel("with-abilities", "default");
    let channel_id = ChannelProviderModel::create(&db, &mut channel)
        .await
        .unwrap();

    let stored = ChannelProviderModel::get_by_id(&db, channel_id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        stored.model_mapping.is_some(),
        "the mapping must be persisted before sync_abilities can derive rows from it"
    );

    // Re-running sync proves the delete-and-rebuild path consumes the persisted row, not only the
    // in-memory Channel used by create().
    ChannelProviderModel::sync_abilities(&db, &stored)
        .await
        .expect("real INSERT into channel_abilities");

    let abilities =
        burncloud_database_channel::ChannelAbilityModel::list_by_channel(&db, channel_id)
            .await
            .expect("real SELECT with the dialect-specific `group` alias");
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
        "models plus both mapping sides become routable abilities"
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

#[tokio::test]
async fn mapping_entries_that_repeat_models_are_deduplicated_before_insert() {
    let (db, path) = fresh_db("mapping_dedup").await;

    let mut channel = sample_channel("dedup", "default");
    channel.models = "shared-model,other-model,shared-model".to_string();
    channel.model_mapping = Some(
        r#"{"alias":"shared-model","shared-model":"other-model"}"#.to_string(),
    );

    let id = ChannelProviderModel::create(&db, &mut channel)
        .await
        .expect("duplicate semantic abilities must not violate the channel_abilities primary key");

    let abilities = burncloud_database_channel::ChannelAbilityModel::list_by_channel(&db, id)
        .await
        .expect("read deduplicated abilities");
    let mut models: Vec<&str> = abilities.iter().map(|ability| ability.model.as_str()).collect();
    models.sort_unstable();
    assert_eq!(
        models,
        vec!["alias", "other-model", "shared-model"],
        "each (group, model, channel_id) is inserted exactly once"
    );

    cleanup(db, &path).await;
}

#[tokio::test]
async fn update_persists_supply_config_without_overwriting_runtime_accounting_state() {
    let (db, path) = fresh_db("update_config").await;
    let mut channel = sample_channel("before", "default");
    let id = ChannelProviderModel::create(&db, &mut channel)
        .await
        .expect("create channel");

    // Simulate values owned by other runtime/accounting writers. ChannelProviderModel::update must not
    // write these columns back from a stale Channel snapshot.
    db.execute_query(&format!(
        "UPDATE channel_providers SET used_quota = 9001, response_time = 321, test_time = 654, auto_ban = 0 WHERE id = {id}"
    ))
    .await
    .expect("seed runtime/accounting state");

    let mut edited = ChannelProviderModel::get_by_id(&db, id)
        .await
        .expect("read channel")
        .expect("channel exists");
    edited.name = "after".to_string();
    edited.model_mapping = Some(r#"{"new-alias":"new-target"}"#.to_string());
    edited.other_info = Some(r#"{"owner":"updated"}"#.to_string());
    edited.tag = Some("updated-tag".to_string());
    edited.setting = Some(r#"{"mode":"updated"}"#.to_string());
    edited.remark = Some("updated remark".to_string());

    ChannelProviderModel::update(&db, &edited)
        .await
        .expect("update Supply configuration");

    let stored = ChannelProviderModel::get_by_id(&db, id)
        .await
        .expect("read updated channel")
        .expect("channel exists");
    assert_eq!(stored.name, "after");
    assert_eq!(
        stored.model_mapping.as_deref(),
        Some(r#"{"new-alias":"new-target"}"#)
    );
    assert_eq!(stored.other_info.as_deref(), Some(r#"{"owner":"updated"}"#));
    assert_eq!(stored.tag.as_deref(), Some("updated-tag"));
    assert_eq!(stored.setting.as_deref(), Some(r#"{"mode":"updated"}"#));
    assert_eq!(stored.remark.as_deref(), Some("updated remark"));

    assert_eq!(stored.used_quota, 9001, "Supply must not overwrite Commerce usage");
    assert_eq!(stored.response_time, Some(321));
    assert_eq!(stored.test_time, Some(654));
    assert_eq!(stored.auto_ban, 0);

    cleanup(db, &path).await;
}
