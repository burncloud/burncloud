use headless_chrome::{Browser, LaunchOptions};
use burncloud_tests::TestClient;
use serde_json::json;
use crate::common;
use std::time::{Instant, Duration};

#[tokio::test]
async fn test_ui_channel_management() -> anyhow::Result<()> {
    let base_url = common::spawn_app().await;
    let admin_client = TestClient::new(&base_url);

    // 2. Seed Data
    let channel_name = format!("UI-Test-{}", uuid::Uuid::new_v4());
    let body = json!({
        "type": 1,
        "key": "sk-test-ui",
        "name": channel_name,
        "base_url": "https://api.openai.com",
        "models": "gpt-3.5-turbo",
        "group": "default",
        "weight": 10,
        "priority": 100
    });
    
    // We expect successful creation
    let res = admin_client.post("/console/api/channel", &body).await.expect("Failed to create channel");
    assert_eq!(res["success"], true);

    let url = base_url.clone();
    let target_name = channel_name.clone();

    tokio::task::spawn_blocking(move || {
        let browser = Browser::new(LaunchOptions {
            headless: true, // Set to false to see the browser popping up!
            ..Default::default()
        })?;

        let tab = browser.new_tab()?;
        tab.navigate_to(&url)?;
        tab.wait_for_element("body")?;

        // 4. Click Sidebar "Channels"
        // Wait for hydration and sidebar render
        // Dioxus hydration might take a moment
        std::thread::sleep(Duration::from_secs(1));

        println!("Looking for sidebar link...");
        let links = tab.find_elements("a")?;
        println!("Found {} links on page.", links.len());
        for l in &links {
             if let Ok(Some(href)) = l.get_attribute_value("href") {
                 println!("Link href: {}", href);
             }
        }

        // CSS selector for anchor with href='/channels'
        // Note: Dioxus might use client-side routing, but href attribute is usually present
        let link = tab.wait_for_element("a[href*='channels']")?; 
        link.click()?;

        // 5. Verify Data
        println!("Checking for text: {}", target_name);
        
        let start = Instant::now();
        let mut found = false;
        while start.elapsed() < Duration::from_secs(5) {
            let result = tab.evaluate("document.body.innerText", false)?;
            
            // Handle JSON value extraction safely
            if let Some(value) = result.value {
                if let Some(text) = value.as_str() {
                    if text.contains(&target_name) {
                        found = true;
                        break;
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        
        if !found {
            // Print body text for debugging
            let result = tab.evaluate("document.body.innerText", false)?;
            let text = result.value.map(|v| v.to_string()).unwrap_or_default();
            panic!("Timeout waiting for '{}'. Page text:\n{}", target_name, text);
        }
        
        println!("Success: Found channel '{}' in UI table.", target_name);

        Ok::<(), anyhow::Error>(())
    }).await??;

    Ok(())
}
