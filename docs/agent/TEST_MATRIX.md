---
doc_id: agent.test-matrix
doc_type: verification-guide
truth: source-derived
status: active
audited_against: b49df42d9660833974e80534ad738e6a51d80926
---

# Test Matrix

The repository's integration/E2E crate is `crates/tests`. Its current tests live under `crates/tests/tests/`, including `api/`, `e2e/`, provider-specific suites, and deployment/provider flows.

## Default verification ladder

1. Format changed Rust: `cargo fmt --check` (or format before checking).
2. Check affected package(s): `cargo check -p <package>`.
3. Run affected package tests: `cargo test -p <package>`.
4. Run relevant `burncloud-tests` target(s) for the user flow.
5. Run broader workspace checks when shared APIs/dependencies changed.

Do not claim tests passed unless they actually ran in the current environment.

## Area → minimum evidence

| Area | Minimum test scope to inspect/run |
|---|---|
| Router core / failover / provider execution | `burncloud-router` tests + relevant files under `crates/tests/tests/api/` |
| Channel routing | router tests + `api/ability_routing.rs` + channel/provider regression tests |
| Auth / login / password | server/user service tests + `api/auth.rs`, `api/auth_handlers.rs`, `e2e/auth_flow.rs`, `e2e/login_flow.rs` |
| Channel Console CRUD | server/channel/service/database tests + `api/channel.rs`, `e2e/channel_flow.rs` |
| API key/token flows | token/service/database tests + `e2e/api_key_flow.rs` and related API tests |
| Billing / usage | router + service-billing/database-billing tests + billing/provider integration tests such as `api/gemini_billing.rs` when relevant |
| UI / Console behavior or styling | affected client crate + static UI/functional/product guards + `staging_browser` runtime audit; use older `e2e/console_pages.rs`, `css_visual_acceptance.rs`, `aesthetic_acceptance.rs` only after confirming their route/text expectations are still current |
| Shared database utilities | affected database crates; test both dialect-sensitive code paths where tests support them |
| Workspace dependency/API changes | targeted tests first, then `cargo check --workspace` and the relevant integration suites |

For current Dioxus visual/click-path work, see `docs/ui/staging-browser.md`. A passing compile is not visual acceptance.

## Test discovery rule

This matrix is a navigation aid, not a complete generated index. Before editing a flow, inspect `crates/tests/tests/` for tests that reference the exact route, symbol, provider, or state being changed.

## Failure handling

If an existing relevant test contradicts a proposed behavior change:

- do not silently update the test to make the PR green;
- first determine whether the test expresses a current contract or is stale;
- document that decision in the PR description.
