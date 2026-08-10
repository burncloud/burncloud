---
doc_id: agent.bootstrap
doc_type: agent-bootstrap
truth: normative
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# BurnCloud — AI Agent Bootstrap

Read this first. Target reading time: under two minutes.

## Operating rule

**Code first. Do not infer behavior from filenames, crate names, old architecture conventions, or product intent.**

For every task:

1. Open [`agent/START_HERE.md`](agent/START_HERE.md).
2. Route the task with [`agent/TASK_ROUTER.md`](agent/TASK_ROUTER.md).
3. Read the listed source before proposing a change.
4. Check [`agent/INVARIANTS.md`](agent/INVARIANTS.md).
5. Run the relevant scope from [`agent/TEST_MATRIX.md`](agent/TEST_MATRIX.md).
6. Follow [`agent/CHANGE_PROTOCOL.md`](agent/CHANGE_PROTOCOL.md).

## Current executable shape

- Workspace members are declared in root `Cargo.toml`.
- Process entry is `src/main.rs`.
- `server` and `router` subcommands both call the same `run_async_server()` path today.
- `burncloud-server` composes management APIs, optional Dioxus LiveView, internal router endpoints, and the data-plane fallback.
- `burncloud-router` owns the data-plane fallback and upstream request execution.
- The router has explicit `/v1/models` and `/api/v1/usage*` routes; unmatched data-plane paths fall back to `proxy_handler()`.
- Protected Console APIs are composed in `crates/server/src/api/mod.rs` and wrapped with `auth_middleware`.

## Important correction from legacy docs

Do **not** assume a strict `Server → Service → Database` dependency chain. Current `burncloud-server` directly depends on database crates, services, the router, common code, and the client. Current architecture documentation describes what exists, not an idealized layering rule.

## Repository-enforced Rust facts

Root workspace lints currently include:

- `clippy::unwrap_used = deny`
- `clippy::expect_used = warn`
- `clippy::disallowed_types = deny`

Dependency versions belong in `[workspace.dependencies]` when shared across workspace crates.

## Runtime documentation

For progressive runtime-flow reading, use `https://burncloud.github.io/` and start from the user action. Treat source links in that site as evidence, but re-check current repository code before modifying behavior.

## Never do this

- Treat a roadmap as implemented behavior.
- Create a call edge because it “looks likely”.
- Copy an old pattern without checking current source.
- Change routing/billing/auth behavior without checking its tests and downstream state effects.
- Add screenshots or binary documentation assets under `docs/`.
