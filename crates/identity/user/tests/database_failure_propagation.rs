#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic_in_result_fn,
    clippy::let_underscore_must_use,
    reason = "Test-only file. A failed assertion is the intended failure signal, and the temporary-database \
              cleanup below is best-effort by design: the file is often already gone, and a cleanup failure \
              must not mask the test's own result. An integration test is its own crate root, so it cannot \
              inherit the crate's allowances."
)]
//! Database-failure propagation for the identity user store (#633, plan item B).
//!
//! The plan's ruled gap was that the user side never proved what happens when the database itself
//! fails: a repository that maps "table is gone" onto `Ok(None)` looks exactly like "no such user",
//! and a caller cannot tell a missing row from a broken schema. `crates/identity/token/tests/
//! token_service_entries.rs` already proves this for the token store by dropping its table through a
//! second connection; this file does the same for `user_accounts`.
//!
//! The pause after the drop is not decoration. The fixture owns one pool and the breaker owns
//! another, and SQLite shows the schema change to the first pool when it next prepares a statement
//! — without the pause a fast machine can still serve the old prepared statement and the test would
//! pass for the wrong reason.

use burncloud_database::{create_database_with_url, Database};
use burncloud_service_user::UserDatabase;

/// A temporary database with the production schema, deleted at the end of the test.
///
/// Mirrors the fixture in `user_rules.rs`: a file rather than `:memory:` (the pool opens several
/// connections and an in-memory SQLite database is per-connection), and an explicit close-and-wait
/// before removal, because dropping a `Database` leaves the file locked on Windows.
struct TempDb {
    db: Database,
    path: std::path::PathBuf,
}

impl TempDb {
    async fn new(tag: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!(
            "bc_user_failure_{}_{}_{}.db",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&path);
        let normalized = path.to_string_lossy().replace('\\', "/");
        // Three slashes: `sqlite:///C:/...` is the absolute form. Two makes SQLite read `C:` as a
        // host and fail with `(code: 14) unable to open database file`.
        let url = format!("sqlite:///{}?mode=rwc", normalized);
        let db = create_database_with_url(&url).await?;
        UserDatabase::init(&db).await?;
        Ok(Self { db, path })
    }

    /// A second pool onto the same file, used to change the schema underneath the first one.
    async fn breaker(&self) -> Result<Database, Box<dyn std::error::Error>> {
        let normalized = self.path.to_string_lossy().replace('\\', "/");
        let url = format!("sqlite:///{}?mode=rwc", normalized);
        Ok(create_database_with_url(&url).await?)
    }

    async fn cleanup(self) {
        let path = self.path.clone();
        self.db.close().await.ok();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let _ = std::fs::remove_file(&path);
        for suffix in ["-wal", "-shm"] {
            let mut candidate = path.as_os_str().to_os_string();
            candidate.push(suffix);
            let _ = std::fs::remove_file(std::path::PathBuf::from(candidate));
        }
    }
}

/// A broken schema must surface as an error, not as an absent user.
///
/// This is the distinction the plan asks for: `Err` (the caller learns the read failed) versus
/// `Ok(None)` / `false` / a silent zero (the caller believes the row simply does not exist).
#[tokio::test]
async fn a_broken_user_schema_is_reported_as_an_error_and_not_as_an_absent_user(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDb::new("dropped-table").await?;

    // Control measurement on the intact schema: the same call returns a well-formed "no such user".
    // Without this the test could not tell "the error comes from the broken schema" from "this call
    // errors for everyone".
    let before = UserDatabase::get_user_by_username(&temp.db, "absent").await;
    assert!(
        matches!(before, Ok(None)),
        "with an intact schema the read must succeed and report absence: {before:?}"
    );

    // Break the schema through the second connection, then let the first pool notice.
    let breaker = temp.breaker().await?;
    breaker
        .execute_query("DROP TABLE user_accounts")
        .await
        .expect("dropping the table must work or the test is not testing a failure");
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    let after = UserDatabase::get_user_by_username(&temp.db, "absent").await;
    assert!(
        after.is_err(),
        "a dropped table must be an error, not an absent user: {after:?}"
    );
    // And the failure must not be reported as a successful empty read either.
    assert!(
        !matches!(after, Ok(None)),
        "mapping a broken schema onto `Ok(None)` hides database failures from every caller"
    );

    drop(breaker);
    temp.cleanup().await;
    Ok(())
}
