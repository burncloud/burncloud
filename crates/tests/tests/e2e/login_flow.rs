#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
use super::*;

#[tokio::test]
async fn test_login_success() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (browser, _) = login_browser(&base_url).await;
    // login_browser already verifies dashboard loads
    let _ = browser.screenshot("login-success");
}

#[tokio::test]
async fn test_login_invalid_credentials() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);

    // Open login page
    browser.open("/login").expect("Failed to open login page");
    browser
        .wait_for_text("登录", 10_000)
        .expect("Login page did not load");

    // Fill with wrong credentials
    browser
        .fill("input:nth-of-type(1)", "nonexistent_user_xyz")
        .expect("Failed to fill username");
    browser
        .fill("input[type='password']", "wrong_password")
        .expect("Failed to fill password");

    // Submit
    browser.click("button").expect("Failed to click login");

    // Wait for error indication or staying on login page
    let result = browser
        .wait_for_text("错误", 5_000)
        .or_else(|_| browser.wait_for_text("error", 3_000));

    // Either we see an error message, or we're still on the login page
    assert!(
        result.is_ok()
            || browser
                .snapshot()
                .map(|s| s.text.contains("登录"))
                .unwrap_or(false),
        "Expected error message or staying on login page after invalid credentials"
    );

    let _ = browser.screenshot("login-invalid-creds");
}

/// Verify the dashboard loads and shows the sidebar navigation.
/// TODO: Add logout test once logout button is implemented in the UI.
#[tokio::test]
async fn test_dashboard_after_login() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    // Verify sidebar navigation items are present
    let snap = browser.snapshot().expect("Failed to snapshot");
    assert!(
        snap.text.contains("仪表盘") && snap.text.contains("模型网络"),
        "Dashboard sidebar not fully loaded. Snapshot: {}",
        snap.text
    );

    let _ = browser.screenshot("dashboard-after-login");
}
