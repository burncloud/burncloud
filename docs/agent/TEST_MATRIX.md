---
doc_id: agent.test-matrix
doc_type: verification-guide
truth: source-derived
status: active
---

# Test Matrix

The repository's integration/E2E crate is `crates/tests`. Its current tests live under `crates/tests/tests/`, including `api/`, `e2e/`, provider-specific suites, and deployment/provider flows.

Security and billing also have dedicated invariant suites close to the owning runtime crates:

- `crates/server/tests/security_invariants.rs` — management/data-plane credential separation, admin authorization, token ownership/redaction, internal control-plane authentication.
- `crates/router/tests/billing_invariants.rs` — log/settlement separation, key-scoped spend, legacy API-key settlement, rotation-transition settlement.
- `.github/workflows/security-billing-invariants.yml` — mandatory PR gate when the protected runtime areas change.

## Default verification ladder

1. Format changed Rust: `cargo fmt --check` (or format before checking).
2. Check affected package(s): `cargo check -p <package>`.
3. Run affected package tests: `cargo test -p <package>`.
4. Run relevant invariant suite when a protected cross-cutting truth is involved.
5. Run relevant `burncloud-tests` target(s) for the user flow.
6. Run broader workspace checks when shared APIs/dependencies changed.

Do not claim tests passed unless they actually ran in the current environment.

## Area → minimum evidence

| Area | Minimum test scope to inspect/run |
|---|---|
| Router core / failover / provider execution | `burncloud-router` tests + relevant files under `crates/tests/tests/api/` |
| Channel routing | router tests + `api/ability_routing.rs` + channel/provider regression tests |
| Auth / login / password | server/user service tests + `api/auth.rs`, `api/auth_handlers.rs`, `e2e/auth_flow.rs`, `e2e/login_flow.rs` |
| Management authorization / security boundary | `cargo test -p burncloud-server --test security_invariants` + adjacent auth/API tests |
| Channel Console CRUD | server/channel/service/database tests + `api/channel.rs`, `e2e/channel_flow.rs` |
| API key/token flows | `security_invariants` + token/service/database tests + `e2e/api_key_flow.rs` and related API tests |
| Billing / usage / quota settlement | `cargo test -p burncloud-router --test billing_invariants --test quota_tests` + service-billing/database-billing tests + provider billing integrations such as `api/gemini_billing.rs` when relevant |
| Internal control-plane mutations | `security_invariants::sensitive_internal_mutations_require_internal_secret` + affected internal handler tests |
| UI / Console styling | affected client crate + `e2e/console_pages.rs`, `css_visual_acceptance.rs`, `aesthetic_acceptance.rs` as applicable |
| Shared database utilities | affected database crates; test both dialect-sensitive code paths where tests support them |
| Workspace dependency/API changes | targeted tests first, then `cargo check --workspace` and the relevant integration suites |

## Security + Billing invariant gate

Changes under the server/router/router-database/router-log ownership paths are checked by `.github/workflows/security-billing-invariants.yml`. Treat a failure here as a violated business/security contract, not as an optional regression signal.

The invariant gate answers these minimum questions:

1. Can a management JWT cross into the inference data plane?
2. Can a non-admin execute an admin management action?
3. Can one user enumerate or mutate another user's API token?
4. Can a sensitive internal mutation run without the configured internal secret?
5. Can usage-log insertion mutate spend quota?
6. Can settling one credential mutate another credential's spend?
7. Can missing or rotation-transition credentials create an unmetered path?

## Test discovery rule

This matrix is a navigation aid, not a complete generated index. Before editing a flow, inspect `crates/tests/tests/` and the owning crate's invariant tests for tests that reference the exact route, symbol, provider, or state being changed.

## Failure handling

If an existing relevant test contradicts a proposed behavior change:

- do not silently update the test to make the PR green;
- first determine whether the test expresses a current contract or is stale;
- document that decision in the PR description.
