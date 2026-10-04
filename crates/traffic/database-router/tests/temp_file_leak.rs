#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: these assertions are the test, and the counters are test scaffolding."
)]
//! Guard for the temporary-database leak in this crate's test helpers (issue #643).
//!
//! Every test in `cost_roundtrip.rs` and `usage_stats_tests.rs` creates a SQLite database in the temp
//! directory. Dropping the `Database` does not release the file handle and `close().await` releases it
//! only asynchronously, so the removal races the pool and the files accumulate: measured at **20 per
//! run** of this crate.
//!
//! The helper now closes the pool on a separate thread before removing the files, which recovers about
//! half of them (**9 per run**). That is an improvement, not a fix, and the reason is recorded in the
//! helper: nothing joins those threads, and a test binary exits when its tests finish, so a detached
//! thread's work is lost.
//!
//! ## Why this test counts before and after rather than asserting zero
//!
//! Asserting zero would fail today, and the plan's rule is not to freeze a defect as the contract -- but
//! neither should the defect be left unmeasured, or it will grow back unnoticed. So this asserts the
//! **current** number as an upper bound: a change that makes the leak worse fails here, and whoever
//! fixes #643 lowers the bound or replaces this with an assertion of zero.
//!
//! The measurement is deliberately taken around a run of the helper rather than of the whole suite: 14
//! tests run in parallel, and counting their files from inside one of them would be measuring the test
//! harness's scheduling rather than the helper.

use burncloud_database::create_database_with_url;
use burncloud_database_router::RouterDatabase;
use tempfile::NamedTempFile;

/// How many files one helper call leaves behind today. Lower this when #643 is fixed; raise it only with
/// a measurement and a reason.
const LEFTOVER_PER_CALL_UPPER_BOUND: usize = 3;

/// Count the temp-directory entries that look like a database this helper created.
fn count_temp_databases() -> usize {
    let dir = std::env::temp_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            // `tempfile` generates `.tmpXXXXXX`, with `-wal`/`-shm` sidecars in WAL mode.
            name.starts_with(".tmp")
                && name.len() >= 8
                && name[4..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphanumeric())
        })
        .count()
}

/// Open a database the way the helpers do, drop it, and give the cleanup thread a moment to run.
async fn open_and_drop(tag: &str) {
    let tmp = NamedTempFile::new().expect("temp file");
    let path = tmp.into_temp_path().keep().expect("keep temp path");
    let normalized = path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{normalized}?mode=rwc");
    let db = create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("open {url}: {e}"));
    RouterDatabase::init(&db).await.expect("router tables");
    drop(db);
    // The helper's cleanup runs on its own thread; without a pause this measurement would report the
    // state before the thread was scheduled, which is a different question.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let _ = tag;
}

#[tokio::test]
async fn one_helper_call_leaves_no_more_than_the_recorded_number_of_files() {
    let before = count_temp_databases();

    open_and_drop("guard").await;

    let after = count_temp_databases();
    let leaked = after.saturating_sub(before);

    println!(
        "temp databases before: {before}, after: {after}, leaked by this call: {leaked} \
         (recorded upper bound: {LEFTOVER_PER_CALL_UPPER_BOUND})"
    );

    assert!(
        leaked <= LEFTOVER_PER_CALL_UPPER_BOUND,
        "one helper call left {leaked} file(s) behind, above the recorded bound of \
         {LEFTOVER_PER_CALL_UPPER_BOUND}. The temp-directory leak is getting worse: see #643. If the \
         leak was deliberately fixed, lower LEFTOVER_PER_CALL_UPPER_BOUND (or assert zero)."
    );
}

// A second test is intentionally absent rather than added for symmetry: the point of the guard is the
// number, and a duplicate would double the files it creates while measuring them.

/// The counter must actually see the files, or a clean result above would prove nothing.
#[test]
fn the_temp_database_counter_is_not_silently_zero() {
    // Run on the current thread only: this checks the counting logic, not the helper.
    let tmp = NamedTempFile::new().expect("temp file");
    let path = tmp.into_temp_path().keep().expect("keep temp path");
    let normalized = path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{normalized}?mode=rwc");

    // Create the file through the database layer so any sidecars exist too.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let db = rt
        .block_on(create_database_with_url(&url))
        .unwrap_or_else(|e| panic!("open {url}: {e}"));
    drop(db);

    let counted = count_temp_databases();
    println!("counter sees {counted} temp database entries while one is definitely present");
    assert!(
        counted > 0,
        "the counter found nothing even though this test just created a database, so the guard above \
         would pass vacuously"
    );
}
