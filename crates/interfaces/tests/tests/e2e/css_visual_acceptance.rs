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
//! CSS / layout visual acceptance for css-optimize-loop.
//!
//! Run: cargo run -p burncloud-loops -- gate css-visual
//! Requires: agent-browser (`npm install -g agent-browser && agent-browser install`)

use super::*;
use serde_json::json;
use std::fs;
use std::path::Path;

struct ConsolePageSpec {
    path: &'static str,
    wait_text: &'static str,
    screenshot: &'static str,
}

const CONSOLE_PAGES: &[ConsolePageSpec] = &[
    ConsolePageSpec {
        path: "/console/dashboard",
        wait_text: "企业控制台",
        screenshot: "visual-dashboard",
    },
    ConsolePageSpec {
        path: "/console/models",
        wait_text: "模型管理",
        screenshot: "visual-models",
    },
    ConsolePageSpec {
        path: "/console/access",
        wait_text: "访问凭证",
        screenshot: "visual-access",
    },
    ConsolePageSpec {
        path: "/console/deploy",
        wait_text: "Model Deployment",
        screenshot: "visual-deploy",
    },
    ConsolePageSpec {
        path: "/console/monitor",
        wait_text: "风控雷达",
        screenshot: "visual-monitor",
    },
    ConsolePageSpec {
        path: "/console/logs",
        wait_text: "日志审查",
        screenshot: "visual-logs",
    },
    ConsolePageSpec {
        path: "/console/users",
        wait_text: "用户管理",
        screenshot: "visual-users",
    },
    ConsolePageSpec {
        path: "/console/settings",
        wait_text: "系统设置",
        screenshot: "visual-settings",
    },
    ConsolePageSpec {
        path: "/console/finance",
        wait_text: "财务",
        screenshot: "visual-finance",
    },
    ConsolePageSpec {
        path: "/console/connect",
        wait_text: "算力互联",
        screenshot: "visual-connect",
    },
    ConsolePageSpec {
        path: "/console/playground",
        wait_text: "演练场",
        screenshot: "visual-playground",
    },
];

fn wait_page_text(browser: &mut AgentBrowser, texts: &[&str], timeout_ms: u64) -> anyhow::Result<()> {
    browser
        .wait_for_any_text(texts, timeout_ms)
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("{e}"))
}

/// DOM + layout checks executed in the browser after each console page loads.
const LAYOUT_CHECK_JS: &str = r#"
(function() {
  const issues = [];
  if (document.documentElement.scrollWidth > window.innerWidth + 2) {
    issues.push('horizontal-overflow');
  }
  const forbidden = /\b(gap-md|gap-lg|gap-xl|gap-xxl|gap-xxxl|p-md|p-lg|mb-xxxl|text-muted-foreground|bg-card|rounded-xl|shadow-sm|text-2xl|text-xxs|text-display|border-\[var\(--bc-border\)\])\b/;
  const badClasses = new Set();
  document.querySelectorAll('[class]').forEach(el => {
    const c = el.className;
    if (typeof c !== 'string') return;
    c.split(/\s+/).forEach(token => {
      if (forbidden.test(token)) badClasses.add(token);
    });
  });
  if (badClasses.size > 0) {
    issues.push('legacy-dom-class:' + Array.from(badClasses).slice(0, 8).join(','));
  }
  if (location.pathname.startsWith('/console') && !document.querySelector('.page-content, .animate-fade-in, .stats-grid')) {
    issues.push('missing-content-shell');
  }
  const primary = getComputedStyle(document.documentElement).getPropertyValue('--bc-primary').trim();
  if (!primary || (!primary.includes('007AFF') && !primary.includes('007aff') && !primary.includes('0, 122, 255'))) {
    issues.push('bad-primary-token:' + primary);
  }
  return issues.length ? issues.join(';') : 'ok';
})()
"#;

fn eval_layout_ok(browser: &mut AgentBrowser, page: &str) -> anyhow::Result<()> {
    let result = browser.eval(LAYOUT_CHECK_JS)?;
    let status = result.as_str().unwrap_or("").trim();
    if status == "ok" {
        Ok(())
    } else {
        let _ = browser.screenshot(&format!("FAIL-layout-{}", page));
        anyhow::bail!("Layout check failed on {}: {}", page, status);
    }
}

fn write_manifest(screenshots: &[String], status: &str) {
    let dir = std::env::var("CSS_VISUAL_ARTIFACTS_DIR").unwrap_or_else(|_| {
        std::env::current_dir()
            .unwrap_or_default()
            .join("data/loops/css-visual/latest")
            .to_string_lossy()
            .to_string()
    });
    let _ = fs::create_dir_all(&dir);
    let manifest = json!({
        "status": status,
        "screenshots": screenshots,
        "pages": CONSOLE_PAGES.iter().map(|p| p.screenshot).collect::<Vec<_>>(),
    });
    let path = Path::new(&dir).join("manifest.json");
    if let Ok(body) = serde_json::to_string_pretty(&manifest) {
        let _ = fs::write(path, body);
    }
}

/// Full console visual acceptance: screenshots + DOM layout rules (V1–V4).
#[tokio::test]
#[ignore = "requires agent-browser and running burncloud server (cargo run -p burncloud-loops -- gate css-visual)"]
async fn css_visual_acceptance() {
    let _ = setup_browser().expect("agent-browser required: npm install -g agent-browser && agent-browser install");
    let base_url = common::spawn_app().await;

    let mut screenshots = Vec::new();
    let mut browser = AgentBrowser::new(&base_url);

    // Design tokens on home (V3 primary color)
    browser.open("/").expect("open home");
    browser
        .wait_for_text("下一代 AI 网关", 15_000)
        .expect("home page load");
    browser.screenshot("visual-home").expect("screenshot home");
    screenshots.push("visual-home.png".to_string());

    let primary = browser
        .eval("getComputedStyle(document.documentElement).getPropertyValue('--bc-primary')")
        .expect("eval primary");
    let color = primary.as_str().unwrap_or("").trim();
    assert!(
        color.contains("007AFF") || color.contains("007aff") || color.contains("0, 122, 255"),
        "Expected Apple primary #007AFF, got: {color}"
    );

    // Console pages — reuse the same browser session (cookies + warm Chrome).
    login_as_admin_in_browser(&base_url, &mut browser).await;

    for page in CONSOLE_PAGES {
        browser.open(page.path).expect("open console page");

        let wait_texts: &[&str] = match page.screenshot {
            "visual-dashboard" => &["企业控制台", "仪表盘", "gpt-4o-mini", "Dashboard"],
            "visual-logs" => &["日志审查", "Logs", "审计"],
            "visual-users" => &["用户管理", "客户列表", "Users"],
            "visual-playground" => &["演练场", "Playground", "playground", "清空", "Clear", "渠道"],
            "visual-access" => &["访问凭证", "创建新凭证", "没有活跃", "API Key", "Access"],
            "visual-deploy" => &["Model Deployment", "Deploy new models", "MODEL ID", "OpenAI", "模型部署"],
            "visual-models" => &["模型管理", "模型网络", "Model Management", "Models"],
            "visual-monitor" => &["风控雷达", "Risk Radar", "当前安全评分", "安全评分", "紧急熔断", "Blocked Attacks", "内容风控"],
            "visual-settings" => &["系统设置", "Settings", "General"],
            "visual-finance" => &["财务", "Finance", "账单", "充值记录", "Billing", "Recharge"],
            "visual-connect" => &["算力互联", "Connect", "互联", "结算"],
            _ => &[page.wait_text],
        };
        wait_page_text(&mut browser, wait_texts, 30_000).unwrap_or_else(|e| {
            let _ = browser.screenshot(&format!("FAIL-load-{}", page.screenshot));
            panic!(
                "Page {} did not load (expected {:?}): {}",
                page.path, wait_texts, e
            );
        });

        browser
            .screenshot(page.screenshot)
            .expect("screenshot console page");
        screenshots.push(format!("{}.png", page.screenshot));

        eval_layout_ok(&mut browser, page.screenshot).unwrap_or_else(|e| {
            write_manifest(&screenshots, "fail");
            panic!("{}", e);
        });
    }

    write_manifest(&screenshots, "pass");
    eprintln!(
        "CSS visual acceptance passed ({} screenshots). Dir: {}",
        screenshots.len(),
        std::env::var("CSS_VISUAL_ARTIFACTS_DIR").unwrap_or_else(|_| "data/loops/css-visual/latest".into())
    );
}
