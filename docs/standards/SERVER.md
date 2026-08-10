---
doc_id: standard.server
doc_type: engineering-standard
truth: source-derived
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# Server / Axum Standards

## Start from current route composition

Management routes are composed in `crates/server/src/api/mod.rs`. Public auth routes and protected routes are separate; the protected router is layered with `auth_middleware`.

Do not place a route based solely on its URL prefix. Check whether it must be public, protected, internal, data-plane, or LiveView before registering it.

## `AppState`

`crates/server/src/lib.rs` defines a cloneable `AppState` containing shared DB/monitor/user/cache/sync state. Existing handlers typically extract `State<AppState>` from Axum.

Follow the existing state shape unless a change requires a new shared dependency. If adding state, consider clone/Arc ownership and whether it belongs at server scope or in a lower-level service.

## API response helpers

`crates/server/src/api/response.rs` defines typed `ok(data)` and `err(message)` helpers producing the common `{ success, data }` / `{ success, message }` JSON shapes.

Use these helpers where the surrounding management API follows that contract. Do not rewrite specialized router/provider responses into this management API shape.

## Dependency reality

Do not enforce the deleted legacy rule “Handler may never call Database”. Current server modules and `burncloud-server` dependencies include database access. Prefer existing service boundaries where they already own business behavior, but treat that as a design choice to evaluate, not a fabricated hard invariant.

## Route changes require flow-level verification

When adding/changing a route, check:

- public vs protected middleware,
- route ordering/catch-all behavior,
- LiveView interaction,
- state extraction,
- response contract,
- integration/E2E tests for the user flow.
