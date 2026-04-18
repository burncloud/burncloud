#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
//! Step 4: Bundle Installation Tests
//!
//! Test running BurnCloud installation on the remote server
//!
//! Run: cargo test -p burncloud-tests --test aliyun_e2e_test -- --ignored step4_install

use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;

const TEST_PASSWORD: &str = "Burncloud@Test123";

/// Run BurnCloud installation command
/// Set env PUBLIC_IP before running
#[test]
#[ignore]
fn test_run_installation() {
    let public_ip = std::env::var("PUBLIC_IP").expect("Set PUBLIC_IP environment variable");

    println!("=== RUN INSTALLATION ===");
    println!("Public IP: {}", public_ip);

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

    let cmd = "set RUST_LOG=info && C:\\burncloud-test\\burncloud.exe install openclaw --bundle C:\\burncloud-test\\openclaw-bundle --auto-deps";
    println!("Command: {}", cmd);

    let mut channel = sess.channel_session().expect("Failed to open channel");
    channel.exec(cmd).expect("Failed to execute command");

    let mut output = String::new();
    channel
        .read_to_string(&mut output)
        .expect("Failed to read output");

    println!("=== INSTALLATION OUTPUT ===");
    println!("{}", output);

    if output.contains("error") || output.contains("Error") || output.contains("ERROR") {
        println!("WARNING: Installation may have errors!");
    }

    println!("Next step: Run test_check_installation_status");
}

/// Check installation status
/// Set env PUBLIC_IP before running
#[test]
#[ignore]
fn test_check_installation_status() {
    let public_ip = std::env::var("PUBLIC_IP").expect("Set PUBLIC_IP environment variable");

    println!("=== CHECK INSTALLATION STATUS ===");

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

    let cmd = "C:\\burncloud-test\\burncloud.exe install openclaw --status";
    let mut channel = sess.channel_session().expect("Failed to open channel");
    channel.exec(cmd).expect("Failed to execute command");

    let mut output = String::new();
    channel
        .read_to_string(&mut output)
        .expect("Failed to read output");

    println!("Status output:\n{}", output);
}

/// Check Node.js installation
/// Set env PUBLIC_IP before running
#[test]
#[ignore]
fn test_check_nodejs() {
    let public_ip = std::env::var("PUBLIC_IP").expect("Set PUBLIC_IP environment variable");

    println!("=== CHECK NODE.JS ===");

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

    let cmd = "set PATH=%USERPROFILE%\\AppData\\Roaming\\fnm\\node-versions\\v24.14.0\\installation;%PATH% && node --version && npm --version";
    let mut channel = sess.channel_session().expect("Failed to open channel");
    channel.exec(cmd).expect("Failed to execute command");

    let mut output = String::new();
    channel
        .read_to_string(&mut output)
        .expect("Failed to read output");

    println!("Node.js output:\n{}", output);
}

/// Check OpenClaw installation
/// Set env PUBLIC_IP before running
#[test]
#[ignore]
fn test_check_openclaw() {
    let public_ip = std::env::var("PUBLIC_IP").expect("Set PUBLIC_IP environment variable");

    println!("=== CHECK OPENCLAW ===");

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

    // Check Node.js in the new installation directory
    let cmd = "dir C:\\Users\\Administrator\\AppData\\Local\\burncloud\\nodejs 2>nul && C:\\Users\\Administrator\\AppData\\Local\\burncloud\\nodejs\\node-v22.14.0-win-x64\\node.exe --version";
    let mut channel = sess.channel_session().expect("Failed to open channel");
    channel.exec(cmd).expect("Failed to execute command");

    let mut output = Vec::new();
    channel
        .read_to_end(&mut output)
        .expect("Failed to read output");

    // Convert to string safely (handles Windows CMD encoding)
    let output_str = String::from_utf8_lossy(&output);
    println!("Node.js from bundle:\n{}", output_str);
}

/// Debug: Check bundle directory structure
#[test]
#[ignore]
fn test_debug_bundle_structure() {
    let public_ip = std::env::var("PUBLIC_IP").expect("Set PUBLIC_IP environment variable");

    println!("=== DEBUG BUNDLE STRUCTURE ===");

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

    // Check bundle directory structure (use simple dir to avoid encoding issues)
    let cmd = "dir C:\\burncloud-test\\openclaw-bundle\\dependencies\\node.js\\windows\\x64";
    let mut channel = sess.channel_session().expect("Failed to open channel");
    channel.exec(cmd).expect("Failed to execute command");

    let mut output = Vec::new();
    use std::io::Read;
    channel
        .read_to_end(&mut output)
        .expect("Failed to read output");

    // Convert to string safely
    let output_str = String::from_utf8_lossy(&output);
    println!("Bundle structure:\n{}", output_str);
}

/// Debug: Run installation with full debug output
#[test]
#[ignore]
fn test_debug_install_with_logs() {
    let public_ip = std::env::var("PUBLIC_IP").expect("Set PUBLIC_IP environment variable");

    println!("=== DEBUG INSTALL WITH LOGS ===");

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

    // Run installation with debug logs
    let cmd = "set RUST_LOG=debug && C:\\burncloud-test\\burncloud.exe install openclaw --bundle C:\\burncloud-test\\openclaw-bundle --auto-deps";
    println!("Command: {}", cmd);

    let mut channel = sess.channel_session().expect("Failed to open channel");
    channel.exec(cmd).expect("Failed to execute command");

    let mut output = Vec::new();
    use std::io::Read;
    channel
        .read_to_end(&mut output)
        .expect("Failed to read output");

    // Convert to string safely
    let output_str = String::from_utf8_lossy(&output);
    println!("Installation output:\n{}", output_str);
}

/// Debug: Check npm global packages and OpenClaw location
#[test]
#[ignore]
fn test_debug_npm_packages() {
    let public_ip = std::env::var("PUBLIC_IP").expect("Set PUBLIC_IP environment variable");

    println!("=== DEBUG NPM PACKAGES ===");

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

    // Check npm global packages
    let nodejs_path =
        r"C:\Users\Administrator\AppData\Local\burncloud\nodejs\node-v22.14.0-win-x64";
    let cmd = format!(
        r#"set PATH={};%PATH% && npm list -g --depth=0 && echo. && echo === NPM PREFIX === && npm config get prefix"#,
        nodejs_path
    );

    let mut channel = sess.channel_session().expect("Failed to open channel");
    channel.exec(&cmd).expect("Failed to execute command");

    let mut output = Vec::new();
    channel
        .read_to_end(&mut output)
        .expect("Failed to read output");

    let output_str = String::from_utf8_lossy(&output);
    println!("NPM packages:\n{}", output_str);
}
