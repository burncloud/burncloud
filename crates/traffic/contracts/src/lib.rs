//! Traffic-owned contracts: the protocol DTOs and the scheduling colour.
//!
//! Moved here by S1-E from `burncloud_common::types`. The public surface is unchanged: the same
//! names are re-exported from `burncloud_common`, so existing consumers keep compiling.
//!
//! Two rules make this a contract rather than an implementation:
//!
//! * it must not depend on the router implementation crate, so Identity can name the scheduling
//!   colour without depending on Traffic's execution machinery;
//! * it must not depend on `sqlx`/`axum`/`tokio`/`tracing` or any other `burncloud-*` crate. The
//!   purity guard in `tests/contract_purity.rs` enforces both.
//!
//! Ownership background: `docs/architecture-s0-baseline.md` §3 assigns the OpenAI protocol DTOs to
//! "Traffic 的协议契约" and `TrafficColor` to Traffic (Identity supplies the identity fact, Traffic
//! interprets it as a scheduling class).

mod openai;
mod traffic_color;

pub use openai::{OpenAIChatChoice, OpenAIChatMessage, OpenAIChatRequest, OpenAIChatResponse};
pub use traffic_color::TrafficColor;
