# burncloud-router

BurnCloud data-plane routing and upstream execution crate.

## Current role

The router receives unmatched data-plane requests from the unified server fallback, performs request admission/routing, selects upstream candidates, executes provider requests, handles passthrough or conversion branches, tracks response/usage/failure state, and participates in billing/log settlement.

The exact behavior is branch-dependent. Read `src/lib.rs` and the relevant helper module before changing a flow.

## Entry points

`create_router_app()` currently registers three explicit data-plane routes:

- `GET /v1/models`
- `GET /api/v1/usage`
- `GET /api/v1/usage/models`

Other unmatched data-plane paths enter `proxy_handler()` through `.fallback(proxy_handler)`.

Internal operator routes for health, price sync, circuit-breaker trip-all, and metrics are returned separately so the server can merge them before LiveView catch-all behavior.

## Important modules

- `src/lib.rs` — router construction, fallback handler, proxy execution, settlement and internal endpoints.
- `src/model_router.rs` — model/channel candidate loading and ranking.
- `src/passthrough.rs` — passthrough decision logic.
- `src/circuit_breaker.rs` / channel-state related modules — failure/availability state.
- `src/price_sync.rs` — price synchronization.
- adaptor/provider modules — runtime protocol/provider behavior.
- `crates/router-aws` — AWS-specific signing/support code.

Use source search rather than this list as an exhaustive module index.

## Dependency boundary

Current `Cargo.toml` directly depends on database/common crates and currently two `burncloud-service-*` crates:

- `burncloud-service-billing`
- `burncloud-service-user`

`crates/router/scripts/check-router-deps.sh` enforces these two service crates as the current whitelist. Adding another direct `burncloud-service-*` dependency requires deliberate architecture review and updating the enforced rule if accepted.

See:

- `docs/agent/INVARIANTS.md`
- `docs/architecture/CURRENT_SYSTEM.md`
- `docs/contracts/ROUTER.md`

## Passthrough and conversion

Passthrough is **conditional**, not a universal “never parse the body” rule. Current code contains native passthrough branches and parsing/conversion branches. Preserve the semantics of the active path and check `src/passthrough.rs` plus the selected branch in `src/lib.rs`.

## Runtime flow

Human-oriented progressive runtime documentation is rendered at:

https://burncloud.github.io/

Use it for navigation, then re-check the current checkout before modifying code.
