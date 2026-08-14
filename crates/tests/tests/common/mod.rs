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
use serde_json::json;
use std::env;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::time::Duration;

static SERVER_HANDLE: OnceLock<ServerHandle> = OnceLock::new();
static TEST_BOOTSTRAP_DONE: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
const TEST_BOOTSTRAP_TOKEN: &str = "burncloud-e2e-bootstrap-token-2026";

#[derive(Debug)]
struct ServerHandle {
    pub base_url: String,
    #[allow(dead_code)]
    process: Option<Child>,
}

pub async fn spawn_app() -> String {
    dotenv().ok();

    // Externally managed E2E targets own their own bootstrap policy. Never
    // inject the test bootstrap credential into a server this harness did not spawn.
    if let Ok(base_url) = env::var("E2E_BASE_URL") {
        println!("TEST: Using E2E_BASE_URL from env: {}", base_url);
        wait_for_server(&base_url).await;
        return base_url;
    }

    let handle = SERVER_HANDLE.get_or_init(|| {
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

        let port = get_free_port();
        println!("TEST: Spawning new server at http://127.0.0.1:{}", port);

        let process = Command::new(binary_path)
            .arg("server")
            .arg("start")
            .env("PORT", port.to_string())
            .env("RUST_LOG", "burncloud=warn")
            .env("NO_PROXY", "*")
            .env(
                "MASTER_KEY",
                "a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8",
            )
            .env("BURNCLOUD_BOOTSTRAP_TOKEN", TEST_BOOTSTRAP_TOKEN)
            .env("BURNCLOUD_PUBLIC_REGISTRATION", "open")
            .env("BURNCLOUD_PUBLIC_SIGNUP_BONUS_USD", "0")
            .env("PRICE_SYNC_INTERVAL_SECS", "999999")
            .env("SKIP_INITIAL_PRICE_SYNC", "1")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Failed to spawn server");

        ServerHandle {
            base_url: format!("http://127.0.0.1:{}", port),
            process: Some(process),
        }
    });

    wait_for_server(&handle.base_url).await;
    if handle.process.is_some() {
        let base_url = handle.base_url.clone();
        TEST_BOOTSTRAP_DONE
            .get_or_init(|| async move {
                ensure_test_bootstrap(&base_url).await;
            })
            .await;
    }

    handle.base_url.clone()
}

async fn ensure_test_bootstrap(base_url: &str) {
    let client = Client::builder()
        .no_proxy()
        .build()
        .expect("reqwest bootstrap client");
    let body = json!({
        "username": "e2e-bootstrap-admin",
        "password": "E2eBootstrapAdmin123!",
        "email": "e2e-bootstrap-admin@example.invalid",
        "bootstrap_token": TEST_BOOTSTRAP_TOKEN
    });

    let payload: serde_json::Value = client
        .post(format!("{base_url}/api/auth/register"))
        .json(&body)
        .send()
        .await
        .expect("bootstrap registration request")
        .json()
        .await
        .expect("bootstrap registration response");

    if payload["success"] == true {
        assert_eq!(payload["data"]["roles"][0], "admin");
        return;
    }

    let message = payload["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("already been completed") || message.contains("already exists"),
        "unexpected bootstrap response: {payload}"
    );
}

fn is_port_open(port: u16) -> bool {
    std::net::TcpStream::connect(format!("127.0.0.1:{port}")).is_ok()
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
        "INSERT OR IGNORE INTO billing_prices (model, currency, input_price, output_price, region) VALUES (?, 'USD', 0, 0, '')",
    )
    .bind(model)
    .execute(&pool)
    .await
    .expect("Failed to insert mock price");
    pool.close().await;

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