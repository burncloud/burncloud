//! Runtime browser audit for the current BurnCloud Dioxus console.
//!
//! This test deliberately does not import the legacy page E2E modules. It uses the
//! same agent-browser CLI, but its route/text contract is tied to the current UI.
//! A dummy upstream is seeded for realistic provider/model/route state and is never
//! invoked, so no paid upstream credential is required.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types)]

use anyhow::{anyhow, bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize)]
struct BrowserResponse {
    success: Option<bool>,
    data: Option<Value>,
    error: Option<String>,
}

struct StagingBrowser {
    base_url: String,
    session: String,
    screenshot_dir: PathBuf,
}

impl StagingBrowser {
    fn new(base_url: &str, screenshot_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&screenshot_dir)?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            session: format!("burncloud-staging-{}", std::process::id()),
            screenshot_dir,
        })
    }

    fn exec(&self, args: &[&str]) -> Result<BrowserResponse> {
        let output = Command::new("agent-browser")
            .arg("--json")
            .arg("--session")
            .arg(&self.session)
            .args(args)
            .env("AGENT_BROWSER_SESSION", &self.session)
            .env("AGENT_BROWSER_ARGS", "--headless=new,--no-sandbox")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("failed to execute agent-browser {}", args.join(" ")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let parsed = serde_json::from_str::<BrowserResponse>(&stdout).unwrap_or(BrowserResponse {
            success: Some(output.status.success()),
            data: None,
            error: if stderr.trim().is_empty() { None } else { Some(stderr.clone()) },
        });

        if !output.status.success() || parsed.success == Some(false) {
            bail!(
                "agent-browser {} failed: stdout={} stderr={} error={:?}",
                args.join(" "),
                stdout,
                stderr,
                parsed.error
            );
        }
        Ok(parsed)
    }

    fn open(&self, path: &str) -> Result<()> {
        self.exec(&["open", &format!("{}{}", self.base_url, path)])?;
        let _ = self.exec(&["wait", "--load", "domcontentloaded"]);
        thread::sleep(Duration::from_millis(700));
        Ok(())
    }

    fn set_viewport(&self, width: u32, height: u32) -> Result<()> {
        let width = width.to_string();
        let height = height.to_string();
        self.exec(&["set", "viewport", &width, &height])?;
        Ok(())
    }

    fn eval(&self, js: &str) -> Result<Value> {
        let response = self.exec(&["eval", js])?;
        let data = response.data.unwrap_or(Value::Null);
        if let Some(result) = data.get("result").cloned() {
            Ok(result)
        } else {
            Ok(data)
        }
    }

    fn body_text(&self) -> Result<String> {
        Ok(self
            .eval("document.body?.innerText || ''")?
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    fn wait_for_text(&self, expected: &str, timeout: Duration) -> Result<String> {
        let start = Instant::now();
        let mut last = String::new();
        while start.elapsed() < timeout {
            last = self.body_text()?;
            if last.contains(expected) {
                return Ok(last);
            }
            thread::sleep(Duration::from_millis(300));
        }
        bail!("timeout waiting for text '{expected}'. Last body text: {last}")
    }

    fn wait_for_path(&self, expected: &str, timeout: Duration) -> Result<()> {
        let start = Instant::now();
        let mut last = Value::Null;
        while start.elapsed() < timeout {
            last = self.eval("location.pathname")?;
            if last.as_str() == Some(expected) {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(250));
        }
        bail!("timeout waiting for path '{expected}', last={last}")
    }

    fn fill(&self, selector: &str, value: &str) -> Result<()> {
        self.exec(&["fill", selector, value])?;
        Ok(())
    }

    fn click_role(&self, role: &str, name: &str, timeout: Duration) -> Result<()> {
        let start = Instant::now();
        let mut last_error = String::new();
        while start.elapsed() < timeout {
            match self.exec(&["find", "role", role, "click", "--name", name]) {
                Ok(_) => return Ok(()),
                Err(error) => last_error = error.to_string(),
            }
            thread::sleep(Duration::from_millis(250));
        }
        bail!("could not click {role} named '{name}': {last_error}")
    }

    fn screenshot_full(&self, name: &str) -> Result<()> {
        let path = self.screenshot_dir.join(format!("{name}.png"));
        let path = path.to_string_lossy().to_string();
        self.exec(&["screenshot", "--full", &path])?;
        Ok(())
    }

    fn close(&self) {
        let _ = self.exec(&["close"]);
    }
}

impl Drop for StagingBrowser {
    fn drop(&mut self) {
        self.close();
    }
}

#[derive(Debug, Serialize)]
struct PageAudit {
    name: String,
    path: String,
    screenshot: String,
    viewport_width: i64,
    viewport_height: i64,
    scroll_width: i64,
    scroll_height: i64,
    horizontal_overflow: bool,
    visible_buttons: i64,
    visible_links: i64,
}

#[derive(Debug, Serialize)]
struct AuditReport {
    base_url: String,
    viewport: String,
    seeded_admin: String,
    seeded_customer: String,
    seeded_provider: String,
    seeded_model: String,
    pages: Vec<PageAudit>,
}

fn base_url() -> String {
    std::env::var("E2E_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string())
}

fn audit_dir() -> PathBuf {
    std::env::var("STAGING_AUDIT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("target/staging-audit"))
}

async fn post_json(client: &Client, url: &str, body: Value, token: Option<&str>) -> Result<Value> {
    let mut request = client.post(url).json(&body);
    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    let response = request.send().await?;
    let status = response.status();
    let text = response.text().await?;
    let value: Value = serde_json::from_str(&text)
        .with_context(|| format!("invalid JSON from {url}: status={status} body={text}"))?;
    if !status.is_success() || value.get("success").and_then(Value::as_bool) != Some(true) {
        bail!("API seed failed: POST {url} -> {status} {value}");
    }
    Ok(value)
}

async fn seed_staging(base_url: &str) -> Result<(String, String, String)> {
    let client = Client::new();
    let admin = "staging-admin";
    let password = "StagingAdmin123!";
    let customer = "staging-customer";

    let admin_response = post_json(
        &client,
        &format!("{base_url}/api/auth/register"),
        json!({
            "username": admin,
            "email": "staging-admin@burncloud.local",
            "password": password,
        }),
        None,
    )
    .await?;
    let admin_data = admin_response.get("data").ok_or_else(|| anyhow!("missing admin data"))?;
    let admin_id = admin_data
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing admin id"))?;
    let admin_token = admin_data
        .get("token")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing admin token"))?;
    let roles = admin_data.get("roles").and_then(Value::as_array).cloned().unwrap_or_default();
    if !roles.iter().any(|role| role.as_str() == Some("admin")) {
        bail!("fresh staging DB did not make first user admin: roles={roles:?}");
    }

    post_json(
        &client,
        &format!("{base_url}/api/auth/register"),
        json!({
            "username": customer,
            "email": "staging-customer@burncloud.local",
            "password": "StagingCustomer123!",
        }),
        None,
    )
    .await?;

    post_json(
        &client,
        &format!("{base_url}/console/api/channel"),
        json!({
            "type": 8,
            "key": "staging-dummy-upstream-key",
            "name": "Staging Dummy Provider",
            "base_url": "http://127.0.0.1:39999",
            "models": "staging-model",
            "group": "default",
            "weight": 100,
            "priority": 0,
            "param_override": null,
            "header_override": null,
            "api_version": null,
            "model_mapping": null,
            "rpm_cap": null,
            "tpm_cap": null,
            "reservation_green": null,
            "reservation_yellow": null,
            "reservation_red": null
        }),
        Some(admin_token),
    )
    .await?;

    post_json(
        &client,
        &format!("{base_url}/console/api/tokens"),
        json!({ "user_id": admin_id, "quota_limit": null }),
        Some(admin_token),
    )
    .await?;

    Ok((admin.to_string(), password.to_string(), customer.to_string()))
}

fn metrics(browser: &StagingBrowser) -> Result<Value> {
    let result = browser.eval(
        r#"JSON.stringify((() => {
            const root = document.documentElement;
            const visible = (el) => {
                const s = getComputedStyle(el);
                const r = el.getBoundingClientRect();
                return s.display !== 'none' && s.visibility !== 'hidden' && r.width > 0 && r.height > 0;
            };
            return {
                path: location.pathname,
                viewport_width: window.innerWidth,
                viewport_height: window.innerHeight,
                scroll_width: root.scrollWidth,
                scroll_height: root.scrollHeight,
                horizontal_overflow: root.scrollWidth > window.innerWidth + 2,
                visible_buttons: [...document.querySelectorAll('button')].filter(visible).length,
                visible_links: [...document.querySelectorAll('a')].filter(visible).length
            };
        })())"#,
    )?;
    let text = result
        .as_str()
        .ok_or_else(|| anyhow!("browser metrics did not return JSON string: {result}"))?;
    Ok(serde_json::from_str(text)?)
}

fn capture_page(
    browser: &StagingBrowser,
    name: &str,
    expected_path: &str,
    expected_text: &str,
    extra_text: &[&str],
    pages: &mut Vec<PageAudit>,
) -> Result<()> {
    browser.wait_for_path(expected_path, Duration::from_secs(15))?;

    // Capture first. If the strict product-copy assertion drifts, the workflow still
    // preserves the actual rendered page for ChatGPT/Codex to inspect.
    browser.screenshot_full(name)?;

    let body = browser.wait_for_text(expected_text, Duration::from_secs(20))?;
    for text in extra_text {
        if !body.contains(text) {
            bail!("{name}: expected page content '{text}' was not present. Body: {body}");
        }
    }

    let m = metrics(browser)?;
    pages.push(PageAudit {
        name: name.to_string(),
        path: m.get("path").and_then(Value::as_str).unwrap_or(expected_path).to_string(),
        screenshot: format!("screenshots/{name}.png"),
        viewport_width: m.get("viewport_width").and_then(Value::as_i64).unwrap_or_default(),
        viewport_height: m.get("viewport_height").and_then(Value::as_i64).unwrap_or_default(),
        scroll_width: m.get("scroll_width").and_then(Value::as_i64).unwrap_or_default(),
        scroll_height: m.get("scroll_height").and_then(Value::as_i64).unwrap_or_default(),
        horizontal_overflow: m.get("horizontal_overflow").and_then(Value::as_bool).unwrap_or(false),
        visible_buttons: m.get("visible_buttons").and_then(Value::as_i64).unwrap_or_default(),
        visible_links: m.get("visible_links").and_then(Value::as_i64).unwrap_or_default(),
    });
    Ok(())
}

fn click_page(
    browser: &StagingBrowser,
    label: &str,
    name: &str,
    path: &str,
    expected_text: &str,
    extra_text: &[&str],
    pages: &mut Vec<PageAudit>,
) -> Result<()> {
    browser.click_role("link", label, Duration::from_secs(10))?;
    capture_page(browser, name, path, expected_text, extra_text, pages)
}

fn write_report(dir: &Path, report: &AuditReport) -> Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(dir.join("report.json"), serde_json::to_string_pretty(report)?)?;

    let mut md = String::from("# BurnCloud Staging Browser Audit\n\n");
    md.push_str(&format!("- Base URL: `{}`\n", report.base_url));
    md.push_str(&format!("- Viewport: **{}**\n", report.viewport));
    md.push_str(&format!(
        "- Seeded provider/model: **{} / {}**\n\n",
        report.seeded_provider, report.seeded_model
    ));
    md.push_str("| Page | Path | Viewport | Scroll | Overflow | Buttons | Links | Screenshot |\n");
    md.push_str("|---|---|---:|---:|---|---:|---:|---|\n");
    for page in &report.pages {
        md.push_str(&format!(
            "| {} | `{}` | {}×{} | {}×{} | {} | {} | {} | `{}` |\n",
            page.name,
            page.path,
            page.viewport_width,
            page.viewport_height,
            page.scroll_width,
            page.scroll_height,
            if page.horizontal_overflow { "❌" } else { "✅" },
            page.visible_buttons,
            page.visible_links,
            page.screenshot,
        ));
    }
    fs::write(dir.join("report.md"), md)?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires a running BurnCloud staging server and agent-browser"]
async fn current_console_visual_and_click_path_audit() -> Result<()> {
    let base_url = base_url();
    let dir = audit_dir();
    let (admin, password, customer) = seed_staging(&base_url).await?;

    let browser = StagingBrowser::new(&base_url, dir.join("screenshots"))?;
    browser.open("/login")?;
    browser.set_viewport(1440, 900)?;

    // Preserve the initial page even when visible login copy changes.
    browser.screenshot_full("00-login")?;
    browser.wait_for_text("BurnCloud Console", Duration::from_secs(20))?;
    browser.wait_for_text("Sign in to Console", Duration::from_secs(10))?;

    browser.fill("input[type='text']", &admin)?;
    browser.fill("input[type='password']", &password)?;
    browser.click_role("button", "Sign in to Console", Duration::from_secs(10))?;

    let mut pages = Vec::new();
    capture_page(
        &browser,
        "01-overview",
        "/",
        "System Overview",
        &["Setup & readiness", "Staging Dummy Provider"],
        &mut pages,
    )?;

    click_page(&browser, "Providers", "02-providers", "/providers", "Provider inventory", &["Staging Dummy Provider", "staging-model"], &mut pages)?;
    browser.click_role("button", "Add Provider", Duration::from_secs(8))?;
    browser.screenshot_full("02a-provider-add-drawer")?;
    browser.wait_for_text("Connect an upstream and define the models it can serve.", Duration::from_secs(8))?;
    browser.click_role("button", "Cancel", Duration::from_secs(5))?;

    click_page(&browser, "Models", "03-models", "/models", "Model availability", &["staging-model", "Single upstream", "No failover redundancy"], &mut pages)?;
    click_page(&browser, "Routes", "04-routes", "/routes", "Routing Groups", &["default", "Single upstream"], &mut pages)?;
    click_page(&browser, "Playground", "05-playground", "/playground", "Playground", &["staging-model", "Send Test Request"], &mut pages)?;
    click_page(&browser, "Logs", "06-logs", "/logs", "Logs", &[], &mut pages)?;
    click_page(&browser, "Evaluation", "07-evaluation", "/evaluation", "Evaluation", &[], &mut pages)?;
    click_page(&browser, "Billing", "08-billing", "/billing", "Billing", &[], &mut pages)?;

    click_page(&browser, "API Keys", "09-api-keys", "/keys", "Credentials", &["staging-admin"], &mut pages)?;
    browser.click_role("button", "Create API Key", Duration::from_secs(8))?;
    browser.screenshot_full("09a-api-key-create-drawer")?;
    browser.wait_for_text("Choose which account will own this router credential.", Duration::from_secs(8))?;
    browser.click_role("button", "Cancel", Duration::from_secs(5))?;

    click_page(&browser, "Customers", "10-customers", "/customers", "Customers", &["staging-customer"], &mut pages)?;
    browser.click_role("button", "Create Customer", Duration::from_secs(8))?;
    browser.screenshot_full("10a-customer-create-drawer")?;
    browser.wait_for_text("Create a business account that can own wallet balance and API access.", Duration::from_secs(8))?;
    browser.click_role("button", "Cancel", Duration::from_secs(5))?;

    click_page(&browser, "Guardrails", "11-guardrails", "/guardrails", "Guardrails", &[], &mut pages)?;
    click_page(&browser, "Team", "12-team", "/team", "Environment operators", &["staging-admin"], &mut pages)?;
    click_page(&browser, "Settings", "13-settings", "/settings", "Settings", &[], &mut pages)?;

    let report = AuditReport {
        base_url: base_url.clone(),
        viewport: "1440x900".to_string(),
        seeded_admin: admin,
        seeded_customer: customer,
        seeded_provider: "Staging Dummy Provider".to_string(),
        seeded_model: "staging-model".to_string(),
        pages,
    };
    write_report(&dir, &report)?;

    browser.click_role("button", "Sign Out", Duration::from_secs(8))?;
    browser.wait_for_path("/login", Duration::from_secs(10))?;
    browser.screenshot_full("99-signed-out")?;
    browser.wait_for_text("BurnCloud Console", Duration::from_secs(10))?;

    let overflow: Vec<&PageAudit> = report.pages.iter().filter(|page| page.horizontal_overflow).collect();
    if !overflow.is_empty() {
        bail!(
            "horizontal overflow detected on: {}",
            overflow.iter().map(|page| page.name.as_str()).collect::<Vec<_>>().join(", ")
        );
    }

    if report.pages.iter().any(|page| page.viewport_width != 1440 || page.viewport_height != 900) {
        bail!("agent-browser did not hold the required 1440x900 viewport; inspect report.json");
    }

    Ok(())
}
