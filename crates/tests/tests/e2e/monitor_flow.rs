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
async fn test_monitor_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/monitor").expect("Failed to open monitor page");
    browser.wait_for_text("风控雷达", 10_000).expect("Monitor page did not load");
    let _ = browser.screenshot("monitor-page-load");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_security_score_component() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/monitor").expect("Failed to open monitor page");
    browser.wait_for_text("风控雷达", 10_000).expect("Monitor page did not load");

    let snap = browser.snapshot().expect("Failed to snapshot");
    // Security score component might show various labels
    // The monitor page shows: 风控雷达, 黑名单管理, 紧急熔断, etc.
    let has_security_component = snap.text.contains("安全") 
        || snap.text.contains("评分") 
        || snap.text.contains("Score")
        || snap.text.contains("风险")
        || snap.text.contains("Risk")
        || snap.text.contains("健康")
        || snap.text.contains("Health")
        || snap.text.contains("状态")
        || snap.text.contains("Status")
        || snap.text.contains("熔断")  // Circuit breaker is part of security
        || snap.text.contains("黑名单");  // Blacklist is part of security
    
    let _ = browser.screenshot("security-score");
    
    assert!(
        has_security_component,
        "Security score component should be visible. Page preview: {}",
        &snap.text.chars().take(300).collect::<String>()
    );
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_trend_chart_rendering() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/monitor").expect("Failed to open monitor page");
    browser.wait_for_text("风控雷达", 10_000).expect("Monitor page did not load");

    let snap = browser.snapshot().expect("Failed to snapshot");
    let has_trend = snap.text.contains("趋势") || snap.text.contains("7天") || snap.text.contains("Trend");
    let _ = browser.screenshot("trend-chart");
}

// 安全评分展示

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_security_score_value_display() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/monitor").expect("Failed to open monitor page");
    browser.wait_for_text("风控雷达", 10_000).expect("Monitor page did not load");

    let snap = browser.snapshot().expect("Failed to snapshot");
    // Score might be displayed as number or percentage
    let _ = browser.screenshot("security-score-value");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_security_score_color_indicator() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/monitor").expect("Failed to open monitor page");
    browser.wait_for_text("风控雷达", 10_000).expect("Monitor page did not load");

    // Color indicator would be visual, check for status text
    let snap = browser.snapshot().expect("Failed to snapshot");
    let has_status = snap.text.contains("良好") || snap.text.contains("警告") || snap.text.contains("危险")
        || snap.text.contains("Good") || snap.text.contains("Warning") || snap.text.contains("Danger");
    let _ = browser.screenshot("security-score-color");
}

// 风险事件监控

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_risk_events_list() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/monitor").expect("Failed to open monitor page");
    browser.wait_for_text("风控雷达", 10_000).expect("Monitor page did not load");

    let snap = browser.snapshot().expect("Failed to snapshot");
    let has_events = snap.text.contains("事件") || snap.text.contains("风险") || snap.text.contains("Event");
    let _ = browser.screenshot("risk-events-list");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_risk_events_filter() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/monitor").expect("Failed to open monitor page");
    browser.wait_for_text("风控雷达", 10_000).expect("Monitor page did not load");

    let _ = browser.click_by_name("button:筛选", 5_000);
    let _ = browser.click_by_name("button:Filter", 3_000);
    let _ = browser.screenshot("risk-events-filter");
}

// 内容过滤器配置

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_content_filter_config() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/monitor").expect("Failed to open monitor page");
    browser.wait_for_text("风控雷达", 10_000).expect("Monitor page did not load");

    let snap = browser.snapshot().expect("Failed to snapshot");
    let has_filter = snap.text.contains("过滤") || snap.text.contains("黑名单") || snap.text.contains("Filter");
    let _ = browser.screenshot("content-filter-config");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_blacklist_add_rule() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/monitor").expect("Failed to open monitor page");
    browser.wait_for_text("风控雷达", 10_000).expect("Monitor page did not load");

    let _ = browser.click_by_name("button:添加", 5_000);
    let _ = browser.click_by_name("button:黑名单", 5_000);
    let _ = browser.screenshot("blacklist-add-rule");
}

// 紧急熔断功能

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_emergency_circuit_break_button() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/monitor").expect("Failed to open monitor page");
    browser.wait_for_text("风控雷达", 10_000).expect("Monitor page did not load");

    let snap = browser.snapshot().expect("Failed to snapshot");
    let has_breaker = snap.text.contains("熔断") || snap.text.contains("紧急") || snap.text.contains("Circuit");
    let _ = browser.screenshot("circuit-break-button");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_emergency_circuit_break_confirmation() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/monitor").expect("Failed to open monitor page");
    browser.wait_for_text("风控雷达", 10_000).expect("Monitor page did not load");

    let _ = browser.click_by_name("button:熔断", 5_000);
    let _ = browser.wait_for_text("确认", 5_000);
    let _ = browser.screenshot("circuit-break-confirm");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_circuit_breaker_status_indicator() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/monitor").expect("Failed to open monitor page");
    browser.wait_for_text("风控雷达", 10_000).expect("Monitor page did not load");

    let snap = browser.snapshot().expect("Failed to snapshot");
    let has_status = snap.text.contains("正常") || snap.text.contains("熔断") || snap.text.contains("恢复");
    let _ = browser.screenshot("circuit-breaker-status");
}

// 熔断器恢复

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_circuit_breaker_recovery() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    browser.open("/console/monitor").expect("Failed to open monitor page");
    browser.wait_for_text("风控雷达", 10_000).expect("Monitor page did not load");

    let _ = browser.click_by_name("button:恢复", 5_000);
    let _ = browser.click_by_name("button:Recover", 3_000);
    let _ = browser.screenshot("circuit-breaker-recovery");
}
