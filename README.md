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
rustup component add rustfmt clippy
cargo install cargo-deny --locked
cargo run -- code init
```

`code init` installs a local Git `pre-commit` hook without starting the application or
creating runtime secrets. It is safe to repeat and also works from a checkout's
subdirectory or linked Git worktree. Windows requires Git for Windows (including
its bundled shell); the hook uses no PowerShell or Unix-only package manager.
An existing regular hook is saved as `pre-commit.burncloud-original` and runs after
the BurnCloud checks pass. Configured `core.hooksPath` and symlink hooks are left
untouched: integrate `sh .github/scripts/pre-commit-checks.sh` into that hook manager
instead, or remove the custom setting before initializing.

Every commit must pass, in order:

```bash
cargo fmt --all -- --check
cargo test --workspace --no-default-features
cargo clippy --workspace --all-targets --no-default-features
cargo deny check
```

The first failure blocks the commit, including missing tools. Stage or stash all
tracked changes and untracked files first: Cargo checks the working tree, so the
hook rejects partial staging. Ignored local files remain subject to the normal
test environment requirements. These checks use the existing workspace lint and
`deny.toml` policies, including advisory checks; they do not add warning overrides
or suppress existing failures. Workspace tests/formatting or known dependency
advisories can therefore block commits until those underlying issues are fixed.
`--no-default-features` matches the Linux Clippy scope; platform desktop CI remains
necessary. Local hooks are not GitHub branch protection and can be bypassed by Git
options, so required remote checks must be configured separately if needed.

Hook regression tests run independently of application dependencies on Linux:
`python3 .github/scripts/test-code-init.py`. They compile the actual Rust installer,
exercise real Git commits, and stub Cargo only to verify gate ordering and failure
propagation. They do not establish that the full workspace passes its checks.

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
