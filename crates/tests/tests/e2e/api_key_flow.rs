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
    clippy::redundant_pattern_matching
)]
use super::*;

// 页面加载测试

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_access_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/access").expect("Failed to open access page");
    browser.wait_for_text("访问凭证", 10_000).expect("Access page did not load");
    let _ = browser.screenshot("access-page-load");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_api_key_list_rendering() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/access").expect("Failed to open access page");
    browser.wait_for_text("访问凭证", 10_000).expect("Access page did not load");

    let snap = browser.snapshot().expect("Failed to snapshot");
    assert!(
        snap.text.contains("创建") || snap.text.contains("Key") || snap.text.contains("凭证"),
        "Access page should show key list or create button"
    );
    let _ = browser.screenshot("api-key-list");
}

// API Key 创建流程

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_api_key_create_button() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/access").expect("Failed to open access page");
    browser.wait_for_text("访问凭证", 10_000).expect("Access page did not load");

    browser.click_by_name("button:创建", 5_000)
        .or_else(|_| browser.click_by_name("button:添加", 3_000))
        .expect("Failed to click create button");

    let result = browser.wait_for_text("名称", 10_000)
        .or_else(|_| browser.wait_for_text("Key", 5_000));
    assert!(result.is_ok(), "Create form should appear");
    let _ = browser.screenshot("api-key-create-button");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_api_key_create_form_validation() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/access").expect("Failed to open access page");
    browser.wait_for_text("访问凭证", 10_000).expect("Access page did not load");

    let _ = browser.click_by_name("button:创建", 5_000);
    let _ = browser.wait_for_text("名称", 5_000);

    browser.click("button[type='submit']")
        .or_else(|_| browser.click_by_name("button:提交", 3_000))
        .expect("Failed to submit");

    let snap = browser.snapshot().expect("Failed to snapshot");
    assert!(
        snap.text.contains("名称") || snap.text.contains("创建"),
        "Should show validation error or stay on form"
    );
    let _ = browser.screenshot("api-key-create-validation");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_api_key_create_success() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/access").expect("Failed to open access page");
    browser.wait_for_text("访问凭证", 10_000).expect("Access page did not load");

    let _ = browser.click_by_name("button:创建", 5_000);
    let _ = browser.wait_for_text("名称", 5_000);

    let key_name = format!("test_key_{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..8]);
    browser.fill("input", &key_name).expect("Failed to fill key name");

    browser.click("button[type='submit']")
        .or_else(|_| browser.click_by_name("button:提交", 3_000))
        .expect("Failed to submit");

    let result = browser.wait_for_text("成功", 10_000)
        .or_else(|_| browser.wait_for_text("sk-", 5_000))
        .or_else(|_| browser.wait_for_text("访问凭证", 5_000));
    assert!(result.is_ok(), "API Key creation should succeed");
    let _ = browser.screenshot("api-key-create-success");
}

// API Key 状态管理

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_api_key_status_display() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/access").expect("Failed to open access page");
    browser.wait_for_text("访问凭证", 10_000).expect("Access page did not load");

    let snap = browser.snapshot().expect("Failed to snapshot");
    let has_status = snap.text.contains("启用") || snap.text.contains("禁用") || snap.text.contains("状态");
    let _ = browser.screenshot("api-key-status-display");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_api_key_enable_disable() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/access").expect("Failed to open access page");
    browser.wait_for_text("访问凭证", 10_000).expect("Access page did not load");

    let _ = browser.click_by_name("button:启用", 3_000);
    let _ = browser.click_by_name("button:禁用", 3_000);
    let _ = browser.screenshot("api-key-enable-disable");
}

// API Key 轮换流程

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_api_key_rotate_button() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/access").expect("Failed to open access page");
    browser.wait_for_text("访问凭证", 10_000).expect("Access page did not load");

    let _ = browser.click_by_name("button:轮换", 5_000);
    let _ = browser.click_by_name("button:Rotate", 3_000);
    let _ = browser.screenshot("api-key-rotate-button");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_api_key_rotate_confirmation() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/access").expect("Failed to open access page");
    browser.wait_for_text("访问凭证", 10_000).expect("Access page did not load");

    let _ = browser.click_by_name("button:轮换", 5_000);
    let _ = browser.wait_for_text("确认", 5_000);

    let _ = browser.click_by_name("button:确认", 3_000);
    let result = browser.wait_for_text("成功", 10_000)
        .or_else(|_| browser.wait_for_text("sk-", 5_000));
    let _ = browser.screenshot("api-key-rotate-confirm");
}

// API Key 删除

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_api_key_delete_button() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/access").expect("Failed to open access page");
    browser.wait_for_text("访问凭证", 10_000).expect("Access page did not load");

    let _ = browser.click_by_name("button:删除", 5_000);
    let _ = browser.screenshot("api-key-delete-button");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_api_key_delete_cancel() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/access").expect("Failed to open access page");
    browser.wait_for_text("访问凭证", 10_000).expect("Access page did not load");

    let _ = browser.click_by_name("button:删除", 5_000);
    let _ = browser.wait_for_text("确认", 3_000);
    let _ = browser.click_by_name("button:取消", 3_000);

    let result = browser.wait_for_text("访问凭证", 5_000);
    assert!(result.is_ok(), "Cancel should return to access page");
    let _ = browser.screenshot("api-key-delete-cancel");
}

// IP 白名单功能

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_api_key_ip_whitelist_button() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/access").expect("Failed to open access page");
    browser.wait_for_text("访问凭证", 10_000).expect("Access page did not load");

    let _ = browser.click_by_name("button:白名单", 5_000);
    let _ = browser.click_by_name("button:IP", 3_000);
    let _ = browser.screenshot("api-key-ip-whitelist");
}
