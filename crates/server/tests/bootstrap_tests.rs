use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256Gcm, Nonce,
};
use burncloud_server::bootstrap::{ensure_master_key, MasterKeySource};
use rand::rngs::OsRng;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
#[cfg(target_os = "linux")]
fn generates_master_key_file_on_first_start() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let tmp = tempfile::tempdir().expect("create tempdir");
    let prev_master = std::env::var("MASTER_KEY").ok();
    let prev_xdg = std::env::var("XDG_CONFIG_HOME").ok();

    std::env::remove_var("MASTER_KEY");
    std::env::set_var("XDG_CONFIG_HOME", tmp.path());

    let key_file = tmp.path().join("burncloud").join("master.key");
    assert!(
        !key_file.exists(),
        "precondition: key file should not exist before ensure_master_key"
    );

    let source = ensure_master_key().expect("ensure_master_key succeeds on first start");
    assert_eq!(source, MasterKeySource::Generated);

    assert!(key_file.exists(), "master.key file was not created");
    let contents = std::fs::read_to_string(&key_file).expect("read master.key");
    let hex_key = contents.trim();
    assert_eq!(
        hex_key.len(),
        64,
        "master.key must be 64 hex chars (32 bytes), got {}",
        hex_key.len()
    );
    assert!(
        hex_key.chars().all(|c| c.is_ascii_hexdigit()),
        "master.key must be pure hex, got: {hex_key}"
    );

    let env_key = std::env::var("MASTER_KEY").expect("MASTER_KEY env var should be set");
    assert_eq!(env_key, hex_key);

    match prev_master {
        Some(v) => std::env::set_var("MASTER_KEY", v),
        None => std::env::remove_var("MASTER_KEY"),
    }
    match prev_xdg {
        Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
}

#[test]
#[cfg(target_os = "linux")]
fn restart_reuses_generated_key_and_decrypts_existing_data() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let tmp = tempfile::tempdir().expect("create tempdir");
    let prev_master = std::env::var("MASTER_KEY").ok();
    let prev_xdg = std::env::var("XDG_CONFIG_HOME").ok();

    std::env::remove_var("MASTER_KEY");
    std::env::set_var("XDG_CONFIG_HOME", tmp.path());

    let first = ensure_master_key().expect("first ensure_master_key succeeds");
    assert_eq!(first, MasterKeySource::Generated);
    let first_hex = std::env::var("MASTER_KEY").expect("MASTER_KEY set after first call");

    let key_bytes: [u8; 32] = hex::decode(&first_hex)
        .expect("hex decode master key")
        .try_into()
        .expect("master key is 32 bytes");
    let plaintext = b"burncloud-secret-payload";
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).expect("init cipher");
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_ref())
        .expect("encrypt sample payload");

    // Simulate restart: clear the env var, leave the key file on disk.
    std::env::remove_var("MASTER_KEY");

    let second = ensure_master_key().expect("second ensure_master_key succeeds");
    assert_eq!(
        second,
        MasterKeySource::File,
        "restart must reuse the existing key file, not regenerate"
    );
    let second_hex = std::env::var("MASTER_KEY").expect("MASTER_KEY set after second call");
    assert_eq!(
        second_hex, first_hex,
        "restart must yield the identical master key"
    );

    let restart_key_bytes: [u8; 32] = hex::decode(&second_hex)
        .expect("hex decode master key after restart")
        .try_into()
        .expect("master key is 32 bytes after restart");
    let restart_cipher =
        Aes256Gcm::new_from_slice(&restart_key_bytes).expect("init cipher after restart");
    let decrypted = restart_cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .expect("decrypt payload with reloaded master key");
    assert_eq!(
        decrypted, plaintext,
        "data encrypted before restart must decrypt after restart"
    );

    match prev_master {
        Some(v) => std::env::set_var("MASTER_KEY", v),
        None => std::env::remove_var("MASTER_KEY"),
    }
    match prev_xdg {
        Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
}

#[test]
#[cfg(target_os = "linux")]
fn env_master_key_takes_precedence_and_skips_file_creation() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let tmp = tempfile::tempdir().expect("create tempdir");
    let prev_master_outer = std::env::var("MASTER_KEY").ok();
    let prev_xdg_outer = std::env::var("XDG_CONFIG_HOME").ok();

    let preset_key = "a".repeat(64);
    std::env::set_var("MASTER_KEY", &preset_key);
    std::env::set_var("XDG_CONFIG_HOME", tmp.path());

    let key_file = tmp.path().join("burncloud").join("master.key");
    assert!(
        !key_file.exists(),
        "precondition: key file should not exist before ensure_master_key"
    );

    let source = ensure_master_key().expect("ensure_master_key succeeds with env var set");
    assert_eq!(
        source,
        MasterKeySource::Env,
        "env var must be the source when MASTER_KEY is preset"
    );

    assert!(
        !key_file.exists(),
        "master.key file must NOT be created when MASTER_KEY env is set"
    );

    let env_key = std::env::var("MASTER_KEY").expect("MASTER_KEY env var should still be set");
    assert_eq!(
        env_key, preset_key,
        "MASTER_KEY env value must be preserved, not overwritten"
    );

    match prev_master_outer {
        Some(v) => std::env::set_var("MASTER_KEY", v),
        None => std::env::remove_var("MASTER_KEY"),
    }
    match prev_xdg_outer {
        Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
}
