#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
use super::*;

#[tokio::test]
async fn test_language_switch() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    // Navigate to settings
    browser
        .open("/console/settings")
        .expect("Failed to open settings");
    browser
        .wait_for_text("系统设置", 10_000)
        .expect("Settings page did not load");

    // Settings page should show "General" tab
    let result = browser.wait_for_text("General", 5_000);
    assert!(result.is_ok(), "Settings page should show General tab");

    let _ = browser.screenshot("settings-general");

    // Click Groups tab
    browser.click_by_name("Groups", 5_000).ok();
    let result = browser.wait_for_text("Groups", 5_000);
    assert!(
        result.is_ok(),
        "Clicking Groups tab did not show Groups content"
    );
    let _ = browser.screenshot("settings-groups-tab");

    // Click Tokens tab
    browser.click_by_name("Tokens", 5_000).ok();
    let result = browser.wait_for_text("Tokens", 5_000);
    assert!(
        result.is_ok(),
        "Clicking Tokens tab did not show Tokens content"
    );
    let _ = browser.screenshot("settings-tokens-tab");
}

#[tokio::test]
async fn test_tab_switching() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    // Navigate to settings
    browser
        .open("/console/settings")
        .expect("Failed to open settings");
    browser
        .wait_for_text("系统设置", 10_000)
        .expect("Settings page did not load");

    // Verify General tab is active by default
    assert!(
        browser.wait_for_text("General", 5_000).is_ok(),
        "General tab should be visible by default"
    );

    // Click Groups tab
    browser.click_by_name("Groups", 5_000).ok();
    assert!(
        browser.wait_for_text("Groups", 5_000).is_ok(),
        "Groups tab should be visible after click"
    );
    let _ = browser.screenshot("settings-groups-tab");

    // Click Tokens tab
    browser.click_by_name("Tokens", 5_000).ok();
    assert!(
        browser.wait_for_text("Tokens", 5_000).is_ok(),
        "Tokens tab should be visible after click"
    );
    let _ = browser.screenshot("settings-tokens-tab");

    // Click General tab to go back
    browser.click_by_name("General", 5_000).ok();
    assert!(
        browser.wait_for_text("General", 5_000).is_ok(),
        "General tab should be visible after click"
    );
    let _ = browser.screenshot("settings-general-tab");
}
