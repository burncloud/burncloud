#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
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

#[derive(Debug)]
struct ServerHandle {
    pub base_url: String,
    #[allow(dead_code)]
    process: Option<Child>, // Keep child alive
}

pub async fn spawn_app() -> String {
    // Load .env
    dotenv().ok();

    let handle = SERVER_HANDLE.get_or_init(|| {
        // 1. Check default port 3000
        if is_port_open(3000) {
            println!("TEST: Reusing existing server at http://127.0.0.1:3000");
            return ServerHandle {
                base_url: "http://127.0.0.1:3000".to_string(),
                process: None,
            };
        }

        // 2. Locate Binary
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
        let manifest_path = PathBuf::from(manifest_dir);
        let root_dir = manifest_path.parent().unwrap().parent().unwrap();

        let binary_path = if cfg!(target_os = "windows") {
            root_dir.join("target/debug/burncloud.exe")
        } else {
            root_dir.join("target/debug/burncloud")
        };

        if !binary_path.exists() {
            panic!(
                "Binary not found at {:?}. Run 'cargo build --bin burncloud' first.",
                binary_path
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

        // Return handle immediately, wait async later
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
    let client = Client::new();
    for _ in 0..120 {
        // 60s timeout (price sync can take ~30s on first run)
        if client
            .get(format!("{}/api/status", url))
            .send()
            .await
            .is_ok()
        {
            return;
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
