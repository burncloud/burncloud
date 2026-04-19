pub mod claude;
pub mod detector;
pub mod dynamic;
pub mod factory;
pub mod gemini;
pub mod mapping;
pub mod vertex;
pub mod zai;

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

/// Generate a unique chat completion ID following OpenAI convention.
#[inline]
pub fn generate_chat_id() -> String {
    format!("chatcmpl-{}", uuid::Uuid::new_v4())
}
