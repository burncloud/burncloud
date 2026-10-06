#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic_in_result_fn,
    reason = "Test-only file: the assertions are the test, and they return `Result` while still failing fast on \
              assertion; clippy.toml's allow-panic-in-tests does not recognise `#[tokio::test]`."
)]
//! The `SettingService` contract (#633, plan section 5 item 19).
//!
//! `SettingService` is four one-line delegations to `SettingDatabase`, so the plan is explicit that these tests
//! must **not** be mock unit tests of each delegation: *"不对每个一行委托单独制造 Mock 单元测试"*. What they
//! assert is the contract a caller of the service meets, exercised against a real database.
//!
//! ## What was already covered, and what was not
//!
//! `crates/platform/storage/database-sys/tests/setting_database_tests.rs` has 11 tests on `SettingDatabase`
//! covering set/get, a missing key returning `None`, overwriting, delete, delete-as-a-noop, `list_all`
//! (empty, populated and ordered), an empty string value, a value with special characters -- which does include
//! an emoji and a JSON body, so Unicode is genuinely exercised -- and a set/delete/set cycle.
//!
//! So the gaps are narrow and specific:
//!
//! 1. **The service layer had no tests at all.** Every listed behaviour was verified one layer down; nothing
//!    asserted that `SettingService` exposes it. A delegation that swapped two arguments, or dropped the
//!    `Option`, would not fail any existing test.
//! 2. **Database error propagation is tested nowhere**, at either layer. grep finds no test that makes a
//!    settings query fail; every existing test drives a healthy database.
//! 3. **Test isolation.** `SettingDatabase::new()` opens `Database::new()`, which honours
//!    `BURNCLOUD_DATABASE_URL` and otherwise uses a **shared default path** under the user's data directory. Two
//!    tests calling it concurrently would share one table. The plan asks for "互相独立的测试库", so these tests
//!    use `new_with_db` over a temporary file instead. `new()` is deliberately **not** tested here, because
//!    testing it would write to that shared location.
//!
//! ## The table, for reference
//!
//! ```sql
//! CREATE TABLE IF NOT EXISTS sys_settings (name TEXT PRIMARY KEY, value TEXT NOT NULL)
//! ```
//!
//! with `INSERT OR REPLACE` as the write, so `set` is an upsert keyed on the **name** alone.

use burncloud_service_setting::SettingDatabase;
use burncloud_service_setting::SettingService;
use std::error::Error;

/// An isolated settings database in a temporary file.
///
/// Three slashes and forward slashes: `sqlite:///C:/...` is absolute on Windows, whereas the two-slash form is
/// not a URL SQLite can open and fails with "(code: 14) unable to open database file" before any assertion.
async fn fresh(tag: &str) -> Result<(SettingDatabase, std::path::PathBuf), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!(
        "bc_setting_{}_{}_{}.db",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::remove_file(&path).ok();
    let normalized = path.to_string_lossy().replace('\\', "/");
    let db =
        burncloud_database::create_database_with_url(&format!("sqlite:///{}?mode=rwc", normalized))
            .await?;
    let settings = SettingDatabase::new_with_db(db).await?;
    Ok((settings, path))
}

/// Close the database, wait for the handle to be released, then remove the file and its WAL companions.
async fn cleanup(settings: SettingDatabase, path: std::path::PathBuf) {
    settings.close().await.ok();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    for suffix in ["", "-wal", "-shm"] {
        let mut candidate = path.clone().into_os_string();
        candidate.push(suffix);
        let candidate = std::path::PathBuf::from(candidate);
        if candidate.exists() {
            std::fs::remove_file(&candidate)
                .unwrap_or_else(|e| panic!("{} was not removed ({e})", candidate.display()));
        }
    }
}

// -------------------------------------------------------------------------------------------
// set / get / delete / list through the service
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn the_service_round_trips_a_value() -> Result<(), Box<dyn Error>> {
    // The baseline: what `set` writes, `get` returns. Without it, every negative assertion below would pass on
    // a service that did nothing at all.
    let (sdb, path) = fresh("roundtrip").await?;

    SettingService::set(&sdb, "site.name", "BurnCloud").await?;
    let value = SettingService::get(&sdb, "site.name").await?;

    println!("get -> {value:?}");
    assert_eq!(
        value,
        Some("BurnCloud".to_string()),
        "the value set is the value got"
    );

    cleanup(sdb, path).await;
    Ok(())
}

#[tokio::test]
async fn a_missing_setting_is_none_rather_than_an_error() -> Result<(), Box<dyn Error>> {
    // "不存在返回 None". The distinction matters to a caller: `Ok(None)` means "not configured, use the
    // default", while an `Err` means the lookup itself failed. A service that mapped a missing row to an error
    // would make every optional setting mandatory to check with a `match`.
    let (sdb, path) = fresh("missing").await?;

    let missing = SettingService::get(&sdb, "never.set").await;
    println!("a missing key -> {missing:?}");

    match missing {
        Ok(None) => {}
        Ok(Some(v)) => panic!("a key nobody set returned a value: {v:?}"),
        Err(e) => panic!("a missing key must not be an error: {e}"),
    }

    cleanup(sdb, path).await;
    Ok(())
}

#[tokio::test]
async fn setting_the_same_key_again_replaces_the_value_and_keeps_one_row(
) -> Result<(), Box<dyn Error>> {
    // "覆盖同一 key", and the part the earlier layer's test does not check: that the row count stays at one.
    // `INSERT OR REPLACE` on a `name` primary key should update in place; a store that appended would leave the
    // old value behind for anything reading by position or ordering.
    let (sdb, path) = fresh("overwrite").await?;

    SettingService::set(&sdb, "same.key", "first").await?;
    SettingService::set(&sdb, "same.key", "second").await?;
    SettingService::set(&sdb, "same.key", "third").await?;

    assert_eq!(
        SettingService::get(&sdb, "same.key").await?,
        Some("third".to_string()),
        "the last write wins"
    );

    let all = SettingService::list_all(&sdb).await?;
    let matching: Vec<_> = all.iter().filter(|s| s.name == "same.key").collect();
    println!("rows after three writes to one key: {}", matching.len());
    assert_eq!(
        matching.len(),
        1,
        "three writes to one key leave one row, not three"
    );
    assert_eq!(all.len(), 1, "and nothing else was written");

    cleanup(sdb, path).await;
    Ok(())
}

#[tokio::test]
async fn delete_removes_the_key_and_is_a_noop_for_one_that_was_never_there(
) -> Result<(), Box<dyn Error>> {
    // `delete` returns `()` rather than a count, so a caller cannot tell "removed" from "was not there" -- which
    // is a contract choice, not a defect. What is asserted is that neither case is an error, and that the key is
    // gone afterwards.
    let (sdb, path) = fresh("delete").await?;

    SettingService::set(&sdb, "doomed", "value").await?;
    SettingService::delete(&sdb, "doomed").await?;
    assert_eq!(
        SettingService::get(&sdb, "doomed").await?,
        None,
        "the key is gone"
    );

    // Deleting it again, and deleting a key that never existed, must both succeed.
    SettingService::delete(&sdb, "doomed").await?;
    SettingService::delete(&sdb, "never.existed").await?;

    assert_eq!(
        SettingService::list_all(&sdb).await?.len(),
        0,
        "and nothing is left behind"
    );

    // **The scope of the delete, which a mutation showed was untested.** The first version of this test used a
    // single key, so a `delete` that issued `DELETE FROM sys_settings` with no `WHERE` was indistinguishable
    // from a correct one -- it removed the only row either way. With neighbours present, the difference is
    // visible.
    for name in ["keep.a", "remove.me", "keep.b"] {
        SettingService::set(&sdb, name, "present").await?;
    }
    SettingService::delete(&sdb, "remove.me").await?;

    let remaining: Vec<String> = SettingService::list_all(&sdb)
        .await?
        .into_iter()
        .map(|s| s.name)
        .collect();
    println!("after deleting one of three: {remaining:?}");
    assert_eq!(
        remaining,
        vec!["keep.a".to_string(), "keep.b".to_string()],
        "delete removes exactly the named key and leaves its neighbours"
    );

    cleanup(sdb, path).await;
    Ok(())
}

#[tokio::test]
async fn list_all_is_empty_for_a_fresh_database_and_ordered_by_name() -> Result<(), Box<dyn Error>>
{
    // "互相独立的测试库": a brand-new database lists nothing, which is the assertion that fails if two tests
    // share a database. Then the ordering, because a settings list is read by a human and by tests.
    let (sdb, path) = fresh("list").await?;

    let empty = SettingService::list_all(&sdb).await?;
    println!("a fresh database lists {} settings", empty.len());
    assert!(
        empty.is_empty(),
        "a new database has no settings, so no other test's writes are visible here"
    );

    // Written in an order that is not sorted, so a `list_all` that returned insertion order would be visible.
    for name in ["zeta", "alpha", "mu"] {
        SettingService::set(&sdb, name, "v").await?;
    }

    let listed = SettingService::list_all(&sdb).await?;
    let names: Vec<&str> = listed.iter().map(|s| s.name.as_str()).collect();
    println!("listed: {names:?}");

    assert_eq!(
        names,
        vec!["alpha", "mu", "zeta"],
        "ordered by name, not by insertion"
    );
    assert_eq!(listed.len(), 3, "and every entry is present");

    // The values travel with the names, so a `list_all` that returned names with empty values would be caught.
    assert!(
        listed.iter().all(|s| s.value == "v"),
        "each entry carries its own value: {listed:?}"
    );

    cleanup(sdb, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// Unicode and the empty value
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn unicode_survives_in_both_the_name_and_the_value() -> Result<(), Box<dyn Error>> {
    // The earlier layer's test covers an emoji and a JSON body in the **value**. Unicode in the **name** is not
    // covered anywhere, and a name is a primary key -- so if a collation or normalisation step were applied to
    // keys, two visually identical names could collide or a lookup could miss.
    let (sdb, path) = fresh("unicode").await?;

    let cases: Vec<(&str, &str)> = vec![
        ("设置.名称", "值"), // Chinese in both
        ("emoji.🔥", "🎉 celebratory"),
        ("accent.é", "café"), // precomposed U+00E9
        ("rtl.שלום", "ערך"),
        ("combining.e\u{0301}", "decomposed"), // e + combining acute: "é" as two code points
    ];

    for (name, value) in &cases {
        SettingService::set(&sdb, name, value).await?;
        let got = SettingService::get(&sdb, name).await?;
        println!("{name:?} -> {got:?}");
        assert_eq!(
            got.as_deref(),
            Some(*value),
            "{name:?} must round-trip byte for byte"
        );
    }

    let all = SettingService::list_all(&sdb).await?;
    assert_eq!(all.len(), cases.len(), "each distinct name is its own row");

    // **`"é"` and `"e"` + U+0301 are different keys.** They render identically but are different byte strings,
    // and this records which behaviour the store has: it compares bytes, so they are two rows. A normalising
    // store would have collapsed them. Recorded rather than endorsed -- either policy is defensible, but a caller
    // has to know which one it is, because a settings file written on one system and read on another can differ
    // in exactly this way.
    //
    // The lookup has to name both forms explicitly; the first version of this test asked for
    // `"combining.é"` (precomposed) while the row it had written was `"combining.e\u{0301}"` (decomposed), and
    // measured `None` -- which is the property being asserted, reached by accident.
    let precomposed = SettingService::get(&sdb, "accent.é").await?;
    let decomposed = SettingService::get(&sdb, "combining.e\u{0301}").await?;
    let mismatched = SettingService::get(&sdb, "combining.é").await?;
    println!(
        "accent.é -> {precomposed:?}, combining.e\\u{{0301}} -> {decomposed:?}, combining.é -> {mismatched:?}"
    );

    assert_eq!(
        precomposed.as_deref(),
        Some("café"),
        "the precomposed key holds its own value"
    );
    assert_eq!(
        decomposed.as_deref(),
        Some("decomposed"),
        "and the decomposed key holds its own"
    );
    assert_eq!(
        mismatched, None,
        "asking for the precomposed spelling of a key written in the decomposed spelling finds nothing, which is \
         what comparing bytes rather than normalising means"
    );

    cleanup(sdb, path).await;
    Ok(())
}

#[tokio::test]
async fn an_empty_value_is_stored_and_is_not_the_same_as_a_missing_key(
) -> Result<(), Box<dyn Error>> {
    // "空字符串规则". The earlier layer covers storing `""`; what it does not cover is the distinction this
    // service's `Option` return exists to express: `Some("")` is a setting that was set to empty, `None` is a
    // setting that was never set. A service that mapped `""` to `None` would make "explicitly blank" and
    // "unset" indistinguishable, and a caller testing `if let Some(v)` would take the wrong branch.
    let (sdb, path) = fresh("empty").await?;

    SettingService::set(&sdb, "blank", "").await?;

    let blank = SettingService::get(&sdb, "blank").await?;
    let unset = SettingService::get(&sdb, "unset").await?;
    println!("blank -> {blank:?}, unset -> {unset:?}");

    assert_eq!(
        blank,
        Some(String::new()),
        "an empty value is Some(\"\"), not None"
    );
    assert_eq!(unset, None, "and an unset key is None");
    assert_ne!(
        blank, unset,
        "the two must be distinguishable, which is what the Option is for"
    );

    // The empty-valued row is listed, so it is a real row rather than one the store skipped.
    let all = SettingService::list_all(&sdb).await?;
    assert_eq!(all.len(), 1, "the blank setting is listed");
    assert_eq!(all[0].name, "blank");

    // And an empty **name** is accepted as a key, which is worth recording because it is the sort of input a
    // caller assumes is rejected. Recorded, not endorsed.
    let empty_name = SettingService::set(&sdb, "", "value under an empty name").await;
    match &empty_name {
        Ok(()) => {
            let got = SettingService::get(&sdb, "").await?;
            println!("an empty name is accepted as a key; get(\"\") -> {got:?}");
            assert_eq!(got.as_deref(), Some("value under an empty name"));
        }
        Err(e) => println!("an empty name is rejected: {e}"),
    }

    cleanup(sdb, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// database error propagation
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_failing_database_query_is_an_error_and_never_a_default() -> Result<(), Box<dyn Error>> {
    // **The gap that exists at every layer.** No test in `database-sys` or here makes a settings query fail;
    // all 11 of the existing ones drive a healthy database. The plan lists "数据库失败传播" separately, and it
    // is the item with the real consequence: a service that turned a failed read into `Ok(None)` would tell a
    // caller "not configured" when the truth is "could not find out", and a default would be used silently.
    //
    // Dropping the table makes every query fail without needing to close the connection -- and closing it is not
    // an option, because `SettingDatabase::close(self)` consumes the handle, leaving nothing to query with.
    //
    // **The drop goes through a second connection to the same file**, because `SettingDatabase`'s `db` field is
    // private and has no accessor. Adding a `pub fn db(&self)` purely so a test could reach the handle would
    // widen the crate's API for the test's convenience, so the test opens its own connection instead -- the
    // ordinary way to observe a store from outside.
    let (sdb, path) = fresh("errors").await?;

    // A working baseline first, so the failures below are attributable to the missing table.
    SettingService::set(&sdb, "before", "works").await?;
    assert_eq!(
        SettingService::get(&sdb, "before").await?,
        Some("works".to_string())
    );

    let normalized = path.to_string_lossy().replace('\\', "/");
    let observer =
        burncloud_database::create_database_with_url(&format!("sqlite:///{}?mode=rwc", normalized))
            .await?;
    observer.execute_query("DROP TABLE sys_settings").await?;
    observer.close().await.ok();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Every read must be an error. `Ok(None)` is the specific wrong answer that matters: it is
    // indistinguishable from an unset key.
    let get = SettingService::get(&sdb, "before").await;
    match &get {
        Ok(None) => panic!(
            "a failed read returned Ok(None), which is exactly the value that means \"not configured\" -- a \
             caller would use a default instead of reporting a failure"
        ),
        Ok(Some(v)) => panic!("a failed read returned a value: {v:?}"),
        Err(e) => println!("get on a dropped table -> {e}"),
    }
    assert!(get.is_err(), "a failed read must be an error");

    let list = SettingService::list_all(&sdb).await;
    match &list {
        Ok(v) => panic!(
            "a failed list returned {} entries; an empty list means \"nothing configured\" and is the wrong \
             answer here",
            v.len()
        ),
        Err(e) => println!("list_all on a dropped table -> {e}"),
    }
    assert!(list.is_err(), "a failed list must be an error");

    // Writes and deletes too. A `set` that reported success without writing would be the worst of the three,
    // because the caller would believe a configuration change had taken effect.
    let set = SettingService::set(&sdb, "after", "value").await;
    assert!(
        set.is_err(),
        "a failed write must be an error, not a silent success"
    );
    println!(
        "set on a dropped table -> {:?}",
        set.err().map(|e| e.to_string())
    );

    let delete = SettingService::delete(&sdb, "before").await;
    assert!(delete.is_err(), "a failed delete must be an error");
    println!(
        "delete on a dropped table -> {:?}",
        delete.err().map(|e| e.to_string())
    );

    cleanup(sdb, path).await;
    Ok(())
}

// -------------------------------------------------------------------------------------------
// isolation
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn two_databases_do_not_see_each_others_settings() -> Result<(), Box<dyn Error>> {
    // "互相独立的测试库", asserted rather than assumed. Both databases are open at once, which is what makes this
    // a test of isolation rather than of ordering: a store backed by a shared file or a global connection pool
    // would show one database's write in the other's listing.
    let (first, first_path) = fresh("iso_a").await?;
    let (second, second_path) = fresh("iso_b").await?;

    SettingService::set(&first, "only.in.first", "a").await?;
    SettingService::set(&second, "only.in.second", "b").await?;

    let in_first = SettingService::list_all(&first).await?;
    let in_second = SettingService::list_all(&second).await?;
    println!(
        "first: {:?}, second: {:?}",
        in_first.iter().map(|s| &s.name).collect::<Vec<_>>(),
        in_second.iter().map(|s| &s.name).collect::<Vec<_>>()
    );

    assert_eq!(
        in_first.len(),
        1,
        "the first database has exactly its own setting"
    );
    assert_eq!(in_first[0].name, "only.in.first");
    assert_eq!(in_second.len(), 1, "and the second has exactly its own");
    assert_eq!(in_second[0].name, "only.in.second");

    assert_eq!(
        SettingService::get(&first, "only.in.second").await?,
        None,
        "and neither can read the other's key"
    );

    // The tables really are in different files, which is the mechanism the isolation rests on.
    assert_ne!(
        first_path, second_path,
        "the two databases were created at different paths"
    );

    cleanup(first, first_path).await;
    cleanup(second, second_path).await;
    Ok(())
}
