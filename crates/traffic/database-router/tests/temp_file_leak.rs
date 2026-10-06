#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: these assertions are the test, and the counters are test scaffolding."
)]
//! Regression guard for issue #643: temporary SQLite databases created by this crate's test helpers must
//! be gone when their guard is dropped.
//!
//! The old helpers spawned a cleanup thread from `Drop` but detached it. The test binary could therefore
//! exit while that thread still owned the database close/removal work, leaving the SQLite file and its
//! `-wal` / `-shm` sidecars in the system temp directory. The fixed lifetime is:
//!
//! `drop guard -> dedicated thread -> Database::close().await -> short OS-handle grace -> remove files -> join`
//!
//! The dedicated thread is still necessary because `Drop` often runs from inside a Tokio runtime and may
//! not enter/block another runtime directly. Joining is safe because the async close happens entirely on
//! the other thread. Cleanup failures are logged rather than panicked so cleanup also runs while another
//! panic is already unwinding.

use burncloud_database::create_database_with_url;
use burncloud_database_router::RouterDatabase;
use tempfile::{Builder, NamedTempFile};

const LEAK_GUARD_PREFIX: &str = "bc_router_cleanup_";

fn unique_guard_prefix(tag: &str) -> String {
    format!("{LEAK_GUARD_PREFIX}{tag}_{}_", std::process::id())
}

fn count_guard_files(prefix: &str) -> usize {
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
        .count()
}

struct TestDb {
    db: Option<burncloud_database::Database>,
    path: std::path::PathBuf,
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let Some(db) = self.db.take() else {
            return;
        };
        let path = self.path.clone();
        let cleanup = std::thread::Builder::new()
            .name("bc-leak-guard-cleanup".to_string())
            .spawn(move || {
                if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    if let Err(e) = rt.block_on(db.close()) {
                        eprintln!("test database close failed (cleanup continues): {e}");
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
                for suffix in ["", "-wal", "-shm"] {
                    let mut candidate = path.clone().into_os_string();
                    candidate.push(suffix);
                    let candidate = std::path::PathBuf::from(candidate);
                    if let Err(e) = std::fs::remove_file(&candidate) {
                        if e.kind() != std::io::ErrorKind::NotFound {
                            eprintln!(
                                "failed to remove test database file {}: {e}",
                                candidate.display()
                            );
                        }
                    }
                }
            });

        match cleanup {
            Ok(handle) => {
                if handle.join().is_err() {
                    eprintln!("test database cleanup thread panicked");
                }
            }
            Err(e) => eprintln!("failed to spawn test database cleanup thread: {e}"),
        }
    }
}

async fn create_guard_db(prefix: &str) -> TestDb {
    let tmp = Builder::new().prefix(prefix).tempfile().expect("temp file");
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
    let prefix = unique_guard_prefix("drop");
    let before = count_guard_files(&prefix);

    {
        let _db = create_guard_db(&prefix).await;
        let during = count_guard_files(&prefix);
        assert!(
            during > before,
            "the leak guard did not observe the temporary database while it was open"
        );
    }

    let after = count_guard_files(&prefix);
    let delta = after as isize - before as isize;
    println!("#643 temp-file count: before={before}, after={after}, delta={delta}");

    assert_eq!(
        after, before,
        "dropping one test database must leave zero new temp files; before={before}, after={after}"
    );
}

#[test]
fn the_counter_tracks_its_own_unique_prefix() {
    let prefix = unique_guard_prefix("counter");
    let before = count_guard_files(&prefix);
    let tmp: NamedTempFile = Builder::new()
        .prefix(&prefix)
        .tempfile()
        .expect("temp file");
    let during = count_guard_files(&prefix);

    assert!(
        during > before,
        "the counter must see a newly-created file with the guard prefix"
    );

    drop(tmp);
    let after = count_guard_files(&prefix);
    assert_eq!(
        after, before,
        "the counter's own proof file must clean itself up"
    );
}
