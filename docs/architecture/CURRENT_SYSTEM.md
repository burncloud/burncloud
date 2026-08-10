---
doc_id: architecture.current-system
doc_type: current-architecture
truth: source-derived
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# Current System Shape

This page describes repository organization and executable composition visible in current code. It is not an aspirational layering diagram.

## Workspace

Root `Cargo.toml` declares a Rust workspace containing, among others:

- `crates/router` — data-plane routing/upstream execution.
- `crates/server` — unified Axum server and management APIs.
- `crates/service/*` — business/service crates.
- `crates/database/*` — database core and domain persistence crates.
- `crates/client` and `crates/client/crates/*` — Dioxus client/features.
- `crates/tests` — integration/E2E test crate.
- installer/download/update/loops/support crates.

Read root `Cargo.toml` for the authoritative member list.

## Process entry

`src/main.rs` is the application entry.

Current dispatch behavior:

- no args on Windows: start server in a background thread, then launch GUI/tray;
- no args on non-Windows: run headless server;
- `client`: Windows GUI path; non-Windows prints guidance;
- `server` and `router`: both execute `run_async_server()`;
- other subcommands: delegated to root CLI handling.

## Unified server composition

`crates/server/src/lib.rs :: create_app` currently constructs one Axum application:

1. initialize monitor/cache;
2. create data-plane router + internal router endpoints;
3. create management API router;
4. add `/health`;
5. optionally merge Dioxus LiveView;
6. attach the data-plane router as fallback;
7. attach request-id, trace, and CORS layers.

This means “Server” is not merely a thin presentation layer in the current implementation.

## Current dependency reality

`burncloud-server` directly depends on:

- core database + domain database crates,
- `burncloud-router`,
- `burncloud-client`,
- multiple service crates,
- common/shared crates.

`burncloud-router` directly depends on database crates, common, and currently `burncloud-service-billing` and `burncloud-service-user` among other dependencies.

Do not enforce an imagined dependency rule from old docs. When changing dependencies, inspect the actual `Cargo.toml` files and avoid creating cycles.

## Runtime flow map

For human-oriented progressive flow/ICFG documentation, use `https://burncloud.github.io/`.

For code changes, begin with [`../agent/TASK_ROUTER.md`](../agent/TASK_ROUTER.md), then re-confirm every relevant branch in source.
