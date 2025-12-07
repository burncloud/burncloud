use headless_chrome::{Browser, LaunchOptions};
use crate::common;
use std::time::Duration;

#[tokio::test]
async fn test_login_and_redirect() -> anyhow::Result<()> {
    let base_url = common::spawn_app().await;
    let login_url = format!("{}/login", base_url);

    tokio::task::spawn_blocking(move || {
        let browser = Browser::new(LaunchOptions {
            headless: true,
            ..Default::default()
        })?;

        let tab = browser.new_tab()?;
        tab.navigate_to(&login_url)?;
        
        // 1. Fill Login Form
        let username_input = tab.wait_for_element("input[type='text']")?;
        username_input.type_text("demo-user")?;
        
        let password_input = tab.wait_for_element("input[type='password']")?;
        password_input.type_text("123456")?; // Default password
        
        // 2. Click Login
        let submit_btn = tab.wait_for_element("button")?;
        submit_btn.click()?;
        
        // 3. Wait for Redirect to Dashboard
        // We expect to see "仪表盘" (Dashboard title) or similar
        std::thread::sleep(Duration::from_secs(2));
        
        let current_url = tab.get_url();
        println!("Current URL after login: {}", current_url);
        
        let body_text = tab.wait_for_element("body")?.get_inner_text()?;
        
        // Since Dioxus is SPA, URL might be base_url/ (root) or base_url/#/ (hash router)
        // or simple base_url/ if History API.
        // We check for Dashboard content.
        if body_text.contains("仪表盘") || body_text.contains("Dashboard") {
            println!("Success: Redirected to Dashboard.");
        } else {
            // If login failed, we might see error message
            if body_text.contains("Invalid credentials") {
                panic!("Login failed with valid credentials.");
            }
            panic!("Did not redirect to Dashboard. Page content:\n{}", body_text);
        }

        Ok::<(), anyhow::Error>(())
    }).await??;

    Ok(())
}
