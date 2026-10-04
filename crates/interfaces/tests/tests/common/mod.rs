#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::let_unit_value,
    clippy::redundant_pattern,
    clippy::manual_is_multiple_of,
    clippy::let_and_return,
    clippy::to_string_trait_impl,
    clippy::to_string_in_format_args,
    clippy::redundant_pattern_matching,
    dead_code
)]
pub mod evidence;

use dotenvy::dotenv;
use reqwest::Client;
use std::env;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::time::Duration;

static SERVER_HANDLE: OnceLock<ServerHandle> = OnceLock::new();

/// A handle to the server the suite is talking to.
///
/// **`process` is a bare `Child` and the server is NOT terminated when the suite ends. That is a recorded
/// defect, not something left unexamined.**
///
/// Measured: after `cargo test -p burncloud-tests --test api_tests`, one `burncloud` process is still running,
/// with a start time matching the run. It has to be terminated by hand, and it holds its port.
///
/// **Two fixes were written, measured, and both failed**, which is why neither is in this file:
///
/// 1. A `Drop` impl calling `Child::kill()` then `wait()`. **Still one process left.**
/// 2. The same `Drop`, but using `taskkill /F /T /PID` on Windows to terminate the process **tree**, on the
///    theory that `burncloud server start` spawns descendants that `kill` does not reach. **Still one process.**
///
/// The reason is neither: **`SERVER_HANDLE` is a `static`, and Rust does not run destructors for `static`
/// values during process exit.** The `Drop` never executes, so the kill strategy inside it is irrelevant. That
/// was established only after the second failure; the first two attempts guessed at the wrong layer.
///
/// The fix is therefore not a `Drop`. It has to be one of:
///
/// * an explicit shutdown after the last test, which this fixture cannot express today because the handle is
///   behind a `OnceLock` and no test owns its lifetime;
/// * the server exiting on its own when its stdin closes, which is how most harnesses get this for free;
/// * a `Drop` on a value that actually gets dropped, i.e. not a `static` -- which means restructuring
///   `spawn_app` rather than patching it.
///
/// The plan's item 29 states the rule for the opposite case -- "只终止自己启动的进程". This is the other half of
/// the same rule, and it is unmet here.
///
/// **Worth checking wherever a test starts a process**, because the failure is silent: the suite reports 71
/// passed either way, and the leftover process holds the port the next run wants.
#[derive(Debug)]
struct ServerHandle {
    pub base_url: String,
    /// `None` when the suite reused a server it did not start -- on port 3000, or one named by `E2E_BASE_URL`.
    /// Killing that one would be exactly the mistake item 29 names.
    process: Option<Child>,
}

pub async fn spawn_app() -> String {
    // Load .env
    dotenv().ok();

    // 0. Check E2E_BASE_URL environment variable first
    if let Ok(base_url) = env::var("E2E_BASE_URL") {
        println!("TEST: Using E2E_BASE_URL from env: {}", base_url);
        wait_for_server(&base_url).await;
        return base_url;
    }

    let handle = SERVER_HANDLE.get_or_init(|| {
        // 1. Reuse port 3000 only when not forcing an isolated test server
        let force_spawn = env::var("E2E_FORCE_SPAWN")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if !force_spawn && is_port_open(3000) {
            println!("TEST: Reusing existing server at http://127.0.0.1:3000");
            return ServerHandle {
                base_url: "http://127.0.0.1:3000".to_string(),
                process: None,
            };
        }
        if force_spawn {
            println!("TEST: E2E_FORCE_SPAWN set — spawning dedicated server (skip :3000 reuse)");
        }

        // 2. Locate Binary
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
        let manifest_path = PathBuf::from(manifest_dir);
        let root_dir = manifest_path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap();

        let binary_path = if cfg!(target_os = "windows") {
            root_dir.join("target/debug/burncloud.exe")
        } else {
            root_dir.join("target/debug/burncloud")
        };

        if !binary_path.exists() {
            // **Cargo does not build this binary for these tests**, because the tests are integration tests of
            // another crate. Eleven tests failed with this message before the prerequisite was met, and the
            // message did not say which command produces the file, so the failure read like a broken test suite
            // rather than a missing build step. The command is now spelled out.
            panic!(
                "The black-box API tests need the server binary, which Cargo does not build for them.\n\
                 Expected: {}\n\
                 Build it first:  cargo build --bin burncloud\n\
                 Then re-run:     cargo test -p burncloud-tests --test api_tests",
                binary_path.display()
            );
        }

        // 3. Pick Port & Spawn
        let port = get_free_port();
        println!("TEST: Spawning new server at http://127.0.0.1:{}", port);

        let process = Command::new(binary_path)
            .arg("server")
            .arg("start")
            .env("PORT", port.to_string())
            .env("RUST_LOG", "burncloud=warn") // Reduce log noise
            .env("NO_PROXY", "*") // Prevent proxy issues
            .env(
                "MASTER_KEY",
                "a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8",
            ) // 64 hex chars for test
            .env("PRICE_SYNC_INTERVAL_SECS", "999999") // Disable price sync for faster startup
            .env("SKIP_INITIAL_PRICE_SYNC", "1") // Skip initial price sync in tests
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Failed to spawn server");

        // Return handle immediately, wait async later. **The child is stored, not owned by anything that gets
        // dropped** -- see `ServerHandle`'s note: it keeps running after the suite exits.
        ServerHandle {
            base_url: format!("http://127.0.0.1:{}", port),
            process: Some(process),
        }
    });

    // 4. Async Wait for Readiness
    // Since multiple tests run in parallel, they might all call this.
    // It's idempotent (GET /status).
    wait_for_server(&handle.base_url).await;

    handle.base_url.clone()
}

fn is_port_open(port: u16) -> bool {
    std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok()
}

fn get_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

async fn wait_for_server(url: &str) {
    let client = Client::builder()
        .no_proxy()
        .build()
        .expect("reqwest client");
    for i in 0..120 {
        // 60s timeout (price sync can take ~30s on first run)
        // Use /health endpoint which doesn't require auth
        if client
            .get(format!("{}/health", url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            return;
        }
        if i > 0 && i % 10 == 0 {
            eprintln!("TEST: waiting for server at {url} ({i}/120)");
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    panic!("Server failed to start at {}", url);
}

#[allow(dead_code)]
pub fn get_root_token() -> String {
    "sk-root-token-123456".to_string()
}

#[allow(dead_code)]
pub fn get_demo_token() -> String {
    "sk-burncloud-demo".to_string()
}

#[allow(dead_code)]
pub fn get_openai_config() -> Option<(String, String)> {
    dotenv().ok();
    let key = env::var("TEST_OPENAI_KEY").ok().filter(|k| !k.is_empty())?;
    let url =
        env::var("TEST_OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com".to_string());
    Some((key, url))
}

// Removed deprecated functions: get_base_url (sync), get_db_pool, seed_demo_data

/// Insert a price entry for a mock model so the router's preflight check passes.
#[allow(dead_code)]
pub async fn insert_mock_price(model: &str) {
    let db_url = std::env::var("BURNCLOUD_DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:///tmp/test_burncloud.db?mode=rwc".to_string());
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .expect("Failed to connect to test DB");
    sqlx::query(
        "INSERT OR IGNORE INTO billing_prices (model, currency, input_price, output_price, region) VALUES (?, 'USD', 0, 0, '')"
    )
    .bind(model)
    .execute(&pool)
    .await
    .expect("Failed to insert mock price");
    pool.close().await;

    // Trigger a price cache refresh via the internal API
    let base_url = SERVER_HANDLE
        .get()
        .map(|h| h.base_url.clone())
        .unwrap_or_default();
    if !base_url.is_empty() {
        let _ = reqwest::Client::new()
            .post(format!("{}/console/internal/prices/sync", base_url))
            .send()
            .await;
    }
}
