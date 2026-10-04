#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Regression guard for #633: the service-user tests must never open the shared default database.
//!
//! Why this is a test and not a comment: the previous version of the suite called
//! `create_default_database()`, which resolves to the developer's real
//! `AppData/Local/BurnCloud/data.db`. Running `cargo test -p burncloud-service-user` therefore
//! inserted four `testuser_<uuid>` rows into live data. A comment would not have caught that.
//!
//! The guard reads the source, because the property being protected is a property of the source.
//! Comments are stripped before matching: the helper's own documentation explains why an in-memory
//! database is wrong and therefore contains the very string a naive `contains` check would flag.

use std::fs;
use std::path::PathBuf;

fn read(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Source with comment lines removed, so assertions match code rather than prose.
fn code_only(text: &str) -> String {
    text.lines()
        .filter(|l| {
            let t = l.trim_start();
            !(t.starts_with("//") || t.starts_with('*') || t.starts_with("/*"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn tests_do_not_open_the_default_database() {
    let code = code_only(&read("src/lib.rs"));
    assert!(
        !code.contains("create_default_database"),
        "src/lib.rs calls `create_default_database`, which opens the developer's real database at \
         %USERPROFILE%/AppData/Local/BurnCloud/data.db. Tests must use the `test_db()` helper \
         instead."
    );
    assert!(
        code.contains("create_database_with_url"),
        "the isolated database helper disappeared from src/lib.rs"
    );
}

#[test]
fn every_database_test_cleans_up_its_temporary_database() {
    // The helper has no `Drop` impl -- a type with one cannot hand its fields out for
    // `close().await` -- so cleanup is an explicit call. If a test stops calling it, the temp
    // directory accumulates databases.
    let code = code_only(&read("src/lib.rs"));
    let creations = code.matches("test_db(\"").count();
    let cleanups = code.matches("temp.cleanup().await").count();
    assert!(
        creations > 0,
        "no test creates a temporary database any more"
    );
    assert_eq!(
        creations, cleanups,
        "{creations} test(s) create a temporary database but only {cleanups} call \
         `temp.cleanup().await`, so the rest leak files into the temp directory"
    );
}

#[test]
fn the_helper_uses_a_file_database_not_memory() {
    // An in-memory SQLite database is per-connection, and the pool opens several connections, so the
    // schema would be invisible to most queries. This repository has been bitten by that before.
    let code = code_only(&read("src/lib.rs"));
    assert!(
        !code.contains(":memory:"),
        "the test database helper uses an in-memory SQLite database; with a connection pool that is \
         a per-connection database, so migrations and assertions would not see the same schema"
    );
    assert!(
        code.contains("sqlite:///"),
        "the helper should build a three-slash absolute sqlite URL (Windows needs `sqlite:///C:/...`)"
    );
}
