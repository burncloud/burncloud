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
    clippy::panic_in_result_fn,
    reason = "integration tests: #[tokio::test] is not recognised by clippy.toml's allow-panic-in-tests, so a failed assertion is the intended failure signal; fixture formatting is intentionally plain"
)]
mod api;
mod common;
mod e2e;
