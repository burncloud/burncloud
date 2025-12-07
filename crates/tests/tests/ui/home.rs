use headless_chrome::{Browser, LaunchOptions};
use crate::common;

#[tokio::test]
async fn test_homepage_load() -> anyhow::Result<()> {
    // 1. Start Server (Ensure it's running)
    let base_url = common::spawn_app().await;
    println!("UI Test: Visiting {}", base_url);

    let url = base_url.clone();
    
    // 2. Run Browser Logic in Blocking Thread
    tokio::task::spawn_blocking(move || {
        let browser = Browser::new(LaunchOptions {
            headless: true,
            ..Default::default()
        })?;

        let tab = browser.new_tab()?;
        
        // 3. Visit
        tab.navigate_to(&url)?;
        
        // Wait for Dioxus to hydrate (look for a specific element usually)
        // If it's SSR, body is immediate. If LiveView/CSR, wait for #main or something.
        // Let's wait for "body" for now.
        tab.wait_for_element("body")?;

        // 4. Assert Title
        let title = tab.get_title()?;
        println!("Page Title: {}", title);
        
        // Dioxus default title might be "Index" or defined in main.rs
        // Let's verify it matches what we expect.
        // Assuming title contains "BurnCloud"
        assert!(title.contains("BurnCloud") || title.contains("Index"), "Unexpected title: {}", title);
        
        Ok::<(), anyhow::Error>(())
    }).await??;

    Ok(())
}
