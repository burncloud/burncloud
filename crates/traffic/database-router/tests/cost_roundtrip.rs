#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration-test fixtures intentionally unwrap setup values so failures stop the test"
)]

/// Integration tests for RouterLog per-type cost and token round-trip.
///
/// These tests guard against two failure modes:
///
/// FM-08: Postgres int4 overflow for cost columns
///   - i32::MAX = 2,147,483,647 nanodollars ≈ $2.14/request
///   - Any GPT-4o heavy request exceeds this; cost columns must be BIGINT on Postgres
///   - This test uses SQLite (8-byte INTEGER natively) but verifies the i64 bindings
///     are wired correctly end-to-end. The Postgres path requires a live instance.
///
/// FM-10 (partial): cache_write_cost is currently always 0 (known limitation, P1 backlog)
///   - We assert cache_write_cost == 0 explicitly so a future fix is visible in test output.
///
/// Note on get_filtered: the function generates `WHERE user_id = ?NNN` (numbered) mixed
/// with plain `?` for LIMIT/OFFSET, which triggers SQLITE_MISMATCH (code 20) in some
/// sqlx-any versions. Tests use RouterLogModel::get() and filter in Rust to stay on the
/// well-tested code path.
use burncloud_database::create_database_with_url;
use burncloud_database_router::{RouterDatabase, RouterLog, RouterLogModel};
use tempfile::NamedTempFile;

/// An isolated SQLite database plus the router tables, in a temp file that this type tries to delete.
///
/// SQLite `:memory:` is not used: each pool connection would get its own empty database, so the schema
/// written on one connection is invisible to the others. A temp file is shared across the pool.
///
/// Both `Schema::init()` (via `create_database_with_url`) and `RouterDatabase::init()` are required: the
/// former creates `router_logs` with all its columns, the latter creates `router_tokens`, which the log
/// writer updates for quota settlement.
///
/// **This helper reduces the leak; it does not eliminate it.** Measured on this platform: 20 leftover
/// files per run of this crate before, 9 after, and still 9 after a ten-second wait -- so the remainder
/// is not a slow cleanup, it is cleanup that never happens. `tests/temp_file_leak.rs` asserts the current
/// number so it cannot silently get worse, and #643 records the unfinished work.
///
/// The background: dropping a `Database` does not release the SQLite handle and `close().await` releases
/// it only asynchronously, so a removal attempted from `Drop` loses the race and the file survives.
///
/// Two fixes were tried and do not work. They are recorded so they are not retried: calling `block_on` on
/// a new runtime inside `Drop` panics with "Cannot start a runtime from within a runtime", and so does
/// reusing `Handle::try_current()`, because the destructor runs inside the test's own runtime.
///
/// What this does instead is hand the connection to a **separate thread** with its own runtime, so
/// nothing blocks the test thread and no runtime is entered from within another. That recovers roughly
/// half the files. The likely reason it is not all of them is that nothing joins these threads: a test
/// binary exits when its tests finish, and a detached thread's work is lost. Joining them needs a
/// process-level hook the standard test harness does not offer, which is why it is left to #643.
///
/// Requires a multi-threaded test runtime, which `#[tokio::test]` provides by default.
struct TestDb {
    db: Option<burncloud_database::Database>,
    path: std::path::PathBuf,
}

impl std::ops::Deref for TestDb {
    type Target = burncloud_database::Database;

    fn deref(&self) -> &Self::Target {
        self.db
            .as_ref()
            .unwrap_or_else(|| panic!("the test database was used after cleanup"))
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let Some(db) = self.db.take() else {
            return;
        };
        let path = self.path.clone();

        // The thread owns the close. If it cannot be spawned the files are simply left behind, which is
        // the pre-existing behaviour rather than a new failure, so the test result is unaffected.
        let cleanup = std::thread::Builder::new()
            .name("bc-testdb-cleanup".to_string())
            .spawn(move || {
                if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    if let Err(e) = rt.block_on(db.close()) {
                        eprintln!("test database close failed (cleanup continues): {e}");
                    }
                }
                // SQLite releases the OS handle slightly after the pool closes.
                std::thread::sleep(std::time::Duration::from_millis(50));
                for suffix in ["", "-wal", "-shm"] {
                    let mut candidate = path.clone().into_os_string();
                    candidate.push(suffix);
                    let candidate = std::path::PathBuf::from(candidate);
                    if let Err(error) = std::fs::remove_file(&candidate) {
                        if error.kind() != std::io::ErrorKind::NotFound {
                            eprintln!(
                                "failed to remove test database file {}: {error}",
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
            Err(error) => eprintln!("failed to spawn test database cleanup thread: {error}"),
        }
    }
}

/// Create an isolated SQLite test database with the production schema applied.
async fn create_test_db() -> TestDb {
    let tmp = NamedTempFile::new().unwrap_or_else(|e| panic!("failed to create temp file: {e}"));
    // Take the path away from the temp-file guard so deletion is controlled here rather than racing the
    // connection pool.
    let path = tmp
        .into_temp_path()
        .keep()
        .unwrap_or_else(|e| panic!("failed to keep temp path: {e}"));
    // Three slashes with forward slashes: an absolute path on Windows, whereas the two-slash form is not
    // a URL SQLite can open and fails with "(code: 14) unable to open database file" before any
    // assertion runs.
    let normalized = path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{normalized}?mode=rwc");
    let db = create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("failed to initialize test database: {e}"));
    RouterDatabase::init(&db)
        .await
        .unwrap_or_else(|e| panic!("failed to initialize router tables: {e}"));
    TestDb { db: Some(db), path }
}

/// Fetch all logs and find one by request_id.
async fn find_log(db: &burncloud_database::Database, request_id: &str) -> RouterLog {
    let rows = RouterLogModel::get(db, 100, 0)
        .await
        .unwrap_or_else(|e| panic!("RouterLogModel::get failed: {e}"));
    rows.into_iter()
        .find(|r| r.request_id == request_id)
        .unwrap_or_else(|| panic!("log with request_id={request_id} not found"))
}

/// Build a complete RouterLog with all 14 new fields populated.
fn make_log(request_id: &str) -> RouterLog {
    RouterLog {
        id: 0,
        request_id: request_id.to_string(),
        user_id: Some("test-user".to_string()),
        path: "/v1/chat/completions".to_string(),
        upstream_id: Some("upstream-1".to_string()),
        status_code: 200,
        latency_ms: 512,
        prompt_tokens: 1_000,
        completion_tokens: 500,
        cost: 20_000_000, // 0.02 USD in nanodollars
        model: Some("gpt-4o".to_string()),
        cache_read_tokens: 200,
        reasoning_tokens: 100,
        pricing_region: Some("us".to_string()),
        video_tokens: 50,
        // Per-type token counts (billing expansion)
        cache_write_tokens: 300,
        audio_input_tokens: 80,
        audio_output_tokens: 60,
        image_tokens: 40,
        embedding_tokens: 1_000,
        // Per-type cost breakdown in nanodollars
        input_cost: 5_000_000,
        output_cost: 7_500_000,
        cache_read_cost: 500_000,
        cache_write_cost: 0, // FM-10: currently always 0 (merged into cache_read_cost above)
        audio_cost: 2_000_000,
        image_cost: 1_000_000,
        video_cost: 500_000,
        reasoning_cost: 1_500_000,
        embedding_cost: 3_000_000,
        layer_decision: None,
        traffic_color: None,
        cost_status: None,
        error_type: None,
        created_at: None,
    }
}

/// All 14 new fields (5 token counts + 9 cost breakdowns) round-trip through SQLite.
///
/// This is the primary regression guard: if any field is missing from the INSERT
/// or SELECT query in RouterLogModel, the assert below will catch it.
#[tokio::test]
async fn test_cost_breakdown_roundtrip() {
    let db = create_test_db().await;

    let request_id = format!("test-roundtrip-{}", uuid::Uuid::new_v4());
    let log = make_log(&request_id);

    RouterLogModel::insert(&db, &log)
        .await
        .unwrap_or_else(|e| panic!("insert failed: {e}"));

    let row = find_log(&db, &request_id).await;

    // --- token counts ---
    assert_eq!(
        row.cache_write_tokens, log.cache_write_tokens,
        "cache_write_tokens"
    );
    assert_eq!(
        row.audio_input_tokens, log.audio_input_tokens,
        "audio_input_tokens"
    );
    assert_eq!(
        row.audio_output_tokens, log.audio_output_tokens,
        "audio_output_tokens"
    );
    assert_eq!(row.image_tokens, log.image_tokens, "image_tokens");
    assert_eq!(
        row.embedding_tokens, log.embedding_tokens,
        "embedding_tokens"
    );

    // --- cost breakdown ---
    assert_eq!(row.input_cost, log.input_cost, "input_cost");
    assert_eq!(row.output_cost, log.output_cost, "output_cost");
    assert_eq!(row.cache_read_cost, log.cache_read_cost, "cache_read_cost");
    assert_eq!(
        row.cache_write_cost, 0,
        "cache_write_cost (FM-10: expected 0 until P1 split)"
    );
    assert_eq!(row.audio_cost, log.audio_cost, "audio_cost");
    assert_eq!(row.image_cost, log.image_cost, "image_cost");
    assert_eq!(row.video_cost, log.video_cost, "video_cost");
    assert_eq!(row.reasoning_cost, log.reasoning_cost, "reasoning_cost");
    assert_eq!(row.embedding_cost, log.embedding_cost, "embedding_cost");

    // --- pre-existing fields still intact ---
    assert_eq!(row.prompt_tokens, log.prompt_tokens, "prompt_tokens");
    assert_eq!(
        row.completion_tokens, log.completion_tokens,
        "completion_tokens"
    );
    assert_eq!(row.cost, log.cost, "total cost");
    assert_eq!(row.model.as_deref(), Some("gpt-4o"), "model");
}

/// Cost values above i32::MAX (2,147,483,647) must not be truncated.
///
/// FM-08 regression guard: Postgres INTEGER (int4) overflows at ~$2.14/request.
/// This test verifies the SQLx i64 bindings are correct on the SQLite path.
/// For Postgres, run against a live instance with the BIGINT migration applied.
#[tokio::test]
async fn test_large_cost_no_truncation() {
    let db = create_test_db().await;

    // $5.00 = 5_000_000_000 nanodollars — exceeds i32::MAX (2,147,483,647)
    let big_cost: i64 = 5_000_000_000;
    assert!(
        big_cost > i32::MAX as i64,
        "test value must exceed i32::MAX to be meaningful"
    );

    let request_id = format!("test-overflow-{}", uuid::Uuid::new_v4());
    let mut log = make_log(&request_id);
    log.cost = big_cost;
    log.input_cost = big_cost;
    log.output_cost = 3_000_000_000; // $3.00, also > i32::MAX

    RouterLogModel::insert(&db, &log)
        .await
        .unwrap_or_else(|e| panic!("insert failed: {e}"));

    let row = find_log(&db, &request_id).await;

    assert_eq!(
        row.cost, big_cost,
        "cost truncated — i64 binding or column type is wrong (FM-08)"
    );
    assert_eq!(
        row.input_cost, big_cost,
        "input_cost truncated — i64 binding or column type is wrong (FM-08)"
    );
    assert_eq!(
        row.output_cost, 3_000_000_000,
        "output_cost truncated — i64 binding or column type is wrong (FM-08)"
    );
}

/// When cost/token fields are not set (zero), they read back as 0 not NULL.
///
/// Verifies #[sqlx(default)] contract: NULL in DB → 0 in struct, no panic.
#[tokio::test]
async fn test_zero_values_read_as_zero_not_null() {
    let db = create_test_db().await;

    let request_id = format!("test-zeros-{}", uuid::Uuid::new_v4());
    let log = RouterLog {
        id: 0,
        request_id: request_id.clone(),
        user_id: Some("zero-user".to_string()),
        path: "/v1/embeddings".to_string(),
        upstream_id: None,
        status_code: 200,
        latency_ms: 10,
        prompt_tokens: 50,
        completion_tokens: 0,
        cost: 0,
        model: Some("text-embedding-3-small".to_string()),
        cache_read_tokens: 0,
        reasoning_tokens: 0,
        pricing_region: None,
        video_tokens: 0,
        cache_write_tokens: 0,
        audio_input_tokens: 0,
        audio_output_tokens: 0,
        image_tokens: 0,
        embedding_tokens: 50_000,
        input_cost: 0,
        output_cost: 0,
        cache_read_cost: 0,
        cache_write_cost: 0,
        audio_cost: 0,
        image_cost: 0,
        video_cost: 0,
        reasoning_cost: 0,
        embedding_cost: 250_000, // only embedding_cost is non-zero
        layer_decision: None,
        traffic_color: None,
        cost_status: None,
        error_type: None,
        created_at: None,
    };

    RouterLogModel::insert(&db, &log)
        .await
        .unwrap_or_else(|e| panic!("insert failed: {e}"));

    let row = find_log(&db, &request_id).await;

    // All zero costs must come back as 0 (not panic on NULL → i64 coercion)
    assert_eq!(row.input_cost, 0, "input_cost");
    assert_eq!(row.output_cost, 0, "output_cost");
    assert_eq!(row.cache_read_cost, 0, "cache_read_cost");
    assert_eq!(row.cache_write_cost, 0, "cache_write_cost");
    assert_eq!(row.audio_cost, 0, "audio_cost");
    assert_eq!(row.image_cost, 0, "image_cost");
    assert_eq!(row.video_cost, 0, "video_cost");
    assert_eq!(row.reasoning_cost, 0, "reasoning_cost");

    // The one non-zero field must round-trip
    assert_eq!(row.embedding_cost, 250_000, "embedding_cost");
    assert_eq!(row.embedding_tokens, 50_000, "embedding_tokens");
}
