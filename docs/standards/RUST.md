---
doc_id: standard.rust
doc_type: engineering-standard
truth: source-derived
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# Rust / Workspace Standards

Only rules with current repository support are listed here.

## Workspace dependency versions

Shared dependency versions are declared in root `[workspace.dependencies]`. When a dependency is shared across workspace crates, prefer `workspace = true` in child manifests instead of introducing a second independent version.

## Workspace lints

Root `Cargo.toml` currently declares:

- `unwrap_used = "deny"`,
- `expect_used = "warn"`,
- `disallowed_types = "deny"`.

Do not state that `expect()` is universally compile-forbidden; the current workspace policy is warning-level, while `unwrap()` is denied by Clippy workspace configuration.

## Async code

The project uses Tokio broadly, including Axum, reqwest, SQLx, background tasks, and server startup. Preserve async boundaries for I/O unless a specific implementation requires blocking work and isolates it appropriately.

## Logging

Current server/router code uses `tracing` and some existing code may use `log`. Follow the local module's established structured logging approach; do not introduce `println!`/`eprintln!` into server/router operational paths for routine logging.

Note: `src/main.rs` contains existing user-facing/startup `println!`/`eprintln!` calls. This document does not falsely claim those macros are absent repository-wide.

## Error handling

Use the error style already established by the crate (`anyhow` at application boundaries, typed errors such as `thiserror` in libraries where applicable). Avoid introducing panic-based control flow into production paths.

## Before adding a rule

A style preference is not a repository invariant merely because it is common Rust practice. Add rules here only when they are reflected by current code, lints, CI, or an explicitly accepted maintainer decision.
