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
//! Aesthetic capture + J1 metrics for aesthetic-optimize-loop.
//!
//! Run: cargo run -p burncloud-loops -- gate aesthetic-metrics

use super::*;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

struct AestheticPageSpec {
    path: &'static str,
    screenshot: &'static str,
    wait_texts: &'static [&'static str],
    guest: bool,
}

const AESTHETIC_PAGES: &[AestheticPageSpec] = &[
    AestheticPageSpec {
        path: "/",
        screenshot: "aesthetic-home",
        wait_texts: &["下一代 AI 网关", "BurnCloud"],
        guest: true,
    },
    AestheticPageSpec {
        path: "/login",
        screenshot: "aesthetic-login",
        wait_texts: &["登录", "Log in", "you@burncloud.com"],
        guest: true,
    },
    AestheticPageSpec {
        path: "/console/dashboard",
        screenshot: "aesthetic-dashboard",
        wait_texts: &["企业控制台", "仪表盘", "Dashboard", "gpt-4o-mini"],
        guest: false,
    },
    AestheticPageSpec {
        path: "/console/models",
        screenshot: "aesthetic-models",
        wait_texts: &["模型管理", "模型网络", "Model Management", "Models"],
        guest: false,
    },
    AestheticPageSpec {
        path: "/console/access",
        screenshot: "aesthetic-access",
        wait_texts: &["访问凭证", "Access", "API Key"],
        guest: false,
    },
    AestheticPageSpec {
        path: "/console/settings",
        screenshot: "aesthetic-settings",
        wait_texts: &["系统设置", "Settings", "General"],
        guest: false,
    },
    AestheticPageSpec {
        path: "/console/finance",
        screenshot: "aesthetic-finance",
        wait_texts: &["财务", "Finance", "账单", "Billing"],
        guest: false,
    },
    AestheticPageSpec {
        path: "/console/monitor",
        screenshot: "aesthetic-monitor",
        wait_texts: &["风控雷达", "Risk Radar", "安全评分", "Blocked Attacks"],
        guest: false,
    },
    AestheticPageSpec {
        path: "/console/playground",
        screenshot: "aesthetic-playground",
        wait_texts: &["演练场", "Playground", "清空", "Clear", "渠道"],
        guest: false,
    },
];

fn artifacts_dir() -> PathBuf {
    std::env::var("AESTHETIC_ARTIFACTS_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|cwd| cwd.join("data/loops/aesthetic/latest"))
        })
        .unwrap_or_else(|| PathBuf::from("data/loops/aesthetic/latest"))
}

fn wait_texts_for(screenshot: &str, fallback: &'static [&'static str]) -> &'static [&'static str] {
    match screenshot {
        "aesthetic-dashboard" => &["企业控制台", "仪表盘", "gpt-4o-mini", "Dashboard"],
        "aesthetic-models" => &["模型管理", "模型网络", "Model Management", "Models"],
        "aesthetic-access" => &["访问凭证", "创建新凭证", "没有活跃的访问凭证", "没有活跃", "API Key", "Access", "凭证"],
        "aesthetic-settings" => &["系统设置", "Settings", "General"],
        "aesthetic-finance" => &["财务", "Finance", "账单", "充值记录", "Billing", "Recharge"],
        "aesthetic-monitor" => &["风控雷达", "Risk Radar", "安全评分", "紧急熔断", "Blocked Attacks"],
        "aesthetic-playground" => &["演练场", "Playground", "playground", "清空", "Clear", "渠道"],
        _ => fallback,
    }
}

/// When `AESTHETIC_FOCUS_PAGE` is set (jobs-aesthetic loop), only capture/metric that page.
fn aesthetic_pages_to_run() -> Vec<&'static AestheticPageSpec> {
    if let Ok(focus) = std::env::var("AESTHETIC_FOCUS_PAGE") {
        let focus = focus.trim();
        if !focus.is_empty() {
            let pages: Vec<_> = AESTHETIC_PAGES
                .iter()
                .filter(|p| p.screenshot == focus)
                .collect();
            if pages.is_empty() {
                panic!(
                    "AESTHETIC_FOCUS_PAGE={focus:?} does not match any aesthetic page key"
                );
            }
            eprintln!("E2E: aesthetic focus page only: {focus}");
            return pages;
        }
    }
    AESTHETIC_PAGES.iter().collect()
}

fn wait_page_text(browser: &mut AgentBrowser, texts: &[&str], timeout_ms: u64) -> anyhow::Result<()> {
    let timeout_ms = if e2e_preview_enabled() {
        timeout_ms.max(30_000)
    } else {
        timeout_ms
    };
    browser
        .wait_for_any_text(texts, timeout_ms)
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("{e}"))
}

/// J1 automatic aesthetic metrics (see crates/loops/acceptance/aesthetic-optimize-acceptance.md §J1).
const METRICS_JS_TEMPLATE: &str = r#"
(function(guest) {
  const issues = [];
  const visible = (el) => {
    const r = el.getBoundingClientRect();
    const s = getComputedStyle(el);
    return r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.display !== 'none';
  };

  const fontSizes = new Set();
  if (!guest) {
    const fontRoot = document.querySelector('.page-content, .console-layout') || document.body;
    fontRoot.querySelectorAll('*').forEach(el => {
      if (!visible(el)) return;
      fontSizes.add(getComputedStyle(el).fontSize);
    });
    if (fontSizes.size > 10) {
      issues.push('too-many-font-sizes:' + fontSizes.size);
    }
  }

  let inlineColorStyles = 0;
  document.querySelectorAll('[style]').forEach(el => {
    const raw = el.getAttribute('style') || '';
    raw.split(';').forEach(part => {
      const decl = part.trim().toLowerCase();
      if (!decl || decl.startsWith('--')) return;
      if (/^(color|background|background-color)\s*:/.test(decl)) inlineColorStyles++;
    });
  });
  if (inlineColorStyles > 0) {
    issues.push('inline-colors:' + inlineColorStyles);
  }

  const primarySelectors = guest
    ? ['button.landing-btn-dark', '.landing-btn-dark']
    : ['.bc-btn-primary', '[class*="btn-primary"]'];
  let primaryCount = 0;
  primarySelectors.forEach(sel => {
    document.querySelectorAll(sel).forEach(el => {
      if (visible(el)) primaryCount++;
    });
  });
  const primaryLimit = guest ? 4 : 3;
  if (primaryCount > primaryLimit) {
    issues.push('too-many-primary-ctas:' + primaryCount);
  }

  if (!guest) {
    const saturated = new Set();
    document.querySelectorAll('.page-content *').forEach(el => {
      if (!visible(el)) return;
      const bg = getComputedStyle(el).backgroundColor;
      const m = bg.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
      if (!m) return;
      const r = +m[1], g = +m[2], b = +m[3];
      const max = Math.max(r, g, b), min = Math.min(r, g, b);
      const area = el.getBoundingClientRect();
      if (area.width * area.height < 8000) return;
      if (max - min > 40 && max > 100) saturated.add(bg);
    });
    if (saturated.size > 2) {
      issues.push('too-many-saturated-blocks:' + saturated.size);
    }

    const content = document.querySelector('.page-content, .stats-grid');
    if (content) {
      const rect = content.getBoundingClientRect();
      const vw = window.innerWidth * window.innerHeight;
      const ratio = (rect.width * rect.height) / Math.max(vw, 1);
      if (ratio > 0.82) issues.push('content-too-dense:' + ratio.toFixed(2));
      if (ratio < 0.12) issues.push('content-too-sparse:' + ratio.toFixed(2));
    }

    const rows = document.querySelectorAll('table tbody tr, .bc-table tbody tr');
    rows.forEach(row => {
      const h = row.getBoundingClientRect().height;
      if (h > 0 && h < 36) issues.push('table-row-too-short');
    });
  }

  const bodySample = document.querySelector(
    guest ? '.login-form p, .landing-page p, main p' : '.page-content p'
  );
  if (bodySample) {
    const fg = getComputedStyle(bodySample).color;
    const bg = getComputedStyle(bodySample.parentElement || document.body).backgroundColor;
    const parse = (c) => {
      const m = c.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
      return m ? [+m[1], +m[2], +m[3]] : [0, 0, 0];
    };
    const lum = (rgb) => {
      const [r, g, b] = rgb.map(v => {
        v /= 255;
        return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
      });
      return 0.2126 * r + 0.7152 * g + 0.0722 * b;
    };
    const l1 = lum(parse(fg)) + 0.05;
    const l2 = lum(parse(bg)) + 0.05;
    const contrast = l1 > l2 ? l1 / l2 : l2 / l1;
    const minContrast = guest ? 3.0 : 4.5;
    if (contrast < minContrast) issues.push('low-body-contrast:' + contrast.toFixed(2));
  }

  return issues.length ? issues.join(';') : 'ok';
})
"#;

fn eval_metrics_ok(browser: &mut AgentBrowser, page: &str, guest: bool) -> anyhow::Result<()> {
    let js = format!("{METRICS_JS_TEMPLATE}({guest})");
    let result = browser.eval(&js)?;
    let status = result.as_str().unwrap_or("").trim();
    if status == "ok" {
        Ok(())
    } else {
        let _ = browser.screenshot(&format!("FAIL-metrics-{}", page));
        anyhow::bail!("Aesthetic metrics failed on {}: {}", page, status);
    }
}

fn seed_review_json(dir: &Path, page_keys: &[&str]) {
    let path = dir.join("review.json");
    if path.exists() {
        return;
    }
    let mut pages = Map::new();
    for key in page_keys {
        pages.insert(
            (*key).to_string(),
            json!({
                "scores": { "C": 0, "F": 0, "D": 0, "A": 0, "R": 0, "P": 0, "E": 0 },
                "p0": ["not-reviewed"],
                "notes": "Agent: score this page from screenshots per aesthetic-optimize-acceptance.md J3",
                "screenshot": format!("{}-viewport.png", key)
            }),
        );
    }
    let review = json!({
        "version": 1,
        "reviewed_at": null,
        "pass": false,
        "pages": pages,
        "global_j4": {
            "page_header_consistent": false,
            "sidebar_consistent": false,
            "card_consistent": false,
            "button_consistent": false,
            "table_consistent": false,
            "empty_state_consistent": false,
            "spacing_consistent": false,
            "motion_consistent": false,
            "notes": "Agent: compare screenshots side-by-side per J4"
        }
    });
    if let Ok(body) = serde_json::to_string_pretty(&review) {
        let _ = fs::write(path, body);
    }
}

fn write_outputs(
    screenshots: &[String],
    metrics: &Value,
    status: &str,
) {
    let dir = artifacts_dir();
    let _ = fs::create_dir_all(&dir);

    let page_keys: Vec<&str> = AESTHETIC_PAGES.iter().map(|p| p.screenshot).collect();
    seed_review_json(&dir, &page_keys);

    let manifest = json!({
        "status": status,
        "screenshots": screenshots,
        "pages": page_keys,
        "review_json": "review.json",
        "metrics_json": "metrics.json",
    });
    let _ = fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap_or_default(),
    );
    let _ = fs::write(
        dir.join("metrics.json"),
        serde_json::to_string_pretty(metrics).unwrap_or_default(),
    );
}

/// Capture aesthetic screenshots + run J1 metrics.
#[tokio::test]
#[ignore = "requires agent-browser and running burncloud server (cargo run -p burncloud-loops -- gate aesthetic-metrics)"]
async fn aesthetic_acceptance() {
    let _ = setup_browser().expect("agent-browser required: npm install -g agent-browser && agent-browser install");
    let base_url = common::spawn_app().await;

    let dir = artifacts_dir();
    let _ = fs::create_dir_all(&dir);
    std::env::set_var("AESTHETIC_ARTIFACTS_DIR", dir.to_string_lossy().to_string());

    let mut browser = AgentBrowser::new(&base_url);
    let mut screenshots = Vec::new();
    let mut metrics_by_page = Map::new();
    let mut console_logged_in = false;

    let mut metrics_failures: Vec<String> = Vec::new();
    let use_preview = e2e_preview_enabled();
    if use_preview {
        eprintln!("E2E: using /preview/* mock routes (E2E_USE_PREVIEW=1)");
    }

    let pages_to_run = aesthetic_pages_to_run();

    for page in pages_to_run {
        let page_started = std::time::Instant::now();
        if !page.guest && !console_logged_in && !use_preview {
            login_as_admin_in_browser(&base_url, &mut browser).await;
            console_logged_in = true;
        }

        let path = e2e_page_path(page.path);
        let open_started = std::time::Instant::now();
        browser.open(&path).expect("open page");
        let open_ms = open_started.elapsed().as_millis() as u64;

        let wait_texts = wait_texts_for(page.screenshot, page.wait_texts);
        let wait_started = std::time::Instant::now();
        let load_result = wait_page_text(&mut browser, wait_texts, 30_000);
        let wait_ms = wait_started.elapsed().as_millis() as u64;

        if let Err(e) = load_result {
            let total_ms = page_started.elapsed().as_millis() as u64;
            eprintln!(
                "[E2E-TIMING] page={} path={} open_ms={open_ms} wait_ms={wait_ms} total_ms={total_ms} status=fail_load",
                page.screenshot, path
            );
            let _ = browser.screenshot(&format!("FAIL-load-{}", page.screenshot));
            write_outputs(&screenshots, &json!({ "status": "fail", "pages": metrics_by_page }), "fail");
            panic!("Page {} did not load: {}", path, e);
        }

        let shot_started = std::time::Instant::now();
        let viewport_name = format!("{}-viewport", page.screenshot);
        let full_name = format!("{}-full", page.screenshot);
        browser
            .screenshot(&viewport_name)
            .expect("viewport screenshot");
        browser.screenshot_full(&full_name).expect("full screenshot");
        screenshots.push(format!("{viewport_name}.png"));
        screenshots.push(format!("{full_name}.png"));
        let _shot_ms = shot_started.elapsed().as_millis();

        let page_status = match eval_metrics_ok(&mut browser, page.screenshot, page.guest) {
            Ok(()) => {
                metrics_by_page.insert(page.screenshot.to_string(), json!({ "status": "pass" }));
                "pass"
            }
            Err(e) => {
                metrics_failures.push(e.to_string());
                metrics_by_page.insert(
                    page.screenshot.to_string(),
                    json!({ "status": "fail", "error": e.to_string() }),
                );
                "fail_metrics"
            }
        };
        let total_ms = page_started.elapsed().as_millis() as u64;
        eprintln!(
            "[E2E-TIMING] page={} path={} open_ms={open_ms} wait_ms={wait_ms} total_ms={total_ms} status={page_status}",
            page.screenshot, path
        );
    }

    let overall_pass = metrics_failures.is_empty();
    let metrics = json!({
        "status": if overall_pass { "pass" } else { "fail" },
        "pages": metrics_by_page,
    });
    write_outputs(
        &screenshots,
        &metrics,
        if overall_pass { "pass" } else { "fail" },
    );

    if overall_pass {
        eprintln!(
            "Aesthetic capture + metrics passed ({} screenshots). Dir: {}",
            screenshots.len(),
            dir.display()
        );
    } else {
        panic!(
            "Aesthetic metrics failed on {} page(s):\n{}",
            metrics_failures.len(),
            metrics_failures.join("\n")
        );
    }
}
