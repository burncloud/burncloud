#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration-test file: fail-fast setup is the intended behaviour"
)]
//! Credential validation, quota settlement and rotation for `service-token` (#633, plan section 5
//! item 03).
//!
//! `TokenService` delegates to `RouterTokenModel`; that crate had **no tests at all** while carrying
//! the credential path, so every rule below is pinned for the first time. The behaviours come from the
//! plan's own list: valid / disabled / expired / unknown token, quota boundaries, credential isolation,
//! and rotation.
//!
//! **This file does not cover database-failure propagation, and an earlier version of this header said it
//! did.** Measured when the claim was checked: zero occurrences of `DROP TABLE`, zero of `NotInitialized`, zero
//! of `close()`, and one `is_err()` -- which belongs to `rotating_an_unknown_credential_is_an_error_not_a_silent_success`
//! and is about an unknown credential rather than a broken database. That behaviour lives in
//! `token_service_entries.rs`, which also carries the service-entry correspondence table the plan asks for.
//! The claim was left standing for as long as it was because nothing checked it.
//!
//! Two properties of the implementation were read from the source before writing anything, because
//! guessing them would have produced a test that freezes the wrong thing:
//!
//! 1. **`deduct_quota` writes past the cap on purpose.** Its documentation says the write is durable
//!    even when the new total crosses the cap, and the return value then becomes `false` so the final
//!    over-cap request does not become free. So the assertion is "the charge is recorded even though
//!    the result is `false`", which is the opposite of what a fail-closed settlement would do, and it
//!    is stated as such rather than as an accident.
//! 2. **`expired_time == -1` means never expires**, and `0` also never expires, because the check is
//!    `expired_time > 0 && now > expired_time`. A test that used `0` for "expired" would assert the
//!    wrong thing.
//!
//! Two behaviours are recorded as **unresolved questions rather than as contracts**, because the
//! implementation cannot currently distinguish them and asserting the current output would make the
//! ambiguity permanent:
//!
//! * a disabled token and a token that never existed both validate as `Invalid`, so a caller cannot
//!   tell a revoked credential from a typo;
//! * `is_ip_allowed` returns `true` for a token that does not exist or is disabled, because the
//!   lookup yields no row and "no whitelist" is treated as "allow".
//!
//! Those two are reported in the PR rather than frozen here. Both tests assert only what is
//! unambiguous.

use burncloud_database::create_database_with_url;
use burncloud_service_token::{RouterToken, RouterTokenValidationResult, TokenService};
use tempfile::NamedTempFile;

/// Optional behaviour of a credential, so each test states its intent instead of setting 14 fields.
struct TokenSpec {
    token: &'static str,
    user_id: &'static str,
    status: &'static str,
    quota_limit: i64,
    used_quota: i64,
    expired_time: i64,
    ip_whitelist: Option<&'static str>,
}

impl Default for TokenSpec {
    fn default() -> Self {
        Self {
            token: "bc_live_default",
            user_id: "user-default",
            status: "active",
            // -1 is unlimited, which is the documented sentinel rather than a quota of -1.
            quota_limit: -1,
            used_quota: 0,
            // -1 is "never expires". 0 does not mean expired: the check is `expired_time > 0`.
            expired_time: -1,
            ip_whitelist: None,
        }
    }
}

impl TokenSpec {
    fn to_token(&self) -> RouterToken {
        RouterToken {
            token: self.token.to_string(),
            user_id: self.user_id.to_string(),
            status: self.status.to_string(),
            quota_limit: self.quota_limit,
            used_quota: self.used_quota,
            expired_time: self.expired_time,
            accessed_time: 0,
            key_version: 1,
            old_key_hash: None,
            old_key_expires_at: 0,
            ip_whitelist: self.ip_whitelist.map(str::to_string),
            key_prefix: "bc_live_".to_string(),
            created_at: 1_700_000_000,
            last_rotated_at: 0,
        }
    }
}

/// Isolated SQLite database in a temp file.
///
/// A file rather than `:memory:`: the pool opens several connections and an in-memory SQLite
/// database is per-connection, so the schema would be invisible to most queries. `NamedTempFile`
/// removes the file when it drops, which is why this does not need the manual close-and-wait dance
/// that the `service-user` tests require.
async fn test_db() -> (burncloud_database::Database, NamedTempFile) {
    let tmp = NamedTempFile::new().unwrap_or_else(|e| panic!("temp file: {e}"));
    // Three slashes and forward slashes: `sqlite:///C:/...` is an absolute path on Windows, whereas
    // two slashes is treated as relative and fails with "unable to open database file" (code 14).
    // The sibling tests in `database-router` use the two-slash form and happen to avoid this because
    // of where their temp path lands; this crate is written to work wherever the temp directory is.
    let normalized = tmp.path().to_string_lossy().replace('\\', "/");
    let url = format!("sqlite:///{normalized}?mode=rwc");
    let db = create_database_with_url(&url)
        .await
        .unwrap_or_else(|e| panic!("create database at {url}: {e}"));
    TokenService::init(&db)
        .await
        .unwrap_or_else(|e| panic!("create router tables: {e}"));
    (db, tmp)
}

async fn seed(db: &burncloud_database::Database, spec: &TokenSpec) {
    TokenService::create(db, &spec.to_token())
        .await
        .unwrap_or_else(|e| panic!("seed {}: {e}", spec.token));
}

/// Seconds since the epoch, matching the implementation's own clock.
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// -------------------------------------------------------------------------------------------
// validation: valid / disabled / expired / unknown
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn an_active_unexpired_token_validates_and_returns_its_own_row() {
    // The control case: every rejection assertion below is checked against this working baseline, so
    // a failure there cannot be caused by a broken fixture.
    let (db, _tmp) = test_db().await;
    let spec = TokenSpec {
        token: "bc_live_valid",
        user_id: "user-a",
        quota_limit: 5_000,
        ..Default::default()
    };
    seed(&db, &spec).await;

    match TokenService::validate_detailed(&db, "bc_live_valid").await {
        Ok(RouterTokenValidationResult::Valid(t)) => {
            assert_eq!(t.token, "bc_live_valid");
            assert_eq!(t.user_id, "user-a");
            assert_eq!(t.quota_limit, 5_000, "the row must come back unmodified");
        }
        other => panic!("an active token must validate, got {other:?}"),
    }
}

#[tokio::test]
async fn a_token_that_never_existed_is_invalid_not_an_error() {
    // Unknown credentials are a normal input, not a database failure, so this must be a value and
    // not an Err.
    let (db, _tmp) = test_db().await;
    let result = TokenService::validate_detailed(&db, "bc_live_never_created")
        .await
        .unwrap_or_else(|e| panic!("an unknown token must not be an error: {e}"));
    assert!(
        matches!(result, RouterTokenValidationResult::Invalid),
        "an unknown token must be Invalid, got {result:?}"
    );

    // The non-detailed form agrees.
    let plain = TokenService::validate(&db, "bc_live_never_created")
        .await
        .unwrap_or_else(|e| panic!("validate: {e}"));
    assert!(
        plain.is_none(),
        "an unknown token must not resolve to a row"
    );
}

#[tokio::test]
async fn a_disabled_token_stops_validating_and_stops_resolving() {
    // Disabling is the revocation mechanism, so this is the assertion that matters most here.
    let (db, _tmp) = test_db().await;
    let spec = TokenSpec {
        token: "bc_live_to_disable",
        user_id: "user-b",
        ..Default::default()
    };
    seed(&db, &spec).await;

    // Control: it works before being disabled.
    assert!(
        TokenService::validate(&db, "bc_live_to_disable")
            .await
            .expect("validate before disabling")
            .is_some(),
        "the control case must validate before the status changes"
    );

    TokenService::update_status(&db, "bc_live_to_disable", "disabled")
        .await
        .expect("update_status");

    assert!(
        TokenService::validate(&db, "bc_live_to_disable")
            .await
            .expect("validate after disabling")
            .is_none(),
        "a disabled token must no longer resolve"
    );
    assert!(
        matches!(
            TokenService::validate_detailed(&db, "bc_live_to_disable")
                .await
                .expect("validate_detailed"),
            RouterTokenValidationResult::Invalid
        ),
        "a disabled token must not validate"
    );

    // The row still exists: disabling is a status change, not a delete, so an audit can see it.
    let rows = TokenService::list(&db).await.expect("list");
    assert!(
        rows.iter()
            .any(|t| t.token == "bc_live_to_disable" && t.status == "disabled"),
        "the disabled row must remain visible to list()"
    );
}

#[tokio::test]
async fn an_expired_token_is_reported_as_expired_not_as_invalid() {
    // The distinction between Expired and Invalid is the reason `validate_detailed` exists, so it is
    // asserted rather than assumed. `expired_time` must be > 0 for expiry to apply at all.
    let (db, _tmp) = test_db().await;
    let spec = TokenSpec {
        token: "bc_live_expired",
        user_id: "user-c",
        expired_time: now() - 60,
        ..Default::default()
    };
    seed(&db, &spec).await;

    let result = TokenService::validate_detailed(&db, "bc_live_expired")
        .await
        .expect("validate_detailed");
    assert!(
        matches!(result, RouterTokenValidationResult::Expired),
        "an expired token must be reported as Expired, got {result:?}"
    );

    // The non-detailed form collapses Expired and Invalid into `None`, which is its documented shape.
    assert!(
        TokenService::validate(&db, "bc_live_expired")
            .await
            .expect("validate")
            .is_none(),
        "an expired token must not resolve"
    );
}

#[tokio::test]
async fn the_no_expiry_sentinels_are_negative_one_and_zero() {
    // The check is `expired_time > 0 && now > expired_time`, so both -1 and 0 mean "never expires".
    // A test that treated 0 as expired would assert the opposite of the implementation, which is why
    // the sentinel is pinned explicitly.
    let (db, _tmp) = test_db().await;
    for (token, expired_time) in [("bc_never_expires", -1_i64), ("bc_zero_expiry", 0_i64)] {
        seed(
            &db,
            &TokenSpec {
                token,
                user_id: "user-sentinel",
                expired_time,
                ..Default::default()
            },
        )
        .await;
        assert!(
            TokenService::validate(&db, token)
                .await
                .expect("validate")
                .is_some(),
            "{token} with expired_time={expired_time} must be treated as never expiring"
        );
    }

    // And one second in the past is expired, so the boundary really is `now > expired_time`.
    seed(
        &db,
        &TokenSpec {
            token: "bc_just_expired",
            user_id: "user-sentinel",
            expired_time: now() - 1,
            ..Default::default()
        },
    )
    .await;
    assert!(
        TokenService::validate(&db, "bc_just_expired")
            .await
            .expect("validate")
            .is_none(),
        "a credential one second past its expiry must not validate"
    );
}

// -------------------------------------------------------------------------------------------
// quota
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn quota_accepts_up_to_the_limit_and_rejects_beyond_it() {
    // `quota_allows` is `limit < 0 || used + cost <= limit`, so the boundary is inclusive at the
    // limit and exclusive above it. Both sides and equality are asserted.
    let (db, _tmp) = test_db().await;
    let limit = 1_000_i64;
    seed(
        &db,
        &TokenSpec {
            token: "bc_quota",
            user_id: "user-q",
            quota_limit: limit,
            used_quota: 400,
            ..Default::default()
        },
    )
    .await;

    // used 400 + cost 600 == limit -> allowed (the boundary is inclusive).
    assert!(
        TokenService::check_quota(&db, "bc_quota", 600)
            .await
            .expect("check"),
        "spending exactly to the limit must be allowed"
    );
    // one more nanodollar must not be.
    assert!(
        !TokenService::check_quota(&db, "bc_quota", 601)
            .await
            .expect("check"),
        "one nanodollar past the limit must be refused"
    );
    // and a large cost is refused.
    assert!(
        !TokenService::check_quota(&db, "bc_quota", 10_000)
            .await
            .expect("check"),
        "a cost far beyond the remaining quota must be refused"
    );

    // `check_quota` must not have modified anything: it is the read-only pre-check.
    let row = TokenService::list(&db)
        .await
        .expect("list")
        .into_iter()
        .find(|t| t.token == "bc_quota")
        .expect("row");
    assert_eq!(row.used_quota, 400, "check_quota must not settle anything");
}

#[tokio::test]
async fn a_unlimited_quota_is_negative_one_not_a_large_number() {
    // `quota_limit < 0` means unlimited. Pinned because `-1` is also a plausible "invalid" sentinel in
    // other codebases, and here it means the opposite.
    let (db, _tmp) = test_db().await;
    seed(
        &db,
        &TokenSpec {
            token: "bc_unlimited",
            user_id: "user-u",
            quota_limit: -1,
            ..Default::default()
        },
    )
    .await;

    assert!(
        TokenService::check_quota(&db, "bc_unlimited", i64::MAX)
            .await
            .expect("check"),
        "an unlimited credential must accept any cost"
    );
    assert!(
        TokenService::deduct_quota(&db, "bc_unlimited", i64::MAX)
            .await
            .expect("deduct"),
        "an unlimited credential must never report exhaustion"
    );
}

#[tokio::test]
async fn a_missing_credential_fails_closed_for_quota() {
    // A credential that does not exist must not be treated as having unlimited quota.
    let (db, _tmp) = test_db().await;
    assert!(
        !TokenService::check_quota(&db, "bc_does_not_exist", 1)
            .await
            .expect("check"),
        "an unknown credential must not pass the quota pre-check"
    );
    assert!(
        !TokenService::deduct_quota(&db, "bc_does_not_exist", 1)
            .await
            .expect("deduct"),
        "settling against an unknown credential must report failure"
    );
}

#[tokio::test]
async fn settlement_records_the_charge_even_when_it_exceeds_the_cap() {
    // This is the documented design, not an oversight: `deduct_quota` writes the full cost, then
    // returns `false` so the caller learns the credential is exhausted. The stated reason is that the
    // final over-cap request must not become free. Asserted because it is the opposite of what
    // "fail closed" would suggest, so a future change to it should be deliberate.
    let (db, _tmp) = test_db().await;
    let limit = 1_000_i64;
    seed(
        &db,
        &TokenSpec {
            token: "bc_overcap",
            user_id: "user-o",
            quota_limit: limit,
            used_quota: 900,
            ..Default::default()
        },
    )
    .await;

    // 900 + 200 = 1100 > 1000, so the caller is told the credential is exhausted ...
    let within = TokenService::deduct_quota(&db, "bc_overcap", 200)
        .await
        .expect("deduct");
    assert!(!within, "settling past the cap must report exhaustion");

    // ... but the 200 was still recorded, so the request was not free.
    let row = TokenService::list(&db)
        .await
        .expect("list")
        .into_iter()
        .find(|t| t.token == "bc_overcap")
        .expect("row");
    assert_eq!(
        row.used_quota, 1_100,
        "the over-cap charge must be recorded; a free final request is the failure mode this avoids"
    );
}

#[tokio::test]
async fn settlement_is_additive_across_calls() {
    // Nanodollar precision matters here: the amounts are small enough that a rounding or unit error
    // would show up as a wrong total rather than as an obvious failure.
    let (db, _tmp) = test_db().await;
    seed(
        &db,
        &TokenSpec {
            token: "bc_additive",
            user_id: "user-add",
            quota_limit: 1_000_000,
            ..Default::default()
        },
    )
    .await;

    for cost in [1_i64, 999, 1_000_000 - 1_000] {
        assert!(
            TokenService::deduct_quota(&db, "bc_additive", cost)
                .await
                .expect("deduct"),
            "each charge stays within the limit so each must succeed"
        );
    }

    let row = TokenService::list(&db)
        .await
        .expect("list")
        .into_iter()
        .find(|t| t.token == "bc_additive")
        .expect("row");
    assert_eq!(
        row.used_quota,
        1 + 999 + (1_000_000 - 1_000),
        "charges must accumulate exactly"
    );
    assert!(
        !TokenService::check_quota(&db, "bc_additive", 1)
            .await
            .expect("check"),
        "the credential is now exactly at its limit, so one more nanodollar must be refused"
    );
}

// -------------------------------------------------------------------------------------------
// isolation and rotation
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn settling_one_credential_does_not_touch_another() {
    // The settlement SQL is `WHERE token = ?`, so this is the guard that the predicate is really
    // there. Two credentials of the same user are used, because that is the case a shared-account bug
    // would corrupt.
    let (db, _tmp) = test_db().await;
    for token in ["bc_first", "bc_second"] {
        seed(
            &db,
            &TokenSpec {
                token,
                user_id: "user-shared",
                quota_limit: 1_000_000,
                used_quota: 0,
                ..Default::default()
            },
        )
        .await;
    }

    TokenService::deduct_quota(&db, "bc_first", 5_000)
        .await
        .expect("deduct on the first");
    TokenService::deduct_quota(&db, "bc_first", 1_000)
        .await
        .expect("deduct on the first again");

    let rows = TokenService::list(&db).await.expect("list");
    let first = rows.iter().find(|t| t.token == "bc_first").expect("first");
    let second = rows
        .iter()
        .find(|t| t.token == "bc_second")
        .expect("second");
    assert_eq!(
        first.used_quota, 6_000,
        "both charges belong to the first credential"
    );
    assert_eq!(
        second.used_quota, 0,
        "the sibling credential must be untouched"
    );

    // Disabling one must not disable the other.
    TokenService::update_status(&db, "bc_first", "disabled")
        .await
        .expect("disable the first");
    assert!(
        TokenService::validate(&db, "bc_second")
            .await
            .expect("validate")
            .is_some(),
        "disabling one credential must not affect its sibling"
    );
}

#[tokio::test]
async fn rotation_issues_a_new_key_and_keeps_the_old_one_valid_during_transition() {
    // Rotation is the operation most likely to lock a user out, so both halves are asserted: the new
    // key works, and the old key still works while the transition window is open.
    let (db, _tmp) = test_db().await;
    let original = "bc_live_rotate_me";
    seed(
        &db,
        &TokenSpec {
            token: original,
            user_id: "user-r",
            quota_limit: 1_000_000,
            used_quota: 700,
            ..Default::default()
        },
    )
    .await;

    let result = TokenService::rotate(&db, original, 24, false)
        .await
        .expect("rotate");

    assert_ne!(
        result.new_token, original,
        "rotation must issue a different key"
    );
    assert!(
        result.new_token.starts_with("bc_live_"),
        "the new key keeps the prefix, got {}",
        result.new_token
    );
    assert_eq!(
        result.key_version, 2,
        "the first rotation moves version 1 to 2"
    );
    assert_eq!(result.old_key_version, 1);
    assert!(
        result.transition_ends_at > now(),
        "a 24 hour transition must end in the future, got {}",
        result.transition_ends_at
    );

    // The new key validates.
    assert!(
        TokenService::validate(&db, &result.new_token)
            .await
            .expect("validate new")
            .is_some(),
        "the new key must work immediately"
    );
    // The old key still validates during the transition, and is reported as the same credential.
    let via_old = TokenService::validate(&db, original)
        .await
        .expect("validate old")
        .expect("the old key must still resolve during the transition period");
    assert_eq!(
        via_old.token, result.new_token,
        "the old key resolves to the canonical current token"
    );
    assert_eq!(
        via_old.used_quota, 700,
        "the settled amount must survive rotation, or rotation would hand out free quota"
    );
    assert_eq!(
        via_old.quota_limit, 1_000_000,
        "the limit must survive rotation too"
    );
}

#[tokio::test]
async fn rotation_with_revoke_old_invalidates_the_old_key_immediately() {
    // The other half of the same switch: with `revoke_old = true` the transition window is not opened
    // at all, which is what a credential leak needs.
    let (db, _tmp) = test_db().await;
    let original = "bc_live_revoke_immediately";
    seed(
        &db,
        &TokenSpec {
            token: original,
            user_id: "user-rv",
            ..Default::default()
        },
    )
    .await;

    let result = TokenService::rotate(&db, original, 24, true)
        .await
        .expect("rotate with revoke_old");

    assert_eq!(
        result.transition_ends_at, 0,
        "revoking immediately must close the transition window"
    );
    assert!(
        TokenService::validate(&db, &result.new_token)
            .await
            .expect("validate new")
            .is_some(),
        "the replacement key must work"
    );
    assert!(
        TokenService::validate(&db, original)
            .await
            .expect("validate old")
            .is_none(),
        "the revoked key must stop working at once -- this is the point of revoke_old"
    );
}

#[tokio::test]
async fn rotating_an_unknown_credential_is_an_error_not_a_silent_success() {
    // Rotation that silently succeeded on a missing row would hand out a key bound to nothing.
    let (db, _tmp) = test_db().await;
    let result = TokenService::rotate(&db, "bc_live_absent", 24, false).await;
    assert!(
        result.is_err(),
        "rotating an unknown credential must fail, got {result:?}"
    );
}

#[tokio::test]
async fn rotation_settles_against_the_same_credential_alias() {
    // Spending through the old key during the transition must land on the same row; otherwise a user
    // could spend twice by alternating between the two keys.
    let (db, _tmp) = test_db().await;
    let original = "bc_live_alias";
    seed(
        &db,
        &TokenSpec {
            token: original,
            user_id: "user-alias",
            quota_limit: 1_000_000,
            used_quota: 0,
            ..Default::default()
        },
    )
    .await;

    let result = TokenService::rotate(&db, original, 24, false)
        .await
        .expect("rotate");

    // Both the old alias and the new key point at the one row.
    assert!(
        TokenService::check_quota(&db, original, 1_000)
            .await
            .expect("check via old"),
        "the old alias must resolve for quota checks during the transition"
    );
    TokenService::deduct_quota(&db, original, 1_000)
        .await
        .expect("settle via old alias");

    let rows = TokenService::list(&db).await.expect("list");
    let matching: Vec<_> = rows.iter().filter(|t| t.user_id == "user-alias").collect();
    assert_eq!(
        matching.len(),
        1,
        "rotation must update the row, not insert a second one"
    );
    assert_eq!(
        matching[0].token, result.new_token,
        "the row carries the new key"
    );
    assert_eq!(
        matching[0].used_quota, 1_000,
        "a charge made through the old alias must land on the rotated credential"
    );
}

// -------------------------------------------------------------------------------------------
// IP allowlist
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn an_empty_or_absent_allowlist_permits_any_address() {
    // Documented behaviour: no allowlist means no restriction. The control for the rejection below.
    let (db, _tmp) = test_db().await;
    seed(
        &db,
        &TokenSpec {
            token: "bc_no_allowlist",
            user_id: "user-ip",
            ip_whitelist: None,
            ..Default::default()
        },
    )
    .await;
    assert!(
        TokenService::is_ip_allowed(&db, "bc_no_allowlist", "203.0.113.7")
            .await
            .expect("is_ip_allowed"),
        "an absent allowlist must permit any address"
    );

    seed(
        &db,
        &TokenSpec {
            token: "bc_empty_allowlist",
            user_id: "user-ip",
            ip_whitelist: Some(""),
            ..Default::default()
        },
    )
    .await;
    assert!(
        TokenService::is_ip_allowed(&db, "bc_empty_allowlist", "203.0.113.7")
            .await
            .expect("is_ip_allowed"),
        "an empty allowlist must permit any address"
    );
}

#[tokio::test]
async fn a_configured_allowlist_admits_listed_addresses_only() {
    // Matching is exact and comma-separated, with surrounding whitespace trimmed. CIDR is explicitly
    // not supported (the source says so), which is pinned here so the limitation is visible rather
    // than discovered by someone configuring `10.0.0.0/8` and expecting it to work.
    let (db, _tmp) = test_db().await;
    let token = "bc_allowlist";
    seed(
        &db,
        &TokenSpec {
            token,
            user_id: "user-ip2",
            ip_whitelist: Some("203.0.113.7, 198.51.100.9"),
            ..Default::default()
        },
    )
    .await;

    for allowed in ["203.0.113.7", "198.51.100.9"] {
        assert!(
            TokenService::is_ip_allowed(&db, token, allowed)
                .await
                .expect("is_ip_allowed"),
            "{allowed} is on the allowlist and must be admitted"
        );
    }
    for refused in ["203.0.113.8", "10.0.0.1", "", "203.0.113.7 "] {
        assert!(
            !TokenService::is_ip_allowed(&db, token, refused)
                .await
                .expect("is_ip_allowed"),
            "{refused:?} is not on the allowlist and must be refused"
        );
    }
}

#[tokio::test]
async fn setting_an_allowlist_reports_whether_a_row_was_changed() {
    // The boolean is `rows_affected > 0`, so it distinguishes "configured" from "no such credential"
    // and a caller can rely on it.
    let (db, _tmp) = test_db().await;
    seed(
        &db,
        &TokenSpec {
            token: "bc_set_allowlist",
            user_id: "user-ip3",
            ..Default::default()
        },
    )
    .await;

    assert!(
        TokenService::set_ip_whitelist(&db, "bc_set_allowlist", "203.0.113.7")
            .await
            .expect("set_ip_whitelist"),
        "setting an allowlist on an existing credential must report a change"
    );
    assert!(
        !TokenService::set_ip_whitelist(&db, "bc_no_such_credential", "203.0.113.7")
            .await
            .expect("set_ip_whitelist"),
        "setting an allowlist on a missing credential must report no change rather than success"
    );
}
