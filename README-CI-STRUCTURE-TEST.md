<div align="center">

# BurnCloud

![Rust](https://img.shields.io/badge/Built_with-Rust-orange?style=for-the-badge&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)
![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-blue?style=for-the-badge)

**Rust-native AI Gateway & Management Platform**

[Runtime Flow & ICFG Atlas](https://burncloud.github.io/) · [Issues / Planning](https://github.com/burncloud/burncloud/issues)

</div>

---

## What is BurnCloud?

BurnCloud is a Rust workspace for routing and operating AI API traffic. The current repository contains a unified Axum server, a data-plane router, management APIs, service/database crates, a Dioxus client, provider/adaptor code, billing/usage logic, and integration/E2E tests.

The repository is evolving quickly, so current source and executable tests are the authority for behavior.

## Current executable shape

The process entry is `crates/interfaces/cli/src/main.rs`.

- `crates/interfaces/common` preserves the legacy shared contract package pending domain-owned contract extraction; no existing type is claimed as Kernel.
- `crates/identity`, `crates/supply`, `crates/traffic`, `crates/commerce`, and `crates/trust` contain domain-owned packages.
- `crates/platform` contains configuration, lifecycle, observability, storage, and repository tooling packages.
- `crates/interfaces/server` builds the unified Axum application, while `crates/interfaces/client` contains the Dioxus client.
- `crates/interfaces/service` preserves the existing service facade, and `crates/interfaces/tests` contains integration/E2E tests.

`burncloud_server::create_app()` currently composes management routes, router internal endpoints, optional LiveView, and the data-plane router as a fallback service.

### Router behavior

The router supports both native passthrough and request/response conversion paths. Passthrough is conditional; do not assume every request body is opaque. Provider/adaptor selection can also be dynamic at runtime.

For the progressive user-action → End-to-End Flow → ICFG → Source view, use the [Runtime Atlas](https://burncloud.github.io/).

## Getting started

### Requirements

- A current stable Rust toolchain suitable for this workspace.
- Windows, Linux, or macOS. The desktop GUI path is Windows-specific in current `crates/interfaces/cli/src/main.rs`; non-Windows uses the server/LiveView path.

### Build

```bash
git clone https://github.com/burncloud/burncloud.git
cd burncloud
cp .env.example .env
cargo build
```

### Run

```bash
# Unified server path
cargo run -- server

# Current code routes `router` through the same run_async_server() path
cargo run -- router

# Desktop client on Windows; non-Windows prints server guidance
cargo run -- client
```

Current defaults in source:

| Setting | Default |
|---|---|
| `HOST` | `127.0.0.1` |
| `PORT` | `3000` |

Other configuration is environment-driven; use `.env.example` and current source as the reference rather than copying values from old docs.

## Data-plane entry

`crates/traffic/router/src/lib.rs :: create_router_app` explicitly registers:

- `GET /v1/models`
- `GET /api/v1/usage`
- `GET /api/v1/usage/models`

Other unmatched data-plane requests enter `proxy_handler()` through the router fallback. For example, `POST /v1/chat/completions` reaches the data plane through this fallback rather than a dedicated Axum Chat handler registration.

A request requires valid runtime configuration such as credentials/tokens and usable upstream Channel configuration.

## Development and tests

Initialize each source checkout once:

```bash
cargo run -- code init
```

`code init` installs local Git `pre-commit` and `commit-msg` hooks without starting the application or
creating runtime secrets. It checks for rustfmt, Clippy and cargo-deny, and installs
missing tools with `rustup component add` or `cargo install --locked cargo-deny`.
If an installation fails, existing hooks remain unchanged; rerun `code init` after
resolving the installation error. It is safe to repeat and also works from a checkout's
subdirectory or linked Git worktree. Windows requires Git for Windows (including
its bundled shell); the hook uses no PowerShell or Unix-only package manager.
Existing regular hooks are saved as `<hook>.burncloud-original` and run along with
the BurnCloud checks. Configured `core.hooksPath` and symlink hooks are left
untouched: integrate the managed sequence (cargo fmt, `cargo run -- code test --staged`,
cargo clippy, cargo deny) into that hook manager instead, or remove the custom setting
before initializing.

Run the same local automation yourself:

```bash
cargo run -- code test                         # select from local changes
cargo run -- code test --plan                  # explain selection without checking
cargo run -- code test --base origin/main      # include committed branch changes
cargo run -- code test --all                   # full workspace, even if clean
cargo run -- code test --last                  # latest result, counts and local log paths
```

`code test` uses Cargo's dependency graph to select changed packages and every
transitive workspace consumer, then runs only the affected Cargo tests with
`--no-default-features`. Root workspace/build configuration and unknown paths can
still force a full-workspace test, while `.github/**`, `clippy.toml` and `deny.toml`
do not expand the test scope. A package's own `Cargo.toml` follows that package's
dependency closure. A clean checkout or test-irrelevant change explicitly reports
that no tests ran; use `--all` to override.

The managed pre-commit hook runs `cargo fmt --all -- --check`, then
`code test --staged`, strict workspace Clippy, and `cargo deny check`. Stage or
stash tracked changes and non-ignored untracked files before committing: Cargo checks
the working tree, so partial staging is rejected. Missing tools and failed commands
block the commit.
Each run records a JSON summary and individual Cargo logs under the repository's
Git directory (`.git/burncloud/checks/`). The terminal shows ✅ or ❌ per check and
counts passed, failed and ignored tests by unit, integration and doc suite. On
successful commits, `commit-msg` appends `BurnCloud-Checks`, `BurnCloud-Fmt`,
`BurnCloud-Tests`, `BurnCloud-Clippy`, `BurnCloud-Deny` and the checked staged-tree
hash to the commit message. For documentation-only commits, fmt/Clippy/deny still report `PASS` while the test receipt reports `SKIP`, because no test-relevant package was selected. Full logs stay local; the compact receipt travels with the
commit. A changed index or HEAD after the checks blocks stamping.
External test prerequisites, ignored files, existing lint/advisory policies and
platform-specific CI still apply. This local command does not replace remote
branch protection or claim to run every CI script/feature combination.

The implementation and regression tests live in the independent Rust package
[`burncloud-code`](crates/platform/code/README.md). `cargo test -p burncloud-code`
checks initialization, selection and real Git commit blocking on Windows and Linux
without building application services; Cargo check execution is stubbed in those
regression tests. There is no Python or Shell test executor. The lightweight
`cargo run -p burncloud-code -- test` also works when application compilation blocks
the root command, using exactly the same Rust implementation.

Typical local checks include targeted package checks/tests plus the relevant integration/E2E flow. Provider/cloud tests may require environment credentials; do not treat unavailable external tests as a pass.

Repository-wide formatting/lint commands include:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
```

Root workspace Clippy configuration currently denies `unwrap_used`, warns on `expect_used`, and denies configured disallowed types.

## Contributing

Before changing code, inspect the real source path, preserve existing behavior, and run the relevant tests. Keep personal agent instructions and working documentation in ignored local `AGENTS.md` and `docs/` paths.

## License

MIT License © BurnCloud contributors
