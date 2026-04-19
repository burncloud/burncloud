#![allow(clippy::expect_used, clippy::disallowed_types)]
//! Local Node.js Installation Test
//!
//! This test validates that Node.js can be installed from a local bundle.
//! Run with: cargo test -p burncloud-tests --test local_nodejs_test -- --nocapture

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn cli_binary_path() -> PathBuf {
    project_root()
        .join("target")
        .join("release")
        .join("burncloud.exe")
}

/// Test 1: Build the CLI
#[test]
fn test_build_cli() {
    println!("=== Building CLI ===");

    let output = Command::new("cargo")
        .args(["build", "--release", "--bin", "burncloud"])
        .current_dir(project_root())
        .output()
        .expect("Failed to build CLI");

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "CLI build failed");
    assert!(cli_binary_path().exists(), "CLI binary not found");
    println!("CLI built: {}", cli_binary_path().display());
}

/// Test 2: Create bundle for openclaw
#[test]
fn test_create_bundle() {
    // First ensure CLI is built
    let build_output = Command::new("cargo")
        .args(["build", "--release", "--bin", "burncloud"])
        .current_dir(project_root())
        .output()
        .expect("Failed to build CLI");
    assert!(build_output.status.success(), "CLI build failed");

    println!("=== Creating Bundle ===");
    let start = Instant::now();

    let bundle_dir = project_root().join("test").join("bundle-output");

    let output = Command::new(cli_binary_path())
        .args([
            "bundle",
            "create",
            "openclaw",
            "-o",
            bundle_dir.to_str().unwrap(),
        ])
        .env("RUST_LOG", "info")
        .output()
        .expect("Failed to create bundle");

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    println!(
        "Bundle creation took: {:.2}s",
        start.elapsed().as_secs_f64()
    );

    assert!(output.status.success(), "Bundle creation failed");

    // Verify bundle structure
    let bundle_path = bundle_dir.join("openclaw-bundle");
    assert!(bundle_path.exists(), "Bundle directory not created");

    // Check manifest
    let manifest_path = bundle_path.join("manifest.json");
    assert!(manifest_path.exists(), "manifest.json not found");

    // Check Node.js dependency
    let nodejs_dep = bundle_path
        .join("dependencies")
        .join("node.js")
        .join("windows")
        .join("x64");
    println!("Node.js dependency path: {}", nodejs_dep.display());

    if nodejs_dep.exists() {
        println!("Node.js dependency found in bundle!");
        for entry in std::fs::read_dir(&nodejs_dep).unwrap() {
            let entry = entry.unwrap();
            println!("  - {}", entry.file_name().to_string_lossy());
        }
    } else {
        println!("WARNING: Node.js dependency NOT found in bundle!");
    }
}

/// Test 3: Run installation locally with verbose logging
#[test]
fn test_local_install_verbose() {
    // First create bundle
    let bundle_dir = project_root().join("test").join("bundle-output");
    let bundle_path = bundle_dir.join("openclaw-bundle");

    if !bundle_path.exists() {
        println!("Bundle not found, creating...");
        test_create_bundle();
    }

    println!("=== Running Local Installation (Verbose) ===");
    let start = Instant::now();

    let output = Command::new(cli_binary_path())
        .args([
            "install",
            "openclaw",
            "--bundle",
            bundle_path.to_str().unwrap(),
            "--auto-deps",
        ])
        .env("RUST_LOG", "debug")
        .output()
        .expect("Failed to run installation");

    println!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    println!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));
    println!("Installation took: {:.2}s", start.elapsed().as_secs_f64());

    if !output.status.success() {
        println!(
            "Installation failed with exit code: {:?}",
            output.status.code()
        );
    }
}

/// Test 4: Check if Node.js is accessible
#[test]
fn test_check_nodejs() {
    println!("=== Checking Node.js Installation ===");

    // Check standard locations on Windows
    let local_app_data = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| "C:\\Users\\Administrator\\AppData\\Local".to_string());
    let app_data = std::env::var("APPDATA")
        .unwrap_or_else(|_| "C:\\Users\\Administrator\\AppData\\Roaming".to_string());

    let locations = vec![
        // BurnCloud installation
        PathBuf::from(&local_app_data)
            .join("burncloud")
            .join("nodejs"),
        // fnm installation
        PathBuf::from(&app_data).join("fnm").join("node-versions"),
    ];

    for loc in locations {
        println!("Checking: {}", loc.display());
        if loc.exists() {
            println!("  EXISTS!");
            if let Ok(entries) = std::fs::read_dir(&loc) {
                for entry in entries.flatten() {
                    println!("    - {}", entry.file_name().to_string_lossy());
                }
            }
        } else {
            println!("  NOT FOUND");
        }
    }

    // Check node --version
    let output = Command::new("cmd").args(["/C", "node --version"]).output();

    match output {
        Ok(o) => {
            println!("node --version: {}", String::from_utf8_lossy(&o.stdout));
        }
        Err(e) => {
            println!("node command failed: {}", e);
        }
    }
}

/// Test 5: Simulate remote installation behavior (without bundle)
/// This should reveal why remote installation takes 1 hour
#[test]
fn test_remote_install_simulation() {
    println!("=== Simulating Remote Installation (No Bundle) ===");

    let output = Command::new(cli_binary_path())
        .args(["install", "openclaw", "--auto-deps"])
        .env("RUST_LOG", "debug")
        .output()
        .expect("Failed to run installation");

    println!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    println!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));

    // This should show the error about unsupported auto-install method for Node.js
    // because install_dependency only supports PackageManager, but Node.js uses DirectDownload
}
