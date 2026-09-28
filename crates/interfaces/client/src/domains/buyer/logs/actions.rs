//! Pure display helpers for the buyer request logs page.

use super::model::LogEntry;

/// Maps a log status to the shared status-badge tone.
pub fn status_tone(status: &str) -> &'static str {
    if status.starts_with('2') {
        "healthy"
    } else if status.starts_with('4') || status.starts_with('5') {
        "warning"
    } else {
        "neutral"
    }
}

/// Returns the compact token label shown in the table.
pub fn token_label(entry: LogEntry) -> String {
    entry.token_summary()
}

/// Produces the trace payload shown in the detail drawer.
pub fn trace_metadata(entry: LogEntry) -> String {
    entry.trace_metadata()
}
