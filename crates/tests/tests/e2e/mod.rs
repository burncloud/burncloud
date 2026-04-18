#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
pub mod agent_browser;
pub mod console_pages;
pub mod design_tokens;
pub mod guest_pages;
pub mod login_flow;
pub mod navigation;
pub mod settings_interactions;

use agent_browser::{is_agent_browser_available, AgentBrowser};
use reqwest::Client;
use serde_json::json;

// Re-use existing spawn_app from common module
use crate::common;

/// Ensure agent-browser is installed. Skips tests if not available.
pub fn setup_browser() -> Option<()> {
    if !is_agent_browser_available() {
        eprintln!("SKIP: agent-browser not installed. Install with: npm install -g agent-browser");
        return None;
    }
    Some(())
}

/// Create a test user via API and return the JWT token.
/// Uses UUID-based username to avoid conflicts between parallel tests.
pub async fn create_test_user(base_url: &str) -> (String, String) {
    let username = format!(
        "e2e_test_{}",
        &uuid::Uuid::new_v4().to_string().replace('-', "")[..8]
    );
    let password = "test123456".to_string();

    let client = Client::new();
    let resp = client
        .post(format!("{}/api/auth/register", base_url))
        .json(&json!({
            "username": username,
            "email": format!("{}@test.burncloud.dev", username),
            "password": password,
        }))
        .send()
        .await
        .expect("Failed to create test user");

    let body: serde_json::Value = resp
        .json()
        .await
        .expect("Failed to parse register response");

    if body["success"].as_bool() != Some(true) {
        // Try login instead (user might already exist from a previous run)
        let login_resp = client
            .post(format!("{}/api/auth/login", base_url))
            .json(&json!({
                "username": username,
                "password": password,
            }))
            .send()
            .await
            .expect("Failed to login test user");
        let login_body: serde_json::Value = login_resp
            .json()
            .await
            .expect("Failed to parse login response");
        let token = login_body["data"]["token"]
            .as_str()
            .expect("No token in login response")
            .to_string();
        return (username, token);
    }

    let token = body["data"]["token"]
        .as_str()
        .expect("No token in register response")
        .to_string();

    (username, token)
}

/// Helper: test that a guest page loads with expected text.
/// Opens the page, waits for the expected text, takes a screenshot.
pub fn test_page_loads(base_url: &str, path: &str, expected_text: &str, screenshot_name: &str) {
    let mut browser = AgentBrowser::new(base_url);
    browser.open(path).expect("Failed to open page");
    browser
        .wait_for_text(expected_text, 10_000)
        .unwrap_or_else(|e| {
            let _ = browser.screenshot(&format!("FAIL-{}", screenshot_name));
            panic!(
                "Page {} failed to load expected text '{}': {}",
                path, expected_text, e
            );
        });
    let _ = browser.screenshot(screenshot_name);
}

/// Login via browser and return the browser instance.
/// Creates a test user, logs in via the browser, returns the browser for further use.
pub async fn login_browser(base_url: &str) -> (AgentBrowser, String) {
    let (username, _token) = create_test_user(base_url).await;
    let mut browser = AgentBrowser::new(base_url);

    // Navigate to login page
    browser.open("/login").expect("Failed to open login page");
    browser
        .wait_for_text("登录", 10_000)
        .expect("Login page did not load");

    // Fill login form using CSS selectors
    // First input is username, password input for password
    browser
        .fill("input:nth-of-type(1)", &username)
        .expect("Failed to fill username");
    browser
        .fill("input[type='password']", "test123456")
        .expect("Failed to fill password");

    // Click the login button
    browser.click("button").expect("Failed to click login");

    // Wait for dashboard sidebar to confirm login
    browser.wait_for_text("仪表盘", 15_000).unwrap_or_else(|e| {
        let _ = browser.screenshot("FAIL-login");
        panic!("Login failed or dashboard did not load: {}", e);
    });

    (browser, username)
}
