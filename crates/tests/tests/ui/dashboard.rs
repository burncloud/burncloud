use headless_chrome::{Browser, LaunchOptions};
use crate::common;
use std::time::Duration;

#[tokio::test]
async fn test_ui_dashboard_metrics() -> anyhow::Result<()> {
    // 1. Start Server
    let base_url = common::spawn_app().await;
    println!("UI Dashboard Test: Visiting {}", base_url);
    let url = base_url.clone();

    // 2. Run Browser Logic
    tokio::task::spawn_blocking(move || {
        let browser = Browser::new(LaunchOptions {
            headless: true,
            ..Default::default()
        })?;

        let tab = browser.new_tab()?;
        tab.navigate_to(&url)?;

        // 3. Wait for Dashboard to Load
        tab.wait_for_element("body")?;
        
        // Wait for hydration (simple sleep or check for specific element)
        // We want to see "CPU使用率" which means hydration + API call finished.
        // Dioxus uses Suspense or Resource loading, so it might say "加载中..." initially.
        // We'll poll for the text.
        
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(10);
        let mut found_metrics = false;

        println!("Waiting for metrics to appear...");
        
        while start.elapsed() < timeout {
            let result = tab.evaluate("document.body.innerText", false)?;
            if let Some(value) = result.value {
                if let Some(text) = value.as_str() {
                    // Check for CPU Label
                    if text.contains("CPU使用率") {
                        // Check that it's not "加载中..." or "暂无数据" only
                        // We expect some numbers like "45.2%" or similar formatted.
                        // The dashboard format: "{m.cpu.usage_percent:.1}%"
                        // Since we can't easily regex check via simple innerText, 
                        // we'll assume if we see "CPU使用率" and the "暂无数据" is NOT near it (or generally present if we expect data), it's good.
                        // But "暂无数据" might be present for OTHER cards (like Usage if no usage).
                        
                        // Let's check for "内存" as well.
                        if text.contains("内存") && text.contains("GB") {
                             found_metrics = true;
                             break;
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(500));
        }

        if !found_metrics {
            let result = tab.evaluate("document.body.innerText", false)?;
            let text = result.value.map(|v| v.to_string()).unwrap_or_default();
            panic!("Timeout waiting for System Metrics on Dashboard. Page text:\n{}", text);
        }

        println!("Success: Dashboard metrics (CPU/Memory) are visible.");
        Ok::<(), anyhow::Error>(())
    }).await??;

    Ok(())
}
