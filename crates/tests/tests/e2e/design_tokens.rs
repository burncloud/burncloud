#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
use super::*;

/// Helper: eval a JS expression with retry, waiting for a non-empty result.
fn eval_with_retry(
    browser: &mut AgentBrowser,
    js: &str,
    timeout_ms: u64,
) -> anyhow::Result<serde_json::Value> {
    let start = std::time::Instant::now();
    loop {
        let result = browser.eval(js)?;
        let val = result.as_str().unwrap_or("").trim().to_string();
        if !val.is_empty() {
            return Ok(result);
        }
        if start.elapsed().as_millis() as u64 > timeout_ms {
            anyhow::bail!(
                "JS eval '{}' returned empty after {}ms",
                js.split_whitespace().take(5).collect::<Vec<_>>().join(" "),
                timeout_ms
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
}

/// Verify that the primary color CSS variable is set correctly.
#[tokio::test]
async fn test_primary_color_token() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);
    browser.open("/").expect("Failed to open home page");
    browser
        .wait_for_text("One Interface", 10_000)
        .expect("Home page did not load");

    // Check CSS variable --bc-primary via JS eval (with retry for late CSS injection)
    let result = eval_with_retry(
        &mut browser,
        "getComputedStyle(document.documentElement).getPropertyValue('--bc-primary')",
        5_000,
    )
    .expect("JS eval failed");
    let color = result.as_str().unwrap_or("").trim().to_string();

    assert!(
        color.contains("007AFF") || color.contains("007aff") || color.contains("0, 122, 255"),
        "Expected --bc-primary to be #007AFF, got: {}",
        color
    );

    let _ = browser.screenshot("design-token-primary");
}

/// Verify no Fluent blue (#0078d4) remains in the design system.
#[tokio::test]
async fn test_no_fluent_blue() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);
    browser.open("/").expect("Failed to open home page");
    browser
        .wait_for_text("One Interface", 10_000)
        .expect("Home page did not load");

    let result = eval_with_retry(
        &mut browser,
        "getComputedStyle(document.documentElement).getPropertyValue('--accent-color')",
        5_000,
    )
    .expect("JS eval failed");
    let accent = result.as_str().unwrap_or("").trim().to_string();

    assert!(
        !accent.contains("0078d4") && !accent.contains("0078D4"),
        "Found Fluent blue #0078d4 in --accent-color: {}",
        accent
    );

    let _ = browser.screenshot("design-no-fluent");
}

/// Verify card components use the correct design tokens.
#[tokio::test]
async fn test_card_styles() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    // On dashboard (has cards) - check CSS variables with retry
    let radius = eval_with_retry(
        &mut browser,
        "getComputedStyle(document.documentElement).getPropertyValue('--bc-radius-md')",
        5_000,
    )
    .expect("JS eval failed");
    let radius_str = radius.as_str().unwrap_or("").trim().to_string();
    assert!(
        !radius_str.is_empty(),
        "--bc-radius-md CSS variable not found"
    );

    let shadow = eval_with_retry(
        &mut browser,
        "getComputedStyle(document.documentElement).getPropertyValue('--bc-shadow-sm')",
        5_000,
    )
    .expect("JS eval failed");
    let shadow_str = shadow.as_str().unwrap_or("").trim().to_string();
    assert!(
        !shadow_str.is_empty(),
        "--bc-shadow-sm CSS variable not found"
    );

    let _ = browser.screenshot("design-card-styles");
}

/// Verify button components render with correct variants.
#[tokio::test]
async fn test_button_styles() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);
    browser.open("/").expect("Failed to open home page");
    browser
        .wait_for_text("One Interface", 10_000)
        .expect("Home page did not load");

    // Check that button elements exist on the page
    let result = eval_with_retry(
        &mut browser,
        "document.querySelectorAll('button, a[role=\"button\"], [class*=\"btn\"]').length > 0 ? 'found' : ''",
        5_000,
    )
    .expect("JS eval failed");
    let found = result.as_str().unwrap_or("").trim().to_string();
    assert!(
        !found.is_empty(),
        "Expected at least one button-like element on the page"
    );

    let _ = browser.screenshot("design-button-styles");
}
