use crate::common;
use std::process::Command;
use std::path::Path;

#[tokio::test]
async fn run_playwright_e2e() {
    // 1. Start the server (or reuse existing)
    let base_url = common::spawn_app().await;
    println!("🔗 Server is running at: {}", base_url);

    // 2. Locate the UI test directory
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let ui_test_dir = Path::new(&manifest_dir).join("tests").join("ui");
    
    println!("📂 Playwright working directory: {:?}", ui_test_dir);

    // Check if node_modules exists, strictly speaking we should probably ensure npm install runs, 
    // but for CI/Test speed we often assume it's prepped. 
    // Let's at least check for package.json
    if !ui_test_dir.join("package.json").exists() {
        panic!("package.json not found in {:?}. Did you move the files correctly?", ui_test_dir);
    }

    // 3. Prepare the command
    let program = if cfg!(target_os = "windows") { "npx.cmd" } else { "npx" };
    
    println!("🚀 Executing: {} playwright test", program);

    let status = Command::new(program)
        .arg("playwright")
        .arg("test")
        .arg("--workers=30")
        .current_dir(&ui_test_dir)
        .env("BASE_URL", &base_url) // Pass the dynamic server URL to Playwright
        // Pass CI env var if needed, or let it inherit
        .env("CI", "true") // Force simple output or CI behavior if desired
        .status()
        .expect("Failed to execute playwright command. Is Node.js/npm installed?");

    // 4. Assert success
    assert!(status.success(), "❌ Playwright tests failed! Check output above for details.");
}
