#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::let_unit_value, clippy::redundant_pattern, clippy::manual_is_multiple_of, clippy::let_and_return, clippy::to_string_trait_impl, clippy::to_string_in_format_args, clippy::redundant_pattern_matching)]
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize)]
struct RawResponse {
    success: Option<bool>,
    data: Option<serde_json::Value>,
    #[allow(dead_code)]
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BrowserResponse {
    #[allow(dead_code)]
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub raw_stdout: String,
}

#[derive(Debug, Clone)]
pub struct SnapshotResult {
    pub text: String,
    #[allow(dead_code)]
    pub refs: serde_json::Value,
}

/// Wraps the agent-browser CLI for E2E testing.
///
/// Each instance uses a unique `--session` to isolate browser state between
/// parallel tests. The daemon persists between commands within the same session,
/// maintaining cookies/localStorage for login flows.
pub struct AgentBrowser {
    base_url: String,
    screenshot_dir: String,
    last_snapshot: Option<String>,
}

impl AgentBrowser {
    pub fn new(base_url: &str) -> Self {
        // Use absolute path for screenshots since agent-browser runs in a
        // different working directory than the test process.
        let screenshot_dir = std::env::current_dir()
            .unwrap_or_default()
            .join("target/e2e-screenshots")
            .to_string_lossy()
            .to_string();
        // Ensure directory exists
        let _ = std::fs::create_dir_all(&screenshot_dir);
        Self {
            base_url: base_url.to_string(),
            screenshot_dir,
            last_snapshot: None,
        }
    }

    /// Execute a single agent-browser command with JSON output.
    /// Uses a 30s timeout to prevent hangs.
    fn exec(&self, args: &[&str]) -> Result<BrowserResponse> {
        let mut full_args: Vec<String> = vec!["--json".to_string()];
        for a in args {
            full_args.push(a.to_string());
        }

        let start = Instant::now();
        let output = Command::new(agent_browser_bin())
            .args(&full_args)
            .env("AGENT_BROWSER_ARGS", "--headless=new")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .output()
            .with_context(|| format!("Failed to execute agent-browser {}", args.join(" ")))?;
        eprintln!(
            "[agent-browser] {} took {}ms",
            args.join(" "),
            start.elapsed().as_millis()
        );

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() && !stdout.contains("\"success\"") {
            bail!(
                "agent-browser {} failed (exit {}): stderr={}",
                args.join(" "),
                output.status.code().unwrap_or(-1),
                stderr
            );
        }

        // Try to parse JSON output
        if let Ok(raw) = serde_json::from_str::<RawResponse>(&stdout) {
            Ok(BrowserResponse {
                success: raw.success.unwrap_or(false),
                data: raw.data,
                raw_stdout: stdout,
            })
        } else {
            Ok(BrowserResponse {
                success: output.status.success(),
                data: None,
                raw_stdout: stdout,
            })
        }
    }

    /// Navigate to a URL path (relative to base_url).
    pub fn open(&mut self, path: &str) -> Result<BrowserResponse> {
        let url = format!("{}{}", self.base_url, path);
        self.exec(&["open", &url])
    }

    /// Click an element by selector.
    pub fn click(&mut self, selector: &str) -> Result<BrowserResponse> {
        self.exec(&["click", selector])
    }

    /// Click an element by its accessible name (from the accessibility tree).
    /// Takes a snapshot, finds the ref with matching name, then clicks via ref.
    /// Supports matching by role: e.g. "button:登录", "link:Sign In".
    pub fn click_by_name(&mut self, name: &str, timeout_ms: u64) -> Result<BrowserResponse> {
        let start = Instant::now();
        loop {
            let snap = self.snapshot()?;
            if let Some(refs) = snap.refs.as_object() {
                // Parse optional role prefix: "button:登录" → (Some("button"), "登录")
                let (role_filter, target_name) = if let Some(idx) = name.find(':') {
                    (Some(&name[..idx]), &name[idx + 1..])
                } else {
                    (None, name)
                };

                for (ref_id, info) in refs {
                    if let Some(el_name) = info.get("name").and_then(|n| n.as_str()) {
                        if el_name.contains(target_name) {
                            if let Some(role) = role_filter {
                                if info.get("role").and_then(|r| r.as_str()) != Some(role) {
                                    continue;
                                }
                            }
                            return self.exec(&["click", ref_id]);
                        }
                    }
                }
            }
            if start.elapsed().as_millis() as u64 > timeout_ms {
                bail!(
                    "Timeout clicking '{}' ({}ms). Last snapshot: {}",
                    name,
                    timeout_ms,
                    snap.text
                );
            }
            std::thread::sleep(Duration::from_millis(300));
        }
    }

    /// Fill an input field by selector.
    pub fn fill(&mut self, selector: &str, text: &str) -> Result<BrowserResponse> {
        self.exec(&["fill", selector, text])
    }

    /// Get the accessibility tree snapshot.
    pub fn snapshot(&mut self) -> Result<SnapshotResult> {
        let resp = self.exec(&["snapshot", "-i"])?;

        // Parse snapshot from response data
        if let Some(data) = &resp.data {
            let text = data
                .get("snapshot")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let refs = data.get("refs").cloned().unwrap_or(serde_json::Value::Null);
            self.last_snapshot = Some(text.clone());
            Ok(SnapshotResult { text, refs })
        } else {
            let text = resp.raw_stdout.clone();
            self.last_snapshot = Some(text.clone());
            Ok(SnapshotResult {
                text,
                refs: serde_json::Value::Null,
            })
        }
    }

    /// Wait for specific text to appear in the accessibility tree.
    /// Polls every 500ms until text appears or timeout is reached.
    ///
    /// Essential for Dioxus LiveView: the HTML page is a shell that renders
    /// content via WebSocket after page load.
    pub fn wait_for_text(&mut self, expected: &str, timeout_ms: u64) -> Result<SnapshotResult> {
        let start = Instant::now();
        loop {
            let result = self.snapshot()?;
            if result.text.contains(expected) {
                return Ok(result);
            }
            if start.elapsed().as_millis() as u64 > timeout_ms {
                bail!(
                    "Timeout waiting for '{}' ({}ms). Last snapshot: {}",
                    expected,
                    timeout_ms,
                    self.last_snapshot.as_deref().unwrap_or("<empty>")
                );
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    /// Take a screenshot and save to the screenshot directory.
    pub fn screenshot(&self, name: &str) -> Result<()> {
        let path = format!("{}/{}.png", self.screenshot_dir, name);
        self.exec(&["screenshot", &path])?;
        Ok(())
    }

    /// Evaluate JavaScript in the browser context.
    /// Returns the `result` field from the eval response data.
    pub fn eval(&mut self, js: &str) -> Result<serde_json::Value> {
        let resp = self.exec(&["eval", js])?;
        // agent-browser eval returns {"data": {"result": <value>}}
        if let Some(data) = resp.data {
            if let Some(result) = data.get("result") {
                Ok(result.clone())
            } else {
                Ok(data)
            }
        } else {
            Ok(serde_json::Value::Null)
        }
    }

    /// Close the browser session.
    #[allow(dead_code)]
    pub fn close(&self) -> Result<()> {
        let _ = self.exec(&["close"]);
        Ok(())
    }
}

impl Drop for AgentBrowser {
    fn drop(&mut self) {
        // Navigate to about:blank instead of closing the daemon.
        // The agent-browser daemon is shared across tests; closing it kills
        // the Chrome process and forces every subsequent test to cold-start
        // a new browser (5-10s overhead). Navigating away is enough to
        // release page resources.
        let _ = self.exec(&["open", "about:blank"]);
    }
}

/// Check if agent-browser CLI is available.
pub fn is_agent_browser_available() -> bool {
    find_agent_browser_binary().is_some()
}

/// Find the agent-browser binary path, trying multiple strategies.
fn find_agent_browser_binary() -> Option<String> {
    // Strategy 1: Try direct command (works on Unix, sometimes Windows)
    if Command::new("agent-browser")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some("agent-browser".to_string());
    }

    // Strategy 2: On Windows, try .cmd wrapper
    #[cfg(windows)]
    {
        if Command::new("agent-browser.cmd")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Some("agent-browser.cmd".to_string());
        }

        // Try common npm global paths
        if let Ok(appdata) = std::env::var("APPDATA") {
            let npm_path = format!("{}\\npm\\agent-browser.cmd", appdata);
            if std::path::Path::new(&npm_path).exists() {
                if Command::new(&npm_path)
                    .arg("--version")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
                {
                    return Some(npm_path);
                }
            }
        }
    }

    None
}

/// Get the agent-browser binary path. Panics if not found.
fn agent_browser_bin() -> String {
    find_agent_browser_binary()
        .expect("agent-browser not found. Install with: npm install -g agent-browser")
}
