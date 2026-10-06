#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::let_unit_value,
    clippy::redundant_pattern,
    clippy::manual_is_multiple_of,
    clippy::let_and_return,
    clippy::to_string_trait_impl,
    clippy::to_string_in_format_args,
    clippy::redundant_pattern_matching,
    reason = "integration-test code: panicking on fixture failures is the intended signal, and fixtures use plain Value formatting and patterns"
)]
pub(crate) mod ability_routing;
pub(crate) mod auth;
pub(crate) mod auth_handlers;
pub(crate) mod channel;
pub(crate) mod claude_relay;
pub(crate) mod gemini_3_pro_image;
pub(crate) mod gemini_billing;
pub(crate) mod gemini_passthrough;
pub(crate) mod gemini_region_pricing;
pub(crate) mod gemini_regression;
pub(crate) mod gemini_thinking;
pub(crate) mod log;
pub(crate) mod monitor;
pub(crate) mod relay;
pub(crate) mod status;
pub(crate) mod user;
