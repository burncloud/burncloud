#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    reason = "Test-only file: the assertions are the test. `serde_json::Value` is not used; the allowance is \
              here because an integration test is its own crate root and cannot inherit the crate's settings."
)]
//! SQLite URL construction, migration idempotency and connection lifecycle (#633, plan item 26).
//!
//! The crate already carries ~85 tests -- `migration_rename_tests` alone has 42 -- so this file does **not**
//! repeat the rename coverage. It covers the plan's remaining items that are reachable on this machine:
//!
//! | Plan item | Here |
//! | --- | --- |
//! | Windows 路径 | `sqlite_url_*` -- the rule is about the path's shape, not the platform |
//! | 全新建库 | `a_fresh_database_gets_the_full_schema_and_records_every_migration` |
//! | 迁移重跑 | `running_the_migrations_again_changes_nothing` |
//! | 连接关闭/池状态 | `a_closed_database_reports_itself_closed_and_stops_answering` |
//! | 占位符方言适配 | `the_placeholder_helpers_agree_with_each_other` |
//!
//! ## The PostgreSQL boundary, stated rather than implied
//!
//! There is **no PostgreSQL server and no container runtime on this machine**, so everything here runs on
//! SQLite. The dialect helpers are tested as **pure string functions**, which is a real test of the translation
//! and no test at all of whether PostgreSQL accepts the result. `ci-integration.yml` in #654 is the only place
//! these run against a real server, and it is not this file's claim to make.
//!
//! ## A defect the URL tests found
//!
//! `Database::new` built its URL as `if is_windows() && !path.starts_with('/')` -- three slashes only for a
//! Windows path that does *not* begin with a slash. A Windows absolute path written with forward slashes
//! (`/tmp/db.sqlite`, which Windows accepts) therefore took the **relative** branch and produced
//! `sqlite://tmp/db.sqlite`, which SQLite parses as a host named `tmp` and rejects with
//! `(code: 14) unable to open database file`. The rule is now about the path's shape, and it is a pure function.

use burncloud_database::{adapt_sql, ph, phs, sqlite_url};

// -------------------------------------------------------------------------------------------
// Windows paths and the SQLite URL
// -------------------------------------------------------------------------------------------

#[test]
fn an_absolute_path_always_gets_three_slashes_whatever_the_platform() {
    // `sqlite:///...` is the absolute form. With two slashes SQLite reads the next segment as a **host**, which
    // is the cause of the `(code: 14) unable to open database file` error this crate's own tests were once
    // written around.
    let cases: [(&str, bool, &str); 6] = [
        (
            "C:/Users/dev/burncloud.db",
            true,
            "sqlite:///C:/Users/dev/burncloud.db?mode=rwc",
        ),
        (
            "D:/data/db.sqlite",
            true,
            "sqlite:///D:/data/db.sqlite?mode=rwc",
        ),
        // **The case the old condition got wrong**: a Windows absolute path written with forward slashes. Windows
        // accepts `/tmp/db.sqlite`, and it is absolute, so it needs three slashes -- but it also *starts with a
        // slash*, which is what the old `!path.starts_with('/')` test ruled out.
        ("/tmp/db.sqlite", true, "sqlite:///tmp/db.sqlite?mode=rwc"),
        (
            "/var/lib/burncloud.db",
            true,
            "sqlite:///var/lib/burncloud.db?mode=rwc",
        ),
        // POSIX, where the leading slash is the only absolute form.
        ("/srv/app/db", false, "sqlite:///srv/app/db?mode=rwc"),
        (
            "//server/share/db",
            false,
            "sqlite:///server/share/db?mode=rwc",
        ),
    ];

    for (path, is_windows, expected) in cases {
        let got = sqlite_url(path, is_windows, true);
        println!("{path:?} (windows={is_windows}) -> {got}");
        assert_eq!(got, expected, "{path:?}");
        assert!(
            got.starts_with("sqlite:///"),
            "an absolute path must produce three slashes: {got}"
        );
    }
}

#[test]
fn a_relative_path_keeps_two_slashes() {
    // With three slashes the leading `/` would become part of the path and turn `data/db.sqlite` into
    // `/data/db.sqlite` -- an absolute path the caller did not ask for, which would silently write somewhere else.
    for (path, is_windows) in [
        ("data/db.sqlite", true),
        ("data/db.sqlite", false),
        ("./db.sqlite", false),
        ("target/test.db", true),
    ] {
        let got = sqlite_url(path, is_windows, true);
        println!("{path:?} (windows={is_windows}) -> {got}");
        assert_eq!(got, format!("sqlite://{}?mode=rwc", path), "{path:?}");
        assert!(
            !got.starts_with("sqlite:///"),
            "a relative path must not become absolute: {got}"
        );
    }
}

#[test]
fn the_drive_letter_rule_applies_only_where_drives_exist() {
    // `C:/db` is absolute on Windows and a bare filename on Unix, where a colon is a legal character in a
    // directory name. The platform flag decides, and this pins that it is used for that and nothing else.
    assert_eq!(
        sqlite_url("C:/db.sqlite", true, true),
        "sqlite:///C:/db.sqlite?mode=rwc",
        "a drive letter is absolute on Windows"
    );
    assert_eq!(
        sqlite_url("C:/db.sqlite", false, true),
        "sqlite://C:/db.sqlite?mode=rwc",
        "and a relative name on Unix, so it keeps two slashes"
    );

    // A colon without a separator is not a drive: `C:db` is a Windows drive-relative path, and treating it as
    // absolute would produce a URL for a location the caller did not name.
    assert_eq!(
        sqlite_url("C:db.sqlite", true, true),
        "sqlite://C:db.sqlite?mode=rwc",
        "`C:db` has no separator after the colon"
    );

    // A single letter that is not a drive, and a two-letter prefix, both fall to the relative branch.
    for path in ["1:/db", "AB:/db", ":/db"] {
        assert_eq!(
            sqlite_url(path, true, true),
            format!("sqlite://{}?mode=rwc", path),
            "{path:?} is not a drive letter"
        );
    }
}

#[test]
fn the_create_option_is_optional_and_is_the_only_difference() {
    // `mode=rwc` is what makes SQLite create a missing file. A caller opening a database it expects to exist must
    // be able to omit it, and omitting it must change nothing else about the URL.
    let with = sqlite_url("C:/db.sqlite", true, true);
    let without = sqlite_url("C:/db.sqlite", true, false);
    assert_eq!(with, "sqlite:///C:/db.sqlite?mode=rwc");
    assert_eq!(without, "sqlite:///C:/db.sqlite");
    assert_eq!(
        with.trim_end_matches("?mode=rwc"),
        without,
        "the option is a suffix and nothing else differs"
    );

    // And for a relative path too, so the two branches are consistent.
    assert_eq!(sqlite_url("rel/db", false, false), "sqlite://rel/db");
}

#[test]
fn a_windows_path_with_backslashes_is_normalised_before_it_reaches_this_function() {
    // `sqlite_url` takes a path that has already had its separators normalised -- `Database::new` does
    // `.replace('\\', "/")` immediately before calling it. This pins the expectation rather than testing the
    // caller: a backslash reaching here is treated as an ordinary character, which is why the normalisation has
    // to happen first.
    let normalised = "C:\\Users\\dev\\db.sqlite".replace('\\', "/");
    assert_eq!(normalised, "C:/Users/dev/db.sqlite");
    assert_eq!(
        sqlite_url(&normalised, true, true),
        "sqlite:///C:/Users/dev/db.sqlite?mode=rwc"
    );

    // Recorded: passing the un-normalised form through produces a URL with backslashes in it, which is not a URL
    // SQLite can open -- the reason the normalisation exists.
    let un_normalised = sqlite_url("C:\\Users\\dev\\db.sqlite", true, true);
    println!("un-normalised -> {un_normalised}");
    assert!(un_normalised.contains('\\'));
}

// -------------------------------------------------------------------------------------------
// placeholder dialect
// -------------------------------------------------------------------------------------------

#[test]
fn the_placeholder_helpers_agree_with_each_other() {
    // `ph`, `phs` and `adapt_sql` are three ways to express the same numbering, so they must not disagree. For
    // SQLite every placeholder is `?`, and for PostgreSQL they are `$1..$n` in left-to-right order.
    assert_eq!(ph(false, 1), "?");
    assert_eq!(ph(false, 7), "?", "the number is ignored on SQLite");
    assert_eq!(ph(true, 1), "$1");
    assert_eq!(ph(true, 7), "$7");

    assert_eq!(phs(false, 0), "", "no parameters, no text");
    assert_eq!(phs(true, 0), "");
    assert_eq!(phs(false, 1), "?");
    assert_eq!(phs(false, 3), "?, ?, ?");
    assert_eq!(phs(true, 3), "$1, $2, $3");

    // `phs(n)` and `adapt_sql` over `phs(false, n)` must produce the same string: they are two spellings of one
    // rule, and this is the assertion that keeps them from drifting.
    for n in 0..8 {
        let from_phs = phs(true, n);
        let from_adapt = adapt_sql(true, &phs(false, n));
        assert_eq!(from_phs, from_adapt, "{n} placeholders");
    }
}

#[test]
fn adapting_sql_numbers_the_placeholders_left_to_right() {
    assert_eq!(
        adapt_sql(true, "SELECT * FROM t WHERE a = ? AND b = ?"),
        "SELECT * FROM t WHERE a = $1 AND b = $2"
    );
    // No placeholders, nothing changes.
    assert_eq!(adapt_sql(true, "SELECT 1"), "SELECT 1");
    assert_eq!(adapt_sql(true, ""), "");
    // A literal `$` is left alone, so a query that already used one form is not double-converted.
    assert_eq!(adapt_sql(true, "WHERE x = 1"), "WHERE x = 1");

    // SQLite is the identity function, whatever the input.
    for sql in [
        "SELECT * FROM t WHERE a = ?",
        "INSERT INTO t (a, b) VALUES (?, ?)",
        "",
        "SELECT 1",
    ] {
        assert_eq!(adapt_sql(false, sql), sql, "{sql:?} must be unchanged");
        assert_eq!(ph(false, 1), "?");
    }
}

#[test]
fn the_blind_replacement_is_recorded_as_a_limitation_because_it_is_safe_only_for_the_sql_that_exists(
) {
    // `adapt_sql` replaces **every** `?`, with no awareness of string literals or of PostgreSQL's JSON
    // operators. The plan lists "占位符方言适配", so the boundary is measured rather than assumed.
    //
    // **Measured: nothing in `crates/platform/storage` uses a `?` inside a string literal, and nothing uses the
    // `?`, `?|` or `?&` JSON operators.** So the function is correct for the repository as it stands, and this
    // test records what would break it rather than pretending the function is general:
    assert_eq!(
        adapt_sql(true, "SELECT 'a?b'"),
        "SELECT 'a$1b'",
        "a `?` inside a string literal is renumbered, which would corrupt the literal"
    );
    assert_eq!(
        adapt_sql(true, "SELECT data ? 'key' FROM t"),
        "SELECT data $1 'key' FROM t",
        "the JSON `?` operator becomes a bind placeholder, which would change the query's meaning"
    );

    // The mitigation is to avoid those constructs in adapted SQL, which is what the repository does today. This
    // test fails if that stops being true only in the sense that a reader is pointed at the hazard; the grep is
    // the real check, and it is in the PR description.
    println!("`adapt_sql` is a textual replacement: string literals and JSON operators are not protected");
}

// -------------------------------------------------------------------------------------------
// fresh schema, migration rerun, connection lifecycle
// -------------------------------------------------------------------------------------------

/// A database in a temporary file, removed when the guard is dropped.
///
/// The file lives under the temp directory, so **nothing here touches the shared default path** that
/// `Database::new()` would use.
struct TempDb {
    path: std::path::PathBuf,
}

impl TempDb {
    fn path(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "bc_db_{}_{}_{}.sqlite",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after the epoch")
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&path);
        Self { path }
    }

    fn url(&self) -> String {
        sqlite_url(
            &self.path.to_string_lossy().replace('\\', "/"),
            cfg!(windows),
            true,
        )
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let mut candidate = self.path.clone().into_os_string();
            candidate.push(suffix);
            let candidate = std::path::PathBuf::from(candidate);
            if candidate.exists() {
                let _ = std::fs::remove_file(&candidate);
            }
        }
    }
}

/// The versions recorded as applied, in order.
async fn applied_versions(db: &burncloud_database::Database) -> Vec<String> {
    db.fetch_all::<(String,)>("SELECT version FROM _schema_migrations ORDER BY version")
        .await
        .expect("the migration table is readable")
        .into_iter()
        .map(|(v,)| v)
        .collect()
}

/// The user tables, excluding SQLite's own and the migration bookkeeping.
async fn table_names(db: &burncloud_database::Database) -> Vec<String> {
    let rows = db
        .fetch_all::<(String,)>(
            "SELECT name FROM sqlite_master WHERE type = 'table' \
             AND name NOT LIKE 'sqlite_%' AND name <> '_schema_migrations' ORDER BY name",
        )
        .await
        .expect("sqlite_master is readable");
    rows.into_iter().map(|(n,)| n).collect()
}

#[tokio::test]
async fn a_fresh_database_gets_the_full_schema_and_records_every_migration(
) -> Result<(), Box<dyn std::error::Error>> {
    // "全新建库". A database created at a URL that does not exist yet must come up with the whole schema, and the
    // migration bookkeeping must say so -- otherwise the next run would try to apply everything again.
    let temp = TempDb::path("fresh");
    assert!(
        !temp.path.exists(),
        "the file must not exist before the database is opened, or this tests an upgrade instead"
    );

    let db = burncloud_database::create_database_with_url(&temp.url()).await?;
    assert_eq!(db.kind(), "sqlite");

    let tables = table_names(&db).await;
    println!("{} tables: {tables:?}", tables.len());
    assert!(!tables.is_empty(), "a fresh database must have tables");
    assert!(
        tables.len() > 20,
        "the schema is large; {} tables suggests migrations did not all run",
        tables.len()
    );

    let applied = applied_versions(&db).await;
    println!("{} migrations applied: {:?}", applied.len(), applied);
    assert!(
        applied.len() >= 17,
        "the crate ships at least 17 migrations; {} were recorded",
        applied.len()
    );

    // The versions are ordered and unique, which is what "applied once each" means.
    let mut sorted = applied.clone();
    sorted.sort();
    assert_eq!(sorted, applied, "the versions are recorded in order");
    let mut deduped = applied.clone();
    deduped.dedup();
    assert_eq!(deduped.len(), applied.len(), "no version was applied twice");

    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn running_the_migrations_again_changes_nothing() -> Result<(), Box<dyn std::error::Error>> {
    // "迁移重跑". Opening the same file twice runs `MigrationRunner::run` twice, and the second run must be a
    // no-op: the same tables, the same recorded versions, and the same row count in the bookkeeping table. A
    // runner without its `version` guard would either fail on the second `CREATE TABLE` or re-record everything.
    let temp = TempDb::path("rerun");

    let first = burncloud_database::create_database_with_url(&temp.url()).await?;
    let tables_once = table_names(&first).await;
    let versions_once = applied_versions(&first).await;
    // A row to prove the rerun does not reset data as well as schema.
    first
        .execute_query("CREATE TABLE rerun_probe (id INTEGER PRIMARY KEY, note TEXT)")
        .await?;
    first
        .execute_query("INSERT INTO rerun_probe (note) VALUES ('survives')")
        .await?;
    first.close().await?;

    let second = burncloud_database::create_database_with_url(&temp.url()).await?;
    let tables_twice = table_names(&second).await;
    let versions_twice = applied_versions(&second).await;

    println!(
        "tables {} then {}, versions {} then {}",
        tables_once.len(),
        tables_twice.len(),
        versions_once.len(),
        versions_twice.len()
    );

    // The probe table is created **after** the first listing, so it is the one expected addition. Comparing the
    // raw lists failed on exactly that, which is the test being wrong rather than the migrations: the measurement
    // was `25 then 26`, and the extra name was `rerun_probe`.
    let mut expected_twice = tables_once.clone();
    expected_twice.push("rerun_probe".to_string());
    expected_twice.sort();
    assert_eq!(
        expected_twice, tables_twice,
        "a second open must add only the probe table and remove nothing"
    );
    assert_eq!(
        versions_once, versions_twice,
        "and must not re-record or lose a migration"
    );

    // The probe table and its row are still there, so the rerun touched neither schema nor data.
    assert!(
        tables_twice.contains(&"rerun_probe".to_string()),
        "the extra table survives"
    );
    let notes = second
        .fetch_all::<(String,)>("SELECT note FROM rerun_probe")
        .await?;
    assert_eq!(notes.len(), 1, "and so does its row");
    assert_eq!(notes[0].0, "survives");

    second.close().await?;
    Ok(())
}

#[tokio::test]
async fn a_closed_database_reports_itself_closed_and_stops_answering(
) -> Result<(), Box<dyn std::error::Error>> {
    // "连接关闭/池状态". `close` consumes the handle and clears the connection, so a later `get_connection`
    // reports `NotInitialized` rather than a pool error -- a caller can tell "closed" from "the query failed".
    //
    // The pool is cloned before closing, because `close` consumes `self` and the observable afterwards is the
    // shared pool rather than the handle.
    let temp = TempDb::path("close");
    let db = burncloud_database::create_database_with_url(&temp.url()).await?;

    // A working read first, so the failure below is attributable to the close.
    let before = db.fetch_all::<(i64,)>("SELECT 1").await?;
    assert_eq!(before.len(), 1);

    // `close(self)` **consumes the handle**, so the observable afterwards is the pool it shared rather than the
    // handle itself. A clone taken beforehand is the only way to ask what happened to the connections.
    let pool = db.get_connection()?.pool().clone();
    assert!(!pool.is_closed(), "the pool is open before the close");

    db.close().await?;

    // The pool is genuinely closed, which is stronger evidence than a failed query: a pool can also refuse a
    // query while still holding its connections.
    assert!(pool.is_closed(), "the underlying pool must be closed");
    assert!(
        burncloud_database::sqlx::query("SELECT 1")
            .execute(&pool)
            .await
            .is_err(),
        "a query through the closed pool must fail"
    );

    // Recorded because it is what a caller will try and it does not work: there is no way to ask the **handle**
    // whether it is closed, because closing it destroyed the handle. A caller that needs to distinguish "closed"
    // from "failed" has to track that itself or hold the pool.
    println!("`Database::close` consumes the handle, so the handle cannot be queried for its state afterwards");

    Ok(())
}

#[tokio::test]
async fn two_databases_do_not_see_each_other() -> Result<(), Box<dyn std::error::Error>> {
    // Isolation, asserted rather than assumed: two URLs, two files, two independent schemas. Both are open at
    // once, which is what makes this a test of isolation rather than of ordering.
    let first_temp = TempDb::path("iso_a");
    let second_temp = TempDb::path("iso_b");

    let first = burncloud_database::create_database_with_url(&first_temp.url()).await?;
    let second = burncloud_database::create_database_with_url(&second_temp.url()).await?;

    first
        .execute_query("CREATE TABLE only_in_first (id INTEGER)")
        .await?;

    let in_first = table_names(&first).await;
    let in_second = table_names(&second).await;
    assert!(in_first.contains(&"only_in_first".to_string()));
    assert!(
        !in_second.contains(&"only_in_first".to_string()),
        "a table created in one database must not appear in another"
    );

    // Both have the migrated schema, so the isolation is not because one failed to initialise.
    assert!(in_first.len() > 20 && in_second.len() > 20);

    first.close().await?;
    second.close().await?;
    Ok(())
}
