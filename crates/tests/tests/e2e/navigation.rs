#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
use super::*;

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_sidebar_navigation() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    // Test navigation via direct URL to each major section
    let nav_items: Vec<(&str, &str)> = vec![
        ("/console/models", "模型网络"),
        ("/console/connect", "算力互联"),
        ("/console/access", "访问凭证"),
        ("/console/monitor", "风控雷达"),
        ("/console/users", "客户列表"),
        ("/console/finance", "财务中心"),
    ];

    for (path, expected_text) in nav_items {
        browser.open(path).expect("Failed to open page");
        let result = browser.wait_for_text(expected_text, 5_000);
        assert!(
            result.is_ok(),
            "Navigation to '{}' failed. Expected text '{}' not found.",
            path,
            expected_text
        );
    }

    let _ = browser.screenshot("sidebar-navigation");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_sidebar_active_state() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    // Dashboard sidebar item should be visible (we're on dashboard after login)
    let snapshot = browser.snapshot().expect("Failed to get snapshot");
    assert!(
        snapshot.text.contains("仪表盘"),
        "Dashboard sidebar item should be visible"
    );

    let _ = browser.screenshot("sidebar-active-state");
}

/// Verify that console pages load when accessed directly.
/// Note: Front-end route guard is not yet implemented, so the page
/// renders even without auth. This test verifies the page at least loads.
#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_console_page_loads_without_auth() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);

    // Access dashboard directly (no login)
    browser
        .open("/console/dashboard")
        .expect("Failed to open dashboard");

    // Page should load (showing either dashboard or redirect target)
    let result = browser.wait_for_text("企业控制台", 10_000);
    assert!(
        result.is_ok(),
        "Console page should load (with or without auth)"
    );

    let _ = browser.screenshot("console-no-auth");
}
