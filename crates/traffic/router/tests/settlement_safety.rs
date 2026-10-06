#![allow(
    clippy::expect_used,
    reason = "Test-only source-contract checks."
)]

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

#[test]
fn router_does_not_replay_non_idempotent_quota_debits() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let source = read(&format!("{manifest}/src/lib.rs"));

    assert!(
        !source.contains("settle_quota_with_retry"),
        "used_quota += cost is not idempotent; application-level settlement replay must stay removed"
    );
    assert!(
        !source.contains("SETTLE_ATTEMPTS"),
        "a failed debit must not be retried blindly without a settlement idempotency key"
    );

    let calls = source.matches("RouterDatabase::deduct_quota(").count();
    assert_eq!(
        calls, 1,
        "one served request must have one production quota-debit call site, found {calls}"
    );
}

#[test]
fn settlement_logging_never_prints_the_full_credential() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let source = read(&format!("{manifest}/src/lib.rs"));

    assert!(
        !source.contains("token = %token"),
        "API credentials must never be emitted verbatim into tracing fields"
    );
    assert!(
        !source.contains("token = %token_for_quota"),
        "cloned API credentials must never be emitted verbatim either"
    );
}

#[test]
fn a_failed_single_settlement_is_observable_without_logging_the_credential() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let database_router = read(&format!(
        "{manifest}/../database-router/src/lib.rs"
    ));

    assert!(
        database_router.contains("quota settlement failed"),
        "the single non-idempotent debit may not be replayed, so an Err must at least be observable"
    );

    let marker = database_router
        .find("quota settlement failed")
        .expect("logging marker must exist");
    let start = marker.saturating_sub(600);
    let end = (marker + 100).min(database_router.len());
    let logging_block = &database_router[start..end];

    assert!(
        logging_block.contains("user_id"),
        "settlement error log needs a non-secret account identifier"
    );
    assert!(
        logging_block.contains("cost"),
        "settlement error log needs the attempted charge amount"
    );
    assert!(
        !logging_block.contains("token =") && !logging_block.contains("token_for_quota"),
        "settlement error log must never contain the credential"
    );
}

#[test]
fn sqlite_writer_contention_is_handled_before_the_debit_fails() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let database_source = read(&format!(
        "{manifest}/../../platform/storage/database/src/database.rs"
    ));

    assert!(
        database_source.contains("SQLITE_BUSY_TIMEOUT_MS"),
        "SQLite connections need a bounded busy wait for concurrent writers"
    );
    assert!(
        database_source.contains("PRAGMA busy_timeout"),
        "busy_timeout must be installed on each pooled SQLite connection"
    );
    assert!(
        database_source.contains("after_connect"),
        "SQLite PRAGMA is connection-scoped, so every pool connection must receive it"
    );
}
