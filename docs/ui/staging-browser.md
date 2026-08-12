---
doc_id: ui.staging-browser
doc_type: verification-guide
truth: source-and-runtime-derived
status: active
---

# BurnCloud Local Staging Browser Review

This is the runtime visual truth path for the current Dioxus console.

Static RSX/CSS review is not sufficient for declaring a UI task complete. For meaningful Console UI changes, use a running BurnCloud server plus a real browser session and inspect the resulting screenshots/click path.

## Primary workflow: local, not GitHub Actions

The browser review runs directly on the developer/Codex machine. GitHub Actions is intentionally not the primary UI review path because it adds queue/install/build latency to every visual iteration.

```text
Local BurnCloud binary
        ↓
Isolated temporary SQLite database
        ↓
BurnCloud server + management APIs + router
        ↓
Dioxus LiveView
        ↓
agent-browser (1440×900)
        ↓
real sidebar clicks + screenshots + report.json/report.md
```

The local runner always supplies `BURNCLOUD_DATABASE_URL` pointing at a dedicated staging database under `target/staging-runtime/`. It does not use `BURNCLOUD_FRESH_DB` against the default database, so normal developer data is not deleted.

## One-time browser install

```bash
npm install -g agent-browser
agent-browser install
```

## One-command staging audit

From the repository root:

```bash
python crates/tests/scripts/run_staging_local.py
```

The runner will:

1. reuse `target/debug/burncloud` when source has not changed;
2. run incremental `cargo build --bin burncloud` only when needed;
3. create an isolated SQLite database;
4. start BurnCloud on `127.0.0.1:3000`;
5. wait for `/health`;
6. seed a deterministic admin, customer, provider, model and API key through real APIs;
7. log in through the visible Dioxus UI;
8. click through the Console with `agent-browser`;
9. write screenshots and machine-readable evidence;
10. stop the server and remove the temporary database.

Useful variants:

```bash
# Force an incremental rebuild before the audit
python crates/tests/scripts/run_staging_local.py --build always

# Reuse the current binary even if source timestamps are newer
python crates/tests/scripts/run_staging_local.py --build never

# Keep the server and its isolated DB alive after the browser audit
python crates/tests/scripts/run_staging_local.py --keep-server

# Keep only the temporary runtime DB for diagnosis
python crates/tests/scripts/run_staging_local.py --keep-runtime

# Use another local port
python crates/tests/scripts/run_staging_local.py --port 3100
```

## Seeded staging state

The audit uses the normal authentication and management APIs. It seeds:

- first-user admin: `staging-admin`;
- business customer: `staging-customer`;
- active dummy provider: `Staging Dummy Provider`;
- model: `staging-model`;
- one active BurnCloud API key.

The dummy upstream is deliberately not invoked. This makes configuration/catalog/access pages realistic without requiring or exposing a paid upstream credential.

## Current browser journey

The audit logs in through the visible `/login` form and follows the rendered Console navigation rather than directly rendering page components:

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

Every run writes to:

```text
target/staging-audit/
├── report.json
├── report.md
├── failure.json          # only on failure
├── server.log
└── screenshots/
    ├── 00-login.png
    ├── 01-overview.png
    ├── 02-providers.png
    ├── 02a-provider-add-drawer.png
    ├── ...
    ├── 13-settings.png
    ├── 99-signed-out.png
    └── zz-failure.png    # only on failure
```

`report.json` records route, viewport, document dimensions, horizontal-overflow state, and visible button/link counts for every audited page.

`failure.json` records the browser error, current path and visible body text so ChatGPT/Codex can diagnose a stopped click path without guessing from source.

## ChatGPT/Codex UI loop

For UI work, use this order:

1. Read the current source for the affected page/behavior.
2. Run `python crates/tests/scripts/run_staging_local.py`.
3. Inspect `target/staging-audit/report.md` and the affected screenshots.
4. State the concrete visual/product/click-path defect before editing.
5. Make the smallest coherent change.
6. Run targeted Rust/static guards for the changed code.
7. Run the local staging browser audit again.
8. Compare the new rendered screenshot and click path with the previous evidence.
9. Continue page-by-page until the visual and click-path acceptance criteria are satisfied.

Do not infer that a page looks correct only because `cargo check` passes.

## Fast third-round review

For the third-round product pass, treat the local staging output as the source of truth for:

- page composition and visual hierarchy;
- spacing, density and overflow;
- loading/empty/error states;
- drawer/modal stability after animation;
- sidebar and in-page click paths;
- whether backend-seeded state is actually visible;
- product wording and control discoverability.

The intended loop is local and repeatable; no CI wait is required between visual edits.

## Persistent/public staging

A persistent public Staging URL is a separate deployment concern. Do not claim one exists unless a real host/tunnel has been configured and verified. If a public staging deployment is later added, point the same `staging_browser_audit.py` acceptance contract at that environment rather than creating a second UI truth model.
