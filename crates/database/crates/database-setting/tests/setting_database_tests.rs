/// Internal tests for `database-setting` — SettingDatabase CRUD operations.
///
/// Each test spins up an isolated SQLite temp-file database so tests are
/// hermetic and never touch the user's default database.
use burncloud_database::create_database_with_url;
use burncloud_database_setting::SettingDatabase;
use tempfile::NamedTempFile;

/// Create an isolated `SettingDatabase` backed by a fresh SQLite temp file.
async fn create_test_setting_db() -> (SettingDatabase, NamedTempFile) {
    let tmp = NamedTempFile::new().expect("failed to create temp file");
    let url = format!("sqlite://{}?mode=rwc", tmp.path().display());
    let db = create_database_with_url(&url)
        .await
        .expect("failed to connect to test database");
    let setting_db = SettingDatabase::new_with_db(db)
        .await
        .expect("failed to initialise SettingDatabase");
    (setting_db, tmp)
}

// ---------------------------------------------------------------------------
// set / get
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_set_and_get_value() {
    let (sdb, _tmp) = create_test_setting_db().await;

    sdb.set("theme", "dark").await.expect("set failed");

    let value = sdb.get("theme").await.expect("get failed");
    assert_eq!(value, Some("dark".to_string()));
}

#[tokio::test]
async fn test_get_missing_key_returns_none() {
    let (sdb, _tmp) = create_test_setting_db().await;

    let value = sdb.get("no-such-key").await.expect("get failed");
    assert!(value.is_none());
}

#[tokio::test]
async fn test_set_overwrites_existing_value() {
    let (sdb, _tmp) = create_test_setting_db().await;

    sdb.set("lang", "en").await.unwrap();
    sdb.set("lang", "zh").await.expect("overwrite failed");

    let value = sdb.get("lang").await.unwrap();
    assert_eq!(value, Some("zh".to_string()));
}

// ---------------------------------------------------------------------------
// delete
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_delete_existing_key() {
    let (sdb, _tmp) = create_test_setting_db().await;

    sdb.set("to-delete", "value").await.unwrap();
    sdb.delete("to-delete").await.expect("delete failed");

    let value = sdb.get("to-delete").await.unwrap();
    assert!(value.is_none());
}

#[tokio::test]
async fn test_delete_nonexistent_key_is_noop() {
    let (sdb, _tmp) = create_test_setting_db().await;

    // Should not return an error
    sdb.delete("phantom-key")
        .await
        .expect("delete of missing key should not fail");
}

// ---------------------------------------------------------------------------
// list_all
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_list_all_empty() {
    let (sdb, _tmp) = create_test_setting_db().await;

    let all = sdb.list_all().await.expect("list_all failed");
    assert!(all.is_empty());
}

#[tokio::test]
async fn test_list_all_returns_all_entries() {
    let (sdb, _tmp) = create_test_setting_db().await;

    sdb.set("alpha", "1").await.unwrap();
    sdb.set("beta", "2").await.unwrap();
    sdb.set("gamma", "3").await.unwrap();

    let all = sdb.list_all().await.expect("list_all failed");
    assert_eq!(all.len(), 3);

    let names: Vec<&str> = all.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"alpha"));
    assert!(names.contains(&"beta"));
    assert!(names.contains(&"gamma"));
}

#[tokio::test]
async fn test_list_all_is_ordered_by_name() {
    let (sdb, _tmp) = create_test_setting_db().await;

    // Insert out of alphabetical order
    sdb.set("zz-last", "z").await.unwrap();
    sdb.set("aa-first", "a").await.unwrap();
    sdb.set("mm-middle", "m").await.unwrap();

    let all = sdb.list_all().await.expect("list_all failed");
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].name, "aa-first");
    assert_eq!(all[1].name, "mm-middle");
    assert_eq!(all[2].name, "zz-last");
}

// ---------------------------------------------------------------------------
// value roundtrip edge cases
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_empty_string_value() {
    let (sdb, _tmp) = create_test_setting_db().await;

    sdb.set("empty-val", "").await.expect("set empty value failed");
    let value = sdb.get("empty-val").await.unwrap();
    assert_eq!(value, Some("".to_string()));
}

#[tokio::test]
async fn test_value_with_special_characters() {
    let (sdb, _tmp) = create_test_setting_db().await;

    let special = r#"{"key": "value", "emoji": "🔥", "newline": "\n"}"#;
    sdb.set("json-setting", special)
        .await
        .expect("set special chars failed");

    let value = sdb.get("json-setting").await.unwrap();
    assert_eq!(value, Some(special.to_string()));
}

#[tokio::test]
async fn test_set_then_delete_then_set_again() {
    let (sdb, _tmp) = create_test_setting_db().await;

    sdb.set("cycle", "first").await.unwrap();
    sdb.delete("cycle").await.unwrap();
    sdb.set("cycle", "second").await.expect("re-set after delete failed");

    let value = sdb.get("cycle").await.unwrap();
    assert_eq!(value, Some("second".to_string()));
}
