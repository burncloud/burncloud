---
doc_id: ui.staging-browser
doc_type: verification-guide
truth: source-and-runtime-derived
status: active
---

# BurnCloud Staging Browser Review

This is the runtime visual truth path for the current Dioxus console.

Static RSX/CSS review is not sufficient for declaring a UI task complete. For meaningful Console UI changes, prefer evidence from a running BurnCloud server plus a real browser session.

## Runtime topology

The GitHub Actions workflow `.github/workflows/staging-browser.yml` starts an isolated real BurnCloud process:

```text
Fresh SQLite database
        ↓
BurnCloud server + management APIs + router
        ↓
Dioxus LiveView
        ↓
agent-browser (1440×900)
        ↓
click-path checks + screenshots + report.json/report.md
```

The audit uses the normal authentication and management APIs. It seeds:

- one first-user admin (`staging-admin`);
- one business customer (`staging-customer`);
- one active dummy OpenAI-compatible provider;
- one model (`staging-model`);
- one active BurnCloud API key.

The dummy upstream is deliberately not invoked. This makes the configuration/catalog/access pages realistic without requiring or exposing a paid upstream credential.

## Current browser journey

The audit logs in through the visible `/login` form and follows the Console navigation rather than directly rendering page components:

```text
Login
  → Overview
  → Providers
      → open Add Provider drawer → Cancel
  → Models
  → Routes
  → Playground
  → Logs
  → Evaluation
  → Billing
  → API Keys
      → open Create API Key drawer → Cancel
  → Customers
      → open Create Customer drawer → Cancel
  → Guardrails
  → Team
  → Settings
  → Sign Out
```

The browser is fixed to **1440×900**. Each primary Console page is checked for horizontal document overflow.

## Evidence contract

Workflow artifact name:

`burncloud-staging-browser-audit`

Expected contents:

```text
target/staging-audit/
├── report.json
├── report.md
├── server.log
└── screenshots/
    ├── 00-login.png
    ├── 01-overview.png
    ├── 02-providers.png
    ├── 02a-provider-add-drawer.png
    ├── ...
    ├── 13-settings.png
    └── 99-signed-out.png
```

`report.json` is machine-readable and records route, viewport, document dimensions, horizontal-overflow state, and visible button/link counts for every audited page.

`report.md` is appended to the GitHub Actions step summary.

## Agent workflow

For a UI task, ChatGPT/Codex should use this order:

1. Read the current source for the affected behavior.
2. Read the latest Staging Browser Audit result when one exists.
3. Inspect the screenshot for the affected page and adjacent click-path states.
4. State the concrete visual/product defect before editing.
5. Make the smallest coherent UI change.
6. Run static UI/functional/product guards.
7. Run the Staging Browser Audit again.
8. Compare the new screenshot and click path with the previous evidence.

Do not infer that a page looks correct only because `cargo check` passes.

## Local run

Start BurnCloud with an isolated database and disabled initial price sync, then run the ignored browser test:

```bash
export HOST=127.0.0.1
export PORT=3000
export E2E_BASE_URL=http://127.0.0.1:3000
export BURNCLOUD_FRESH_DB=1
export MASTER_KEY=a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8
export SKIP_INITIAL_PRICE_SYNC=1
export PRICE_SYNC_INTERVAL_SECS=999999

cargo build --bin burncloud
./target/debug/burncloud server
```

In another shell:

```bash
npm install -g agent-browser@0.33.0
agent-browser install --with-deps
cargo test -p burncloud-tests --test staging_browser -- --ignored --nocapture
```

## Public staging

The CI staging environment is intentionally ephemeral and private to the GitHub runner. It is enough for automated visual/click-path evidence and for agents that can read GitHub Actions artifacts.

A persistent public Staging URL is a separate deployment concern. Do not claim one exists unless a real host/tunnel has been configured and verified. When a public staging deployment is added, keep this same browser audit as the acceptance contract and point a deployment-specific runner at that URL rather than creating a second UI truth model.
