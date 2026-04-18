#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
//! Step 5: Installation Verification Tests
//!
//! Verify the complete installation is working
//!
//! Run: cargo test -p burncloud-tests --test aliyun_e2e_test -- --ignored step5_verify

use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;

const TEST_PASSWORD: &str = "Burncloud@Test123";

/// Node.js installation path from bundle
const NODEJS_PATH: &str =
    r"C:\Users\Administrator\AppData\Local\burncloud\nodejs\node-v22.14.0-win-x64";

/// Full verification of installation
/// Set env PUBLIC_IP before running
#[test]
#[ignore]
fn test_full_verification() {
    let public_ip = std::env::var("PUBLIC_IP").expect("Set PUBLIC_IP environment variable");

    println!("=== FULL VERIFICATION ===");
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

    // 1. Check status
    println!("\n1. Checking installation status...");
    let mut channel = sess.channel_session().expect("Failed to open channel");
    channel
        .exec("C:\\burncloud-test\\burncloud.exe install openclaw --status")
        .expect("Failed to execute");
    let mut output = Vec::new();
    channel.read_to_end(&mut output).expect("Failed to read");
    let status_output = String::from_utf8_lossy(&output);
    println!("{}", status_output);

    let status_ok = status_output.contains("Installed");
    println!("Status: {}", if status_ok { "PASS" } else { "FAIL" });

    // 2. Check Node.js (from bundle installation)
    println!("\n2. Checking Node.js...");
    let mut channel = sess.channel_session().expect("Failed to open channel");
    let node_cmd = format!(r#"set PATH={};%PATH% && node --version"#, NODEJS_PATH);
    channel.exec(&node_cmd).expect("Failed to execute");
    let mut output = Vec::new();
    channel.read_to_end(&mut output).expect("Failed to read");
    let node_output = String::from_utf8_lossy(&output);
    println!("{}", node_output.trim());

    let node_ok = node_output.starts_with("v") || node_output.contains("v22");
    println!("Node.js: {}", if node_ok { "PASS" } else { "FAIL" });

    // 3. Check OpenClaw npm package is installed
    // OpenClaw is a web application, not a CLI tool, so we verify the npm package exists
    println!("\n3. Checking OpenClaw npm package...");
    let mut channel = sess.channel_session().expect("Failed to open channel");
    let openclaw_cmd = format!(
        r#"set PATH={};%PATH% && npm list -g openclaw --depth=0 2>nul | findstr openclaw"#,
        NODEJS_PATH
    );
    channel.exec(&openclaw_cmd).expect("Failed to execute");
    let mut output = Vec::new();
    channel.read_to_end(&mut output).expect("Failed to read");
    let openclaw_output = String::from_utf8_lossy(&output);
    println!("{}", openclaw_output.trim());

    let openclaw_ok = openclaw_output.contains("openclaw");
    println!("OpenClaw: {}", if openclaw_ok { "PASS" } else { "FAIL" });

    // 4. Check OpenClaw dist directory exists (proves npm package was fully installed)
    println!("\n4. Checking OpenClaw installation directory...");
    let mut channel = sess.channel_session().expect("Failed to open channel");
    let dist_cmd = format!(
        r#"dir "{}\node_modules\openclaw\dist" 2>nul | findstr "canvas-host control-ui""#,
        NODEJS_PATH
    );
    channel.exec(&dist_cmd).expect("Failed to execute");
    let mut output = Vec::new();
    channel.read_to_end(&mut output).expect("Failed to read");
    let dist_output = String::from_utf8_lossy(&output);

    let dist_ok = dist_output.contains("canvas-host") || dist_output.contains("control-ui");
    println!("OpenClaw dist: {}", if dist_ok { "PASS" } else { "FAIL" });

    // Summary
    println!("\n=== VERIFICATION SUMMARY ===");
    let all_ok = status_ok && node_ok && openclaw_ok && dist_ok;
    if all_ok {
        println!("ALL CHECKS PASSED");
    } else {
        println!("SOME CHECKS FAILED");
        if !status_ok {
            println!("  - Installation status: FAILED");
        }
        if !node_ok {
            println!("  - Node.js: FAILED");
        }
        if !openclaw_ok {
            println!("  - OpenClaw npm package: FAILED");
        }
        if !dist_ok {
            println!("  - OpenClaw dist directory: FAILED");
        }
    }

    assert!(all_ok, "Installation verification failed");
}

/// Quick health check - just verify services are running
/// Set env PUBLIC_IP before running
#[test]
#[ignore]
fn test_quick_health_check() {
    let public_ip = std::env::var("PUBLIC_IP").expect("Set PUBLIC_IP environment variable");

    println!("=== QUICK HEALTH CHECK ===");

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

    // Just check node works (from bundle installation)
    let mut channel = sess.channel_session().expect("Failed to open channel");
    let node_cmd = format!(
        r#"set PATH={};%PATH% && node -e "console.log('OK')""#,
        NODEJS_PATH
    );
    channel.exec(&node_cmd).expect("Failed to execute");
    let mut output = Vec::new();
    channel.read_to_end(&mut output).expect("Failed to read");
    let output_str = String::from_utf8_lossy(&output);

    if output_str.contains("OK") {
        println!("Node.js is working");
    } else {
        panic!("Node.js is not working: {}", output_str);
    }
}
