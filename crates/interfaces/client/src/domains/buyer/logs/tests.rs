use super::{
    actions::{status_tone, token_label, trace_metadata},
    model::{find_log, LogEntry, LOG_ENTRIES},
    state::LogsState,
};
use crate::app::router::routes::Route;

#[test]
fn reference_rows_match_the_four_request_records() {
    assert_eq!(LOG_ENTRIES.len(), 4);
    assert_eq!(
        LOG_ENTRIES
            .iter()
            .map(|entry| entry.request_id)
            .collect::<Vec<_>>(),
        vec![
            "req_98fa01b2",
            "req_98fa01b1",
            "req_98fa01b0",
            "req_98fa01af"
        ]
    );
    assert_eq!(LOG_ENTRIES[0].timestamp, "14:28:11.402");
    assert_eq!(LOG_ENTRIES[0].model, "DeepSeek V3 (671B)");
    assert_eq!(LOG_ENTRIES[0].prompt_tokens, 240);
    assert_eq!(LOG_ENTRIES[0].completion_tokens, 820);
    assert_eq!(LOG_ENTRIES[0].cost, "$0.000263");
    assert_eq!(LOG_ENTRIES[2].region, "AP-East (Tokyo)");
    assert_eq!(LOG_ENTRIES[2].failover_count, 1);
    assert_eq!(LOG_ENTRIES[3].total_latency, "512 ms");
}

#[test]
fn log_details_preserve_receipt_enclave_and_trace_values() {
    let entry = find_log("req_98fa01b2").expect("reference request");

    assert_eq!(entry.receipt, "#RCPT-982A1-NITRO");
    assert_eq!(
        entry.enclave,
        "Execution completed inside Confidential Computing enclave with zero telemetry retention."
    );
    assert_eq!(entry.rate_limit_remaining, 1198);
    assert_eq!(entry.total_tokens(), 1_060);
    assert_eq!(entry.token_summary(), "240 in / 820 out");
    assert_eq!(
        entry.trace_metadata(),
        "{\"x-burncloud-tier\":\"standard\",\"x-burncloud-region\":\"US-West (Oregon)\",\"x-burncloud-request-id\":\"req_98fa01b2\",\"x-ratelimit-remaining\":1198,\"failover-count\":0}"
    );
}

#[test]
fn default_state_has_no_selection_and_exposes_all_reference_rows() {
    let state = LogsState::default();

    assert!(state.search.is_empty());
    assert_eq!(state.selected_request_id, None);
    assert_eq!(state.entries(), &LOG_ENTRIES);
    assert_eq!(state.selected_entry(), None);
}

#[test]
fn opening_switching_and_closing_details_updates_selection() {
    let mut state = LogsState::default();

    state.open_details("req_98fa01b2");
    assert_eq!(
        state.selected_entry().expect("first request").request_id,
        "req_98fa01b2"
    );

    state.open_details("req_98fa01b0");
    assert_eq!(
        state.selected_entry().expect("switched request").request_id,
        "req_98fa01b0"
    );

    state.open_details("req_missing");
    assert_eq!(
        state
            .selected_entry()
            .expect("selection remains valid")
            .request_id,
        "req_98fa01b0"
    );

    state.close_details();
    assert_eq!(state.selected_entry(), None);
    assert_eq!(state.selected_request_id, None);
}

#[test]
fn search_and_refresh_do_not_mutate_reference_rows_or_selection() {
    let mut state = LogsState::default();
    state.open_details("req_98fa01b1");
    let before = state.selected_entry().expect("selected request");

    state.set_search("deepseek");
    state.refresh();

    assert_eq!(state.search, "deepseek");
    assert_eq!(state.entries(), &LOG_ENTRIES);
    assert_eq!(state.selected_entry(), Some(before));
}

#[test]
fn log_display_helpers_match_shared_badge_conventions() {
    let entry = LOG_ENTRIES[1];

    assert_eq!(status_tone(entry.status), "healthy");
    assert_eq!(token_label(entry), "110 in / 420 out");
    assert!(trace_metadata(entry).contains("req_98fa01b1"));
    assert_eq!(status_tone("500 Internal Server Error"), "warning");
    assert_eq!(status_tone("pending"), "neutral");
}

#[test]
fn logs_route_parses_to_the_dedicated_page() {
    assert!(matches!("/buyer/logs".parse::<Route>(), Ok(Route::Logs {})));
}

#[test]
fn log_entries_are_copyable_for_drawer_rendering() {
    let first: LogEntry = LOG_ENTRIES[0];
    let second = first;
    assert_eq!(first, second);
}
