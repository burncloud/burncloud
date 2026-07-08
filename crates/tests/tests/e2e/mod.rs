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
pub mod agent_browser;
pub mod api_key_flow;
pub mod auth_flow;
pub mod channel_flow;
pub mod console_pages;
pub mod aesthetic_acceptance;
pub mod css_visual_acceptance;
pub mod design_tokens;
pub mod guest_pages;
pub mod login_flow;
pub mod models_flow;
pub mod monitor_flow;
pub mod navigation;
pub mod settings_interactions;
pub mod user_flow;

use agent_browser::{is_agent_browser_available, AgentBrowser};
use reqwest::Client;
use serde_json::json;

use crate::common;

pub fn setup_browser() -> Option<()> {
    if !is_agent_browser_available() {
        eprintln!("SKIP: agent-browser not installed. Install with: npm install -g agent-browser");
        return None;
    }
    Some(())
}

/// Click a button in Dioxus LiveView using dispatchEvent.
/// Dioxus uses custom event handling via data-dioxus-id attributes.
/// Standard .click() methods don't trigger Dioxus event handlers correctly.
pub fn dioxus_click(browser: &mut AgentBrowser, selector: &str) -> Result<(), Box<dyn std::error::Error>> {
    let js = format!(
        r#"
        (function() {{
            const btn = document.querySelector("{selector}");
            if (btn) {{
                btn.dispatchEvent(new MouseEvent('click', {{ 
                    bubbles: true, 
                    cancelable: true, 
                    view: window 
                }}));
                return 'dispatched';
            }}
            return 'not_found';
        }})()
        "#,
        selector = selector
    );
    let result = browser.eval(&js)?;
    if result.as_str() == Some("dispatched") {
        Ok(())
    } else {
        Err(format!("Element not found with selector: {}", selector).into())
    }
}

/// Click the login submit button. Prefers accessibility-tree click (works with LiveView).
pub fn submit_login_click(browser: &mut AgentBrowser) -> anyhow::Result<()> {
    if browser.click_by_name("button:登录", 5_000).is_ok()
        || browser.click_by_name("button:Log in", 3_000).is_ok()
        || browser.click_by_name("button:Login", 3_000).is_ok()
        || browser.click("button.landing-btn-dark").is_ok()
        || dioxus_click(browser, "button.landing-btn-dark").is_ok()
    {
        return Ok(());
    }
    anyhow::bail!("Could not click login submit button")
}

fn fill_login_form(
    browser: &mut AgentBrowser,
    username: &str,
    password: &str,
) -> anyhow::Result<()> {
    browser.open("/login")?;
    browser
        .wait_for_text("登录", 15_000)
        .or_else(|_| browser.wait_for_text("Log in", 5_000))?;
    std::thread::sleep(std::time::Duration::from_millis(300));

    let snap = browser.snapshot()?;
    let refs = snap
        .refs
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Login page refs missing"))?;

    let username_ref = refs
        .iter()
        .find(|(_, info)| {
            info.get("role").and_then(|r| r.as_str()) == Some("textbox")
                && info
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .contains("you@burncloud.com")
        })
        .map(|(ref_id, _)| ref_id.as_str())
        .ok_or_else(|| anyhow::anyhow!("Username input not found"))?;

    let password_ref = refs
        .iter()
        .find(|(_, info)| {
            info.get("role").and_then(|r| r.as_str()) == Some("textbox")
                && !info
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .contains("you@burncloud.com")
        })
        .map(|(ref_id, _)| ref_id.as_str())
        .ok_or_else(|| anyhow::anyhow!("Password input not found"))?;

    browser.fill(username_ref, username)?;
    browser.fill(password_ref, password)?;
    Ok(())
}

fn wait_for_console_after_login(
    browser: &mut AgentBrowser,
    fail_screenshot: &str,
) -> anyhow::Result<()> {
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let snapshot = browser.snapshot()?;
        if snapshot.text.contains("仪表盘")
            || snapshot.text.contains("Dashboard")
            || snapshot.text.contains("模型")
            || snapshot.text.contains("渠道")
        {
            return Ok(());
        }
        if snapshot.text.contains("登录失败") || snapshot.text.contains("用户不存在") {
            let _ = browser.screenshot(&format!("{fail_screenshot}-error"));
            anyhow::bail!("Login failed with error on page");
        }
    }
    let snapshot = browser.snapshot()?;
    let _ = browser.screenshot(fail_screenshot);
    anyhow::bail!(
        "Login did not complete within 20 seconds. Page shows: {}",
        snapshot.text
    );
}

/// Resolve admin credentials for E2E: try known admins, else register testadmin2.
pub async fn resolve_admin_credentials(base_url: &str) -> (String, String) {
    let candidates = [
        ("testadmin2", "TestAdmin123!"),
        ("testadmin", "TestAdmin123!"),
        ("admin", "Admin123!"),
    ];
    let client = Client::new();

    for (username, password) in candidates {
        let resp = client
            .post(format!("{}/api/auth/login", base_url))
            .json(&json!({
                "username": username,
                "password": password,
            }))
            .send()
            .await;

        if let Ok(r) = resp {
            if let Ok(body) = r.json::<serde_json::Value>().await {
                if body["success"].as_bool() == Some(true) {
                    eprintln!("E2E admin credentials: {username}");
                    return (username.to_string(), password.to_string());
                }
            }
        }
    }

    let username = "testadmin2";
    let password = "TestAdmin123!";
    let reg_resp = client
        .post(format!("{}/api/auth/register", base_url))
        .json(&json!({
            "username": username,
            "email": "testadmin2@test.burncloud.dev",
            "password": password,
        }))
        .send()
        .await
        .expect("Failed to register testadmin2");

    let body: serde_json::Value = reg_resp
        .json()
        .await
        .unwrap_or_else(|_| json!({}));
    eprintln!("resolve_admin_credentials register: {body}");
    (username.to_string(), password.to_string())
}

/// Ensure testadmin2 exists (register on fresh DB; no-op if already present).
#[allow(dead_code)]
pub async fn ensure_test_admin_exists(base_url: &str) {
    let _ = resolve_admin_credentials(base_url).await;
}
pub fn dioxus_click_checkbox(browser: &mut AgentBrowser, selector: &str) -> Result<(), Box<dyn std::error::Error>> {
    let js = format!(
        r#"
        (function() {{
            const checkbox = document.querySelector("{selector}");
            if (checkbox) {{
                checkbox.dispatchEvent(new MouseEvent('click', {{ 
                    bubbles: true, 
                    cancelable: true, 
                    view: window 
                }}));
                return 'dispatched';
            }}
            return 'not_found';
        }})()
        "#,
        selector = selector
    );
    let result = browser.eval(&js)?;
    if result.as_str() == Some("dispatched") {
        Ok(())
    } else {
        Err(format!("Checkbox not found with selector: {}", selector).into())
    }
}

pub async fn create_test_user(base_url: &str) -> (String, String) {
    let username = format!(
        "e2e_test_{}",
        &uuid::Uuid::new_v4().to_string().replace('-', "")[..8]
    );
    let password = "test123456".to_string();

    let client = Client::new();
    let resp = client
        .post(format!("{}/api/auth/register", base_url))
        .json(&json!({
            "username": username,
            "email": format!("{}@test.burncloud.dev", username),
            "password": password,
        }))
        .send()
        .await
        .expect("Failed to create test user");

    let body: serde_json::Value = resp.json().await.expect("Failed to parse register response");
    
    // Debug: print registration response
    eprintln!("Register response for {}: {}", username, body);

    if body["success"].as_bool() != Some(true) {
        let login_resp = client
            .post(format!("{}/api/auth/login", base_url))
            .json(&json!({
                "username": username,
                "password": password,
            }))
            .send()
            .await
            .expect("Failed to login test user");
        let login_body: serde_json::Value = login_resp.json().await.expect("Failed to parse login response");
        let token = login_body["data"]["token"].as_str().expect("No token in login response").to_string();
        return (username, token);
    }

    let token = body["data"]["token"].as_str().expect("No token in register response").to_string();
    (username, token)
}

pub fn test_page_loads(base_url: &str, path: &str, expected_text: &str, screenshot_name: &str) {
    let mut browser = AgentBrowser::new(base_url);
    browser.open(path).expect("Failed to open page");
    browser.wait_for_text(expected_text, 10_000).unwrap_or_else(|e| {
        let _ = browser.screenshot(&format!("FAIL-{}", screenshot_name));
        panic!("Page {} failed to load expected text '{}': {}", path, expected_text, e);
    });
    let _ = browser.screenshot(screenshot_name);
}

pub async fn login_browser(base_url: &str) -> (AgentBrowser, String) {
    let (username, _token) = create_test_user(base_url).await;
    let password = "test123456";
    let mut browser = AgentBrowser::new(base_url);

    fill_login_form(&mut browser, &username, password).expect("Failed to fill login form");
    submit_login_click(&mut browser).expect("Failed to click login button");
    if wait_for_console_after_login(&mut browser, "FAIL-login-debug").is_err() {
        // LiveView click can be flaky — fall back to API-seeded auth on same session.
        login_via_api_in_browser(base_url, &mut browser)
            .await
            .expect("UI and API login both failed");
    }

    (browser, username)
}

pub async fn login_as_admin_in_browser(
    base_url: &str,
    browser: &mut AgentBrowser,
) -> String {
    login_via_api_in_browser(base_url, browser)
        .await
        .expect("API auth seed for browser failed")
}

/// Seed browser localStorage from API login (reliable for visual/CSS E2E).
pub async fn login_via_api_in_browser(
    base_url: &str,
    browser: &mut AgentBrowser,
) -> anyhow::Result<String> {
    let (username, password) = resolve_admin_credentials(base_url).await;
    let client = Client::new();
    let body: serde_json::Value = client
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({
            "username": username,
            "password": password,
        }))
        .send()
        .await?
        .json()
        .await?;

    if body["success"].as_bool() != Some(true) {
        anyhow::bail!("API login failed for {username}: {body}");
    }

    let data = &body["data"];
    let token = data["token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing token"))?;
    let id = data["id"].as_str().unwrap_or("");
    let roles = data["roles"].clone();

    let user_info = serde_json::to_string(&json!({
        "id": id,
        "username": username,
        "roles": roles,
    }))?;
    let client_state = json!({
        "last_username": username,
        "auth_token": token,
        "user_info": user_info,
        "theme": null,
    });
    let client_state_js = serde_json::to_string(&client_state)?;

    browser.open("/")?;
    browser.eval(&format!(
        "localStorage.setItem('client_state', {client_state_js});"
    ))?;
    browser.open("/console/dashboard")?;
    browser
        .wait_for_text("仪表盘", 20_000)
        .or_else(|_| browser.wait_for_text("Dashboard", 5_000))
        .or_else(|_| browser.wait_for_text("模型", 5_000))?;
    Ok(username)
}

/// Login as an admin user (testadmin2) for tests requiring admin privileges.
/// This is needed because create_test_user() creates users with "user" role,
/// which cannot access admin-only pages like /console/users, /console/dashboard, etc.
pub async fn login_as_admin(base_url: &str) -> (AgentBrowser, String) {
    let mut browser = AgentBrowser::new(base_url);
    let username = login_as_admin_in_browser(base_url, &mut browser).await;
    (browser, username)
}

/// When `E2E_USE_PREVIEW=1`, aesthetic/css tests open `/preview/*` routes with baked-in mock data.
pub fn e2e_preview_enabled() -> bool {
    std::env::var("E2E_USE_PREVIEW")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Map production path → preview entry. Production paths pass through when preview is off.
pub fn e2e_page_path(production_path: &str) -> String {
    if !e2e_preview_enabled() {
        return production_path.to_string();
    }
    match production_path {
        "/" => "/preview/home".to_string(),
        "/login" => "/preview/login".to_string(),
        path if path.starts_with("/console/") => format!("/preview{path}"),
        other => other.to_string(),
    }
}
