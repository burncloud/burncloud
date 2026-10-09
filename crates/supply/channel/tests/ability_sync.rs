#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test, and unwrap reports a failed precondition."
)]
//! Ability synchronisation for `supply/database-channel` (#633, plan section 5 item 07).
//!
//! The existing `sqlite_row_mapping.rs` covers row mapping: a channel survives insert and select, the
//! `group` column decodes, and ability rows round-trip. It does **not** call `update`, and it never
//! exercises the stale-row behaviour of `sync_abilities` -- which is where a wrong result routes traffic
//! to a channel that should no longer serve it.
//!
//! What `sync_abilities` does, read from the source before any assertion was written:
//!
//! 1. `DELETE FROM channel_abilities WHERE channel_id = ?` -- **all** rows for that channel;
//! 2. if `status != 1`, return: a disabled channel ends up with **no** abilities;
//! 3. otherwise insert the cartesian product of `models` and `group`, both comma-split and trimmed.
//!
//! The table's key is `PRIMARY KEY (group, model, channel_id)` and the insert is a plain `INSERT`, so a
//! repeated name in either list is a uniqueness violation rather than a silent no-op. That is measured
//! below rather than assumed.

use burncloud_database::{create_database_with_url, Database};
use burncloud_database_channel::{ChannelAbilityModel, ChannelProviderModel};
use burncloud_supply_contracts::Channel;

/// A fresh SQLite file database with the real migrations applied.
///
/// A file rather than `:memory:`: the pool opens several connections and an in-memory SQLite database is
/// per-connection, so migrations and assertions would see different databases. The URL needs the
/// three-slash form -- the two-slash form is not a URL SQLite can open on Windows.
async fn fresh_db(tag: &str) -> (Database, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "bc_ability_{}_{}_{}.db",
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

/// Close the pool, wait for the handle to be released, then delete the database.
///
/// Same measured reason as the neighbouring test file: dropping a `Database` leaves the file locked, and
/// `close().await` releases the handle slightly later still, so a removal needs a short wait first.
async fn cleanup(db: Database, path: std::path::PathBuf) {
    db.close().await.ok();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    std::fs::remove_file(&path)
        .unwrap_or_else(|e| panic!("test database {} was not removed ({e})", path.display()));
}

/// A channel with a given model list, group list and status.
fn channel(name: &str, models: &str, groups: &str, status: i32) -> Channel {
    Channel {
        id: 0,
        type_: 1,
        key: format!("sk-{name}"),
        status,
        name: name.to_string(),
        weight: 0,
        created_time: None,
        test_time: None,
        response_time: None,
        base_url: Some("https://example.invalid".to_string()),
        models: models.to_string(),
        group: groups.to_string(),
        used_quota: 0,
        model_mapping: None,
        priority: 0,
        auto_ban: 0,
        other_info: None,
        tag: None,
        setting: None,
        param_override: None,
        header_override: None,
        remark: None,
        api_version: None,
        pricing_region: None,
        rpm_cap: None,
        tpm_cap: None,
        reservation_green: None,
        reservation_yellow: None,
        reservation_red: None,
    }
}

async fn insert_channel(db: &Database, ch: &mut Channel) -> i32 {
    ChannelProviderModel::create(db, ch)
        .await
        .unwrap_or_else(|e| panic!("create channel {}: {e}", ch.name))
}

/// The `(group, model)` pairs stored for a channel, sorted, so assertions read as sets.
async fn abilities_of(db: &Database, channel_id: i32) -> Vec<(String, String)> {
    let rows = ChannelAbilityModel::list_by_channel(db, channel_id)
        .await
        .unwrap_or_else(|e| panic!("list abilities for {channel_id}: {e}"));
    let mut pairs: Vec<(String, String)> = rows.into_iter().map(|a| (a.group, a.model)).collect();
    pairs.sort();
    pairs
}

fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = items
        .iter()
        .map(|(g, m)| (g.to_string(), m.to_string()))
        .collect();
    v.sort();
    v
}

// -------------------------------------------------------------------------------------------
// stale rows: the behaviour that decides whether traffic is routed correctly after a change
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn changing_the_model_list_removes_the_abilities_for_the_old_models() {
    // The failure this prevents: a channel that no longer serves `gpt-4o` keeps an ability row saying it
    // does, so requests for that model are routed to an upstream that cannot answer them.
    let (db, path) = fresh_db("stale_model").await;
    let mut ch = channel("stale", "gpt-4o,gpt-4o-mini", "default", 1);
    let id = insert_channel(&db, &mut ch).await;
    ch.id = id;

    ChannelProviderModel::sync_abilities(&db, &ch)
        .await
        .expect("first sync");
    assert_eq!(
        abilities_of(&db, id).await,
        pairs(&[("default", "gpt-4o"), ("default", "gpt-4o-mini")]),
        "the control case: both models are recorded"
    );

    ch.models = "gpt-4o-mini".to_string();
    ChannelProviderModel::sync_abilities(&db, &ch)
        .await
        .expect("second sync");

    let after = abilities_of(&db, id).await;
    assert_eq!(
        after,
        pairs(&[("default", "gpt-4o-mini")]),
        "the ability for the removed model must be gone, not merely unused"
    );
    assert!(
        !after.iter().any(|(_, m)| m == "gpt-4o"),
        "a stale ability for gpt-4o would route requests to a channel that no longer serves it"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn changing_the_group_list_removes_the_abilities_for_the_old_groups() {
    // Same failure with groups: a channel moved out of `vip` must stop answering `vip` requests.
    let (db, path) = fresh_db("stale_group").await;
    let mut ch = channel("regroup", "gpt-4o", "default,vip", 1);
    let id = insert_channel(&db, &mut ch).await;
    ch.id = id;

    ChannelProviderModel::sync_abilities(&db, &ch)
        .await
        .expect("first sync");
    assert_eq!(abilities_of(&db, id).await.len(), 2, "two groups");

    ch.group = "default".to_string();
    ChannelProviderModel::sync_abilities(&db, &ch)
        .await
        .expect("second sync");

    let after = abilities_of(&db, id).await;
    assert_eq!(after, pairs(&[("default", "gpt-4o")]));
    assert!(
        !after.iter().any(|(g, _)| g == "vip"),
        "the vip ability must be removed when the channel leaves that group"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn a_disabled_channel_ends_up_with_no_abilities() {
    // The early return in step 2 means a disabled channel keeps nothing, which is what makes disabling it
    // take effect. Asserted after a sync that *had* produced abilities, so this is about removal.
    let (db, path) = fresh_db("disable").await;
    let mut ch = channel("disabling", "gpt-4o", "default", 1);
    let id = insert_channel(&db, &mut ch).await;
    ch.id = id;

    ChannelProviderModel::sync_abilities(&db, &ch)
        .await
        .expect("sync while enabled");
    assert_eq!(abilities_of(&db, id).await.len(), 1, "the control case");

    ch.status = 0;
    ChannelProviderModel::sync_abilities(&db, &ch)
        .await
        .expect("sync while disabled");

    assert!(
        abilities_of(&db, id).await.is_empty(),
        "a disabled channel must have no abilities, otherwise it keeps receiving traffic"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn re_enabling_a_channel_restores_its_abilities() {
    // The other direction, so the assertion above is not satisfied by a one-way door.
    let (db, path) = fresh_db("reenable").await;
    let mut ch = channel("toggling", "gpt-4o", "default", 0);
    let id = insert_channel(&db, &mut ch).await;
    ch.id = id;

    ChannelProviderModel::sync_abilities(&db, &ch)
        .await
        .expect("sync while disabled");
    assert!(abilities_of(&db, id).await.is_empty(), "disabled: nothing");

    ch.status = 1;
    ChannelProviderModel::sync_abilities(&db, &ch)
        .await
        .expect("sync while enabled");
    assert_eq!(
        abilities_of(&db, id).await,
        pairs(&[("default", "gpt-4o")]),
        "re-enabling must restore the abilities, which is what makes the disable reversible"
    );

    cleanup(db, path).await;
}

// -------------------------------------------------------------------------------------------
// idempotency and isolation
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn syncing_the_same_channel_twice_leaves_the_same_abilities() {
    // The plan asks for repeated sync explicitly: the implementation deletes before inserting, so a
    // missing delete would double the rows -- and a duplicated ability is a duplicated routing candidate.
    let (db, path) = fresh_db("idempotent").await;
    let mut ch = channel("idempotent", "a,b", "g1,g2", 1);
    let id = insert_channel(&db, &mut ch).await;
    ch.id = id;

    ChannelProviderModel::sync_abilities(&db, &ch)
        .await
        .expect("first sync");
    let first = abilities_of(&db, id).await;
    assert_eq!(first.len(), 4, "2 models x 2 groups");

    ChannelProviderModel::sync_abilities(&db, &ch)
        .await
        .expect("second sync");
    let second = abilities_of(&db, id).await;

    assert_eq!(
        first, second,
        "syncing the same content twice must not change the ability set"
    );
    assert_eq!(second.len(), 4, "and must not duplicate the rows");

    cleanup(db, path).await;
}

#[tokio::test]
async fn abilities_are_scoped_to_one_channel() {
    // The delete and the insert are both keyed on `channel_id`. If the delete lost its predicate, syncing
    // one channel would wipe every other channel's abilities; if the insert bound the wrong id, a model
    // would be attributed to the wrong upstream. Two channels with different model sets are synced in turn.
    let (db, path) = fresh_db("scope").await;

    let mut first = channel("scope-a", "model-a", "default", 1);
    let first_id = insert_channel(&db, &mut first).await;
    first.id = first_id;

    let mut second = channel("scope-b", "model-b", "default", 1);
    let second_id = insert_channel(&db, &mut second).await;
    second.id = second_id;

    ChannelProviderModel::sync_abilities(&db, &first)
        .await
        .expect("sync first");
    // Syncing the second channel must not disturb the first.
    ChannelProviderModel::sync_abilities(&db, &second)
        .await
        .expect("sync second");

    assert_eq!(
        abilities_of(&db, first_id).await,
        pairs(&[("default", "model-a")]),
        "the first channel's abilities must survive a sync of the second"
    );
    assert_eq!(
        abilities_of(&db, second_id).await,
        pairs(&[("default", "model-b")]),
        "the second channel gets its own abilities"
    );

    // And re-syncing the first must not touch the second.
    ChannelProviderModel::sync_abilities(&db, &first)
        .await
        .expect("re-sync first");
    assert_eq!(
        abilities_of(&db, second_id).await,
        pairs(&[("default", "model-b")]),
        "re-syncing one channel must not clear another channel's abilities"
    );

    cleanup(db, path).await;
}

// -------------------------------------------------------------------------------------------
// the comma lists, and what a duplicate entry does
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn empty_and_whitespace_entries_in_the_lists_are_ignored() {
    // The split filters empty entries and trims the rest, so a trailing comma or a space must not create an
    // ability with an empty model name -- which would match a request with no model.
    let (db, path) = fresh_db("messy").await;
    let mut ch = channel("messy", " gpt-4o ,, gpt-4o-mini ,", "default, ,vip", 1);
    let id = insert_channel(&db, &mut ch).await;
    ch.id = id;

    ChannelProviderModel::sync_abilities(&db, &ch)
        .await
        .expect("sync");

    let got = abilities_of(&db, id).await;
    assert_eq!(
        got,
        pairs(&[
            ("default", "gpt-4o"),
            ("default", "gpt-4o-mini"),
            ("vip", "gpt-4o"),
            ("vip", "gpt-4o-mini"),
        ]),
        "whitespace is trimmed and empty entries are dropped"
    );
    assert!(
        !got.iter().any(|(_, m)| m.is_empty()),
        "an empty model name would match a request with no model"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn a_duplicate_name_in_the_model_list_is_reported_as_an_error() {
    // Measured, not assumed. The table's key is `PRIMARY KEY (group, model, channel_id)` and the insert is a
    // plain `INSERT` with no `OR IGNORE`, so a repeated model name is a uniqueness violation.
    //
    // This matters because `models` comes from operator configuration and `gpt-4o,gpt-4o` is an easy typo.
    // What the assertion fixes is that the failure is **visible**: silently dropping the duplicate would
    // also be defensible, but silently keeping both would not, and an error the caller ignores is at least
    // an error the caller can see.
    let (db, path) = fresh_db("dup_model").await;
    // Created without the duplicate: `create` syncs abilities itself, so a duplicate here would fail the
    // insert rather than reaching the call under test.
    let mut ch = channel("dup-model", "gpt-4o", "default", 1);
    let id = insert_channel(&db, &mut ch).await;
    ch.id = id;

    // The duplicate arrives the way an operator would introduce it: through an edit.
    ch.models = "gpt-4o,gpt-4o".to_string();
    let result = ChannelProviderModel::sync_abilities(&db, &ch).await;
    assert!(
        result.is_err(),
        "a duplicated model name collides on PRIMARY KEY (group, model, channel_id), so sync must report \
         an error rather than succeed; got {result:?}"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn a_duplicate_name_in_the_group_list_is_reported_as_an_error() {
    // Same mechanism on the other axis, kept separate because the two lists are split independently.
    let (db, path) = fresh_db("dup_group").await;
    let mut ch = channel("dup-group", "gpt-4o", "default", 1);
    let id = insert_channel(&db, &mut ch).await;
    ch.id = id;

    ch.group = "default,default".to_string();
    let result = ChannelProviderModel::sync_abilities(&db, &ch).await;
    assert!(
        result.is_err(),
        "a duplicated group name collides on the same primary key, so sync must report an error"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn model_mapping_may_target_a_model_already_declared_in_models() {
    // A mapping alias normally points at the actual upstream model, which may already be present in
    // `models`. That overlap is configuration, not a duplicate operator typo, so it must not fail the
    // channel create on PRIMARY KEY (group, model, channel_id).
    let (db, path) = fresh_db("mapping_overlap").await;
    let mut ch = channel("mapping-overlap", "gpt-4o", "default", 1);
    ch.model_mapping = Some(r#"{"friendly":"gpt-4o"}"#.to_string());

    let id = insert_channel(&db, &mut ch).await;
    assert_eq!(
        abilities_of(&db, id).await,
        pairs(&[("default", "friendly"), ("default", "gpt-4o")]),
        "mapping aliases are added while an already-declared target is de-duplicated"
    );

    let stored = ChannelProviderModel::get_by_id(&db, id)
        .await
        .expect("get_by_id")
        .expect("created channel exists");
    assert_eq!(
        stored.model_mapping.as_deref(),
        Some(r#"{"friendly":"gpt-4o"}"#),
        "the mapping used to build abilities must also survive the database round-trip"
    );

    cleanup(db, path).await;
}

// -------------------------------------------------------------------------------------------
// update, persisted configuration, and synchronized ability rows
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn update_changes_the_channel_row_and_preserves_its_id() {
    // `update` is never called by the existing test file. Kept minimal: the point is that a changed field
    // reaches the row and the row is still retrievable by id afterwards.
    let (db, path) = fresh_db("update").await;
    let mut ch = channel("updatable", "gpt-4o", "default", 1);
    let id = insert_channel(&db, &mut ch).await;
    ch.id = id;

    ch.name = "renamed".to_string();
    ch.models = "gpt-4o,gpt-4o-mini".to_string();
    ch.model_mapping = Some(r#"{"friendly":"gpt-4o"}"#.to_string());
    ch.other_info = Some(r#"{"owner":"supply"}"#.to_string());
    ch.tag = Some("updated".to_string());
    ch.setting = Some(r#"{"retry":3}"#.to_string());
    ch.remark = Some("updated remark".to_string());
    ChannelProviderModel::update(&db, &ch)
        .await
        .expect("update");

    let reloaded = ChannelProviderModel::get_by_id(&db, id)
        .await
        .expect("get_by_id")
        .expect("the channel must still exist after an update");

    assert_eq!(
        reloaded.id, id,
        "the primary key must be preserved by an update"
    );
    assert_eq!(reloaded.name, "renamed");
    assert_eq!(reloaded.models, "gpt-4o,gpt-4o-mini");
    assert_eq!(
        reloaded.model_mapping.as_deref(),
        Some(r#"{"friendly":"gpt-4o"}"#)
    );
    assert_eq!(
        reloaded.other_info.as_deref(),
        Some(r#"{"owner":"supply"}"#)
    );
    assert_eq!(reloaded.tag.as_deref(), Some("updated"));
    assert_eq!(reloaded.setting.as_deref(), Some(r#"{"retry":3}"#));
    assert_eq!(reloaded.remark.as_deref(), Some("updated remark"));
    assert_eq!(
        abilities_of(&db, id).await,
        pairs(&[
            ("default", "friendly"),
            ("default", "gpt-4o"),
            ("default", "gpt-4o-mini"),
        ]),
        "update persists model_mapping and synchronizes its alias without duplicating the target"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn update_also_settles_the_ability_rows() {
    // Measured: `update` ends with `Self::sync_abilities`, so changing the model list through `update`
    // is enough -- a caller does not have to remember a second call. The first draft of this file asserted
    // the opposite and the test run disproved it.
    //
    // This is the behaviour that prevents the stale-ability failure guarded earlier in this file: if
    // `update` wrote the channel row and left the abilities alone, every edit would leave the old models
    // registered until someone called sync separately.
    let (db, path) = fresh_db("update_syncs").await;
    let mut ch = channel("update-syncs", "gpt-4o", "default", 1);
    let id = insert_channel(&db, &mut ch).await;
    ch.id = id;
    assert_eq!(
        abilities_of(&db, id).await,
        pairs(&[("default", "gpt-4o")]),
        "create already synced the abilities"
    );

    ch.models = "gpt-4o-mini".to_string();
    ChannelProviderModel::update(&db, &ch)
        .await
        .expect("update");

    assert_eq!(
        abilities_of(&db, id).await,
        pairs(&[("default", "gpt-4o-mini")]),
        "update settles the abilities itself, so the removed model must already be gone"
    );

    // And the channel row agrees with the abilities, so the two are not out of step.
    let reloaded = ChannelProviderModel::get_by_id(&db, id)
        .await
        .expect("get_by_id")
        .expect("found");
    assert_eq!(reloaded.models, "gpt-4o-mini");

    cleanup(db, path).await;
}

#[tokio::test]
async fn deleting_a_channel_also_removes_its_abilities() {
    // Measured: `delete` removes the ability rows first and the channel row second, so no ability is left
    // pointing at a channel that does not exist. The first draft asserted the opposite; the test run showed
    // `left: []`.
    //
    // Worth pinning because the two statements are not in one transaction: a failure between them would
    // leave abilities without a channel, which is the state this test would then catch.
    let (db, path) = fresh_db("delete").await;
    let mut ch = channel("deletable", "gpt-4o,gpt-4o-mini", "default,vip", 1);
    let id = insert_channel(&db, &mut ch).await;
    ch.id = id;
    assert_eq!(abilities_of(&db, id).await.len(), 4, "the control case");

    ChannelProviderModel::delete(&db, id).await.expect("delete");

    let gone = ChannelProviderModel::get_by_id(&db, id)
        .await
        .expect("get_by_id after delete");
    assert!(gone.is_none(), "the channel row must be gone");

    assert!(
        abilities_of(&db, id).await.is_empty(),
        "delete must remove the channel's abilities too, otherwise they keep advertising a channel that no \
         longer exists"
    );

    // A second delete is not an error: both statements are idempotent DELETEs.
    ChannelProviderModel::delete(&db, id)
        .await
        .expect("deleting an already-deleted channel must not error");

    cleanup(db, path).await;
}

#[tokio::test]
async fn failed_create_rolls_back_provider_and_all_abilities() {
    let (db, path) = fresh_db("atomic_create").await;
    let mut ch = channel("atomic-create", "same,same", "default", 1);
    let before = ChannelProviderModel::list(&db, 1000, 0)
        .await
        .unwrap()
        .len();

    assert!(ChannelProviderModel::create(&db, &mut ch).await.is_err());
    assert_eq!(
        ch.created_time, None,
        "failed creation must not publish an uncommitted timestamp"
    );
    assert_eq!(
        ch.id, 0,
        "failed creation must not publish an uncommitted ID"
    );
    assert_eq!(
        ChannelProviderModel::list(&db, 1000, 0)
            .await
            .unwrap()
            .len(),
        before,
        "provider INSERT must roll back when the second ability INSERT fails"
    );
    let conn = db.get_connection().unwrap();
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM channel_abilities")
        .fetch_one(conn.pool())
        .await
        .unwrap();
    assert_eq!(row.0, 0, "partial abilities must also be rolled back");

    cleanup(db, path).await;
}

#[tokio::test]
async fn failed_standalone_ability_sync_preserves_existing_rows() {
    let (db, path) = fresh_db("atomic_sync").await;
    let mut ch = channel("sync", "original", "default", 1);
    let id = insert_channel(&db, &mut ch).await;
    let original = abilities_of(&db, id).await;

    ch.models = "duplicate,duplicate".to_string();
    assert!(ChannelProviderModel::sync_abilities(&db, &ch)
        .await
        .is_err());
    assert_eq!(
        abilities_of(&db, id).await,
        original,
        "failed standalone sync must preserve the existing abilities"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn failed_update_preserves_original_provider_and_abilities() {
    let (db, path) = fresh_db("atomic_update").await;
    let mut ch = channel("before", "old", "default", 1);
    let id = insert_channel(&db, &mut ch).await;
    let original_abilities = abilities_of(&db, id).await;

    ch.name = "after".to_string();
    ch.models = "same,same".to_string();
    assert!(ChannelProviderModel::update(&db, &ch).await.is_err());

    let stored = ChannelProviderModel::get_by_id(&db, id)
        .await
        .unwrap()
        .expect("original provider remains");
    assert_eq!(stored.name, "before", "provider UPDATE must roll back");
    assert_eq!(stored.models, "old");
    assert_eq!(
        abilities_of(&db, id).await,
        original_abilities,
        "ability DELETE and partial INSERT must roll back"
    );

    cleanup(db, path).await;
}
