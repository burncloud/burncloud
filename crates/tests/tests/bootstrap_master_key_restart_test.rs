//! Integration test for MASTER_KEY bootstrap restart behavior (issue #101).
//!
//! Verifies that after a first-run generates a key file, a subsequent
//! "restart" (a second call to `ensure_master_key` with no `MASTER_KEY`
//! in the environment) reuses the exact same key from disk, so data
//! encrypted before the restart can still be decrypted afterwards.
//!
//! Linux-only: relies on `XDG_CONFIG_HOME` to redirect `dirs::config_dir()`
//! into a tempdir so it never touches the developer's real config.
//!
//! Lives in its own test file so it gets its own process and does not
//! race with `bootstrap_master_key_test.rs` on the process-global
//! `MASTER_KEY` / `XDG_CONFIG_HOME` env vars.

#![cfg(target_os = "linux")]

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use burncloud_database_upstream::get_master_key;
use burncloud_server::bootstrap::ensure_master_key;

#[test]
fn restart_reuses_generated_key_and_decrypts_prior_ciphertext() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    std::env::set_var("XDG_CONFIG_HOME", tmp.path());
    std::env::remove_var("MASTER_KEY");

    let key_path = tmp.path().join("burncloud").join("master.key");
    assert!(!key_path.exists(), "precondition: key file should not pre-exist");

    // First run: generates and persists the key.
    ensure_master_key().expect("first bootstrap should succeed");
    let first_hex = std::env::var("MASTER_KEY").expect("MASTER_KEY exported after first run");
    let file_hex_after_first = std::fs::read_to_string(&key_path)
        .expect("read key file after first run")
        .trim()
        .to_string();
    assert_eq!(first_hex, file_hex_after_first);

    // Encrypt a payload with the freshly generated key.
    let key_bytes = get_master_key().expect("parse first-run key");
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).expect("init cipher");
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let plaintext = b"sk-restart-roundtrip-secret";
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_ref())
        .expect("encrypt succeeds");

    // Simulate a process restart: wipe the env var so `ensure_master_key`
    // must re-hydrate from the on-disk key file.
    std::env::remove_var("MASTER_KEY");
    assert!(key_path.exists(), "key file must survive the simulated restart");

    ensure_master_key().expect("second bootstrap should succeed");
    let second_hex = std::env::var("MASTER_KEY").expect("MASTER_KEY exported after restart");

    // Same key, same file, unchanged on disk.
    assert_eq!(
        second_hex, first_hex,
        "restart must reuse the same MASTER_KEY, not regenerate"
    );
    let file_hex_after_restart = std::fs::read_to_string(&key_path)
        .expect("read key file after restart")
        .trim()
        .to_string();
    assert_eq!(
        file_hex_after_restart, file_hex_after_first,
        "key file contents must not change across restart"
    );

    // Ciphertext produced before the restart is still decryptable after.
    let key_bytes_after = get_master_key().expect("parse post-restart key");
    assert_eq!(
        key_bytes_after, key_bytes,
        "raw key bytes must be identical across restart"
    );
    let cipher_after = Aes256Gcm::new_from_slice(&key_bytes_after).expect("init cipher post-restart");
    let decrypted = cipher_after
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .expect("decrypt with post-restart key succeeds");
    assert_eq!(decrypted, plaintext);
}
