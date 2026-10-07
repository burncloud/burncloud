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
    dead_code,
    reason = "integration-test fixture: panicking when the server or fixtures are unavailable is the intended signal, and some helpers are only used by one test target"
)]
pub(crate) mod evidence;

use burncloud_database::create_database_with_url;
use burncloud_database_router::RouterDatabase;
use burncloud_database_user::UserDatabase;
use burncloud_service_user::JwtSecret;
use burncloud_tests::TestClient;
use dotenvy::dotenv;
use reqwest::Client;
use serde_json::json;
use std::env;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

static SERVER_HANDLE: OnceLock<ServerHandle> = OnceLock::new();
static ADMIN_JWT: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();

const TEST_JWT_SECRET: &str = "burncloud-api-tests-jwt-secret";
const TEST_INTERNAL_SECRET: &str = "burncloud-api-tests-internal-secret";

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

/// Owns a short-lived in-process server and its isolated administrator identity.
///
/// Dropping the handle aborts the listener; unlike the external-server fixture,
/// nothing is stored in a static process handle.
pub(crate) struct IsolatedApp {
    pub(crate) base_url: String,
    admin_token: String,
    task: tokio::task::JoinHandle<()>,
}

impl IsolatedApp {
    pub(crate) fn admin_client(&self) -> TestClient {
        TestClient::new(&self.base_url).with_token(&self.admin_token)
    }
}

impl Drop for IsolatedApp {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Start an in-process Console server on an ephemeral port, with a unique
/// shared-memory SQLite database. Unlike spawn_app, this fixture never touches
/// E2E_BASE_URL, port 3000, or the developer's on-disk database.
pub(crate) async fn spawn_isolated_app() -> IsolatedApp {
    if env::var("MASTER_KEY").is_err() {
        env::set_var(
            "MASTER_KEY",
            "a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8",
        );
    }

    let database_url = format!(
        "sqlite:file:burncloud_api_test_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4()
    );
    let db = create_database_with_url(&database_url)
        .await
        .expect("open isolated API-test database");
    RouterDatabase::init(&db)
        .await
        .expect("initialize Router test schema");
    UserDatabase::init(&db)
        .await
        .expect("initialize User test schema");

    let jwt_secret =
        JwtSecret::new("burncloud-api-test-jwt-secret").expect("test JWT secret must be valid");
    let internal_secret =
        burncloud_server::InternalSecret::new("burncloud-api-test-internal-secret")
            .expect("test internal secret must be valid");
    let router = burncloud_server::create_app(Arc::new(db), false, jwt_secret, internal_secret)
        .await
        .expect("create isolated API-test server");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind isolated API-test port");
    let address = listener.local_addr().expect("read isolated server address");
    let base_url = format!("http://{address}");
    let task = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, router).await {
            eprintln!("isolated API-test server stopped with error: {error}");
        }
    });

    wait_for_server(&base_url).await;

    // The first real user in a fresh database gets the administrator role.
    // Keep its JWT scoped to this server; the external fixture's OnceCell
    // must not share credentials across separate isolated databases.
    let username = format!("isolated-admin-{}", uuid::Uuid::new_v4());
    let registration = TestClient::new(&base_url)
        .post(
            "/api/auth/register",
            &json!({
                "username": username,
                "password": "Password123!",
                "email": format!("{username}@example.invalid")
            }),
        )
        .await
        .expect("register isolated test administrator");
    assert!(
        registration["success"].as_bool().unwrap_or(false),
        "isolated administrator registration failed: {registration}"
    );
    assert_eq!(
        registration["data"]["roles"]
            .as_array()
            .map(|roles| roles.iter().any(|role| role.as_str() == Some("admin"))),
        Some(true),
        "first isolated user must have administrator role: {registration}"
    );
    let admin_token = registration["data"]["token"]
        .as_str()
        .unwrap_or_else(|| panic!("isolated registration returned no JWT: {registration}"))
        .to_string();

    IsolatedApp {
        base_url,
        admin_token,
        task,
    }
}

pub(crate) async fn spawn_app() -> String {
    // Load .env
    dotenv().ok();

    // 0. Check E2E_BASE_URL environment variable first
    if let Ok(base_url) = env::var("E2E_BASE_URL") {
        println!("TEST: Using E2E_BASE_URL from env: {}", base_url);
        wait_for_server(&base_url).await;
        return base_url;
    }

    let handle = SERVER_HANDLE.get_or_init(|| {
        // 1. No implicit localhost reuse. A stale developer/CI server may have
        // different auth state or a different database and makes the black-box
        // suite non-deterministic. E2E_BASE_URL above is the explicit opt-in
        // for testing an externally managed server.
        //
        // Give the spawned server its own database so the first registered
        // account is predictably the administrator for this test process.
        let db_path = std::env::temp_dir().join(format!(
            "burncloud-api-tests-{}.db",
            std::process::id()
        ));
        let normalized_db_path = db_path.to_string_lossy().replace('\\', "/");
        let database_url = format!("sqlite:///{}?mode=rwc", normalized_db_path);
        let _ = std::fs::remove_file(&db_path);
        std::env::set_var("BURNCLOUD_DATABASE_URL", &database_url);

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

        let target_dir = env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    root_dir.join(path)
                }
            })
            .unwrap_or_else(|| root_dir.join("target"));
        let binary_name = if cfg!(target_os = "windows") {
            "burncloud.exe"
        } else {
            "burncloud"
        };
        let binary_path = target_dir.join("debug").join(binary_name);

        if !binary_path.exists() {
            panic!(
                "The black-box API tests need the prebuilt server binary.\n\
                 Expected: {}\n\
                 Build it first:  cargo build -p burncloud --bin burncloud --no-default-features\n\
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
            .env("BURNCLOUD_DATABASE_URL", &database_url)
            .env("JWT_SECRET", TEST_JWT_SECRET)
            .env("BURNCLOUD_INTERNAL_SECRET", TEST_INTERNAL_SECRET)
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
    ensure_test_admin(&handle.base_url).await;

    handle.base_url.clone()
}

async fn ensure_test_admin(base_url: &str) -> String {
    ADMIN_JWT
        .get_or_init(|| async {
            let username = format!("burncloud-ci-admin-{}", std::process::id());
            let password = "burncloud-ci-admin-password";
            let client = Client::builder()
                .no_proxy()
                .build()
                .expect("test admin client");

            let response = client
                .post(format!("{base_url}/api/auth/register"))
                .json(&json!({
                    "username": username,
                    "password": password,
                    "email": format!("{username}@example.invalid")
                }))
                .send()
                .await
                .expect("register test admin");
            let status = response.status();
            let body: serde_json::Value = response
                .json()
                .await
                .expect("test admin registration must return JSON");

            assert!(
                status.is_success(),
                "test admin registration failed with {status}: {body}"
            );
            assert_eq!(
                body["data"]["roles"]
                    .as_array()
                    .map(|roles| roles.iter().any(|role| role.as_str() == Some("admin"))),
                Some(true),
                "isolated test database must promote the first real user to admin: {body}"
            );

            body["data"]["token"]
                .as_str()
                .unwrap_or_else(|| panic!("test admin registration returned no JWT: {body}"))
                .to_string()
        })
        .await
        .clone()
}

pub(crate) async fn admin_client(base_url: &str) -> TestClient {
    let token = ensure_test_admin(base_url).await;
    TestClient::new(base_url).with_token(&token)
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

#[allow(
    dead_code,
    reason = "integration-test fixture helper, not called by every test target"
)]
pub(crate) fn get_root_token() -> String {
    "sk-root-token-123456".to_string()
}

#[allow(
    dead_code,
    reason = "integration-test fixture helper, not called by every test target"
)]
pub(crate) fn get_demo_token() -> String {
    "sk-burncloud-demo".to_string()
}

#[allow(
    dead_code,
    reason = "integration-test fixture helper, not called by every test target"
)]
pub(crate) fn get_openai_config() -> Option<(String, String)> {
    dotenv().ok();
    let key = env::var("TEST_OPENAI_KEY").ok().filter(|k| !k.is_empty())?;
    let url =
        env::var("TEST_OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com".to_string());
    Some((key, url))
}

// Removed deprecated functions: get_base_url (sync), get_db_pool, seed_demo_data

/// Insert a price entry for a mock model so the router's preflight check passes.
#[allow(
    dead_code,
    reason = "integration-test fixture helper, not called by every test target"
)]
pub(crate) async fn insert_mock_price(model: &str) {
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
        reqwest::Client::new()
            .post(format!("{}/console/internal/prices/sync", base_url))
            .send()
            .await
            .ok();
    }
}
