use headless_chrome::{Browser, LaunchOptions};
use crate::common;

#[tokio::test]
async fn test_login_page_render() -> anyhow::Result<()> {
    let base_url = common::spawn_app().await;
    let url = format!("{}/login", base_url);

    tokio::task::spawn_blocking(move || {
        let browser = Browser::new(LaunchOptions {
            headless: true,
            ..Default::default()
        })?;

        let tab = browser.new_tab()?;
        tab.navigate_to(&url)?;
        
        // Wait for the login form
        tab.wait_for_element(".auth-container")?;
        
        let title = tab.get_title()?;
        println!("Login Page Title: {}", title);
        
        // Check for "用户名" label or input
        let content = tab.find_element(".auth-card")?.get_inner_text()?;
        assert!(content.contains("用户名"), "Username label not found");
        assert!(content.contains("登录"), "Login button text not found");

        Ok::<(), anyhow::Error>(())
    }).await??;

    Ok(())
}
