pub mod claude;
pub mod detector;
pub mod dynamic;
pub mod factory;
pub mod gemini;
pub mod mapping;
pub mod vertex;

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current Unix timestamp in seconds.
/// Falls back to 0 if system time is before Unix epoch (extremely rare).
#[inline]
pub fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
