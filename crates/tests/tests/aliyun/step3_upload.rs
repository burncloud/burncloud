#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
//! Step 3: File Upload Tests via SFTP
//!
//! Test uploading BurnCloud CLI and bundle via SFTP
//!
//! Run: cargo test -p burncloud-tests --test aliyun_e2e_test -- --ignored step3_upload

use ssh2::Session;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;

const TEST_PASSWORD: &str = "Burncloud@Test123";

/// Upload burncloud.exe and openclaw-bundle to the server
/// Set env PUBLIC_IP before running
#[test]
#[ignore]
fn test_upload_files() {
    let public_ip = std::env::var("PUBLIC_IP").expect("Set PUBLIC_IP environment variable");

    // Get project root
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let project_root = Path::new(&manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to get project root");

    let cli_path = project_root.join("target/release/burncloud.exe");
    let bundle_path = project_root.join("target/release/openclaw-bundle");

    println!("=== UPLOAD FILES ===");
    println!("Public IP: {}", public_ip);
    println!("CLI: {}", cli_path.display());
    println!("Bundle: {}", bundle_path.display());

    // Connect via SSH
    let tcp = TcpStream::connect(format!("{}:22", public_ip)).expect("Failed to connect via TCP");
    let mut sess = Session::new().expect("Failed to create SSH session");
    sess.set_tcp_stream(tcp);
    sess.handshake().expect("SSH handshake failed");
    sess.userauth_password("Administrator", TEST_PASSWORD)
        .expect("SSH authentication failed");

    // Create remote directory
    {
        let mut channel = sess.channel_session().expect("Failed to open channel");
        channel
            .exec("cmd /c \"if not exist C:\\burncloud-test mkdir C:\\burncloud-test\"")
            .expect("Failed to create directory");
        channel.wait_close().ok();
    }

    // Open SFTP
    let sftp = sess.sftp().expect("Failed to open SFTP");

    // Upload CLI
    if cli_path.exists() {
        println!("  Uploading burncloud.exe...");
        let mut local_file = std::fs::File::open(&cli_path).expect("Failed to open local CLI");
        let mut buffer = Vec::new();
        local_file
            .read_to_end(&mut buffer)
            .expect("Failed to read CLI");

        let remote_path = Path::new("C:/burncloud-test/burncloud.exe");
        let mut remote_file = sftp
            .create(remote_path)
            .expect("Failed to create remote file");
        remote_file.write_all(&buffer).expect("Failed to write CLI");
        println!("  CLI uploaded ({} bytes)", buffer.len());
    } else {
        panic!("CLI not found: {}", cli_path.display());
    }

    // Upload bundle
    if bundle_path.exists() {
        println!("  Uploading bundle...");
        upload_directory(&sftp, &bundle_path, "C:/burncloud-test/openclaw-bundle");
        println!("  Bundle uploaded");
    } else {
        panic!("Bundle not found: {}", bundle_path.display());
    }

    println!("=== UPLOAD COMPLETE ===");
    println!("Next step: Run test_run_installation");
}

/// Upload directory recursively via SFTP
fn upload_directory(sftp: &ssh2::Sftp, local: &Path, remote: &str) {
    let _ = sftp.mkdir(Path::new(remote), 0o755); // Ignore error if exists

    for entry in std::fs::read_dir(local).expect("Failed to read directory") {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let remote_path = format!("{}/{}", remote, name);

        if path.is_dir() {
            upload_directory(sftp, &path, &remote_path);
        } else {
            let mut local_file = std::fs::File::open(&path).expect("Failed to open file");
            let mut buffer = Vec::new();
            local_file
                .read_to_end(&mut buffer)
                .expect("Failed to read file");

            let mut remote_file = sftp
                .create(Path::new(&remote_path))
                .expect("Failed to create remote file");
            remote_file
                .write_all(&buffer)
                .expect("Failed to write file");
        }
    }
}

/// Verify uploaded files exist on server
/// Set env PUBLIC_IP before running
#[test]
#[ignore]
fn test_verify_uploaded_files() {
    let public_ip = std::env::var("PUBLIC_IP").expect("Set PUBLIC_IP environment variable");

    // Connect via SSH
    let tcp = TcpStream::connect(format!("{}:22", public_ip)).expect("Failed to connect via TCP");
    let sess = {
        let mut sess = Session::new().expect("Failed to create SSH session");
        sess.set_tcp_stream(tcp);
        sess.handshake().expect("SSH handshake failed");
        sess.userauth_password("Administrator", TEST_PASSWORD)
            .expect("SSH authentication failed");
        sess
    };

    // Check files exist
    let sftp = sess.sftp().expect("Failed to open SFTP");

    let cli_exists = sftp
        .stat(Path::new("C:/burncloud-test/burncloud.exe"))
        .is_ok();
    let bundle_exists = sftp
        .stat(Path::new("C:/burncloud-test/openclaw-bundle"))
        .is_ok();

    println!("=== FILE CHECK ===");
    println!(
        "burncloud.exe: {}",
        if cli_exists { "EXISTS" } else { "MISSING" }
    );
    println!(
        "openclaw-bundle: {}",
        if bundle_exists { "EXISTS" } else { "MISSING" }
    );

    assert!(cli_exists, "burncloud.exe not found");
    assert!(bundle_exists, "openclaw-bundle not found");
}
