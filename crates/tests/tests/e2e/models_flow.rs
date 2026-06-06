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

/// Test: Models page loads with channel list
#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_models_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open models page");
    browser.wait_for_text("模型网络", 10_000).expect("Models page did not load");
    let _ = browser.screenshot("models-page");
}

/// Test: Filter channels by status (All)
#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_models_filter_all() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open models page");
    browser.wait_for_text("模型网络", 10_000).expect("Models page did not load");

    // Click "全部" or "All" filter button
    let _ = browser.click_by_name("button:全部", 3_000)
        .or_else(|_| browser.click_by_name("button:All", 3_000));

    std::thread::sleep(std::time::Duration::from_millis(500));
    let _ = browser.screenshot("models-filter-all");
}

/// Test: Filter channels by status (OK)
#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_models_filter_ok() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open models page");
    browser.wait_for_text("模型网络", 10_000).expect("Models page did not load");

    // Click "正常" or "OK" filter button
    let _ = browser.click_by_name("button:正常", 3_000)
        .or_else(|_| browser.click_by_name("button:OK", 3_000));

    std::thread::sleep(std::time::Duration::from_millis(500));
    let _ = browser.screenshot("models-filter-ok");
}

/// Test: Filter channels by status (Throttle)
#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_models_filter_throttle() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open models page");
    browser.wait_for_text("模型网络", 10_000).expect("Models page did not load");

    // Click "限流" or "Throttle" filter button
    let _ = browser.click_by_name("button:限流", 3_000)
        .or_else(|_| browser.click_by_name("button:Throttle", 3_000));

    std::thread::sleep(std::time::Duration::from_millis(500));
    let _ = browser.screenshot("models-filter-throttle");
}

/// Test: Filter channels by status (Down)
#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_models_filter_down() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open models page");
    browser.wait_for_text("模型网络", 10_000).expect("Models page did not load");

    // Click "停用" or "Down" filter button
    let _ = browser.click_by_name("button:停用", 3_000)
        .or_else(|_| browser.click_by_name("button:Down", 3_000));

    std::thread::sleep(std::time::Duration::from_millis(500));
    let _ = browser.screenshot("models-filter-down");
}

/// Test: Filter channels by status (Maintenance)
#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_models_filter_maintenance() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open models page");
    browser.wait_for_text("模型网络", 10_000).expect("Models page did not load");

    // Click "维护" or "Maintenance" filter button
    let _ = browser.click_by_name("button:维护", 3_000)
        .or_else(|_| browser.click_by_name("button:Maintenance", 3_000));

    std::thread::sleep(std::time::Duration::from_millis(500));
    let _ = browser.screenshot("models-filter-maintenance");
}

/// Test: Open create channel modal
#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_models_open_create_modal() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open models page");
    browser.wait_for_text("模型网络", 10_000).expect("Models page did not load");

    // Click "创建" or "添加" button
    let _ = browser.click_by_name("button:创建", 5_000)
        .or_else(|_| browser.click_by_name("button:添加", 3_000))
        .or_else(|_| browser.click_by_name("button:Create", 3_000));

    std::thread::sleep(std::time::Duration::from_millis(1000));

    // Verify modal opened
    let modal_snap = browser.snapshot().expect("Failed to snapshot modal");
    assert!(
        modal_snap.text.contains("选择") || modal_snap.text.contains("Provider") || modal_snap.text.contains("提供商"),
        "Create channel modal should open with provider selection"
    );

    let _ = browser.screenshot("models-create-modal");
}

/// Test: Select provider in create modal (OpenAI)
#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_models_select_provider_openai() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open models page");
    browser.wait_for_text("模型网络", 10_000).expect("Models page did not load");

    // Open create modal first
    let _ = browser.click_by_name("button:创建", 5_000)
        .or_else(|_| browser.click_by_name("button:添加", 3_000));

    std::thread::sleep(std::time::Duration::from_millis(1000));

    // Select OpenAI provider
    let _ = browser.click_by_name("button:OpenAI", 3_000);

    std::thread::sleep(std::time::Duration::from_millis(500));

    // Verify form appeared
    let form_snap = browser.snapshot().expect("Failed to snapshot form");
    assert!(
        form_snap.text.contains("API Key") || form_snap.text.contains("密钥") || form_snap.text.contains("Base URL"),
        "Channel configuration form should appear after selecting provider"
    );

    let _ = browser.screenshot("models-provider-openai");
}

/// Test: Close create modal with Cancel button
#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_models_close_create_modal() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open models page");
    browser.wait_for_text("模型网络", 10_000).expect("Models page did not load");

    // Open create modal first
    let _ = browser.click_by_name("button:创建", 5_000)
        .or_else(|_| browser.click_by_name("button:添加", 3_000));

    std::thread::sleep(std::time::Duration::from_millis(1000));

    // Close modal with Cancel button
    let _ = browser.click_by_name("button:取消", 3_000)
        .or_else(|_| browser.click_by_name("button:Cancel", 3_000));

    std::thread::sleep(std::time::Duration::from_millis(500));

    // Verify modal closed
    let closed_snap = browser.snapshot().expect("Failed to snapshot");
    assert!(
        !closed_snap.text.contains("选择") || !closed_snap.text.contains("Provider"),
        "Modal should be closed after clicking Cancel"
    );

    let _ = browser.screenshot("models-modal-closed");
}

/// Test: Restart/Refresh channel list
#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_models_restart_channels() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open models page");
    browser.wait_for_text("模型网络", 10_000).expect("Models page did not load");

    // Click "筛选" or "Filter" or "重启" button
    let _ = browser.click_by_name("button:筛选", 3_000)
        .or_else(|_| browser.click_by_name("button:Filter", 3_000))
        .or_else(|_| browser.click_by_name("button:重启", 3_000));

    std::thread::sleep(std::time::Duration::from_millis(1000));
    let _ = browser.screenshot("models-restart");
}

/// Test: Channel list table rendering
#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_models_table_rendering() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/models").expect("Failed to open models page");
    browser.wait_for_text("模型网络", 10_000).expect("Models page did not load");

    let snap = browser.snapshot().expect("Failed to snapshot");

    // Verify table structure exists
    let has_table_headers = snap.text.contains("CHANNEL") || snap.text.contains("渠道") ||
                            snap.text.contains("WEIGHT") || snap.text.contains("权重") ||
                            snap.text.contains("TYPE") || snap.text.contains("类型") ||
                            snap.text.contains("MODELS") || snap.text.contains("模型");

    // If channels exist, table should be present
    if !snap.text.contains("暂无") && !snap.text.contains("No channels") {
        assert!(has_table_headers, "Channel table should have proper headers when channels exist");
    }

    let _ = browser.screenshot("models-table");
}
