#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic_in_result_fn,
    reason = "Test-only file: the assertions are the test, and clippy.toml's allow-panic-in-tests does not recognise #[tokio::test]"
)]
//! Download and installation records, settings upsert, and the isolation between task ids (#633, plan item 27).
//!
//! The crate had 11 tests, all on `SettingDatabase`. The other two modules -- `download` and `installer` -- had
//! **one test between them**, and it only asserted on a struct in memory.
//!
//! ## First, what the plan asks to confirm: which modules actually compile
//!
//! > 先确认下载/安装接口哪些实际参与编译
//!
//! All three are declared in `lib.rs` -- `pub mod download; pub mod installer; pub mod setting;` -- and all three
//! export public types. **Nothing here is an uncompiled file**, unlike `commerce/database-billing`, where two
//! modules were present on disk and absent from the module tree. So the interface under test is the whole crate.
//!
//! ## The constructors this needed
//!
//! `DownloadDB::new()` and `InstallerDB::new()` both open `Database::new()`, which honours
//! `BURNCLOUD_DATABASE_URL` and otherwise uses the **shared default path** under the user's data directory. There
//! was no way to hand either one an existing connection, so a test of persistence would have written real
//! download and installation records into the developer's database.
//!
//! `new_with_db` was added to both, matching the `SettingDatabase::new_with_db` that already existed. Every test
//! below uses it with a temporary file; **nothing here touches the default path**.

use burncloud_database::sqlite_url;
use burncloud_database_sys::{DownloadDB, InstallerDB, SettingDatabase, SysInstallation};

/// A database in a temporary file, created with the real migrations, removed when the guard is dropped.
///
/// The URL comes from `burncloud_database::sqlite_url`, which is the function this crate's own tests were fixed
/// to use: a two-slash URL with a Windows path is rejected by SQLite with `(code: 14)`.
struct TempDb {
    path: std::path::PathBuf,
}

impl TempDb {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "bc_sysdb_{}_{}_{}.sqlite",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after the epoch")
                .as_nanos()
        ));
        std::fs::remove_file(&path).ok();
        Self { path }
    }

    fn url(&self) -> String {
        sqlite_url(
            &self.path.to_string_lossy().replace('\\', "/"),
            cfg!(windows),
            true,
        )
    }

    /// A download store over a fresh database.
    async fn downloads(&self) -> Result<DownloadDB, Box<dyn std::error::Error>> {
        let db = burncloud_database::create_database_with_url(&self.url()).await?;
        Ok(DownloadDB::new_with_db(db).await?)
    }

    /// An installation store over a fresh database.
    async fn installations(&self) -> Result<InstallerDB, Box<dyn std::error::Error>> {
        let db = burncloud_database::create_database_with_url(&self.url()).await?;
        Ok(InstallerDB::new_with_db(db).await?)
    }

    /// A settings store over a fresh database.
    async fn settings(&self) -> Result<SettingDatabase, Box<dyn std::error::Error>> {
        let db = burncloud_database::create_database_with_url(&self.url()).await?;
        Ok(SettingDatabase::new_with_db(db).await?)
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let mut candidate = self.path.clone().into_os_string();
            candidate.push(suffix);
            let candidate = std::path::PathBuf::from(candidate);
            if candidate.exists() {
                std::fs::remove_file(&candidate).ok();
            }
        }
    }
}

// -------------------------------------------------------------------------------------------
// downloads: state, progress, and the isolation between task ids
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_download_moves_through_its_states_and_the_progress_is_recorded(
) -> Result<(), Box<dyn std::error::Error>> {
    // The lifecycle a download manager drives: added, active, complete. The progress figures are read back
    // rather than assumed, because they are three separate optional columns.
    let temp = TempDb::new("dl_states");
    let downloads = temp.downloads().await?;
    let gid = "task-1";

    downloads
        .add(
            gid,
            vec!["https://example.invalid/file.bin".to_string()],
            Some("/tmp/downloads"),
            Some("file.bin"),
        )
        .await?;

    let added = downloads
        .list(None)
        .await?
        .into_iter()
        .find(|d| d.gid == gid)
        .expect("the task is listed");
    println!(
        "after add: status={} uris={} dir={:?}",
        added.status, added.uris, added.download_dir
    );
    // **`uris` is stored as a JSON array, not as the raw URL.** `add` takes a `Vec<String>` and serialises it,
    // so a single URI comes back as `["https://..."]`. The test asserts the encoded form because that is the
    // contract; a reader comparing against the input would otherwise think the value was corrupted.
    assert_eq!(
        added.uris, r#"["https://example.invalid/file.bin"]"#,
        "the uri list is stored as JSON"
    );
    assert!(
        added.uris.contains("example.invalid"),
        "and the URL is present in it"
    );
    assert_eq!(added.download_dir.as_deref(), Some("/tmp/downloads"));
    assert_eq!(added.filename.as_deref(), Some("file.bin"));
    // **A task is created `active`, not `pending`.** `add` writes the status literally (`VALUES (?, 'active',
    // ?, ?, ?, ?)`), so the first assertion a caller can make about a new task is that it is already active.
    // Recorded rather than judged: whether a freshly queued download should be `active` before the manager has
    // touched it is a decision for whoever owns the state machine.
    assert_eq!(
        added.status, "active",
        "`add` hard-codes the initial status rather than taking one"
    );

    downloads.update_status(gid, "active").await?;
    downloads.update_progress(gid, 1000, 400, 50).await?;

    let active = downloads
        .list(Some("active"))
        .await?
        .into_iter()
        .find(|d| d.gid == gid)
        .expect("the task is active");
    println!(
        "after progress: total={:?} completed={:?} speed={:?}",
        active.total_length, active.completed_length, active.download_speed
    );
    assert_eq!(active.total_length, Some(1000));
    assert_eq!(active.completed_length, Some(400));
    assert_eq!(active.download_speed, Some(50));

    // The filter is by status, so the completed list must not contain it yet.
    assert!(
        downloads
            .list(Some("complete"))
            .await?
            .iter()
            .all(|d| d.gid != gid),
        "a task filtered by another status must not appear"
    );

    downloads.update_status(gid, "complete").await?;
    let complete = downloads
        .list(Some("complete"))
        .await?
        .into_iter()
        .find(|d| d.gid == gid)
        .expect("the task is complete");
    // Progress survives the status change, which is what makes a completed entry readable.
    assert_eq!(complete.completed_length, Some(400));

    Ok(())
}

#[tokio::test]
async fn get_returns_one_download_by_gid_and_none_for_a_missing_gid(
) -> Result<(), Box<dyn std::error::Error>> {
    // 按 GID 查询必须只返回匹配记录；不存在的 GID 只能返回 None，查询本身的错误仍需传播。
    let temp = TempDb::new("dl_get");
    let downloads = temp.downloads().await?;

    downloads
        .add(
            "lookup-gid",
            vec!["https://example.invalid/lookup".to_string()],
            Some("/tmp/lookup"),
            Some("lookup.bin"),
        )
        .await?;

    let found = downloads
        .get("lookup-gid")
        .await?
        .expect("the record exists");
    assert_eq!(found.gid, "lookup-gid");
    assert_eq!(found.filename.as_deref(), Some("lookup.bin"));
    assert!(downloads.get("missing-gid").await?.is_none());

    Ok(())
}

#[tokio::test]
async fn different_task_ids_do_not_overwrite_each_other() -> Result<(), Box<dyn std::error::Error>>
{
    // "不同任务 ID 不互相覆盖". `gid` is the primary key, so three tasks must be three rows with independent
    // state -- a store keyed on anything else, or one that used `INSERT OR REPLACE` with a wrong key, would show
    // up as a lost task.
    let temp = TempDb::new("dl_isolation");
    let downloads = temp.downloads().await?;

    for (gid, dir) in [("a", "/tmp/a"), ("b", "/tmp/b"), ("c", "/tmp/c")] {
        downloads
            .add(
                gid,
                vec![format!("https://example.invalid/{gid}")],
                Some(dir),
                None,
            )
            .await?;
    }
    assert_eq!(
        downloads.list(None).await?.len(),
        3,
        "three tasks, three rows"
    );

    // Moving one through its states leaves the others alone.
    downloads.update_status("b", "active").await?;
    downloads.update_progress("b", 500, 100, 10).await?;

    let all = downloads.list(None).await?;
    for (gid, dir) in [("a", "/tmp/a"), ("b", "/tmp/b"), ("c", "/tmp/c")] {
        let row = all
            .iter()
            .find(|d| d.gid == gid)
            .expect("every task is listed");
        assert_eq!(
            row.download_dir.as_deref(),
            Some(dir),
            "{gid} kept its own directory"
        );
    }

    let b = all.iter().find(|d| d.gid == "b").expect("b is listed");
    assert_eq!(b.total_length, Some(500), "b has its own progress");
    for other in ["a", "c"] {
        let row = all.iter().find(|d| d.gid == other).expect("listed");
        // **Every task is `active` from `add`**, so status is not the field that shows isolation here -- and the
        // numeric progress columns are `Some(0)` rather than `None`, because they default to zero in the schema
        // rather than being left NULL. That is the same empty-value pattern as `installed_at` above, in numeric
        // form, and it means `is_some()` is not a way to tell "no progress reported" from "zero progress".
        //
        // The discrimination is therefore by **value**: `b` has 500 and the others have 0, so a store that wrote
        // progress to the wrong row would be caught.
        assert_eq!(
            row.status, "active",
            "{other} is active because `add` creates every task that way"
        );
        assert_ne!(
            row.total_length,
            Some(500),
            "{other} must not have b's total"
        );
        assert_eq!(row.total_length, Some(0), "{other} has the column default");
        assert_eq!(row.completed_length, Some(0));
        assert_eq!(row.download_speed, Some(0));
    }

    // Deleting one leaves the other two.
    downloads.delete("b").await?;
    let remaining: Vec<String> = downloads
        .list(None)
        .await?
        .into_iter()
        .map(|d| d.gid)
        .collect();
    println!("after deleting b: {remaining:?}");
    assert_eq!(remaining.len(), 2, "one task deleted, two left");
    assert!(!remaining.contains(&"b".to_string()));

    Ok(())
}

#[tokio::test]
async fn a_task_id_can_be_replaced_which_is_how_a_restart_re_attaches(
) -> Result<(), Box<dyn std::error::Error>> {
    // "中断记录恢复". A download interrupted and resumed under a new id must keep its record: `update_gid` moves
    // the row rather than creating a second one, which is what lets a restarted manager find the partial file's
    // progress again. The count is asserted, because a "move" implemented as insert-then-delete would leave two
    // rows if the delete failed.
    let temp = TempDb::new("dl_gid");
    let downloads = temp.downloads().await?;

    downloads
        .add(
            "old-gid",
            vec!["https://example.invalid/partial".to_string()],
            Some("/tmp"),
            None,
        )
        .await?;
    downloads.update_status("old-gid", "active").await?;
    downloads.update_progress("old-gid", 900, 300, 20).await?;

    downloads.update_gid("old-gid", "new-gid").await?;

    let all = downloads.list(None).await?;
    println!(
        "after update_gid: {:?}",
        all.iter().map(|d| &d.gid).collect::<Vec<_>>()
    );

    assert_eq!(all.len(), 1, "the row was moved, not duplicated");
    let moved = &all[0];
    assert_eq!(moved.gid, "new-gid", "under the new id");
    assert_eq!(moved.status, "active", "with its state");
    assert_eq!(
        moved.completed_length,
        Some(300),
        "and its progress, which is the point of re-attaching rather than restarting"
    );

    assert!(
        downloads
            .list(None)
            .await?
            .iter()
            .all(|d| d.gid != "old-gid"),
        "and the old id is gone"
    );

    Ok(())
}

#[tokio::test]
async fn a_unicode_path_round_trips_through_the_download_record(
) -> Result<(), Box<dyn std::error::Error>> {
    // "时间字段与 Unicode 路径往返". A download directory and filename chosen by a user can contain any Unicode,
    // and both are stored as text. A store that lost or mangled them would put the file somewhere the user did
    // not ask for -- which is the failure that matters, more than the record being wrong.
    let temp = TempDb::new("dl_unicode");
    let downloads = temp.downloads().await?;

    let dir = "/tmp/下载/音楽 🎵";
    let filename = "曲目—01 «test».bin";
    downloads
        .add(
            "unicode-task",
            vec!["https://example.invalid/x".to_string()],
            Some(dir),
            Some(filename),
        )
        .await?;

    let row = downloads
        .list(None)
        .await?
        .into_iter()
        .find(|d| d.gid == "unicode-task")
        .expect("the task is listed");
    println!("dir={:?} filename={:?}", row.download_dir, row.filename);

    assert_eq!(
        row.download_dir.as_deref(),
        Some(dir),
        "the directory round-trips byte for byte"
    );
    assert_eq!(
        row.filename.as_deref(),
        Some(filename),
        "and so does the filename"
    );

    // Several URIs at once: `add` takes a `Vec` and the column is a single text field, so this pins how the list
    // is represented rather than assuming it is dropped.
    downloads
        .add(
            "multi-uri",
            vec![
                "https://one.invalid/a".to_string(),
                "https://two.invalid/b".to_string(),
            ],
            None,
            None,
        )
        .await?;
    let multi = downloads
        .list(None)
        .await?
        .into_iter()
        .find(|d| d.gid == "multi-uri")
        .expect("listed");
    println!("two uris stored as: {:?}", multi.uris);
    assert!(
        multi.uris.contains("one.invalid") && multi.uris.contains("two.invalid"),
        "both uris must survive, however they are joined: {:?}",
        multi.uris
    );

    Ok(())
}

// -------------------------------------------------------------------------------------------
// installations: the receipt
// -------------------------------------------------------------------------------------------

/// A record with only the fields a test asserts on.
fn installation(id: &str) -> SysInstallation {
    SysInstallation {
        software_id: id.to_string(),
        name: format!("software {id}"),
        version: Some("1.0.0".to_string()),
        status: "installing".to_string(),
        install_dir: Some("/opt/test".to_string()),
        install_method: Some("script".to_string()),
        installed_at: None,
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        error_message: None,
    }
}

#[tokio::test]
async fn an_installation_receipt_moves_through_its_states() -> Result<(), Box<dyn std::error::Error>>
{
    // "安装记录状态更新". The three helpers are the documented way through the lifecycle, and each must leave the
    // fields the next stage reads in a sensible state -- in particular `installed_at`, which only a successful
    // install should set.
    let temp = TempDb::new("inst_states");
    let installations = temp.installations().await?;

    installations.mark_installing("app", "Test App").await?;
    let installing = installations
        .get("app")
        .await?
        .expect("the record exists after mark_installing");
    println!(
        "installing: status={} installed_at={:?}",
        installing.status, installing.installed_at
    );
    assert_eq!(installing.status, "installing");
    assert_eq!(installing.name, "Test App");
    // **`installed_at` comes back as `Some("")`, not `None`.** `mark_installing` sets the field to `None`, and
    // the column is a nullable `DATETIME`, so the obvious expectation is `None`. It is not: `upsert` binds
    // `record.installed_at.clone().unwrap_or_default()`, so a `None` becomes an **empty string** and the column
    // holds `''` rather than SQL `NULL`.
    //
    // The consequence is not cosmetic, and it is why this is asserted rather than noted:
    // `ON CONFLICT ... installed_at = COALESCE(excluded.installed_at, sys_installations.installed_at)` was
    // written to **keep** an existing installation time when a later upsert does not carry one. Because the
    // excluded value is `''` and not `NULL`, `COALESCE` returns `''` rather than falling through, so the
    // intended preservation does not happen -- the write wins with an empty string.
    //
    // Recorded, not fixed: binding a real `NULL` needs a parameter API that can carry an `Option`, and this
    // store binds `Vec<String>`. Changing that is a change to the binding layer, not to this function.
    assert_eq!(
        installing.installed_at.as_deref(),
        Some(""),
        "a `None` timestamp is stored as an empty string, so a caller testing `is_some()` sees a value"
    );
    assert!(
        installing.install_dir.as_deref().unwrap_or("").is_empty(),
        "and the other optional text columns behave the same way"
    );

    installations
        .mark_failed("app", "the download was corrupt")
        .await?;
    let failed = installations
        .get("app")
        .await?
        .expect("the record survives a failure");
    println!(
        "failed: status={} error={:?}",
        failed.status, failed.error_message
    );
    assert_eq!(failed.status, "failed");
    assert_eq!(
        failed.error_message.as_deref(),
        Some("the download was corrupt"),
        "the failure reason must be recorded, or a retry has nothing to report"
    );
    // The same empty-string-instead-of-NULL pattern as above: a failed install still reports `Some("")` for its
    // timestamp rather than `None`, so "was it installed?" cannot be answered with `is_some()`.
    assert_eq!(
        failed.installed_at.as_deref(),
        Some(""),
        "a failure is not an installation, but the timestamp field is an empty string rather than absent"
    );

    // A retry succeeds, and the error from the previous attempt must not linger.
    installations
        .mark_installed(
            "app",
            "Test App",
            Some("2.0.0"),
            Some("/opt/app"),
            Some("script"),
        )
        .await?;
    let installed = installations.get("app").await?.expect("the record exists");
    println!(
        "installed: status={} version={:?} installed_at={:?} error={:?}",
        installed.status, installed.version, installed.installed_at, installed.error_message
    );
    assert_eq!(installed.status, "installed");
    assert_eq!(installed.version.as_deref(), Some("2.0.0"));
    assert!(
        installed.installed_at.is_some(),
        "a successful install records when it happened"
    );
    assert_eq!(installed.install_dir.as_deref(), Some("/opt/app"));

    // The status filter is a separate axis, and the record now appears under its new status.
    let listed = installations.list(Some("installed")).await?;
    assert!(
        listed.iter().any(|r| r.software_id == "app"),
        "the record is listed under its current status"
    );
    assert!(
        installations
            .list(Some("failed"))
            .await?
            .iter()
            .all(|r| r.software_id != "app"),
        "and not under the one it left"
    );

    Ok(())
}

#[tokio::test]
async fn an_upsert_replaces_the_receipt_for_the_same_software_and_adds_another(
) -> Result<(), Box<dyn std::error::Error>> {
    // `upsert` is keyed on `software_id`, so the same id replaces and a different id adds. The row count is the
    // assertion that distinguishes the two, and it is what a wrong key would break.
    let temp = TempDb::new("inst_upsert");
    let installations = temp.installations().await?;

    installations.upsert(&installation("one")).await?;
    installations.upsert(&installation("one")).await?;
    let after_two_same = installations.list(None).await?;
    println!(
        "after two upserts of `one`: {} row(s)",
        after_two_same.len()
    );
    assert_eq!(
        after_two_same.len(),
        1,
        "the same id replaces rather than duplicating"
    );

    installations.upsert(&installation("two")).await?;
    let after_other = installations.list(None).await?;
    assert_eq!(after_other.len(), 2, "a different id adds");

    // An update through `upsert` really changes the stored row rather than leaving the old values.
    let mut changed = installation("one");
    changed.name = "renamed".to_string();
    changed.version = Some("9.9.9".to_string());
    installations.upsert(&changed).await?;

    let one = installations.get("one").await?.expect("still there");
    assert_eq!(one.name, "renamed");
    assert_eq!(one.version.as_deref(), Some("9.9.9"));

    // `update_status` with an error message is the other way to record a failure, and it must store the message.
    installations
        .update_status("two", "failed", Some("disk full"))
        .await?;
    let two = installations.get("two").await?.expect("still there");
    assert_eq!(two.status, "failed");
    assert_eq!(two.error_message.as_deref(), Some("disk full"));

    // Deleting one leaves the other, so the key is not shared.
    installations.delete("one").await?;
    assert!(installations.get("one").await?.is_none());
    assert!(installations.get("two").await?.is_some());

    Ok(())
}

#[tokio::test]
async fn installations_survive_a_reopen_so_a_restart_can_read_them(
) -> Result<(), Box<dyn std::error::Error>> {
    // The point of a receipt: a record written before a restart must be readable after one. Two handles on the
    // same file, in sequence, with the first closed in between -- the shape a restart has.
    let temp = TempDb::new("inst_restart");

    {
        let installations = temp.installations().await?;
        installations
            .mark_installed(
                "persisted",
                "Persisted",
                Some("3.1.4"),
                Some("/opt/persisted"),
                Some("npm"),
            )
            .await?;
        // The handle is dropped here, which closes the pool.
    }

    let reopened = temp.installations().await?;
    let row = reopened
        .get("persisted")
        .await?
        .expect("a record written before the reopen is readable after it");
    println!(
        "after reopen: status={} version={:?} method={:?}",
        row.status, row.version, row.install_method
    );
    assert_eq!(row.status, "installed");
    assert_eq!(row.version.as_deref(), Some("3.1.4"));
    assert_eq!(row.install_method.as_deref(), Some("npm"));
    assert!(row.installed_at.is_some(), "the timestamp survives too");

    Ok(())
}

// -------------------------------------------------------------------------------------------
// settings: upsert, delete and the error path
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn settings_upsert_replaces_and_delete_removes() -> Result<(), Box<dyn std::error::Error>> {
    // `setting_database_tests.rs` already covers get/set/delete/list in eleven tests, so this adds only the
    // structure-level property the plan names: an upsert is a **replace**, and the row count says so.
    let temp = TempDb::new("set_upsert");
    let settings = temp.settings().await?;

    settings.set("k", "first").await?;
    settings.set("k", "second").await?;
    let rows = settings.list_all().await?;
    println!("after two sets of one key: {} row(s)", rows.len());
    assert_eq!(
        rows.len(),
        1,
        "the same key replaces rather than accumulating"
    );
    assert_eq!(settings.get("k").await?.as_deref(), Some("second"));

    // A distinct key adds.
    settings.set("other", "value").await?;
    assert_eq!(settings.list_all().await?.len(), 2);

    // Deleting a key that is not there is not an error, and does not change the count.
    settings.delete("absent").await?;
    assert_eq!(
        settings.list_all().await?.len(),
        2,
        "deleting nothing removes nothing"
    );
    settings.delete("k").await?;
    assert_eq!(settings.list_all().await?.len(), 1);
    assert_eq!(settings.get("k").await?, None, "and the key is gone");

    Ok(())
}

#[tokio::test]
async fn a_failing_query_is_an_error_for_all_three_stores() -> Result<(), Box<dyn std::error::Error>>
{
    // "错误传播". Every store must report a broken query rather than answering with an empty result, which a
    // caller would read as "nothing recorded". Dropping the tables is what makes the query fail without closing
    // the connection, which is not an option because `close` consumes the handle.
    let temp = TempDb::new("errors");

    let downloads = temp.downloads().await?;
    let installations = temp.installations().await?;
    let settings = temp.settings().await?;

    // Working baselines first, so the failures below are attributable to the dropped tables.
    downloads
        .add("g", vec!["https://x.invalid".to_string()], None, None)
        .await?;
    installations.mark_installing("s", "S").await?;
    settings.set("k", "v").await?;

    // Drop the three tables through a separate connection, because the stores do not expose theirs.
    let observer = burncloud_database::create_database_with_url(&temp.url()).await?;
    for table in ["sys_downloads", "sys_installations", "sys_settings"] {
        observer
            .execute_query(&format!("DROP TABLE {table}"))
            .await?;
    }
    observer.close().await?;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Downloads: a list must not read as "no downloads".
    let listed = downloads.list(None).await;
    match &listed {
        Ok(rows) => panic!(
            "a broken list returned {} rows; empty means \"none recorded\"",
            rows.len()
        ),
        Err(e) => println!("download list -> {e}"),
    }
    assert!(listed.is_err());
    assert!(
        downloads.update_status("g", "active").await.is_err(),
        "a failed write"
    );
    assert!(downloads.delete("g").await.is_err(), "a failed delete");

    // Installations: the same, and the receipt lookup in particular.
    let receipt = installations.get("s").await;
    match &receipt {
        Ok(Some(_)) => panic!("a broken lookup returned a receipt"),
        // `Ok(None)` is the wrong answer and the dangerous one: it means "never installed", so a caller would
        // reinstall or overwrite.
        Ok(None) => panic!("a broken lookup returned Ok(None), which reads as \"not installed\""),
        Err(e) => println!("installation get -> {e}"),
    }
    assert!(receipt.is_err());
    assert!(installations.list(None).await.is_err());
    assert!(installations
        .mark_installed("s", "S", None, None, None)
        .await
        .is_err());

    // Settings: `get` is the same trap, and `list_all` would read as "nothing configured".
    let got = settings.get("k").await;
    match &got {
        Ok(Some(v)) => panic!("a broken lookup returned {v:?}"),
        Ok(None) => panic!("a broken lookup returned Ok(None), which reads as \"not configured\""),
        Err(e) => println!("setting get -> {e}"),
    }
    assert!(got.is_err());
    assert!(settings.list_all().await.is_err());

    Ok(())
}

#[tokio::test]
async fn the_two_stores_do_not_see_each_others_records() -> Result<(), Box<dyn std::error::Error>> {
    // Isolation between two databases, asserted rather than assumed. Both handles are open at once, so this is a
    // test of separation rather than of ordering.
    let first_temp = TempDb::new("iso_a");
    let second_temp = TempDb::new("iso_b");

    let first = first_temp.downloads().await?;
    let second = second_temp.downloads().await?;

    first
        .add(
            "only-first",
            vec!["https://x.invalid".to_string()],
            None,
            None,
        )
        .await?;

    let in_first = first.list(None).await?;
    let in_second = second.list(None).await?;
    println!(
        "first {} row(s), second {} row(s)",
        in_first.len(),
        in_second.len()
    );

    assert_eq!(in_first.len(), 1, "the first store has its task");
    assert!(
        in_second.is_empty(),
        "and the second has none, so the two are separate databases"
    );

    Ok(())
}
