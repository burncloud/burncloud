<div align="center">

# BurnCloud

![Rust](https://img.shields.io/badge/Built_with-Rust-orange?style=for-the-badge&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)
![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-blue?style=for-the-badge)

**Rust-native AI Gateway & Management Platform**

[Runtime Flow & ICFG Atlas](https://burncloud.github.io/) · [AI Agent Engineering Docs](docs/README.md) · [Issues / Planning](https://github.com/burncloud/burncloud/issues)

</div>

---

## What is BurnCloud?

BurnCloud is a Rust workspace for routing and operating AI API traffic. The current repository contains a unified Axum server, a data-plane router, management APIs, service/database crates, a Dioxus client, provider/adaptor code, billing/usage logic, and integration/E2E tests.

The repository is evolving quickly, so current source and executable tests are the authority for behavior. The engineering docs are intentionally code-first and avoid mixing future product plans with implemented behavior.

## Current executable shape

The process entry is `src/main.rs`.

- `crates/server` builds the unified Axum application.
- `crates/router` owns the data-plane fallback and upstream execution.
- `crates/service/*` contains service/business components.
- `crates/database/*` contains database core and domain persistence code using SQLx.
- `crates/client` and `crates/client/crates/*` contain Dioxus client/features.
- `crates/tests` is the integration/E2E test crate.

`burncloud_server::create_app()` currently composes management routes, router internal endpoints, optional LiveView, and the data-plane router as a fallback service. See [Current System Shape](docs/architecture/CURRENT_SYSTEM.md) for the source-derived summary.

### Router behavior

The router supports both native passthrough and request/response conversion paths. Passthrough is conditional; do not assume every request body is opaque. Provider/adaptor selection can also be dynamic at runtime.

For the progressive user-action → End-to-End Flow → ICFG → Source view, use the [Runtime Atlas](https://burncloud.github.io/).

## Getting started

### Requirements

- A current stable Rust toolchain suitable for this workspace.
- Windows, Linux, or macOS. The desktop GUI path is Windows-specific in current `src/main.rs`; non-Windows uses the server/LiveView path.

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

`crates/router/src/lib.rs :: create_router_app` explicitly registers:

- `GET /v1/models`
- `GET /api/v1/usage`
- `GET /api/v1/usage/models`

Other unmatched data-plane requests enter `proxy_handler()` through the router fallback. For example, `POST /v1/chat/completions` reaches the data plane through this fallback rather than a dedicated Axum Chat handler registration.

A request requires valid runtime configuration such as credentials/tokens and usable upstream Channel configuration.

## Development and tests

Start with the AI-agent/task docs even when working manually:

- [Start Here](docs/agent/START_HERE.md)
- [Task Router](docs/agent/TASK_ROUTER.md)
- [Verified Invariants](docs/agent/INVARIANTS.md)
- [Test Matrix](docs/agent/TEST_MATRIX.md)

Typical local checks include targeted package checks/tests plus the relevant integration/E2E flow. Provider/cloud tests may require environment credentials; do not treat unavailable external tests as a pass.

Repository-wide formatting/lint commands include:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
```

Root workspace Clippy configuration currently denies `unwrap_used`, warns on `expect_used`, and denies configured disallowed types.

## Documentation policy

`docs/` is an AI-agent engineering harness, not a product roadmap archive.

Truth priority:

`Current source > executable tests > current contracts/invariants > current architecture docs > engineering standards > explanatory docs`.

Future product planning belongs in GitHub Issues/Projects/PRs until implemented.

## Contributing

Before changing code, read [docs/README.md](docs/README.md). Route the task by user/runtime behavior, inspect the real source path, preserve verified invariants, and run the relevant tests.

## License

MIT License © BurnCloud contributors
