#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
//! Step 2: SSH Installation and Connection Tests
//!
//! Test installing SSH on Windows ECS and connecting via SSH
//!
//! Run: cargo test -p burncloud-tests --test aliyun_e2e_test -- --ignored 02_ssh

use super::*;
use ssh2::Session;
use std::net::TcpStream;
use std::time::Duration;

const TEST_PASSWORD: &str = "Burncloud@Test123";

/// Install SSH via Cloud Assistant
/// Set env INSTANCE_ID before running
#[test]
#[ignore]
fn test_install_ssh() {
    let instance_id = std::env::var("INSTANCE_ID").expect("Set INSTANCE_ID environment variable");

    let ecs = AliyunECS::from_config_file_with_region("cn-shenzhen")
        .expect("Failed to load Aliyun config");

    let invoke_id = ecs
        .install_ssh(&instance_id)
        .expect("Failed to install SSH");

    println!("=== SSH INSTALLATION TRIGGERED ===");
    println!("Instance ID: {}", instance_id);
    println!("Invoke ID: {}", invoke_id);
    println!("Next step: Wait ~60 seconds, then run test_wait_for_ssh");
}

/// Wait for SSH to be available
/// Set env PUBLIC_IP before running
#[test]
#[ignore]
fn test_wait_for_ssh() {
    let public_ip = std::env::var("PUBLIC_IP").expect("Set PUBLIC_IP environment variable");

    println!("Waiting for SSH at {}:22...", public_ip);
    let start = std::time::Instant::now();

    loop {
        let elapsed = start.elapsed().as_secs();
        if elapsed > 180 {
            panic!("Timeout waiting for SSH");
        }

        if let Ok(_) = TcpStream::connect(format!("{}:22", public_ip)) {
            println!("=== SSH PORT OPEN ===");
            println!("Public IP: {}", public_ip);
            println!("Time: {}s", elapsed);
            // Give SSH service time to initialize
            std::thread::sleep(Duration::from_secs(5));
            println!("Next step: Run test_ssh_connect");
            return;
        }

        if elapsed % 10 == 0 {
            println!("  [{}s] Waiting...", elapsed);
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

/// Test SSH connection
/// Set env PUBLIC_IP before running
#[test]
#[ignore]
fn test_ssh_connect() {
    let public_ip = std::env::var("PUBLIC_IP").expect("Set PUBLIC_IP environment variable");

    println!("Connecting to {} via SSH...", public_ip);

    let tcp = TcpStream::connect(format!("{}:22", public_ip)).expect("Failed to connect via TCP");

    let mut sess = Session::new().expect("Failed to create SSH session");
    sess.set_tcp_stream(tcp);
    sess.handshake().expect("SSH handshake failed");

    sess.userauth_password("Administrator", TEST_PASSWORD)
        .expect("SSH authentication failed");

    println!("=== SSH CONNECTED ===");
    println!("Public IP: {}", public_ip);
    println!("User: Administrator");

    // Test command execution
    let mut channel = sess.channel_session().expect("Failed to open channel");
    channel.exec("whoami").expect("Failed to execute command");

    let mut output = String::new();
    use std::io::Read;
    channel
        .read_to_string(&mut output)
        .expect("Failed to read output");
    println!("Command output: {}", output.trim());

    println!("Next step: Run test_upload_files");
}
