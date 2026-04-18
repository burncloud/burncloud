#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
use super::*;

#[tokio::test]
async fn test_dashboard_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;
    // Already on dashboard after login, verify heading
    browser
        .wait_for_text("企业控制台", 10_000)
        .expect("Dashboard heading not found");
    let _ = browser.screenshot("console-dashboard");
}

#[tokio::test]
async fn test_models_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;
    browser
        .open("/console/models")
        .expect("Failed to open models");
    browser
        .wait_for_text("模型网络", 10_000)
        .expect("Models page did not load");
    let _ = browser.screenshot("console-models");
}

#[tokio::test]
async fn test_access_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;
    browser
        .open("/console/access")
        .expect("Failed to open access");
    browser
        .wait_for_text("访问凭证", 10_000)
        .expect("Access page did not load");
    let _ = browser.screenshot("console-access");
}

#[tokio::test]
async fn test_deploy_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;
    browser
        .open("/console/deploy")
        .expect("Failed to open deploy");
    browser
        .wait_for_text("Model Deployment", 10_000)
        .expect("Deploy page did not load");
    let _ = browser.screenshot("console-deploy");
}

#[tokio::test]
async fn test_monitor_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;
    browser
        .open("/console/monitor")
        .expect("Failed to open monitor");
    browser
        .wait_for_text("风控雷达", 10_000)
        .expect("Monitor page did not load");
    let _ = browser.screenshot("console-monitor");
}

#[tokio::test]
async fn test_logs_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;
    browser.open("/console/logs").expect("Failed to open logs");
    browser
        .wait_for_text("Logs", 10_000)
        .expect("Logs page did not load");
    let _ = browser.screenshot("console-logs");
}

#[tokio::test]
async fn test_users_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;
    browser
        .open("/console/users")
        .expect("Failed to open users");
    browser
        .wait_for_text("客户列表", 10_000)
        .expect("Users page did not load");
    let _ = browser.screenshot("console-users");
}

#[tokio::test]
async fn test_settings_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;
    browser
        .open("/console/settings")
        .expect("Failed to open settings");
    browser
        .wait_for_text("系统设置", 10_000)
        .expect("Settings page did not load");
    let _ = browser.screenshot("console-settings");
}

#[tokio::test]
async fn test_billing_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;
    browser
        .open("/console/finance")
        .expect("Failed to open finance");
    browser
        .wait_for_text("财务中心", 10_000)
        .expect("Finance page did not load");
    let _ = browser.screenshot("console-finance");
}

#[tokio::test]
async fn test_connect_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;
    browser
        .open("/console/connect")
        .expect("Failed to open connect");
    browser
        .wait_for_text("算力互联", 10_000)
        .expect("Connect page did not load");
    let _ = browser.screenshot("console-connect");
}

#[tokio::test]
async fn test_playground_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;
    browser
        .open("/console/playground")
        .expect("Failed to open playground");
    browser
        .wait_for_text("Playground", 10_000)
        .expect("Playground page did not load");
    let _ = browser.screenshot("console-playground");
}
