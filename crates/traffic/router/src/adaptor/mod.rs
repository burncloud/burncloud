pub(crate) mod claude;
pub(crate) mod detector;
pub(crate) mod dynamic;
pub(crate) mod factory;
pub(crate) mod gemini;
pub(crate) mod mapping;
pub(crate) mod vertex;
pub(crate) mod zai;

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current Unix timestamp in seconds.
/// Falls back to 0 if system time is before Unix epoch (extremely rare).
#[inline]
pub(crate) fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Generate a unique chat completion ID following OpenAI convention.
#[inline]
pub(crate) fn generate_chat_id() -> String {
    format!("chatcmpl-{}", uuid::Uuid::new_v4())
}
