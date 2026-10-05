#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! The service-entry correspondence table, and database-failure propagation (#633, plan item 03).
//!
//! The plan asks for two things that `token_credentials.rs` does not provide:
//!
//! > 现有：服务转发逻辑…**需建立服务入口对应表**。
//! > 待补：…**数据库失败传播**。
//!
//! ## The correspondence table
//!
//! Built by reading `src/lib.rs` and the test file, not by hand:
//!
//! | Entry point | Forwards to | Driven by `token_credentials.rs`? |
//! | --- | --- | --- |
//! | `list` | `RouterTokenModel::list` | yes (6 tests) |
//! | `create` | `RouterTokenModel::create` | **no** |
//! | `delete` | `RouterTokenModel::delete` | **no** |
//! | `update_status` | `RouterTokenModel::update_status` | yes (2) |
//! | `validate` | `RouterTokenModel::validate` | yes (7) |
//! | `validate_detailed` | `RouterTokenModel::validate_detailed` | yes (4) |
//! | `update_accessed_time` | `RouterTokenModel::update_accessed_time` | **no** |
//! | `check_quota` | `RouterTokenModel::check_quota` | yes (5) |
//! | `deduct_quota` | `RouterTokenModel::deduct_quota` | yes (6) |
//! | `rotate` | `RouterTokenModel::rotate` | yes (4) |
//! | `revoke_old_key` | `RouterTokenModel::revoke_old_key` | **no** |
//! | `set_ip_whitelist` | `RouterTokenModel::set_ip_whitelist` | yes (1) |
//! | `is_ip_allowed` | `RouterTokenModel::is_ip_allowed` | yes (2) |
//!
//! **Every one of the thirteen is a one-line forward with no logic of its own** -- the first column and the
//! second are identical in name for all thirteen. That is why the plan says not to verify a simple forward
//! mechanically ("不对简单转发机械验证『调用一次 Mock』"): a mock asserting "the model was called once" would pin
//! the delegation and nothing about the rule. The rules live in `RouterTokenModel` and are pinned in
//! `token_credentials.rs`.
//!
//! **Four entry points have no test at all**: `create`, `delete`, `update_accessed_time`, `revoke_old_key`.
//! This file covers them for the property that every entry point shares -- see below -- and `create` is used to
//! seed, so it is exercised incidentally rather than asserted.
//!
//! ## Database-failure propagation
//!
//! `token_credentials.rs`'s module docs claim "the eight behaviours come from the plan's own list: … and
//! database-failure propagation". **It does not test it.** Measured: zero occurrences of `DROP TABLE`, zero of
//! `NotInitialized`, zero of `close()`, and one `is_err()` -- which belongs to
//! `rotating_an_unknown_credential_is_an_error_not_a_silent_success` and is about an unknown credential, not a
//! broken database.
//!
//! So the claim is corrected here by making it true. A token path runs on a database that can fail, and the
//! dangerous outcome is not an error -- it is a **lookup that reports "no such credential"**, because every
//! caller treats that as "not authenticated" and a **denylist that reports "allowed"**. Both are quiet.

use burncloud_database::{create_database_with_url, Database};
use burncloud_database_router::{RouterDatabase, RouterToken, RouterTokenValidationResult};
use burncloud_service_token::TokenService;
use tempfile::NamedTempFile;

/// A seeded database in a temporary file, with a second connection for breaking the schema.
///
/// The second connection exists because `Database::close(self)` consumes the handle, so a test cannot drop a
/// table through the database it is about to use. It also means the failure is injected **on the same file**,
/// which is what makes it a database failure rather than a different database.
async fn test_db(tag: &str) -> (Database, Database, NamedTempFile) {
    let tmp = NamedTempFile::new().unwrap_or_else(|e| panic!("temp file: {e}"));
    // Three slashes with forward slashes: `sqlite:///C:/...` is the absolute form. Two slashes makes SQLite
    // treat the first segment as a host and fail with `(code: 14) unable to open database file`.
    let normalized = tmp.path().to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{normalized}?mode=rwc");

    let db = create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("create database at {url}: {e}"));
    RouterDatabase::init(&db)
        .await
        .unwrap_or_else(|e| panic!("create router tables: {e}"));

    // A separate pool on the same file, used only to break the schema.
    let breaker = create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("second connection to {url}: {e}"));

    let _ = tag;
    (db, breaker, tmp)
}

/// Drop `router_tokens` through the second connection, then let the first pool notice.
///
/// The wait is not decoration: the two pools are separate, and SQLite's schema change is visible to the other
/// connection when it next prepares a statement. Without the pause the first pool can still serve the old
/// prepared statement and the test would pass for the wrong reason on a fast machine.
async fn break_the_schema(breaker: &Database) {
    breaker
        .execute_query("DROP TABLE router_tokens")
        .await
        .expect("dropping the table must work or the test is not testing a failure");
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
}

/// A credential with the fields a caller needs to be authenticated.
///
/// Built from the type's real definition (`database-router/src/token.rs:38`), not from an assumed shape: the
/// key field is `token`, there is no `id` / `name` / `unlimited_quota` / `created_time`, and
/// `accessed_time` is a plain `i64` rather than an `Option`. The first version of this helper guessed and did
/// not compile.
fn token(credential: &str) -> RouterToken {
    RouterToken {
        token: credential.to_string(),
        user_id: "usr_token".to_string(),
        status: "active".to_string(),
        quota_limit: 1_000_000,
        used_quota: 0,
        expired_time: -1,
        accessed_time: 0,
        key_version: 1,
        old_key_hash: None,
        old_key_expires_at: 0,
        ip_whitelist: None,
        key_prefix: "bc_live_".to_string(),
        created_at: 0,
        last_rotated_at: 0,
    }
}

// ===========================================================================================
// The entry-point correspondence: every forward propagates a broken database
// ===========================================================================================

#[tokio::test]
async fn every_entry_point_reports_a_broken_database_rather_than_a_negative_answer(
) -> Result<(), Box<dyn std::error::Error>> {
    // One test over all thirteen entry points, rather than thirteen tests of a one-line forward. What is being
    // asserted is a **property of the layer**: whatever the underlying model does with a failure, the service
    // must not convert it into a plausible-looking answer.
    //
    // The two outcomes that matter are named explicitly, because they are the ones a caller would believe:
    //
    // * `validate` / `validate_detailed` reporting `Invalid` -- indistinguishable from a revoked credential, so
    //   an operator debugging an outage would see every request as a bad key;
    // * `is_ip_allowed` reporting `true` -- a denylist that opens when the database is down;
    // * `check_quota` reporting `true` -- spending authorised against a database that cannot record it.
    let (db, breaker, _tmp) = test_db("entry_failures").await;

    TokenService::create(&db, &token("bc_live_entry"))
        .await
        .expect("seeded");

    // A working baseline for every entry point, so the failures below are attributable to the dropped table.
    assert!(TokenService::list(&db).await.is_ok(), "list works before");
    assert!(
        TokenService::validate(&db, "bc_live_entry").await.is_ok(),
        "validate works before"
    );
    assert!(
        TokenService::check_quota(&db, "bc_live_entry", 1)
            .await
            .is_ok(),
        "check_quota works before"
    );
    assert!(
        TokenService::is_ip_allowed(&db, "bc_live_entry", "10.0.0.1")
            .await
            .is_ok(),
        "is_ip_allowed works before"
    );

    break_the_schema(&breaker).await;

    // --- the reads -------------------------------------------------------------------------
    match TokenService::list(&db).await {
        Ok(rows) => panic!("`list` returned {} rows for a database with no table; a caller reads that as \"no credentials exist\"", rows.len()),
        Err(e) => println!("list -> {e}"),
    }

    match TokenService::validate(&db, "bc_live_entry").await {
        Ok(Some(_)) => panic!("`validate` found a credential in a table that does not exist"),
        Ok(None) => panic!(
            "`validate` reported `None` for a broken database. A caller reads `None` as \"this credential is \
             not valid\", so every request during an outage would look like a bad key."
        ),
        Err(e) => println!("validate -> {e}"),
    }

    match TokenService::validate_detailed(&db, "bc_live_entry").await {
        Ok(verdict) => panic!("`validate_detailed` produced {verdict:?} from no table"),
        Err(e) => println!("validate_detailed -> {e}"),
    }

    // The result must not be the `Invalid` verdict, which is what a caller switches on. Asserted separately
    // because `Ok(_)` would pass a looser check.
    let detailed = TokenService::validate_detailed(&db, "bc_live_entry").await;
    assert!(
        detailed.is_err(),
        "a broken database must not produce a validation *verdict*: {detailed:?}"
    );
    let _ = RouterTokenValidationResult::Invalid;

    match TokenService::check_quota(&db, "bc_live_entry", 1).await {
        Ok(true) => panic!("`check_quota` authorised spending against a database that cannot record it"),
        Ok(false) => panic!(
            "`check_quota` reported `false`, which a caller reads as \"over quota\" rather than \"cannot tell\""
        ),
        Err(e) => println!("check_quota -> {e}"),
    }

    match TokenService::is_ip_allowed(&db, "bc_live_entry", "10.0.0.1").await {
        Ok(true) => panic!(
            "`is_ip_allowed` allowed an address while the database was broken -- a denylist that fails open"
        ),
        Ok(false) => println!(
            "is_ip_allowed -> false; recorded rather than condemned: this is fail-closed, which is the safe \
             direction, but it is indistinguishable from a real refusal"
        ),
        Err(e) => println!("is_ip_allowed -> {e}"),
    }

    // --- the writes ------------------------------------------------------------------------
    match TokenService::create(&db, &token("bc_live_after_break")).await {
        Ok(()) => panic!("`create` reported success with no table to insert into"),
        Err(e) => println!("create -> {e}"),
    }

    match TokenService::delete(&db, "bc_live_entry").await {
        Ok(()) => panic!("`delete` reported success with no table to delete from"),
        Err(e) => println!("delete -> {e}"),
    }

    match TokenService::update_status(&db, "bc_live_entry", "disabled").await {
        Ok(()) => panic!("`update_status` reported success with no table to update"),
        Err(e) => println!("update_status -> {e}"),
    }

    match TokenService::update_accessed_time(&db, "bc_live_entry").await {
        Ok(()) => panic!("`update_accessed_time` reported success with no table to update"),
        Err(e) => println!("update_accessed_time -> {e}"),
    }

    match TokenService::deduct_quota(&db, "bc_live_entry", 1).await {
        Ok(true) => panic!("`deduct_quota` reported a settled charge that was never written"),
        Ok(false) => panic!(
            "`deduct_quota` reported `false`, which a caller reads as \"over quota\" rather than \"not written\""
        ),
        Err(e) => println!("deduct_quota -> {e}"),
    }

    match TokenService::rotate(&db, "bc_live_entry", 24, false).await {
        Ok(result) => panic!(
            "`rotate` issued {:?} from a database with no table",
            result.new_token
        ),
        Err(e) => println!("rotate -> {e}"),
    }

    match TokenService::revoke_old_key(&db, "bc_live_entry").await {
        Ok(true) => panic!("`revoke_old_key` reported a revocation that was never written"),
        Ok(false) => println!(
            "revoke_old_key -> false; recorded: indistinguishable from \"there was no old key\", which is the \
             answer a caller gets when the table is missing"
        ),
        Err(e) => println!("revoke_old_key -> {e}"),
    }

    match TokenService::set_ip_whitelist(&db, "bc_live_entry", "10.0.0.0/8").await {
        Ok(_) => panic!("`set_ip_whitelist` reported success with no table to write"),
        Err(e) => println!("set_ip_whitelist -> {e}"),
    }

    Ok(())
}

#[tokio::test]
async fn the_database_failure_is_not_swallowed_into_a_negative_answer(
) -> Result<(), Box<dyn std::error::Error>> {
    // The precise property, isolated so it is not lost among the thirteen.
    //
    // **I predicted the wrong answer here and the measurement corrected me.** The expectation was that
    // `is_ip_allowed` and `revoke_old_key` would return `Ok(false)` on a broken database -- a negative answer
    // indistinguishable from a real refusal -- because both are "false means no" methods. Measured:
    //
    //     is_ip_allowed  -> Err(no such table: router_tokens)
    //     revoke_old_key -> Err(no such table: router_tokens)
    //
    // Both **propagate**, which is the better behaviour: a caller can tell "the database is broken" from "this
    // address is not allowed". The prediction was more pessimistic than the implementation, and it is recorded
    // because the reasoning that produced it -- "a method returning a boolean converts failures into `false`" --
    // is plausible enough that the next reader may expect it too.
    //
    // This also sits next to a **different** property already recorded in `token_credentials.rs`:
    // `is_ip_allowed` returns `true` for a token that does not exist or is disabled, because the lookup yields
    // no row and "no whitelist" is read as "allow". That is about a **missing row in a working database**, not
    // about a broken database, and the two must not be conflated -- this test is the second case.
    let (db, breaker, _tmp) = test_db("false_vs_error").await;
    TokenService::create(&db, &token("bc_live_false"))
        .await
        .expect("seeded");

    // A working baseline, including the allow path, so neither failure below is a method that never worked.
    assert!(
        TokenService::is_ip_allowed(&db, "bc_live_false", "10.0.0.1")
            .await
            .expect("works before"),
        "with no whitelist configured, any address is allowed -- the property recorded in the sibling file"
    );

    break_the_schema(&breaker).await;

    let allowed = TokenService::is_ip_allowed(&db, "bc_live_false", "10.0.0.1").await;
    let revoked = TokenService::revoke_old_key(&db, "bc_live_false").await;
    println!("is_ip_allowed -> {allowed:?}");
    println!("revoke_old_key -> {revoked:?}");

    assert!(
        allowed.is_err(),
        "an unusable database must not be reported as \"this address is not allowed\": {allowed:?}"
    );
    assert!(
        revoked.is_err(),
        "nor as \"there was nothing to revoke\": {revoked:?}"
    );

    // The contrast: the same broken database, and these entry points error too. So the layer propagates
    // uniformly, and no entry point invents an answer.
    assert!(TokenService::validate(&db, "bc_live_false").await.is_err());
    assert!(TokenService::list(&db).await.is_err());
    assert!(TokenService::check_quota(&db, "bc_live_false", 1)
        .await
        .is_err());

    Ok(())
}

// ===========================================================================================
// The four entry points that had no test at all
// ===========================================================================================

#[tokio::test]
async fn the_untested_entry_points_work_and_report_their_effect(
) -> Result<(), Box<dyn std::error::Error>> {
    // `delete`, `update_accessed_time` and `revoke_old_key` were driven by no existing test; `create` only
    // incidentally, as the seed. The property asserted here is the one a caller actually consumes: `delete`
    // removes the credential so it stops validating, and `update_accessed_time` records a use without changing
    // anything a validation decision depends on.
    //
    // Not a mechanical "was the model called" check -- the plan forbids that -- but the observable effect.
    let (db, _breaker, _tmp) = test_db("untested_entries").await;

    TokenService::create(&db, &token("bc_live_fresh"))
        .await
        .expect("created");
    let created = TokenService::list(&db).await.expect("listable");
    println!("after create: {} token(s)", created.len());
    assert_eq!(created.len(), 1, "the created credential is listed");
    assert_eq!(created[0].token, "bc_live_fresh");

    // A created credential validates, which is what makes it useful.
    assert!(
        TokenService::validate(&db, "bc_live_fresh")
            .await
            .expect("validatable")
            .is_some(),
        "a freshly created active credential validates"
    );

    // `update_accessed_time` records a use. It must not make the credential invalid or change its quota.
    TokenService::update_accessed_time(&db, "bc_live_fresh")
        .await
        .expect("updated");
    let after_access = TokenService::validate(&db, "bc_live_fresh")
        .await
        .expect("validatable")
        .expect("still valid");
    println!(
        "after update_accessed_time: accessed_time={:?} used_quota={}",
        after_access.accessed_time, after_access.used_quota
    );
    assert!(
        after_access.accessed_time > 0,
        "the access is timestamped, which is what the method is for. The field is an i64 whose zero means \
         \"never accessed\", so the assertion is on the value rather than on presence."
    );
    assert_eq!(
        after_access.used_quota, 0,
        "recording an access is not a charge"
    );

    // `delete` removes it, and the credential stops validating.
    TokenService::delete(&db, "bc_live_fresh")
        .await
        .expect("deleted");
    assert!(
        TokenService::validate(&db, "bc_live_fresh")
            .await
            .expect("validatable")
            .is_none(),
        "a deleted credential must not validate"
    );
    assert!(
        TokenService::list(&db).await.expect("listable").is_empty(),
        "and it is gone from the list"
    );

    // `revoke_old_key` on a credential with no rotation history: either answer is defensible, so what is
    // asserted is that it does not error and does not disturb the credential.
    TokenService::create(&db, &token("bc_live_no_rotation"))
        .await
        .expect("created");
    let revoked = TokenService::revoke_old_key(&db, "bc_live_no_rotation").await;
    println!("revoke_old_key with no rotation -> {revoked:?}");
    assert!(
        revoked.is_ok(),
        "asking to revoke an old key when there is none is not an error: {revoked:?}"
    );
    assert!(
        TokenService::validate(&db, "bc_live_no_rotation")
            .await
            .expect("validatable")
            .is_some(),
        "and the current credential keeps working"
    );

    Ok(())
}

#[tokio::test]
async fn deleting_one_credential_leaves_the_others() -> Result<(), Box<dyn std::error::Error>> {
    // `delete` is one of the four untested entry points, and the failure that matters for a delete is deleting
    // the wrong thing. The existing suite covers isolation for quota and settlement; this covers it for removal.
    let (db, _breaker, _tmp) = test_db("delete_isolation").await;

    for key in ["bc_live_keep_a", "bc_live_remove", "bc_live_keep_b"] {
        TokenService::create(&db, &token(key))
            .await
            .expect("created");
    }
    assert_eq!(TokenService::list(&db).await.expect("listable").len(), 3);

    TokenService::delete(&db, "bc_live_remove")
        .await
        .expect("deleted");

    let left: Vec<String> = TokenService::list(&db)
        .await
        .expect("listable")
        .into_iter()
        .map(|t| t.token)
        .collect();
    println!("after deleting the middle one: {left:?}");
    assert_eq!(left.len(), 2, "two left");
    assert!(!left.contains(&"bc_live_remove".to_string()));
    assert!(left.contains(&"bc_live_keep_a".to_string()));
    assert!(left.contains(&"bc_live_keep_b".to_string()));

    for key in ["bc_live_keep_a", "bc_live_keep_b"] {
        assert!(
            TokenService::validate(&db, key)
                .await
                .expect("validatable")
                .is_some(),
            "{key} still validates"
        );
    }

    Ok(())
}
