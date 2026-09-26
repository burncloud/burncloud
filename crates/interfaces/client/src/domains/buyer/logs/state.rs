//! Local interaction state for the buyer request logs page.

use super::model::{find_log, LogEntry, LOG_ENTRIES};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LogsState {
    pub search: String,
    pub selected_request_id: Option<&'static str>,
}

impl LogsState {
    /// Updates the search input without filtering the static reference rows.
    pub fn set_search(&mut self, search: impl Into<String>) {
        self.search = search.into();
    }

    /// Opens the detail drawer for a known request.
    ///
    /// Unknown IDs are ignored so a stale DOM event cannot make the state point
    /// at a record that cannot be rendered.
    pub fn open_details(&mut self, request_id: impl AsRef<str>) {
        if let Some(entry) = find_log(request_id.as_ref()) {
            self.selected_request_id = Some(entry.request_id);
        }
    }

    /// Closes the detail drawer and clears the selected request.
    pub fn close_details(&mut self) {
        self.selected_request_id = None;
    }

    /// Returns the selected request, if the drawer is currently open.
    pub fn selected_entry(&self) -> Option<LogEntry> {
        self.selected_request_id.and_then(find_log)
    }

    /// Returns all static rows. Search is intentionally display-only for now.
    pub const fn entries(&self) -> &'static [LogEntry; 4] {
        &LOG_ENTRIES
    }

    /// Refresh is intentionally a no-op until the logs API is connected.
    pub fn refresh(&mut self) {}
}
