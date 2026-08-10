---
doc_id: agent.task-router
doc_type: agent-routing
truth: source-derived
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# Task Router — Behavior to Source

Use this before repository-wide search. Paths are starting points, not substitutes for reading code.

| Task / user behavior | Primary source | Related source | Tests / evidence to inspect |
|---|---|---|---|
| Data-plane request entry, fallback routing | `crates/router/src/lib.rs` (`create_router_app`, `proxy_handler`, `proxy_logic`) | `crates/server/src/lib.rs` (`create_app`) | `crates/tests/tests/api/relay.rs`, provider-specific API tests |
| Channel candidate loading / ranking / affinity | `crates/router/src/model_router.rs` | router scheduler/affinity/channel-state modules | `crates/tests/tests/api/ability_routing.rs`, routing/provider tests |
| Provider passthrough / conversion / retry | `crates/router/src/lib.rs`, `crates/router/src/passthrough.rs` | `crates/router/crates/router-aws`, adaptor modules | `crates/tests/tests/api/claude_relay.rs`, `gemini_passthrough.rs`, `gemini_regression.rs` and affected provider tests |
| Streaming usage / response handling | `crates/router/src/lib.rs` | `burncloud-service-billing` parser/usage code | streaming/provider regression tests under `crates/tests/tests/api/` |
| Billing / cost / quota settlement | `crates/router/src/lib.rs` | `crates/service/crates/billing`, `crates/database/crates/billing` | `crates/tests/tests/api/gemini_billing.rs`, affected billing/router tests |
| Public register/login/password reset | `crates/server/src/api/auth.rs` | `crates/service/crates/user`, `crates/database/crates/user` | `crates/tests/tests/api/auth.rs`, `api/auth_handlers.rs`, `e2e/auth_flow.rs`, `e2e/login_flow.rs` |
| Protected Console API authentication | `crates/server/src/api/mod.rs`, auth middleware implementation | JWT/user service code | auth API/E2E tests |
| Channel CRUD in Console | `crates/server/src/api/channel.rs` | `crates/service/crates/channel`, `crates/database/crates/channel` | `crates/tests/tests/api/channel.rs`, `e2e/channel_flow.rs` |
| API token management | `crates/server/src/api/token.rs` | `crates/service/crates/token`, router/database token code | token/API-key flow tests, especially `crates/tests/tests/e2e/api_key_flow.rs` |
| Logs / usage / monitoring | server log/monitor API modules | router log service/database and monitor service | `crates/tests/tests/api/log.rs`, `api/monitor.rs` |
| UI / Console page behavior | affected crate under `crates/client/crates/` | `crates/client` shared components/routes | `crates/tests/tests/e2e/console_pages.rs`, `css_visual_acceptance.rs`, `aesthetic_acceptance.rs`, relevant flow tests |
| Process startup / CLI dispatch | `src/main.rs`, `src/cli/` | `crates/server`, `crates/client` | affected crate tests and startup/status tests |
| Database dialect / SQL | `crates/database`, affected child database crate | `crates/database/src/placeholder.rs` | affected database/service tests plus relevant E2E flow |
| Installer / download / update | corresponding crate under `crates/installer`, `crates/download`, `crates/auto-update` | caller crate | package-local tests/examples and affected E2E |

## If the task does not fit

1. Identify the user-visible or operator-visible entry point.
2. Locate the route/CLI/UI event/background trigger.
3. Follow direct calls until the first stable ownership boundary.
4. Add a new row here **only if that behavior will recur as an engineering task**.

Do not add a row solely because a new file exists.
