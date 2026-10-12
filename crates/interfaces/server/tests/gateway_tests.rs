#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::let_and_return,
    clippy::disallowed_types,
    clippy::panic_in_result_fn,
    reason = "integration test: assertions are the failure signal, and clippy.toml's allow-panic-in-tests does not recognise #[tokio::test]"
)]

//! Unified gateway end-to-end coverage: the dashboard shell, the administrator boundary in
//! front of the console API, and the data-plane credential boundary.
//!
//! The port is kernel-assigned (`bind(0)` plus `local_addr`) so this binary can run beside the
//! other server tests, and readiness is established by polling `/health` instead of sleeping
//! for a fixed duration. The shape follows the `spawn_server` helper that
//! `tests/security_invariants.rs` already uses in this directory: `start_server` binds
//! internally and returns `()`, so a test can only learn the port if it owns the listener.

mod test_utils;

use burncloud_database::Database;
use burncloud_server::InternalSecret;
use burncloud_service_user::{JwtSecret, UserService};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

const READY_TIMEOUT: Duration = Duration::from_secs(20);
const READY_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// `create_app` starts the background price sync unless this is set, and these tests do not
/// need it.
fn configure_gateway_env() {
    std::env::set_var("SKIP_INITIAL_PRICE_SYNC", "1");
}

/// Build the application through the public constructor and serve it on a kernel-assigned
/// port, which is what removes the hardcoded-port race the previous revision carried.
async fn spawn_gateway(
    db: Arc<Database>,
    jwt_secret: JwtSecret,
    internal_secret: InternalSecret,
    enable_liveview: bool,
) -> anyhow::Result<String> {
    let app =
        burncloud_server::create_app(db, enable_liveview, jwt_secret, internal_secret).await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .unwrap_or_else(|e| panic!("gateway test server failed: {e}"));
    });
    Ok(format!("http://{addr}"))
}

/// Poll the unauthenticated liveness probe rather than sleeping a fixed duration.
async fn await_gateway_ready(client: &Client, base: &str) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + READY_TIMEOUT;
    loop {
        if let Ok(response) = client.get(format!("{base}/health")).send().await {
            if response.status() == StatusCode::OK {
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("the unified gateway at {base} never became ready");
        }
        tokio::time::sleep(READY_POLL_INTERVAL).await;
    }
}

#[tokio::test]
async fn test_unified_gateway_routing() -> anyhow::Result<()> {
    configure_gateway_env();
    let db = test_utils::make_isolated_db().await;
    let jwt_secret = test_utils::test_jwt_secret();

    // `UserService::register_user` promotes the first registered real user to `admin`, and
    // `auth::admin_middleware` reads that role back from the database rather than trusting
    // the token.
    let service = UserService::new(jwt_secret.clone());
    let admin_id = service
        .register_user(&db, "gateway-admin", "test-password", None)
        .await?;
    let admin_jwt = service.generate_token(&admin_id, "gateway-admin")?.token;

    // The dashboard shell is mounted only when LiveView is enabled, so `/` is asserted on
    // that side of the switch, which is the coverage the previous revision intended.
    let base = spawn_gateway(
        db,
        jwt_secret,
        test_utils::test_internal_secret(),
        true,
    )
    .await?;
    let client = Client::new();
    await_gateway_ready(&client, &base).await?;

    // 1. The LiveView dashboard shell is served without a credential.
    let shell = client.get(format!("{base}/")).send().await?;
    assert_eq!(shell.status(), StatusCode::OK);
    let shell_body = shell.text().await?;
    assert!(
        shell_body.to_ascii_lowercase().contains("<!doctype html"),
        "the dashboard must answer with an HTML document, got: {shell_body}"
    );
    assert!(
        shell_body.contains("BurnCloud"),
        "the dashboard shell must carry the application title, got: {shell_body}"
    );

    // 2. `/console/api/channel` is mounted on the admin router inside the authenticated
    //    router, so an anonymous caller cannot reach it. (The previous revision expected
    //    `200` here, which the current mounting makes impossible; asserting `401` requires
    //    the boundary to be genuinely enforced.)
    let anonymous = client
        .get(format!("{base}/console/api/channel"))
        .send()
        .await?;
    assert_eq!(
        anonymous.status(),
        StatusCode::UNAUTHORIZED,
        "an anonymous call must not reach an administrator-only management surface"
    );

    // 3. The same surface stays functional for an administrator and answers with JSON.
    let authorized = client
        .get(format!("{base}/console/api/channel"))
        .bearer_auth(&admin_jwt)
        .send()
        .await?;
    assert_eq!(
        authorized.status(),
        StatusCode::OK,
        "administrator access to the management surface must remain functional"
    );
    let authorized_body = authorized.text().await?;
    serde_json::from_str::<Value>(&authorized_body)
        .expect("the channel listing must be JSON");

    // 4. The data plane stays closed to a caller with no credential, and to one whose
    //    credential cannot be validated.
    let demand = serde_json::json!({"model": "gateway-boundary-model", "messages": []});
    let without_credential = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&demand)
        .send()
        .await?;
    assert_eq!(
        without_credential.status(),
        StatusCode::UNAUTHORIZED,
        "the router must demand a credential on the LLM path"
    );

    let invalid_credential = client
        .post(format!("{base}/v1/chat/completions"))
        .bearer_auth("invalid-sk")
        .json(&demand)
        .send()
        .await?;
    assert_eq!(
        invalid_credential.status(),
        StatusCode::UNAUTHORIZED,
        "an unusable credential must not open the LLM path"
    );

    Ok(())
}
