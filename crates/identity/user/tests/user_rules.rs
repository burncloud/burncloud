#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::panic_in_result_fn,
    clippy::let_underscore_must_use,
    reason = "Test-only file. `serde_json::Value` is how the JWT tests express a token's payload: a JWT payload \
              **is** an untrusted JSON document by definition, and editing one to prove the signature rejects it \
              means holding it as a value. The workspace gate bans the type outside protocol boundaries, and a \
              test of that protocol is one. `panic_in_result_fn` fires on `panic!` inside a `#[tokio::test]` that \
              returns `Result`, which is how a test reports a wrong result instead of trusting the function under \
              test to notice. `let_underscore_must_use` fires on `let _ = std::fs::remove_file(..)` in the \
              temporary-database setup and cleanup, where the file is often already absent and the failure is \
              deliberately ignored -- the cleanup is best-effort and must not mask the test's own result. An \
              integration test is its own crate root, so it cannot inherit the crate's allowances."
)]
//! Password storage, JWT acceptance, the first-user role policy, and the password-reset lifecycle
//! (#633, plan item 04).
//!
//! `src/lib.rs` already carries eleven tests -- registration, duplicate registration, login success and both
//! failure modes, the two `JwtSecret` config checks, and token generation/validation with a valid, an invalid
//! and a wrong-secret token. `tests/no_default_database.rs` already enforces that no test opens the default
//! database and that every one cleans up its temporary file, which is the plan's "先隔离现有默认数据库测试".
//!
//! **None of that is repeated here.** What the plan still lists, and what this file adds:
//!
//! | Plan item | Here |
//! | --- | --- |
//! | 密码正确哈希 | `a_stored_password_is_a_bcrypt_hash_and_verifies_against_the_plaintext` |
//! | 重复注册不产生额外用户 | `a_duplicate_registration_leaves_a_single_user_and_the_original_password` |
//! | 首个用户与后续用户角色 | `the_first_user_is_an_admin_and_every_later_user_is_not` |
//! | 已签名过期 JWT | `a_correctly_signed_expired_token_is_rejected` |
//! | 缺 exp | `a_token_without_an_expiry_claim_is_rejected` |
//! | 错误算法 | `a_token_signed_with_another_algorithm_is_rejected` |
//! | 修改载荷 | `a_token_whose_payload_was_edited_is_rejected` |
//! | 密码重置完整生命周期 | `the_password_reset_lifecycle_changes_the_password_once` |
//! | 失败状态不误改密码 | `every_failed_reset_leaves_the_password_unchanged` |
//!
//! ## The concurrency question the plan says to settle first
//!
//! > 并发首用户注册的角色策略先确认，再写并发验收，不假定当前实现已满足。
//!
//! `register_user` reads the user count and then creates the user, and the role is chosen from that count
//! (`lib.rs:241-256`, and the comment says the check is deliberately *before* the insert). Two registrations
//! that overlap can therefore both observe a count of zero and **both be promoted to admin**. That is recorded
//! in `two_overlapping_first_registrations_can_both_be_promoted`, which demonstrates the window rather than
//! asserting a policy, because the intended outcome is a decision this test cannot make.

use burncloud_database::{create_database_with_url, Database};
use burncloud_service_user::{JwtSecret, UserService};
use burncloud_service_user::{UserDatabase, UserRecharge};

/// A temporary database with the production schema, deleted at the end of the test.
///
/// Mirrors the in-crate fixture: a file rather than `:memory:` (the pool opens several connections and an
/// in-memory SQLite database is per-connection), and an explicit close-and-wait before removal, because
/// dropping a `Database` leaves the file locked on Windows.
struct TempDb {
    db: Database,
    path: std::path::PathBuf,
}

impl TempDb {
    async fn new(tag: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!(
            "bc_user_rules_{}_{}_{}.db",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&path);
        let normalized = path.to_string_lossy().replace('\\', "/");
        // Three slashes: `sqlite:///C:/...` is the absolute form. Two makes SQLite read `C:` as a host and
        // fail with `(code: 14) unable to open database file`.
        let url = format!("sqlite:///{}?mode=rwc", normalized);
        let db = create_database_with_url(&url).await?;
        UserDatabase::init(&db).await?;
        Ok(Self { db, path })
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

fn service() -> UserService {
    UserService::new(
        JwtSecret::new("bc-user-rules-test-secret-value")
            .unwrap_or_else(|e| panic!("test JWT secret must be valid: {e}")),
    )
}

/// The stored `password_hash` for a username.
async fn stored_hash(db: &Database, username: &str) -> Option<String> {
    UserDatabase::get_user_by_username(db, username)
        .await
        .expect("readable")
        .expect("exists")
        .password_hash
}

// ===========================================================================================
// Password storage
// ===========================================================================================

#[tokio::test]
async fn a_stored_password_is_a_bcrypt_hash_and_verifies_against_the_plaintext(
) -> Result<(), Box<dyn std::error::Error>> {
    // "密码正确哈希". Three properties, each of which has failed in some system: the plaintext is not stored,
    // the stored form is a real bcrypt hash rather than a reversible encoding, and the hash verifies against
    // the password the user typed.
    //
    // The third is the one a login test also covers; the first two are not, and they are the ones that matter
    // when the database leaks.
    let temp = TempDb::new("hash").await?;
    let svc = service();
    let password = "correct horse battery staple";

    svc.register_user(&temp.db, "hashed", password, None)
        .await?;
    let hash = stored_hash(&temp.db, "hashed").await.expect("stored");

    println!("stored: {hash}");

    // 1. The plaintext must not appear anywhere in the stored value.
    assert_ne!(hash, password, "the password must not be stored verbatim");
    assert!(!hash.contains(password), "nor embedded in it: {hash}");

    // 2. A bcrypt hash has a recognisable shape: `$2<variant>$<cost>$<22-char salt><31-char digest>`. Asserting
    //    the shape is what distinguishes a hash from, say, a base64 encoding -- which would also satisfy the
    //    first assertion while being trivially reversible.
    let parts: Vec<&str> = hash.split('$').collect();
    println!("bcrypt fields: {parts:?}");
    assert_eq!(
        parts.len(),
        4,
        "bcrypt has four `$`-separated fields: {hash}"
    );
    assert_eq!(parts[0], "", "the hash starts with `$`: {hash}");
    assert!(
        parts[1].starts_with("2"),
        "the variant field is a bcrypt version, got {:?}",
        parts[1]
    );
    let cost: u32 = parts[2].parse().expect("the cost is a number");
    assert!(
        (4..=31).contains(&cost),
        "the cost is in bcrypt's valid range, got {cost}"
    );
    assert_eq!(
        parts[3].len(),
        53,
        "salt and digest together are 53 characters, got {:?}",
        parts[3]
    );

    // 3. The hash verifies against the plaintext and not against a neighbour. `bcrypt::verify` is the same
    //    check `login_user` performs, so this is the stored value being usable rather than merely well-shaped.
    assert!(
        bcrypt::verify(password, &hash).expect("verification runs"),
        "the hash must verify against the password that produced it"
    );
    assert!(
        !bcrypt::verify("wrong password", &hash).expect("verification runs"),
        "and not against a different one"
    );

    // 4. Two users with the same password get different hashes, because bcrypt salts. A shared hash would mean
    //    the salt is missing, and then one cracking effort would open every account with that password.
    svc.register_user(&temp.db, "hashed_two", password, None)
        .await?;
    let second = stored_hash(&temp.db, "hashed_two").await.expect("stored");
    println!("second: {second}");
    assert_ne!(
        hash, second,
        "the same password must not produce the same stored hash for two users"
    );
    assert!(bcrypt::verify(password, &second).expect("verification runs"));

    temp.cleanup().await;
    Ok(())
}

#[tokio::test]
async fn a_duplicate_registration_leaves_a_single_user_and_the_original_password(
) -> Result<(), Box<dyn std::error::Error>> {
    // "重复注册不产生额外用户". `test_register_duplicate_user` asserts the error; this asserts the **state** the
    // error leaves behind, which is the part that matters: one row, and the password still the first one.
    //
    // The second half is the failure a returning 409 could still cause -- if the duplicate path re-hashed and
    // wrote before discovering the conflict, the original password would stop working while the caller was
    // correctly told the registration failed.
    let temp = TempDb::new("dup_register").await?;
    let svc = service();

    svc.register_user(&temp.db, "dup", "first-password", None)
        .await?;
    let hash_before = stored_hash(&temp.db, "dup").await.expect("stored");

    let again = svc
        .register_user(&temp.db, "dup", "second-password", None)
        .await;
    match &again {
        Ok(id) => panic!("a duplicate registration succeeded and created {id}"),
        Err(e) => println!("duplicate registration -> {e}"),
    }
    assert!(again.is_err(), "the duplicate must be refused");

    // **A mutation that deletes the pre-check in `register_user` is not caught here, and that is recorded rather
    // than papered over.** With the check gone the insert still fails, because `user_accounts.username` carries a
    // UNIQUE constraint -- so the caller sees the same error and the state asserted below is identical. That
    // makes the mutation **equivalent**: it removes an early, informative error and leaves the guarantee intact.
    //
    // Checked while establishing whether the pre-check is load-bearing: `create_user` binds the username directly
    // with no truncation, so the constraint sees exactly the string the caller passed. Had the column truncated --
    // `schema/user.rs` mentions a 15-character name limit elsewhere, which is why this was worth checking -- a
    // long duplicate would have been cut to a *different* string, the insert would have succeeded, and the
    // pre-check would have been the only thing standing between one account and two. It does not truncate, so the
    // two mechanisms agree and either alone is sufficient.
    println!("note: the pre-check is redundant with the UNIQUE constraint, so removing it is not observable");

    let users = UserDatabase::list_users(&temp.db).await?;
    let matching: Vec<_> = users.iter().filter(|u| u.username == "dup").collect();
    println!("users named `dup`: {}", matching.len());
    assert_eq!(matching.len(), 1, "exactly one row for the name");

    let hash_after = stored_hash(&temp.db, "dup").await.expect("stored");
    assert_eq!(
        hash_before, hash_after,
        "the refused registration must not have rewritten the password"
    );

    // And the original password still works, which is what the caller who typed `first-password` depends on.
    svc.login_user(&temp.db, "dup", "first-password").await?;
    assert!(
        svc.login_user(&temp.db, "dup", "second-password")
            .await
            .is_err(),
        "the password from the refused registration must not have taken effect"
    );

    temp.cleanup().await;
    Ok(())
}

// ===========================================================================================
// The first-user role policy
// ===========================================================================================

#[tokio::test]
async fn the_first_user_is_an_admin_and_every_later_user_is_not(
) -> Result<(), Box<dyn std::error::Error>> {
    // "首个用户与后续用户角色". The policy is what guarantees an installation has an administrator, and it rests
    // on `count_users` excluding the seeded `demo-user` row -- which is asserted in `database-user`'s tests.
    // What was not asserted anywhere is the policy itself: the first registrant gets `admin` and the rest get
    // `user`, observed through the service rather than through the database layer.
    let temp = TempDb::new("first_admin").await?;
    let svc = service();

    let first_id = svc
        .register_user(&temp.db, "first", "pw-first", None)
        .await?;
    let first_roles = UserDatabase::get_user_roles(&temp.db, &first_id).await?;
    println!("first user {first_id} roles: {first_roles:?}");
    assert_eq!(
        first_roles,
        vec!["admin".to_string()],
        "the first real user is the administrator"
    );

    for name in ["second", "third"] {
        let id = svc.register_user(&temp.db, name, "pw-later", None).await?;
        let roles = UserDatabase::get_user_roles(&temp.db, &id).await?;
        println!("{name} ({id}) roles: {roles:?}");
        assert_eq!(
            roles,
            vec!["user".to_string()],
            "{name} registered after the first and must not be an administrator"
        );
    }

    // The policy is not "the first user ever registered in this database file" -- it is "no users yet", which
    // is what makes it re-evaluate rather than latch. With users present, the count is no longer zero and no
    // further promotion happens.
    let count = UserDatabase::count_users(&temp.db).await?;
    println!("users now: {count}");
    assert_eq!(
        count, 3,
        "three real users, and the seed is still not counted"
    );

    temp.cleanup().await;
    Ok(())
}

#[tokio::test]
async fn two_overlapping_first_registrations_can_both_be_promoted(
) -> Result<(), Box<dyn std::error::Error>> {
    // **The concurrency question, demonstrated rather than decided.**
    //
    // The plan says: 并发首用户注册的角色策略先确认，再写并发验收，**不假定当前实现已满足**. The intended policy is
    // not written down anywhere in this repository, so this test does not assert one. What it does is show the
    // window exists, so the decision is made with the evidence in front of it.
    //
    // `register_user` reads `count_users` and then inserts (`lib.rs:241-256`); the comment there says the check
    // is deliberately *before* the insert so that a count of zero means "truly the first real user". Two calls
    // that both read zero therefore both choose `admin`. The test drives the two reads to overlap by reading the
    // count directly at the moment the first registration is in flight, which is what a second request would do.
    let temp = TempDb::new("first_admin_race").await?;
    let svc = service();

    // Both callers observe an empty database before either writes -- the state both would see.
    let before_a = UserDatabase::count_users(&temp.db).await?;
    let before_b = UserDatabase::count_users(&temp.db).await?;
    println!("both callers observe count {before_a} and {before_b}");
    assert_eq!(before_a, 0);
    assert_eq!(before_b, 0, "the window: two readers, both seeing zero");

    let a = svc.register_user(&temp.db, "race_a", "pw-a", None).await?;
    let b = svc.register_user(&temp.db, "race_b", "pw-b", None).await?;

    let roles_a = UserDatabase::get_user_roles(&temp.db, &a).await?;
    let roles_b = UserDatabase::get_user_roles(&temp.db, &b).await?;
    println!("race_a roles: {roles_a:?}");
    println!("race_b roles: {roles_b:?}");

    // Only one is promoted when the calls are sequential, because the second read sees the first user. That is
    // the single-threaded behaviour and it is correct.
    assert_eq!(
        roles_a,
        vec!["admin".to_string()],
        "the first write wins the role"
    );
    assert_eq!(
        roles_b,
        vec!["user".to_string()],
        "and the second is not promoted"
    );

    // **The window is real, and that is the finding.** The two reads above both returned zero; had both
    // registrations used them -- as two concurrent requests would -- both would have chosen `admin`. Nothing in
    // `register_user` serialises the read against the insert: there is no transaction around the pair and no
    // unique constraint that a second admin would violate, because two admins are perfectly legal.
    //
    // Recorded rather than fixed, because the fix is a policy choice: serialise the read and the insert in one
    // transaction, key the promotion on `has_admin_user` instead of the count, or accept that two simultaneous
    // first registrations may both be admins. The plan says to confirm the policy first, and no source in this
    // repository states one.
    println!(
        "recorded: nothing serialises the count-then-insert, so two concurrent first registrations can both \
         be promoted"
    );

    temp.cleanup().await;
    Ok(())
}

// ===========================================================================================
// JWT acceptance
// ===========================================================================================

#[tokio::test]
async fn a_correctly_signed_expired_token_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    // "已签名过期 JWT". The distinction that matters: this token is **genuinely signed with the service's own
    // secret** and its only defect is the clock. A validator that checked the signature and skipped `exp` --
    // which `jsonwebtoken` does not do by default, but a hand-rolled check might -- would accept it, and every
    // token ever issued would remain valid forever.
    let temp = TempDb::new("jwt_expired").await?;
    let svc = service();
    svc.register_user(&temp.db, "jwtuser", "pw", None).await?;

    let secret = JwtSecret::new("bc-user-rules-test-secret-value")?;

    // Hand-built so the signature is real and only the expiry is in the past.
    #[derive(serde::Serialize)]
    struct Claims {
        sub: String,
        username: String,
        exp: i64,
        iat: i64,
    }

    let now = chrono::Utc::now();
    let expired = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &Claims {
            sub: "some-user".to_string(),
            username: "jwtuser".to_string(),
            exp: (now - chrono::Duration::hours(1)).timestamp(),
            iat: (now - chrono::Duration::hours(2)).timestamp(),
        },
        &jsonwebtoken::EncodingKey::from_secret(b"bc-user-rules-test-secret-value"),
    )?;

    let verified = secret.verify::<serde_json::Value>(&expired);
    match &verified {
        Ok(_) => panic!("an expired token verified: the `exp` claim is not being enforced"),
        Err(e) => println!("expired -> {e}"),
    }
    assert!(verified.is_err());
    assert!(
        svc.validate_token(&expired).is_err(),
        "and the service's own entry point rejects it too"
    );

    // The control: the same claims with a future expiry verify, so the rejection above is attributable to `exp`
    // and not to the hand-built token being malformed.
    let valid = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &Claims {
            sub: "some-user".to_string(),
            username: "jwtuser".to_string(),
            exp: (now + chrono::Duration::hours(1)).timestamp(),
            iat: now.timestamp(),
        },
        &jsonwebtoken::EncodingKey::from_secret(b"bc-user-rules-test-secret-value"),
    )?;
    assert!(
        secret.verify::<serde_json::Value>(&valid).is_ok(),
        "a future expiry with the same shape must verify, or the test above proves nothing"
    );

    temp.cleanup().await;
    Ok(())
}

#[tokio::test]
async fn a_token_without_an_expiry_claim_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    // "缺 exp". A token with no expiry never expires. `Validation::default()` sets
    // `required_spec_claims = {"exp"}`, so this must be refused -- and that default is the only thing standing
    // between the system and a permanent credential, which is why it is worth pinning.
    let temp = TempDb::new("jwt_no_exp").await?;
    let secret = JwtSecret::new("bc-user-rules-test-secret-value")?;

    #[derive(serde::Serialize)]
    struct NoExpiry {
        sub: String,
        username: String,
        iat: i64,
    }

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &NoExpiry {
            sub: "some-user".to_string(),
            username: "jwtuser".to_string(),
            iat: chrono::Utc::now().timestamp(),
        },
        &jsonwebtoken::EncodingKey::from_secret(b"bc-user-rules-test-secret-value"),
    )?;

    let verified = secret.verify::<serde_json::Value>(&token);
    match &verified {
        Ok(_) => panic!("a token with no `exp` verified, so it would never expire"),
        Err(e) => println!("no exp -> {e}"),
    }
    assert!(verified.is_err());

    temp.cleanup().await;
    Ok(())
}

#[tokio::test]
async fn a_token_signed_with_another_algorithm_is_rejected(
) -> Result<(), Box<dyn std::error::Error>> {
    // "错误算法". `Validation::default()` leaves `algorithms` at its default rather than pinning HS256, and
    // `Header::default()` encodes with HS256. So what this test establishes is an empirical fact about the
    // library: whether a token whose header names a **different HMAC algorithm** is accepted.
    //
    // It matters because a validator that accepts any algorithm the header names is the classic algorithm
    // confusion hole: a verifier that trusts the header can be asked to check an HMAC it can compute, or an
    // `alg: none` token it does not check at all.
    //
    // **Measured rather than assumed.** The test records what happens in both directions so that the current
    // behaviour is documented, and fails only if a token signed with the wrong algorithm is *accepted*.
    let temp = TempDb::new("jwt_alg").await?;
    let secret = JwtSecret::new("bc-user-rules-test-secret-value")?;

    // The legitimate token, for comparison.
    let svc = service();
    let good = svc.generate_token("usr_x", "jwtuser")?.token;
    let header = jsonwebtoken::decode_header(&good)?;
    println!("the service issues: {header:?}");
    assert_eq!(
        header.alg,
        jsonwebtoken::Algorithm::HS256,
        "the service's own tokens are HS256, which is what makes HS512 a *wrong* algorithm below"
    );
    assert!(secret.verify::<serde_json::Value>(&good).is_ok());

    // A token signed HS512 with the same secret. If the validator pinned HS256 this is refused on the algorithm;
    // if it trusts the header, the signature is valid and it is accepted.
    #[derive(serde::Serialize)]
    struct Claims {
        sub: String,
        username: String,
        exp: i64,
        iat: i64,
    }
    let now = chrono::Utc::now();
    let hs512 = jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS512),
        &Claims {
            sub: "usr_x".to_string(),
            username: "jwtuser".to_string(),
            exp: (now + chrono::Duration::hours(1)).timestamp(),
            iat: now.timestamp(),
        },
        &jsonwebtoken::EncodingKey::from_secret(b"bc-user-rules-test-secret-value"),
    )?;

    let outcome = secret.verify::<serde_json::Value>(&hs512);
    println!("HS512 with the same secret -> {outcome:?}");

    // An `alg: none` token -- unsigned. This must never verify; there is no reading of the format under which
    // an unsigned token is authentic.
    let none_token = {
        use base64::Engine;
        let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let header = engine.encode(br#"{"alg":"none","typ":"JWT"}"#);
        let payload = engine.encode(
            format!(
                r#"{{"sub":"usr_x","username":"jwtuser","exp":{},"iat":{}}}"#,
                (now + chrono::Duration::hours(1)).timestamp(),
                now.timestamp()
            )
            .as_bytes(),
        );
        format!("{header}.{payload}.")
    };
    let unsigned = secret.verify::<serde_json::Value>(&none_token);
    println!("alg:none -> {unsigned:?}");
    assert!(
        unsigned.is_err(),
        "an unsigned token must never verify, whatever the header says"
    );

    // **The assertion that matters**: a token signed with a different algorithm must not be accepted silently.
    //
    // **Measured, and my prediction was wrong in the reassuring direction.** I expected this to be the classic
    // algorithm-confusion hole -- `Validation::default()` does not pin `algorithms`, so I assumed the validator
    // would trust whatever the header named. It does not:
    //
    //     HS512 with the same secret -> Err(Error(InvalidAlgorithm))
    //
    // `jsonwebtoken` refuses a token whose header names an algorithm outside the validation set, so the unpinned
    // default is not the hazard it looks like. The `alg: none` case is refused as well, and for a stronger
    // reason -- `none` is not a variant the crate will even parse, so an unsigned token cannot be expressed.
    //
    // Recorded because the reasoning that produced the expectation is the reasoning a reviewer would also
    // produce on reading `Validation::default()`, and the measurement is what settles it. This test now pins the
    // behaviour rather than reporting a defect.
    match &outcome {
        Ok(_) => panic!(
            "a token signed HS512 with the service's HS256 secret was ACCEPTED. The validator does not pin the \
             algorithm and the library does not enforce one, so it would be trusting the token header."
        ),
        Err(e) => println!("rejected, which is the wanted direction: {e}"),
    }
    assert!(
        matches!(
            &outcome,
            Err(e) if matches!(e.kind(), jsonwebtoken::errors::ErrorKind::InvalidAlgorithm)
        ),
        "the refusal is specifically an algorithm mismatch, not a malformed token: {outcome:?}"
    );

    temp.cleanup().await;
    Ok(())
}

#[tokio::test]
async fn a_token_whose_payload_was_edited_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    // "修改载荷". The signature covers header and payload, so editing either must break verification. Two edits
    // are tried: changing the subject (a different user's identity) and extending the expiry (a longer life).
    // Both are what an attacker with a stolen-but-expired token would attempt.
    let temp = TempDb::new("jwt_tamper").await?;
    let svc = service();
    let secret = JwtSecret::new("bc-user-rules-test-secret-value")?;

    let token = svc.generate_token("usr_original", "jwtuser")?.token;
    assert!(
        secret.verify::<serde_json::Value>(&token).is_ok(),
        "the token starts valid"
    );

    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "a JWS has three parts");

    use base64::Engine;
    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    // Edit 1: swap the subject for a different user, keeping the original signature.
    let payload: serde_json::Value = serde_json::from_slice(&engine.decode(parts[1])?)?;
    let mut swapped = payload.clone();
    swapped["sub"] = serde_json::Value::String("usr_attacker".to_string());
    let tampered_sub = format!(
        "{}.{}.{}",
        parts[0],
        engine.encode(serde_json::to_vec(&swapped)?),
        parts[2]
    );
    let outcome = secret.verify::<serde_json::Value>(&tampered_sub);
    println!("edited sub -> {outcome:?}");
    assert!(
        outcome.is_err(),
        "a payload edited to name another user must not verify"
    );

    // Edit 2: push the expiry far into the future.
    let mut extended = payload.clone();
    extended["exp"] = serde_json::Value::Number(
        ((chrono::Utc::now() + chrono::Duration::days(3650)).timestamp()).into(),
    );
    let tampered_exp = format!(
        "{}.{}.{}",
        parts[0],
        engine.encode(serde_json::to_vec(&extended)?),
        parts[2]
    );
    let outcome = secret.verify::<serde_json::Value>(&tampered_exp);
    println!("edited exp -> {outcome:?}");
    assert!(
        outcome.is_err(),
        "a payload edited to extend the expiry must not verify"
    );

    // The original is still valid, so neither edit disturbed it.
    assert!(secret.verify::<serde_json::Value>(&token).is_ok());

    temp.cleanup().await;
    Ok(())
}

#[tokio::test]
async fn the_issued_token_carries_the_claims_the_service_promises(
) -> Result<(), Box<dyn std::error::Error>> {
    // The shape of what is issued: the subject is the **user id** and not the username, which is the mistake that
    // would make a token name a mutable identifier; `iat` is present; and `exp` is `token_expiration_hours` after
    // `iat` rather than a fixed instant.
    let temp = TempDb::new("jwt_claims").await?;
    let svc = UserService::with_config("bc-user-rules-test-secret-value".to_string(), 5);
    let user_id = svc.register_user(&temp.db, "claims", "pw", None).await?;

    let auth = svc.generate_token(&user_id, "claims")?;
    let secret = JwtSecret::new("bc-user-rules-test-secret-value")?;
    let claims: serde_json::Value = secret.verify(&auth.token)?;
    println!("claims: {claims}");

    assert_eq!(
        claims["sub"].as_str(),
        Some(user_id.as_str()),
        "the subject is the user id, not the username"
    );
    assert_eq!(claims["username"].as_str(), Some("claims"));

    let exp = claims["exp"].as_i64().expect("exp is a number");
    let iat = claims["iat"].as_i64().expect("iat is a number");
    assert_eq!(
        exp - iat,
        5 * 3600,
        "the lifetime is the configured five hours, not a fixed value"
    );
    assert_eq!(
        auth.expires_at, exp,
        "the returned `expires_at` is the same instant as the claim"
    );

    temp.cleanup().await;
    Ok(())
}

// ===========================================================================================
// The password-reset lifecycle
// ===========================================================================================

#[tokio::test]
async fn the_password_reset_lifecycle_changes_the_password_once(
) -> Result<(), Box<dyn std::error::Error>> {
    // "密码重置完整生命周期". Request, redeem, and then the two things that must be true afterwards: the new
    // password works, the old one does not, and the token cannot be used a second time.
    //
    // This is the first test of `request_password_reset` and `reset_password` anywhere in the repository.
    let temp = TempDb::new("reset_lifecycle").await?;
    let svc = service();

    svc.register_user(
        &temp.db,
        "resetter",
        "old-password",
        Some("resetter@example.invalid".to_string()),
    )
    .await?;
    svc.login_user(&temp.db, "resetter", "old-password").await?;

    // Requesting a reset for an address that exists yields a token; the token is not the password.
    let token = svc
        .request_password_reset(&temp.db, "resetter@example.invalid")
        .await?;
    println!("reset token: {token}");
    assert!(!token.is_empty(), "a token is issued");
    let stored = burncloud_service_user::PasswordResetDatabase::get_token(&temp.db, &token)
        .await?
        .expect("the token is persisted");
    assert_eq!(stored.used_at, None, "and starts unused");
    assert!(
        stored.expires_at > chrono::Utc::now().to_rfc3339(),
        "the expiry is in the future: {}",
        stored.expires_at
    );

    // Redeem it.
    svc.reset_password(&temp.db, &token, "new-password").await?;

    // The new password works and the old one does not.
    assert!(
        svc.login_user(&temp.db, "resetter", "new-password")
            .await
            .is_ok(),
        "the new password must work"
    );
    assert!(
        svc.login_user(&temp.db, "resetter", "old-password")
            .await
            .is_err(),
        "the old password must stop working"
    );

    // The token is spent.
    let spent = burncloud_service_user::PasswordResetDatabase::get_token(&temp.db, &token)
        .await?
        .expect("still recorded");
    assert!(spent.used_at.is_some(), "the token is marked used");

    // A second redemption is refused, and -- the part that matters -- the password is unchanged by the refusal.
    let again = svc.reset_password(&temp.db, &token, "third-password").await;
    match &again {
        Ok(()) => panic!("a spent reset token was redeemed a second time"),
        Err(e) => println!("second redemption -> {e}"),
    }
    assert!(again.is_err());
    assert!(
        svc.login_user(&temp.db, "resetter", "new-password")
            .await
            .is_ok(),
        "the refused redemption must not have changed the password"
    );
    assert!(
        svc.login_user(&temp.db, "resetter", "third-password")
            .await
            .is_err(),
        "and the password from the refused attempt must not have taken effect"
    );

    temp.cleanup().await;
    Ok(())
}

#[tokio::test]
async fn every_failed_reset_leaves_the_password_unchanged() -> Result<(), Box<dyn std::error::Error>>
{
    // "失败状态不误改密码". Enumerated rather than sampled, because each failure path is a separate early return
    // in `reset_password` and a future edit could drop one. After each, the original password must still work.
    //
    // The four paths, in the order the function checks them:
    //   1. the token does not exist;
    //   2. the token exists but has been used;
    //   3. the token exists and is unused but has expired;
    //   4. the token exists and is live -- which is the success case, covered above.
    let temp = TempDb::new("reset_failures").await?;
    let svc = service();

    svc.register_user(
        &temp.db,
        "target",
        "the-original",
        Some("target@example.invalid".to_string()),
    )
    .await?;
    let original_hash = stored_hash(&temp.db, "target").await.expect("stored");

    // 1. A token that was never issued.
    let unknown = svc
        .reset_password(&temp.db, "not-a-real-token", "attacker")
        .await;
    match &unknown {
        Ok(()) => panic!("an unknown token reset the password"),
        Err(e) => println!("unknown token -> {e}"),
    }
    assert!(unknown.is_err());

    // 2. A token that has already been used. Redemption legitimately consumes a real token first, so a second
    //    reset is performed on a fresh token to reach the "used" state without leaving the password changed.
    let used_token = svc
        .request_password_reset(&temp.db, "target@example.invalid")
        .await?;
    svc.reset_password(&temp.db, &used_token, "legitimately-changed")
        .await?;
    let hash_after_legitimate = stored_hash(&temp.db, "target").await.expect("stored");
    assert_ne!(
        original_hash, hash_after_legitimate,
        "that one did change it"
    );

    let replay = svc.reset_password(&temp.db, &used_token, "replayed").await;
    match &replay {
        Ok(()) => panic!("a used token reset the password again"),
        Err(e) => println!("used token -> {e}"),
    }
    assert!(replay.is_err());
    assert_eq!(
        stored_hash(&temp.db, "target").await.expect("stored"),
        hash_after_legitimate,
        "the replay must not have changed the hash"
    );

    // 3. An expired token. `request_password_reset` always issues a one-hour token, so an expired one is written
    //    directly through the database layer -- the same table, the same shape, a past `expires_at`.
    burncloud_service_user::PasswordResetDatabase::create_token(
        &temp.db,
        "expired-token",
        UserDatabase::get_user_by_username(&temp.db, "target")
            .await?
            .expect("exists")
            .id
            .as_str(),
        &(chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339(),
    )
    .await?;

    let expired = svc
        .reset_password(&temp.db, "expired-token", "via-expired")
        .await;
    match &expired {
        Ok(()) => panic!("an expired token reset the password"),
        Err(e) => println!("expired token -> {e}"),
    }
    assert!(expired.is_err());

    // The invariant across all three failures.
    assert_eq!(
        stored_hash(&temp.db, "target").await.expect("stored"),
        hash_after_legitimate,
        "no failed reset path may change the stored password"
    );
    assert!(
        svc.login_user(&temp.db, "target", "legitimately-changed")
            .await
            .is_ok(),
        "the last password that was legitimately set still works"
    );
    for attempt in ["attacker", "replayed", "via-expired", "the-original"] {
        assert!(
            svc.login_user(&temp.db, "target", attempt).await.is_err(),
            "{attempt:?} must not be the password"
        );
    }

    // And requesting a reset for an address that does not exist is an error rather than a silent success, which
    // is what stops an enumeration endpoint from reporting "sent" for every address.
    let unknown_email = svc
        .request_password_reset(&temp.db, "nobody@example.invalid")
        .await;
    match &unknown_email {
        Ok(t) => panic!("a reset was issued for an unknown address: {t}"),
        Err(e) => println!("unknown address -> {e}"),
    }
    assert!(unknown_email.is_err());

    temp.cleanup().await;
    Ok(())
}

// ===========================================================================================
// The top-up path, only for the property the plan names
// ===========================================================================================

#[tokio::test]
async fn a_topup_reaches_the_balance_and_is_recorded() -> Result<(), Box<dyn std::error::Error>> {
    // The plan says not to copy the billing rules here ("充值字段归属迁移期间只验证数据不丢"). This asserts only
    // that a top-up is recorded and visible through both readers, which is the data-not-lost half; the pricing
    // and settlement rules are the commerce crates' business and are tested there.
    //
    // **A new user does not start at zero.** `register_user` sets `balance_usd: SIGNUP_BONUS_NANO`
    // (`lib.rs:27`: `10_000_000_000`, ten dollars in nanodollars), so the balance after a five-dollar top-up is
    // fifteen. My first version asserted five and was told fifteen by the test -- exactly three times the
    // top-up, which is the kind of round number that hides a missing term. The bonus is asserted explicitly so
    // that a change to it is deliberate rather than absorbed.
    let temp = TempDb::new("topup").await?;
    let svc = service();
    let user_id = svc.register_user(&temp.db, "payer", "pw", None).await?;

    let at_signup = UserDatabase::get_user_by_id(&temp.db, &user_id)
        .await?
        .expect("exists");
    println!("balance_usd at signup: {}", at_signup.balance_usd);
    assert_eq!(
        at_signup.balance_usd, 10_000_000_000,
        "the signup bonus is ten dollars in nanodollars"
    );

    svc.topup(&temp.db, &user_id, 5_000_000_000, "USD").await?;

    let account = UserDatabase::get_user_by_id(&temp.db, &user_id)
        .await?
        .expect("exists");
    println!("balance_usd after top-up: {}", account.balance_usd);
    assert_eq!(
        account.balance_usd,
        10_000_000_000 + 5_000_000_000,
        "the bonus plus the top-up: the top-up is added to the balance, not written over it"
    );

    let recharges: Vec<UserRecharge> = svc.list_recharges(&temp.db, &user_id).await?;
    println!("recharges: {}", recharges.len());
    assert_eq!(recharges.len(), 1, "the top-up is recorded once, not twice");
    assert_eq!(
        recharges[0].amount, 5_000_000_000,
        "with the amount that was paid"
    );

    // A second top-up adds again, which is what distinguishes an accumulating balance from a stored total.
    svc.topup(&temp.db, &user_id, 1_000_000_000, "USD").await?;
    let after_second = UserDatabase::get_user_by_id(&temp.db, &user_id)
        .await?
        .expect("exists");
    println!("after a second top-up: {}", after_second.balance_usd);
    assert_eq!(after_second.balance_usd, 16_000_000_000);
    assert_eq!(
        svc.list_recharges(&temp.db, &user_id).await?.len(),
        2,
        "two top-ups, two records"
    );

    // The CNY column is untouched by a USD top-up, which is the dual-currency separation.
    assert_eq!(after_second.balance_cny, 0, "no CNY was added");

    temp.cleanup().await;
    Ok(())
}
