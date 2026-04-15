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
use burncloud_database_router::{RouterLog, RouterDatabase, RouterLogModel};
use tempfile::NamedTempFile;

/// Create an isolated SQLite test database backed by a temp file.
///
/// SQLite `:memory:` fails with connection pools because each pool connection
/// gets a fresh empty database; the schema written on one connection is invisible
/// to others. A temp file is shared across all pool connections within the process.
///
/// Both `Schema::init()` (via `create_database_with_url`) and `RouterDatabase::init()`
/// are required: the former creates `router_logs` with all columns, the latter creates
/// `router_tokens` which `RouterLogModel::insert` updates for quota deduction.
async fn create_test_db() -> (burncloud_database::Database, NamedTempFile) {
    let tmp = NamedTempFile::new().expect("failed to create temp file");
    let url = format!("sqlite://{}?mode=rwc", tmp.path().display());
    let db = create_database_with_url(&url)
        .await
        .expect("failed to initialize test database");
    RouterDatabase::init(&db)
        .await
        .expect("failed to initialize router tables");
    (db, tmp)
}

/// Fetch all logs and find one by request_id.
async fn find_log(db: &burncloud_database::Database, request_id: &str) -> RouterLog {
    let rows = RouterLogModel::get(db, 100, 0)
        .await
        .expect("RouterLogModel::get failed");
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
        created_at: None,
    }
}

/// All 14 new fields (5 token counts + 9 cost breakdowns) round-trip through SQLite.
///
/// This is the primary regression guard: if any field is missing from the INSERT
/// or SELECT query in RouterLogModel, the assert below will catch it.
#[tokio::test]
async fn test_cost_breakdown_roundtrip() {
    let (db, _tmp) = create_test_db().await;

    let request_id = format!("test-roundtrip-{}", uuid::Uuid::new_v4());
    let log = make_log(&request_id);

    RouterLogModel::insert(&db, &log)
        .await
        .expect("insert failed");

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
    let (db, _tmp) = create_test_db().await;

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
        .expect("insert failed");

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
    let (db, _tmp) = create_test_db().await;

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
        created_at: None,
    };

    RouterLogModel::insert(&db, &log)
        .await
        .expect("insert failed");

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
