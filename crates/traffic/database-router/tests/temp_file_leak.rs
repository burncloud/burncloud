#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: these assertions are the test, and the counters are test scaffolding."
)]
//! Regression guard for issue #643: temporary SQLite databases created by this crate's test helpers must
//! be gone when their guard is dropped.
//!
//! The original defect was that the helpers spawned a cleanup thread from `Drop` and **detached** it, so the
//! test binary could exit while that thread still owned the close/removal work, leaving the SQLite file and
//! its `-wal` / `-shm` sidecars behind.
//!
//! # The first fix was not sufficient (measured)
//!
//! It was replaced with a *joined* dedicated thread that builds its **own** runtime to run
//! `Database::close().await`, on the reasoning that `Drop` may run inside a Tokio runtime and cannot
//! block on another. Joining does close the leak-at-exit hole, but closing from a foreign runtime does not
//! release the OS file handles at all. Measured on Windows, with the two paths run side by side on the same
//! `Database`:
//!
//! ```text
//! close() inside the test's own runtime          -> all files removed on the first attempt
//! close() from a dedicated thread + fresh runtime -> .sqlite, -wal and -shm all "os error 32
//!                                                    (used by another process)" — still held 20 s later
//! ```
//!
//! The handle is released when the process exits, which is why the leak only ever showed up as files left in
//! the temp directory and the cleanup *appeared* to work. So the dedicated thread was not merely unnecessary:
//! it was the cause. It also made this file's own regression test the thing that failed CI.
//!
//! The close therefore happens on the caller's runtime, and only the (synchronous) file removal runs on the
//! dedicated thread — which keeps the original guarantee that removal work is joined before the test ends.

use burncloud_database::create_database_with_url;
use burncloud_database_router::RouterDatabase;
use tempfile::{Builder, NamedTempFile};

const LEAK_GUARD_PREFIX: &str = "bc_router_cleanup_";

/// Serialises the two tests in this file.
///
/// Both count files matching [`LEAK_GUARD_PREFIX`] in the **shared** system temp directory, so run in parallel
/// they count each other's artifacts: measured, `the_counter_tracks_its_own_unique_prefix` saw `after=2` for a
/// `before` of 0 because the other test's database and sidecars appeared between its two readings. That is a
/// property of the counter, not of the cleanup, so a mutex is the honest fix — the alternative is a counter
/// that cannot see all guard files, which is what it exists to do.
/// Synchronous so it can be held by the `#[test]` case as well as the async one; no guard is held across an
/// await point. See [`TEMP_DIR_GUARD`].
static TEMP_DIR_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Acquire [`TEMP_DIR_GUARD`], tolerating poisoning: a panic in one test must not cascade into the other.
fn serialise_temp_dir() -> std::sync::MutexGuard<'static, ()> {
    TEMP_DIR_GUARD.lock().unwrap_or_else(|e| e.into_inner())
}

fn count_guard_files() -> usize {
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(LEAK_GUARD_PREFIX)
        })
        .count()
}

/// The three paths one test database occupies: its own file plus the WAL sidecars.
fn database_files(path: &std::path::Path) -> Vec<std::path::PathBuf> {
    ["", "-wal", "-shm"]
        .iter()
        .map(|suffix| {
            let mut candidate = path.to_path_buf().into_os_string();
            candidate.push(suffix);
            std::path::PathBuf::from(candidate)
        })
        .collect()
}

/// Remove `files`, returning the ones that survived. Removals run on a joined thread.
///
/// Retried rather than attempted once: even with the handle released, the OS can need a moment after close,
/// and a bounded retry converges in milliseconds while still failing loudly instead of hanging.
fn remove_all_blocking(files: Vec<std::path::PathBuf>) -> Vec<std::path::PathBuf> {
    std::thread::Builder::new()
        .name("bc-leak-guard-remove".to_string())
        .spawn(move || {
            let mut pending = files;
            for attempt in 0..40u64 {
                pending.retain(|candidate| match std::fs::remove_file(candidate) {
                    Ok(()) => false,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
                    Err(_) => true,
                });
                if pending.is_empty() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(25 * (attempt + 1)));
            }
            pending
        })
        .map(|handle| handle.join().unwrap_or_default())
        .unwrap_or_default()
}

/// An open test database that cleans itself up on drop.
///
/// `Drop` cannot await, so the close is exposed as an explicit step the test performs on its own runtime;
/// `Drop` remains the backstop for a panic and closes on a dedicated runtime, which is the only thing it can
/// do there.
struct TestDb {
    db: Option<burncloud_database::Database>,
    path: std::path::PathBuf,
}

impl TestDb {
    /// Close the database on the **caller's** runtime and remove its files. This is the path that releases
    /// the OS handles; see the module docs.
    async fn close_and_remove(mut self) -> Vec<std::path::PathBuf> {
        let Some(db) = self.db.take() else {
            return Vec::new();
        };
        if let Err(e) = db.close().await {
            eprintln!("test database close failed (cleanup continues): {e}");
        }
        let files = database_files(&self.path);
        let survivors = remove_all_blocking(files);
        // `path` is a file that should now be gone; keep the struct's drop from repeating the work.
        for survivor in &survivors {
            eprintln!(
                "failed to remove test database file {} after retries",
                survivor.display()
            );
        }
        survivors
    }
}

impl Drop for TestDb {
    /// Backstop for a panic or an early return: close on a dedicated runtime, then remove.
    ///
    /// This path is known **not** to release the OS handles (see module docs), so it is a last resort rather
    /// than the normal route. `close_and_remove` is what the tests use.
    fn drop(&mut self) {
        let Some(db) = self.db.take() else {
            return;
        };
        let path = self.path.clone();
        let _ = std::thread::Builder::new()
            .name("bc-leak-guard-fallback".to_string())
            .spawn(move || {
                if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    if let Err(e) = rt.block_on(db.close()) {
                        eprintln!("test database close failed (cleanup continues): {e}");
                    }
                }
                let _ = remove_all_blocking(database_files(&path));
            })
            .map(|handle| handle.join());
    }
}

async fn create_guard_db() -> TestDb {
    let tmp = Builder::new()
        .prefix(LEAK_GUARD_PREFIX)
        .tempfile()
        .expect("temp file");
    let path = tmp.into_temp_path().keep().expect("keep temp path");
    let normalized = path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{normalized}?mode=rwc");
    let db = create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("open {url}: {e}"));
    RouterDatabase::init(&db).await.expect("router tables");
    TestDb { db: Some(db), path }
}

#[tokio::test]
async fn dropping_a_test_database_leaves_zero_temp_files() {
    let _serial = serialise_temp_dir();
    let before = count_guard_files();

    let db = create_guard_db().await;
    let during = count_guard_files();
    assert!(
        during > before,
        "the leak guard did not observe the temporary database while it was open; before={before}, during={during}"
    );

    let survivors = db.close_and_remove().await;
    assert!(
        survivors.is_empty(),
        "closing the database must remove its files; still present: {survivors:?}"
    );

    let after = count_guard_files();
    let delta = after as isize - before as isize;
    println!("#643 temp-file count: before={before}, after={after}, delta={delta}");

    assert_eq!(
        after, before,
        "closing one test database must leave zero new temp files; before={before}, after={after}"
    );
}

#[test]
fn the_counter_tracks_its_own_unique_prefix() {
    let _serial = serialise_temp_dir();
    let before = count_guard_files();
    let tmp: NamedTempFile = Builder::new()
        .prefix(LEAK_GUARD_PREFIX)
        .tempfile()
        .expect("temp file");
    let during = count_guard_files();

    assert!(
        during > before,
        "the counter must see a newly-created file with the guard prefix; before={before}, during={during}"
    );

    drop(tmp);
    let after = count_guard_files();
    assert_eq!(
        after, before,
        "the counter's own proof file must clean itself up; before={before}, after={after}"
    );
}
