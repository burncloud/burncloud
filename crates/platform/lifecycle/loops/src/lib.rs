#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    reason = "This crate is the development-loop harness: it drives external tools (agent-browser, \
              cargo, the API-test binary) and reports what they emit. `clippy.toml` permits dynamic \
              JSON at explicit protocol/adaptor boundaries, and every `serde_json::Value` in this \
              crate is exactly that -- it reads `review.json`, `manifest.json` and `metrics.json`, \
              which are produced by agent-browser and are not defined or written anywhere in this \
              repository, so their schema is owned by that tool rather than by a domain type here. \
              The `unwrap_used`/`expect_used` allowances above are the pre-existing ones; \
              `disallowed_types` is added because the workspace gate fails on this crate without it."
)]

pub mod cli;
pub mod gates;
pub mod lock;
pub mod log;
pub mod loops;
pub mod page_progress;
pub mod paths;
pub mod process;
pub mod prompt;
pub mod server;
pub mod state;
