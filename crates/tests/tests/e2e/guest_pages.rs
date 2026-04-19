#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
use super::*;

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_home_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    // Home page shows heading "One Interface. Every Intelligence."
    test_page_loads(&base_url, "/", "One Interface", "home");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_login_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    // Login page shows "登录" button
    test_page_loads(&base_url, "/login", "登录", "login");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_register_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    // Register page shows "开始体验" button
    test_page_loads(&base_url, "/register", "开始体验", "register");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_login_navigation_from_home() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);
    browser.open("/").expect("Failed to open home page");
    browser
        .wait_for_text("Sign In", 10_000)
        .expect("Home page did not load");

    // Click Sign In link
    browser
        .click_by_name("link:Sign In", 5_000)
        .expect("Failed to click Sign In");
    browser
        .wait_for_text("请输入密码", 10_000)
        .expect("Did not navigate to login page");

    let _ = browser.screenshot("home-to-login-nav");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_register_navigation_from_login() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);
    browser.open("/login").expect("Failed to open login page");
    browser
        .wait_for_text("登录", 10_000)
        .expect("Login page did not load");

    // Click register link ("还没有账号? 立即注册")
    browser
        .click_by_name("link:注册", 5_000)
        .expect("Failed to click register link");
    browser
        .wait_for_text("开始体验", 10_000)
        .expect("Did not navigate to register page");

    let _ = browser.screenshot("login-to-register-nav");
}
