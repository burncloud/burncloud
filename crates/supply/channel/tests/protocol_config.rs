#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test, and unwrap reports a failed precondition."
)]
//! Protocol configuration for `supply/database-channel` (#633, plan section 5 item 07).
//!
//! `ChannelProtocolConfigModel::upsert`, `get_by_type_version` and `get_default` were called by **no
//! test** before this file. They are the last part of this crate's public API with zero coverage.
//!
//! Read from the source before writing assertions, because two of these behaviours are not what the
//! method names suggest:
//!
//! 1. `upsert` uses `ON CONFLICT(channel_type, api_version) DO UPDATE`, and the table has
//!    `UNIQUE(channel_type, api_version)`. So the identity of a config is the **(type, version) pair**,
//!    not its id, and a second upsert with the same pair updates rather than inserts.
//! 2. When `is_default` is true, `upsert` **first clears `is_default` for every row of that channel
//!    type** and then writes. That is what makes `get_default` unambiguous -- and it also means a config
//!    can be left with **no** default at all, which is a state worth pinning.
//! 0. **The schema seeds this table.** The migrations insert configs for channel types 1-4
//!    (`default`, `2023-06-01`, `2024-02-01`, `v1` with Anthropic / Azure / Gemini endpoints), and they
//!    carry `is_default = 1`. Measured by running a `list` and reading the rows. So a test here must
//!    either use channel types the seed does not touch or filter to its own rows; the first draft assumed
//!    an empty table and failed.
//!
//! 3. `get_default` selects `WHERE channel_type = ? AND is_default = 1` with no ordering, so it would
//!    return an arbitrary row if two defaults existed. The clearing above is the only thing preventing
//!    that, which is why (2) is tested rather than assumed.

use burncloud_database::{create_database_with_url, Database};
use burncloud_supply_channel::{ChannelProtocolConfigInput, ChannelProtocolConfigModel};

/// A fresh SQLite file database with the real migrations applied.
///
/// A file rather than `:memory:`: the pool opens several connections and an in-memory SQLite database is
/// per-connection, so migrations and assertions would see different databases. The URL needs the
/// three-slash form; the two-slash form is not a URL SQLite can open on Windows.
async fn fresh_db(tag: &str) -> (Database, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "bc_protocol_{}_{}_{}.db",
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

/// Close the pool, wait for the handle to be released, then remove the database and its WAL companions.
///
/// The database runs in WAL mode, so it also creates `-wal` and `-shm` files. Removing only the path the
/// caller passes leaves those behind -- measured at 26 leftover files in the temp directory for the
/// sibling test file before they were included here.
async fn cleanup(db: Database, path: std::path::PathBuf) {
    db.close().await.ok();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    for suffix in ["", "-wal", "-shm"] {
        let mut candidate = path.clone().into_os_string();
        candidate.push(suffix);
        let candidate = std::path::PathBuf::from(candidate);
        if candidate.exists() {
            std::fs::remove_file(&candidate).unwrap_or_else(|e| {
                panic!(
                    "test database file {} was not removed ({e}); the temp directory would fill up",
                    candidate.display()
                )
            });
        }
    }
}

fn input(channel_type: i32, api_version: &str, is_default: bool) -> ChannelProtocolConfigInput {
    ChannelProtocolConfigInput {
        channel_type,
        api_version: api_version.to_string(),
        is_default: Some(is_default),
        chat_endpoint: Some(format!("/v{channel_type}/chat")),
        embed_endpoint: Some(format!("/v{channel_type}/embed")),
        models_endpoint: None,
        request_mapping: Some("{\"a\":\"b\"}".to_string()),
        response_mapping: None,
        detection_rules: None,
    }
}

// -------------------------------------------------------------------------------------------
// the (type, version) identity
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_config_round_trips_and_is_found_by_its_type_and_version() {
    // The control case for everything below: one config written, read back by the pair that identifies it.
    let (db, path) = fresh_db("roundtrip").await;

    ChannelProtocolConfigModel::upsert(&db, &input(1, "v1", false))
        .await
        .expect("upsert");

    let found = ChannelProtocolConfigModel::get_by_type_version(&db, 1, "v1")
        .await
        .expect("get_by_type_version")
        .expect("the config that was just written must be found");

    assert_eq!(found.channel_type, 1);
    assert_eq!(found.api_version, "v1");
    assert_eq!(found.chat_endpoint.as_deref(), Some("/v1/chat"));
    assert_eq!(found.embed_endpoint.as_deref(), Some("/v1/embed"));
    assert_eq!(found.request_mapping.as_deref(), Some("{\"a\":\"b\"}"));
    assert_eq!(
        found.models_endpoint, None,
        "an absent endpoint stays absent"
    );
    assert!(!found.is_default_bool(), "it was written as a non-default");

    cleanup(db, path).await;
}

#[tokio::test]
async fn the_same_type_and_version_are_updated_rather_than_inserted() {
    // The table has `UNIQUE(channel_type, api_version)` and `upsert` is an ON CONFLICT DO UPDATE, so the
    // pair is the identity. A second write with the same pair must update the existing row -- if it
    // inserted instead, the unique index would reject it, and if the identity were the id, `list` would
    // show two rows and `get_by_type_version` would be ambiguous.
    let (db, path) = fresh_db("upsert_same").await;

    ChannelProtocolConfigModel::upsert(&db, &input(2, "v1", false))
        .await
        .expect("first upsert");

    let mut second = input(2, "v1", false);
    second.chat_endpoint = Some("/changed".to_string());
    ChannelProtocolConfigModel::upsert(&db, &second)
        .await
        .expect("second upsert must update, not fail");

    let found = ChannelProtocolConfigModel::get_by_type_version(&db, 2, "v1")
        .await
        .expect("get")
        .expect("found");
    assert_eq!(
        found.chat_endpoint.as_deref(),
        Some("/changed"),
        "the second write must have replaced the endpoint"
    );

    // Filtered to this test's (type, version) pair. The schema seeds rows for channel types 1-4 -- type 2
    // carries a "2023-06-01" config -- so filtering by type alone would count a seeded row too.
    let for_pair: Vec<_> = ChannelProtocolConfigModel::list(&db, 200, 0)
        .await
        .expect("list")
        .into_iter()
        .filter(|c| c.channel_type == 2 && c.api_version == "v1")
        .collect();
    assert_eq!(
        for_pair.len(),
        1,
        "the same (type, version) pair must be one row, not two; got {for_pair:?}"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn different_versions_of_one_type_are_separate_configs() {
    // `api_version` is part of the identity, so two versions of a type coexist rather than replacing each
    // other. This is what lets a provider's endpoints change without stranding the older version.
    let (db, path) = fresh_db("versions").await;

    ChannelProtocolConfigModel::upsert(&db, &input(3, "v1", false))
        .await
        .expect("v1");
    ChannelProtocolConfigModel::upsert(&db, &input(3, "v2", false))
        .await
        .expect("v2");

    let v1 = ChannelProtocolConfigModel::get_by_type_version(&db, 3, "v1")
        .await
        .expect("get v1")
        .expect("v1 must exist");
    let v2 = ChannelProtocolConfigModel::get_by_type_version(&db, 3, "v2")
        .await
        .expect("get v2")
        .expect("v2 must exist");

    assert_eq!(v1.api_version, "v1");
    assert_eq!(v2.api_version, "v2");
    assert_ne!(v1.id, v2.id, "they are distinct rows");

    cleanup(db, path).await;
}

#[tokio::test]
async fn an_unknown_type_or_version_reads_as_none() {
    // Two ways to miss: a type that does not exist, and a version of a type that does. The second is the
    // interesting one, because a lookup that ignored `api_version` would return the wrong config.
    let (db, path) = fresh_db("unknown").await;
    ChannelProtocolConfigModel::upsert(&db, &input(4, "v1", false))
        .await
        .expect("upsert");

    let unknown_type = ChannelProtocolConfigModel::get_by_type_version(&db, 99, "v1")
        .await
        .expect("an unknown type is a value, not an error");
    assert!(
        unknown_type.is_none(),
        "an unknown channel type must not resolve"
    );

    let unknown_version = ChannelProtocolConfigModel::get_by_type_version(&db, 4, "v9")
        .await
        .expect("an unknown version is a value, not an error");
    assert!(
        unknown_version.is_none(),
        "a version of an existing type must not fall back to another version's config"
    );

    let unknown_default = ChannelProtocolConfigModel::get_default(&db, 99)
        .await
        .expect("an unknown type is a value, not an error");
    assert!(unknown_default.is_none());

    cleanup(db, path).await;
}

// -------------------------------------------------------------------------------------------
// default handling: the clearing behaviour
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn setting_a_new_default_clears_the_previous_one_for_that_type() {
    // This is the behaviour that makes `get_default` unambiguous. `get_default` selects
    // `WHERE channel_type = ? AND is_default = 1` with **no ordering**, so if two rows were default it
    // would return an arbitrary one -- and which config an adapter uses would depend on row order. The
    // clearing in `upsert` is the only thing preventing that, so it is asserted directly rather than
    // inferred from `get_default` succeeding.
    let (db, path) = fresh_db("default_switch").await;

    ChannelProtocolConfigModel::upsert(&db, &input(5, "v1", true))
        .await
        .expect("v1 as default");
    let first = ChannelProtocolConfigModel::get_default(&db, 5)
        .await
        .expect("get_default")
        .expect("v1 must be the default");
    assert_eq!(first.api_version, "v1");

    // Setting v2 as the default must demote v1.
    ChannelProtocolConfigModel::upsert(&db, &input(5, "v2", true))
        .await
        .expect("v2 as default");

    let v1 = ChannelProtocolConfigModel::get_by_type_version(&db, 5, "v1")
        .await
        .expect("get v1")
        .expect("v1 must still exist");
    assert!(
        !v1.is_default_bool(),
        "v1 must no longer be the default, otherwise two rows claim it and get_default is arbitrary"
    );

    let current = ChannelProtocolConfigModel::get_default(&db, 5)
        .await
        .expect("get_default")
        .expect("v2 must now be the default");
    assert_eq!(current.api_version, "v2");

    // Exactly one default for the type, counted rather than assumed.
    let defaults: Vec<_> = ChannelProtocolConfigModel::list(&db, 100, 0)
        .await
        .expect("list")
        .into_iter()
        .filter(|c| c.channel_type == 5 && c.is_default_bool())
        .collect();
    assert_eq!(
        defaults.len(),
        1,
        "exactly one config of a type may be the default; got {defaults:?}"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn clearing_the_default_flag_leaves_the_type_with_no_default() {
    // The other side of the same mechanism, and a state worth pinning: `upsert` clears the type's defaults
    // only when the incoming row is itself a default. So writing the current default again with
    // `is_default = false` leaves the type with **no** default at all, and `get_default` returns `None`.
    //
    // Whether that should be possible is a design question -- a caller that requires a default would want
    // an error rather than `None` -- so this records the behaviour rather than asserting it is correct.
    let (db, path) = fresh_db("default_cleared").await;

    ChannelProtocolConfigModel::upsert(&db, &input(6, "v1", true))
        .await
        .expect("v1 as default");
    assert!(
        ChannelProtocolConfigModel::get_default(&db, 6)
            .await
            .expect("get_default")
            .is_some(),
        "the control case: a default exists"
    );

    ChannelProtocolConfigModel::upsert(&db, &input(6, "v1", false))
        .await
        .expect("rewrite without the default flag");

    let after = ChannelProtocolConfigModel::get_default(&db, 6)
        .await
        .expect("get_default must not error");
    assert!(
        after.is_none(),
        "unsetting the flag on the only config leaves the type with no default, and get_default must say \
         so rather than fall back to a non-default row"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn a_default_for_one_type_does_not_affect_another_type() {
    // The clearing statement is scoped `WHERE channel_type = ?`. If that predicate were lost, making a
    // config default for one type would clear every other type's default -- and each provider family would
    // silently lose its configuration.
    let (db, path) = fresh_db("default_scope").await;

    ChannelProtocolConfigModel::upsert(&db, &input(7, "v1", true))
        .await
        .expect("type 7 default");
    ChannelProtocolConfigModel::upsert(&db, &input(8, "v1", true))
        .await
        .expect("type 8 default");

    let seven = ChannelProtocolConfigModel::get_default(&db, 7)
        .await
        .expect("get_default 7")
        .expect("type 7 must still have its default");
    let eight = ChannelProtocolConfigModel::get_default(&db, 8)
        .await
        .expect("get_default 8")
        .expect("type 8 must have its default");

    assert_eq!(seven.channel_type, 7);
    assert_eq!(eight.channel_type, 8);

    // And a third, non-default config for type 7 must not disturb type 8.
    ChannelProtocolConfigModel::upsert(&db, &input(7, "v2", false))
        .await
        .expect("type 7 v2, not default");
    assert!(
        ChannelProtocolConfigModel::get_default(&db, 8)
            .await
            .expect("get_default 8")
            .is_some(),
        "adding a config for one type must not clear another type's default"
    );

    cleanup(db, path).await;
}

// -------------------------------------------------------------------------------------------
// list and pagination
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn list_orders_by_type_then_version_and_respects_limit_and_offset() {
    // `list` has `ORDER BY channel_type, api_version` with a SQL LIMIT/OFFSET, so the order is stable and
    // pagination is exact. Both halves are asserted: a pagination bug that returned the first page twice
    // would still produce the right count, so the page contents are compared as well.
    //
    // The channel types are 90 and above, which the schema seed does not use, and the assertions filter to
    // those types anyway -- the seed was the reason the first version of this test failed.
    let (db, path) = fresh_db("list").await;

    for (ty, ver) in [(93, "v1"), (91, "v2"), (91, "v1"), (92, "v1")] {
        ChannelProtocolConfigModel::upsert(&db, &input(ty, ver, false))
            .await
            .unwrap_or_else(|e| panic!("upsert {ty}/{ver}: {e}"));
    }

    /// The rows this test wrote, in the order `list` returned them.
    async fn mine(db: &Database) -> Vec<(i32, String)> {
        ChannelProtocolConfigModel::list(db, 200, 0)
            .await
            .expect("list")
            .into_iter()
            .filter(|c| c.channel_type >= 90)
            .map(|c| (c.channel_type, c.api_version))
            .collect()
    }

    assert_eq!(
        mine(&db).await,
        vec![
            (91, "v1".to_string()),
            (91, "v2".to_string()),
            (92, "v1".to_string()),
            (93, "v1".to_string()),
        ],
        "ordered by channel_type then api_version"
    );

    // Pagination is applied by SQL over the whole table, so the offsets below are chosen from the real
    // position of these rows rather than from the filtered view. What is asserted is that the pages are
    // consecutive and do not repeat -- a limit/offset bug shows up as an overlap or a gap.
    let first = ChannelProtocolConfigModel::list(&db, 4, 0)
        .await
        .expect("first page");
    let second = ChannelProtocolConfigModel::list(&db, 4, 4)
        .await
        .expect("second page");
    let first_ids: Vec<i32> = first.iter().map(|c| c.id).collect();
    let second_ids: Vec<i32> = second.iter().map(|c| c.id).collect();
    assert_eq!(first.len(), 4, "the first page must be full");
    assert!(
        first_ids.iter().all(|id| !second_ids.contains(id)),
        "the second page must not repeat rows from the first: {first_ids:?} then {second_ids:?}"
    );

    // Ordered over the whole table, so the pages are monotonic in the sort key.
    let key = |c: &burncloud_supply_channel::ChannelProtocolConfig| {
        (c.channel_type, c.api_version.clone())
    };
    let mut sorted: Vec<(i32, String)> = first.iter().map(key).collect();
    let mut all = sorted.clone();
    sorted.sort();
    assert_eq!(sorted, all, "each page must be in the documented order");
    all.extend(second.iter().map(key));
    let mut sorted_all = all.clone();
    sorted_all.sort();
    assert_eq!(
        sorted_all, all,
        "the two pages together must still be in order, so the offset continues rather than restarting"
    );

    // An offset past the end returns nothing rather than wrapping.
    let beyond = ChannelProtocolConfigModel::list(&db, 10, 1000)
        .await
        .expect("offset past the end");
    assert!(
        beyond.is_empty(),
        "an offset past the last row must return nothing rather than wrap around; got {beyond:?}"
    );

    cleanup(db, path).await;
}

#[tokio::test]
async fn a_limit_of_zero_returns_nothing() {
    // The boundary in the other direction. Written because `LIMIT 0` is a legitimate value that must not be
    // treated as "no limit" -- a caller asking for zero rows and receiving all of them would be a silent
    // pagination failure.
    let (db, path) = fresh_db("limit_zero").await;
    ChannelProtocolConfigModel::upsert(&db, &input(1, "v1", false))
        .await
        .expect("upsert");

    let none = ChannelProtocolConfigModel::list(&db, 0, 0)
        .await
        .expect("list with limit 0");
    assert!(
        none.is_empty(),
        "a limit of 0 must return no rows even though the schema seeds this table, got {none:?}"
    );

    cleanup(db, path).await;
}

// -------------------------------------------------------------------------------------------
// delete
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn delete_reports_whether_a_row_was_removed_and_removes_only_that_row() {
    // The boolean is `rows_affected > 0`, so it distinguishes "deleted" from "no such id" and a caller can
    // rely on it. The second half matters more: the other configs must survive.
    let (db, path) = fresh_db("delete").await;

    ChannelProtocolConfigModel::upsert(&db, &input(1, "v1", false))
        .await
        .expect("keeper");
    ChannelProtocolConfigModel::upsert(&db, &input(2, "v1", false))
        .await
        .expect("doomed");

    let doomed = ChannelProtocolConfigModel::get_by_type_version(&db, 2, "v1")
        .await
        .expect("get")
        .expect("found");

    assert!(
        ChannelProtocolConfigModel::delete(&db, doomed.id)
            .await
            .expect("delete"),
        "deleting an existing id must report a change"
    );
    assert!(
        !ChannelProtocolConfigModel::delete(&db, doomed.id)
            .await
            .expect("delete again"),
        "deleting an id that is already gone must report no change rather than an error"
    );

    assert!(
        ChannelProtocolConfigModel::get_by_type_version(&db, 2, "v1")
            .await
            .expect("get deleted")
            .is_none(),
        "the deleted config must be gone"
    );
    assert!(
        ChannelProtocolConfigModel::get_by_type_version(&db, 1, "v1")
            .await
            .expect("get keeper")
            .is_some(),
        "deleting one config must not remove another"
    );

    cleanup(db, path).await;
}
