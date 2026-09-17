// Router core — LLM request/response handling — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

mod adaptor;
pub mod affinity;
mod aimd_limiter;
mod balancer;
pub mod channel_state;
mod circuit_breaker;
mod config;
pub mod exchange_rate;
mod limiter;
pub mod local_attachment;
pub mod model_router;
pub mod order_type;
pub mod passthrough;
pub mod price_sync;
pub mod rate_budget;
pub mod response_parser;
pub mod response_quality;
mod scheduler;
mod state;
pub mod stream_parser;
mod stream_peek;
pub mod token_counter;

/// Peek first chunk timeout (seconds). Used to detect immediate errors (auth, rate limit).
/// Set to same as request timeout - rely on TCP keepalive for server crash detection.
const PEEK_FIRST_CHUNK_TIMEOUT_SECS: u64 = 36000;

// ============================================================
// Empty Response Counter - Track consecutive empty responses
// ============================================================
use std::collections::HashMap;
use std::sync::RwLock as StdRwLock;
