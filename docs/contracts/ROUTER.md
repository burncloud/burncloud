---
doc_id: contract.router-current
doc_type: runtime-contract
truth: source-derived
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# Router — Current Runtime Contract

This is a compact current-behavior contract. It intentionally does not repeat the full Runtime Atlas.

## Entry contract

`crates/server/src/lib.rs :: create_app` installs the data-plane router as the unified app's fallback service.

`crates/router/src/lib.rs :: create_router_app` explicitly registers:

- `GET /v1/models`,
- `GET /api/v1/usage`,
- `GET /api/v1/usage/models`,

then uses `fallback(proxy_handler)` for other unmatched data-plane requests.

Therefore a route such as `POST /v1/chat/completions` is not required to have a dedicated Axum handler registration to enter the router.

## Internal operator endpoints

The router also creates internal routes under the common internal prefix for:

- health,
- forced price sync,
- trip-all circuit breaker,
- Prometheus metrics.

They are returned separately so server composition can merge them before LiveView catch-all behavior.

## Provider execution contract

The fallback path can:

- authenticate/admit a request,
- inspect request data required for routing,
- select/rank candidates,
- choose passthrough or conversion behavior,
- perform external HTTP calls,
- process normal or streaming responses,
- update routing health/failure state,
- collect usage/cost and enqueue/log settlement behavior.

Concrete provider/adaptor implementation may be selected dynamically. Do not document a dynamic adaptor target as a fixed call edge without source proof for that branch.

## Passthrough is conditional

Legacy documentation described “Don't Touch the Body” as universal router law. Current code contains both passthrough and body-parsing/conversion paths. The safe current rule is:

> Preserve native passthrough when the active branch selects it; do not assume every router request is body-opaque.

Before changing protocol behavior, inspect `crates/router/src/passthrough.rs` and the selected branch in `crates/router/src/lib.rs`.

## Failure/failover

Provider errors are not one uniform return path. Error classification, retry/failover decisions, circuit/channel state updates, and affinity effects are branch-dependent. Any change to error handling must inspect the exact status/network/streaming branch and relevant integration tests.

## Human runtime drill-down

The Docusaurus Runtime Flow & ICFG site is `https://burncloud.github.io/`. Use it for navigation, then validate against the current checkout before making changes.
