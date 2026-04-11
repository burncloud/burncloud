//! Integration test for MASTER_KEY bootstrap (issue #101).
//!
//! Verifies the first-run behavior: with no `MASTER_KEY` env var and no
//! existing key file, `ensure_master_key` generates a key file, exports it
//! to the environment, and the generated key is a usable AES-256-GCM key.
//!
//! Linux-only: the test relies on `XDG_CONFIG_HOME` to redirect
//! `dirs::config_dir()` into a tempdir so it never touches the developer's
//! real `~/.config/burncloud/master.key`.

#![cfg(target_os = "linux")]

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use burncloud_database_upstream::get_master_key;
use burncloud_server::bootstrap::ensure_master_key;

#[test]
fn first_run_generates_and_uses_master_key() {
    let tmp = tempfile::tempdir().expect("create tempdir");

    // Isolate from the developer's real config dir and any ambient env.
    std::env::set_var("XDG_CONFIG_HOME", tmp.path());
    std::env::remove_var("MASTER_KEY");

    let expected_path = tmp.path().join("burncloud").join("master.key");
    assert!(
        !expected_path.exists(),
        "precondition failed: key file should not pre-exist"
    );

    ensure_master_key().expect("bootstrap should succeed on first run");

    // 1. Key file was created on disk.
    assert!(
        expected_path.exists(),
        "expected key file at {}",
        expected_path.display()
    );
    let file_contents = std::fs::read_to_string(&expected_path).expect("read key file");
    let file_hex = file_contents.trim();
    assert_eq!(file_hex.len(), 64, "key file should contain 64 hex chars");
    assert!(
        file_hex.chars().all(|c| c.is_ascii_hexdigit()),
        "key file should be hex"
    );

    // 2. Env var was populated with the same value the file holds.
    let env_hex = std::env::var("MASTER_KEY").expect("MASTER_KEY should be exported to env");
    assert_eq!(env_hex, file_hex, "env MASTER_KEY must match the file");

    // 3. The generated key is usable as an AES-256-GCM key — round-trip works.
    let key_bytes = get_master_key().expect("get_master_key should parse the generated key");
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).expect("init cipher");
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let plaintext = b"sk-bootstrap-smoke-test";
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_ref())
        .expect("encrypt succeeds");
    let decrypted = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .expect("decrypt succeeds");
    assert_eq!(decrypted, plaintext);

    // 4. File permissions are 0o600 on Unix.
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(&expected_path)
        .expect("stat key file")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "key file must be 0600");
}
