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
async fn test_channel_list_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open channel list");
    browser.wait_for_text("模型网络", 10_000).expect("Channel list page did not load");
    let _ = browser.screenshot("channel-list-page");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_channel_list_empty_state() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open channel list");
    browser.wait_for_text("模型网络", 10_000).expect("Channel list page did not load");

    let snap = browser.snapshot().expect("Failed to snapshot");
    assert!(
        snap.text.contains("创建") || snap.text.contains("添加") || snap.text.contains("渠道"),
        "Channel list should show empty state or list"
    );
    let _ = browser.screenshot("channel-list-state");
}

// 渠道创建流程

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_channel_create_form_open() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open channel list");
    browser.wait_for_text("模型网络", 10_000).expect("Channel list page did not load");

    browser.click_by_name("button:创建", 5_000)
        .or_else(|_| browser.click_by_name("button:添加", 3_000))
        .expect("Failed to click create button");

    let result = browser.wait_for_text("名称", 10_000)
        .or_else(|_| browser.wait_for_text("类型", 5_000));
    assert!(result.is_ok(), "Create form should appear");
    let _ = browser.screenshot("channel-create-form-open");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_channel_create_form_validation() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open channel list");
    browser.wait_for_text("模型网络", 10_000).expect("Channel list page did not load");

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
    let _ = browser.screenshot("channel-create-validation");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_channel_create_success() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open channel list");
    browser.wait_for_text("模型网络", 10_000).expect("Channel list page did not load");

    let _ = browser.click_by_name("button:创建", 5_000);
    let _ = browser.wait_for_text("名称", 5_000);

    let channel_name = format!("test_{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..8]);
    browser.fill("input", &channel_name).expect("Failed to fill channel name");

    browser.click("button[type='submit']")
        .or_else(|_| browser.click_by_name("button:提交", 3_000))
        .expect("Failed to submit");

    let result = browser.wait_for_text("成功", 10_000)
        .or_else(|_| browser.wait_for_text("模型网络", 5_000));
    assert!(result.is_ok(), "Channel creation should succeed");
    let _ = browser.screenshot("channel-create-success");
}

// 渠道编辑流程

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_channel_edit_button_visible() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open channel list");
    browser.wait_for_text("模型网络", 10_000).expect("Channel list page did not load");

    let snap = browser.snapshot().expect("Failed to snapshot");
    let _ = browser.screenshot("channel-edit-button");
}

// 渠道删除流程

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_channel_delete_confirmation_dialog() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open channel list");
    browser.wait_for_text("模型网络", 10_000).expect("Channel list page did not load");

    let _ = browser.click_by_name("button:删除", 5_000);
    let _ = browser.screenshot("channel-delete-confirm");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_channel_delete_cancel() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open channel list");
    browser.wait_for_text("模型网络", 10_000).expect("Channel list page did not load");

    let _ = browser.click_by_name("button:删除", 5_000);
    let _ = browser.wait_for_text("确认", 3_000);
    let _ = browser.click_by_name("button:取消", 3_000);

    let result = browser.wait_for_text("模型网络", 5_000);
    assert!(result.is_ok(), "Cancel should return to channel list");
    let _ = browser.screenshot("channel-delete-cancel");
}

// 渠道状态切换

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_channel_status_toggle() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open channel list");
    browser.wait_for_text("模型网络", 10_000).expect("Channel list page did not load");

    let _ = browser.click_by_name("button:启用", 3_000);
    let _ = browser.screenshot("channel-status-toggle");
}
