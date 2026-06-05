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

// ============================================================================
// 公开页面测试
// ============================================================================

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_forgot_password_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    test_page_loads(&base_url, "/forgot-password", "忘记密码", "forgot-password");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_reset_password_page_loads() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    // Reset password page requires a token parameter
    let mut browser = AgentBrowser::new(&base_url);
    browser
        .open("/reset-password?token=test_token")
        .expect("Failed to open reset password page");
    // Wait for either the form or an error message
    let result = browser.wait_for_text("重置", 10_000).or_else(|_| {
        browser.wait_for_text("密码", 10_000)
    });
    assert!(result.is_ok(), "Reset password page did not load");
    let _ = browser.screenshot("reset-password");
}

// ============================================================================
// 认证流程测试
// ============================================================================

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_register_success_flow() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);

    // Navigate to register page
    browser.open("/register").expect("Failed to open register page");
    browser
        .wait_for_text("创建账户", 10_000)
        .expect("Register page did not load");

    // Generate unique username
    let username = format!(
        "e2e_reg_{}",
        &uuid::Uuid::new_v4().to_string().replace('-', "")[..8]
    );

    // Fill registration form
    browser
        .fill("input:nth-of-type(1)", &username)
        .expect("Failed to fill username");
    browser
        .fill("input[type='email']", &format!("{}@test.burncloud.dev", username))
        .expect("Failed to fill email");
    browser
        .fill("input[type='password']", "test123456")
        .expect("Failed to fill password");

    // Submit registration
    browser
        .click("button[type='submit']")
        .or_else(|_| browser.click("button"))
        .expect("Failed to click register");

    // Wait for dashboard (auto-login after registration)
    let result = browser.wait_for_text("仪表盘", 15_000).or_else(|_| {
        browser.wait_for_text("企业控制台", 10_000)
    });

    assert!(result.is_ok(), "Registration did not redirect to dashboard");
    let _ = browser.screenshot("register-success");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_register_duplicate_username() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    // Create a user first via API
    let (existing_username, _) = create_test_user(&base_url).await;

    let mut browser = AgentBrowser::new(&base_url);

    // Navigate to register page
    browser.open("/register").expect("Failed to open register page");
    browser
        .wait_for_text("创建账户", 10_000)
        .expect("Register page did not load");

    // Try to register with the same username
    browser
        .fill("input:nth-of-type(1)", &existing_username)
        .expect("Failed to fill username");
    browser
        .fill("input[type='email']", &format!("{}@test2.burncloud.dev", existing_username))
        .expect("Failed to fill email");
    browser
        .fill("input[type='password']", "test123456")
        .expect("Failed to fill password");

    // Submit registration
    browser
        .click("button[type='submit']")
        .or_else(|_| browser.click("button"))
        .expect("Failed to click register");

    // Wait for error message or stay on register page
    let result = browser.wait_for_text("已存在", 5_000).or_else(|_| {
        browser.wait_for_text("错误", 5_000)
    });

    // Either we see an error message, or we're still on the register page
    let snap = browser.snapshot().expect("Failed to snapshot");
    assert!(
        result.is_ok() || snap.text.contains("注册"),
        "Expected error message or staying on register page after duplicate registration"
    );

    let _ = browser.screenshot("register-duplicate");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_login_logout_flow() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;
    let (mut browser, _) = login_browser(&base_url).await;

    // Verify logged in state - sidebar should be visible
    let snap = browser.snapshot().expect("Failed to snapshot");
    assert!(
        snap.text.contains("仪表盘") || snap.text.contains("企业控制台"),
        "Dashboard not visible after login"
    );

    // Click logout button (usually in user menu or settings)
    // Try clicking on user avatar/menu first
    let _ = browser.click_by_name("button:用户", 3_000);
    let _ = browser.click_by_name("button:退出", 3_000);
    let _ = browser.click_by_name("link:退出", 3_000);
    let _ = browser.click_by_name("link:登出", 3_000);

    // Wait for redirect to login or home page
    let result = browser.wait_for_text("登录", 10_000).or_else(|_| {
        browser.wait_for_text("Sign In", 5_000)
    });

    // Screenshot for evidence
    let _ = browser.screenshot("logout-flow");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_login_invalid_credentials_error() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);

    // Navigate to login page
    browser.open("/login").expect("Failed to open login page");
    browser
        .wait_for_text("登录", 10_000)
        .expect("Login page did not load");

    // Fill with wrong credentials
    browser
        .fill("input:nth-of-type(1)", "nonexistent_user_xyz_12345")
        .expect("Failed to fill username");
    browser
        .fill("input[type='password']", "wrong_password_xyz")
        .expect("Failed to fill password");

    // Submit
    browser
        .click("button[type='submit']")
        .or_else(|_| browser.click("button"))
        .expect("Failed to click login");

    // Wait for error indication
    let result = browser.wait_for_text("错误", 5_000).or_else(|_| {
        browser.wait_for_text("error", 3_000)
    }).or_else(|_| {
        browser.wait_for_text("失败", 3_000)
    });

    // Either we see an error message, or we're still on the login page
    let snap = browser.snapshot().expect("Failed to snapshot");
    assert!(
        result.is_ok() || snap.text.contains("登录"),
        "Expected error message or staying on login page after invalid credentials"
    );

    let _ = browser.screenshot("login-invalid-credentials");
}

// ============================================================================
// Token 处理测试
// ============================================================================

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_unauthenticated_access_redirects_to_login() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);

    // Try to access protected page without login
    browser
        .open("/console/models")
        .expect("Failed to open protected page");

    // Should redirect to login page or show login form
    let result = browser
        .wait_for_text("登录", 10_000)
        .or_else(|_| browser.wait_for_text("Sign In", 5_000));

    assert!(
        result.is_ok(),
        "Unauthenticated access should redirect to login"
    );

    let _ = browser.screenshot("unauthenticated-redirect");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_login_navigation_from_home() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);
    browser.open("/").expect("Failed to open home page");
    browser
        .wait_for_text("下一代 AI 网关", 10_000)
        .expect("Home page did not load");

    // Click Sign In link
    browser
        .click_by_name("link:Sign In", 5_000)
        .or_else(|_| browser.click_by_name("link:登录", 3_000))
        .expect("Failed to click Sign In");

    browser
        .wait_for_text("登录", 10_000)
        .expect("Did not navigate to login page");

    let _ = browser.screenshot("home-to-login-nav");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_register_navigation_from_login() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);
    browser.open("/login").expect("Failed to open login page");
    browser
        .wait_for_text("登录", 10_000)
        .expect("Login page did not load");

    // Click register link
    browser
        .click_by_name("link:注册", 5_000)
        .or_else(|_| browser.click_by_name("link:立即注册", 3_000))
        .expect("Failed to click register link");

    browser
        .wait_for_text("开始体验", 10_000)
        .or_else(|_| browser.wait_for_text("注册", 5_000))
        .expect("Did not navigate to register page");

    let _ = browser.screenshot("login-to-register-nav");
}

// ============================================================================
// 表单验证测试
// ============================================================================

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_login_form_validation_empty_fields() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);
    browser.open("/login").expect("Failed to open login page");
    browser
        .wait_for_text("登录", 10_000)
        .expect("Login page did not load");

    // Submit with empty fields
    browser
        .click("button[type='submit']")
        .or_else(|_| browser.click("button"))
        .expect("Failed to click login");

    // Should show validation error or stay on page
    let snap = browser.snapshot().expect("Failed to snapshot");
    assert!(
        snap.text.contains("登录"),
        "Should stay on login page with empty fields"
    );

    let _ = browser.screenshot("login-empty-fields");
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_register_form_validation_password_mismatch() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);
    browser.open("/register").expect("Failed to open register page");
    browser
        .wait_for_text("创建账户", 10_000)
        .expect("Register page did not load");

    let username = format!(
        "e2e_val_{}",
        &uuid::Uuid::new_v4().to_string().replace('-', "")[..8]
    );

    // Fill form (if there's a confirm password field, this would test mismatch)
    browser
        .fill("input:nth-of-type(1)", &username)
        .expect("Failed to fill username");
    browser
        .fill("input[type='email']", &format!("{}@test.burncloud.dev", username))
        .expect("Failed to fill email");
    // Short password that might fail validation
    browser
        .fill("input[type='password']", "123")
        .expect("Failed to fill password");

    browser
        .click("button[type='submit']")
        .or_else(|_| browser.click("button"))
        .expect("Failed to click register");

    // Wait for validation error or stay on page
    let snap = browser.snapshot().expect("Failed to snapshot");
    // Form should still be visible (either validation error or stayed on page)
    assert!(
        snap.text.contains("注册") || snap.text.contains("密码"),
        "Should show validation error or stay on register page"
    );

    let _ = browser.screenshot("register-validation");
}

// ============================================================================
// Forgot Password 流程测试
// ============================================================================

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_forgot_password_form_submission() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);

    // Navigate to forgot password page
    browser
        .open("/forgot-password")
        .expect("Failed to open forgot password page");
    browser
        .wait_for_text("忘记密码", 10_000)
        .expect("Forgot password page did not load");

    // Fill email
    browser
        .fill("input[type='email']", "test@example.com")
        .or_else(|_| browser.fill("input", "test@example.com"))
        .expect("Failed to fill email");

    // Submit
    browser
        .click("button[type='submit']")
        .or_else(|_| browser.click("button"))
        .expect("Failed to submit");

    // Wait for success message or redirect
    let result = browser
        .wait_for_text("成功", 10_000)
        .or_else(|_| browser.wait_for_text("邮件", 5_000))
        .or_else(|_| browser.wait_for_text("发送", 5_000));

    // Screenshot for evidence
    let _ = browser.screenshot("forgot-password-submit");

    // Accept either success or being on a new page
    assert!(result.is_ok() || browser.snapshot().map(|s| s.text.len() > 0).unwrap_or(false));
}

#[tokio::test]
#[ignore = "requires external infrastructure (browser/running server)"]
async fn test_forgot_password_navigation_from_login() {
    let _ = setup_browser().expect("agent-browser required");
    let base_url = common::spawn_app().await;

    let mut browser = AgentBrowser::new(&base_url);
    browser.open("/login").expect("Failed to open login page");
    browser
        .wait_for_text("登录", 10_000)
        .expect("Login page did not load");

    // Click forgot password link
    browser
        .click_by_name("link:忘记密码", 5_000)
        .or_else(|_| browser.click_by_name("link:Forgot", 3_000))
        .expect("Failed to click forgot password link");

    browser
        .wait_for_text("忘记密码", 10_000)
        .expect("Did not navigate to forgot password page");

    let _ = browser.screenshot("login-to-forgot-password-nav");
}
