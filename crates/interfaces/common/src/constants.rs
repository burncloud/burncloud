//! Interfaces URL constants.
//!
//! S1-F removed `API_PREFIX`, `WS_PATH`, `get_base_url` and `get_api_url`: a rename-based probe
//! (see #615) showed they had no consumer anywhere in the workspace. The two that remain are
//! consumed, so they stay with the Interfaces layer that owns the URL layout -- they are not
//! Kernel material and not a domain fact.

/// Default HTTP port, used by the CLI when no port is given.
pub const DEFAULT_PORT: u16 = 3000;

/// Internal (non-public) console prefix, used by the router's health path and price-sync calls.
pub const INTERNAL_PREFIX: &str = "/console/internal";
