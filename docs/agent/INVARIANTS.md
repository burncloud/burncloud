---
doc_id: agent.invariants
doc_type: engineering-invariants
truth: source-derived
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# Verified Engineering Invariants

These are high-value facts visible in current code. If a change intentionally alters one, update code, tests, and this file together.

## Runtime and route composition

### INV-RUNTIME-001 — `server` and `router` CLI subcommands currently share server startup

`src/main.rs` dispatches both `server` and `router` to `run_async_server()`, which calls `burncloud_server::start_server(...)`.

**Evidence:** `src/main.rs :: main`, `run_async_server`.

### INV-RUNTIME-002 — Server is a unified Axum application

`burncloud_server::create_app()` composes:

- top-level `/health`,
- management/public/protected API routes,
- router internal endpoints,
- optional Dioxus LiveView,
- the data-plane router as `fallback_service`.

**Evidence:** `crates/server/src/lib.rs :: create_app`.

### INV-ROUTER-001 — Data plane uses explicit utility routes plus a fallback

`create_router_app()` explicitly registers:

- `GET /v1/models`,
- `GET /api/v1/usage`,
- `GET /api/v1/usage/models`,

and sends other unmatched data-plane requests to `proxy_handler()`.

**Evidence:** `crates/router/src/lib.rs :: create_router_app`.

### INV-ROUTER-002 — Internal router routes must precede LiveView catch-all behavior

Router internal health/price-sync/circuit-breaker/metrics routes are constructed separately and merged by server before optional LiveView. Current source comments and composition both enforce this ordering.

**Evidence:** `crates/router/src/lib.rs :: create_router_app`; `crates/server/src/lib.rs :: create_app`.

## Authentication

### INV-AUTH-001 — Public and protected management routes are separate routers

`crates/server/src/api/mod.rs` merges public auth routes without auth middleware and wraps the protected route group with `auth_middleware`.

**Evidence:** `crates/server/src/api/mod.rs :: routes`.

## Database

### INV-DB-001 — SQLite and PostgreSQL placeholder syntax is abstracted

`crates/database/src/placeholder.rs` provides `ph`, `phs`, and `adapt_sql` to generate/translate placeholders for SQLite (`?`) and PostgreSQL (`$n`). New cross-database SQL should not assume one placeholder dialect when the query is expected to work on both databases.

**Evidence:** `crates/database/src/placeholder.rs`.

## Workspace

### INV-WORKSPACE-001 — Shared dependency versions are centralized

Root `Cargo.toml` declares `[workspace.dependencies]`; workspace crates predominantly consume shared dependencies with `workspace = true`.

### INV-WORKSPACE-002 — Clippy denies `unwrap_used`

Root workspace lint config currently sets `unwrap_used = "deny"`, `expect_used = "warn"`, and `disallowed_types = "deny"`.

## Non-invariants

Do **not** treat these legacy ideas as current invariants:

- “Server may only call Service and may never depend on Database.” Current `burncloud-server` directly depends on database crates.
- “Every E2E test mirrors the HTTP path as its filesystem path.” Current tests are organized under `crates/tests/tests/` by API/E2E/provider concerns, not strict route-path mirroring.
- “Router never parses a request body.” Current router code contains parsing/conversion paths; passthrough is conditional, not a universal no-parse law.
