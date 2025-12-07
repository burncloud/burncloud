use dotenvy::dotenv;
use std::env;
use std::process::{Command, Child, Stdio};
use std::path::PathBuf;
use std::time::Duration;
use std::net::TcpListener;
use std::sync::OnceLock;
use reqwest::Client;

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
            panic!("Binary not found at {:?}. Run 'cargo build --bin burncloud' first.", binary_path);
        }

        // 3. Pick Port & Spawn
        let port = get_free_port();
        println!("TEST: Spawning new server at http://127.0.0.1:{}", port);

        let process = Command::new(binary_path)
            .arg("server")
            .arg("start")
            .env("PORT", port.to_string())
            .stdout(Stdio::null()) 
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
    for _ in 0..50 { // 5s timeout
        if client.get(format!("{}/api/status", url)).send().await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("Server failed to start at {}", url);
}

pub fn get_root_token() -> String {
    "sk-root-token-123456".to_string()
}

pub fn get_demo_token() -> String {
    "sk-burncloud-demo".to_string()
}

pub fn get_openai_config() -> Option<(String, String)> {
    dotenv().ok();
    let key = env::var("TEST_OPENAI_KEY").ok().filter(|k| !k.is_empty())?;
    let url = env::var("TEST_OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com".to_string());
    Some((key, url))
}

// Removed deprecated functions: get_base_url (sync), get_db_pool, seed_demo_data