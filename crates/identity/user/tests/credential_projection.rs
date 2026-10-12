#![allow(
    clippy::unwrap_used,
    reason = "test assertions fail fast for credential projection"
)]
//! SEC-001 (#612): default JSON and Debug representations cannot disclose credentials.

use burncloud_service_user::{UserAccount, UserAccountInput, UserApiKey};

const HASH: &str = "$2b$12$highly_sensitive_test_hash";
const KEY: &str = "sk-123456789012345678901234567890123456789012345678";

fn account() -> UserAccount {
    UserAccount {
        id: "test-user".to_string(),
        username: "alice".to_string(),
        email: Some("alice@example.com".to_string()),
        password_hash: Some(HASH.to_string()),
        github_id: None,
        status: 1,
        balance_usd: 100,
        balance_cny: 0,
        preferred_currency: Some("USD".to_string()),
    }
}

fn api_key() -> UserApiKey {
    UserApiKey {
        id: 7,
        user_id: "test-user".to_string(),
        key: KEY.to_string(),
        status: 1,
        name: Some("test".to_string()),
        remain_quota: 100,
        unlimited_quota: false,
        used_quota: 10,
        created_time: Some(1),
        accessed_time: Some(2),
        expired_time: -1,
    }
}

#[test]
fn password_hash_is_internal_only_in_both_account_types() {
    let account = account();
    let json = serde_json::to_value(&account).unwrap();
    assert!(json.get("password_hash").is_none());
    assert_eq!(json["username"], "alice");
    assert!(!json.to_string().contains(HASH));

    let debug = format!("{account:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains(HASH));

    let input = UserAccountInput::from(account);
    assert_eq!(input.password_hash.as_deref(), Some(HASH));
    let input_json = serde_json::to_value(&input).unwrap();
    assert!(input_json.get("password_hash").is_none());
    assert!(!input_json.to_string().contains(HASH));
    let input_debug = format!("{input:?}");
    assert!(input_debug.contains("[REDACTED]"));
    assert!(!input_debug.contains(HASH));
}

#[test]
fn stored_key_remains_usable_but_serialization_and_debug_only_show_mask() {
    let token = api_key();
    assert_eq!(token.key, KEY);
    assert_eq!(token.masked_key(), "sk-****5678");
    let json = serde_json::to_value(&token).unwrap();
    assert_eq!(json["key"], "sk-****5678");
    assert_eq!(json["remain_quota"], 100);
    assert!(!json.to_string().contains(KEY));

    let listed = serde_json::to_value(vec![token.clone()]).unwrap();
    assert_eq!(listed[0]["key"], "sk-****5678");
    assert!(!listed.to_string().contains(KEY));

    let debug = format!("{token:?}");
    assert!(debug.contains("sk-****5678"));
    assert!(!debug.contains(KEY));
    assert_eq!(token.key, KEY, "serialization must not mutate the key");
}

#[test]
fn malformed_keys_do_not_leak_partial_secrets() {
    assert_eq!(UserApiKey::mask_key(""), "****");
    assert_eq!(UserApiKey::mask_key("sk-short"), "****");
    assert_eq!(UserApiKey::mask_key("not-a-key-123456"), "****");
    assert_eq!(UserApiKey::mask_key("sk-1234😀"), "****");
}
