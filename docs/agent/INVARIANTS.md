---
doc_id: agent.invariants
doc_type: engineering-invariants
truth: source-derived
status: active
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

The unified server also applies `security_boundary_middleware` across explicit routes and the data-plane fallback.

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

## Authentication and authorization

### INV-AUTH-001 — Public and protected management routes are separate routers

`crates/server/src/api/mod.rs` merges public auth routes without auth middleware and wraps the protected route group with `auth_middleware`.

**Evidence:** `crates/server/src/api/mod.rs :: routes`.

### INV-AUTH-002 — Console JWTs are management-plane credentials, not inference credentials

The unified server rejects a valid Console JWT when it is presented as the bearer/API credential for `/v1/*` or `/api/v1/*`. Data-plane inference must use an API credential rather than a management session token.

**Evidence:** `crates/server/src/api/auth.rs :: security_boundary_middleware`; `crates/server/tests/security_invariants.rs :: console_jwt_cannot_authenticate_data_plane`.

### INV-AUTH-003 — Administrative Console operations require current admin authorization

Channel, logs/usage administration, monitor/security, and cache management routes require both a valid JWT and the current `admin` role from the database. User registration from the Console, user listing, and balance top-up also perform admin authorization in their handlers.

**Evidence:** `crates/server/src/api/mod.rs :: routes`; `crates/server/src/api/auth.rs :: admin_middleware`; `crates/server/src/api/user.rs`; `crates/server/tests/security_invariants.rs :: regular_users_cannot_execute_admin_management_actions`.

### INV-AUTH-004 — API-token management is owner-scoped with admin override

A non-admin authenticated user may list/create/manage only their own router tokens. Administrators may manage tokens across users. List/detail responses expose a token hint, not the bearer secret; create and rotate are the explicit one-time secret disclosure points.

**Evidence:** `crates/server/src/api/token.rs`; `crates/server/tests/security_invariants.rs :: token_management_is_owner_scoped_and_redacted`.

## Internal control plane

### INV-INTERNAL-001 — Sensitive internal mutations fail closed without the internal secret

`POST /console/internal/prices/sync` and `POST /console/internal/circuit-breaker/trip-all` require `BURNCLOUD_INTERNAL_SECRET` to be configured and the matching value in `X-Internal-Secret`. If the server has no configured secret, these mutations are unavailable rather than unauthenticated.

**Evidence:** `crates/server/src/api/auth.rs :: security_boundary_middleware`; `crates/server/tests/security_invariants.rs :: sensitive_internal_mutations_require_internal_secret`.

## Billing and quota

### INV-BILLING-001 — Credential quota fields represent spend, not token counts

`router_tokens.quota_limit` / `router_tokens.used_quota` and the corresponding `user_api_keys` quota values used by settlement are nanodollar spend values. Usage-log insertion does not mutate spend quota.

**Evidence:** `crates/database/crates/router/src/token.rs :: RouterTokenModel::{check_quota,deduct_quota}`; `crates/database/crates/router/src/lib.rs :: RouterDatabase::insert_log`; `crates/router/tests/billing_invariants.rs :: router_log_insert_never_mutates_spend_quota`.

### INV-BILLING-002 — Spend settlement is credential-scoped and actual cost is durable

Settlement updates only the credential that authorized the request. A missing/inactive credential fails closed. If exact post-response cost crosses a configured cap, the actual cost is still recorded and settlement returns `false`; subsequent quota checks reject further spend. A valid old key during a rotation transition settles against its canonical current token.

**Evidence:** `crates/database/crates/router/src/token.rs :: RouterTokenModel::{check_quota,deduct_quota}`; `crates/router/tests/quota_tests.rs`; `crates/router/tests/billing_invariants.rs`.

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
