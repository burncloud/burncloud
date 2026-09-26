use super::{model::LogEntry, state::LogsState};
use crate::{
    i18n::{strings, Locale, LocaleStrings},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName},
    },
};
use dioxus::prelude::*;

fn close_details(mut state: Signal<LogsState>) {
    state.with_mut(LogsState::close_details);
}

fn render_details(entry: LogEntry, copy: LocaleStrings, state: Signal<LogsState>) -> Element {
    rsx! {
        div { class: "logs-drawer-layer",
            div {
                class: "logs-drawer-backdrop",
                role: "presentation",
                onclick: move |_| close_details(state),
            }
            div { class: "logs-drawer-shell",
                aside {
                    class: "logs-drawer",
                    role: "dialog",
                    aria_modal: "true",
                    aria_label: format!("{} {}", copy.logs_request_prefix, entry.request_id),
                    div { class: "logs-drawer-header",
                        div {
                            h2 { {format!("{} {}", copy.logs_request_prefix, entry.request_id)} }
                            p { {format!("{} • {}", entry.timestamp, entry.region)} }
                        }
                        button {
                            r#type: "button",
                            class: "icon-button logs-drawer-close",
                            title: copy.logs_close,
                            aria_label: copy.logs_close,
                            onclick: move |_| close_details(state),
                            Icon { name: IconName::X, size: 16 }
                        }
                    }
                    div { class: "logs-drawer-body",
                        section { class: "logs-drawer-summary",
                            div { class: "logs-drawer-status",
                                span { {copy.logs_col_status} }
                                span { class: "badge-success", {entry.status} }
                            }
                            div { class: "logs-drawer-metrics",
                                div {
                                    span { {copy.logs_ttft} }
                                    strong { {entry.ttft} }
                                }
                                div {
                                    span { {copy.logs_total_duration} }
                                    strong { {entry.total_latency} }
                                }
                            }
                        }
                        section { class: "logs-detail-section",
                            h4 { {copy.logs_token_cost_heading} }
                            div { class: "logs-detail-card",
                                div { class: "logs-detail-row", span { {format!("{}:", copy.logs_prompt_tokens)} } strong { {entry.prompt_tokens.to_string()} } }
                                div { class: "logs-detail-row", span { {format!("{}:", copy.logs_completion_tokens)} } strong { {entry.completion_tokens.to_string()} } }
                                div { class: "logs-detail-row logs-detail-charge",
                                    span { {format!("{}:", copy.logs_total_charge)} }
                                    strong { {entry.cost} }
                                }
                            }
                        }
                        section { class: "logs-detail-section",
                            h4 { {copy.logs_attestation_heading} }
                            div { class: "logs-attestation",
                                div { class: "logs-attestation-heading",
                                    Icon { name: IconName::Shield, size: 16 }
                                strong { {copy.logs_receipt} }
                                }
                                p { {copy.logs_enclave_description} }
                            }
                        }
                        section { class: "logs-detail-section",
                            h4 { {copy.logs_trace_heading} }
                            pre { class: "logs-trace", {entry.trace_metadata_pretty()} }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn BuyerLogs() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = *strings(locale());
    let mut state = use_signal(LogsState::default);
    let snapshot = state.read().clone();
    let selected = snapshot.selected_entry();

    rsx! {
        BuyerShell {
            div { class: "logs-stack",
                section { class: "page-header logs-page-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.logs_title} }
                        p { {copy.logs_subtitle} }
                    }
                }
                div { class: "logs-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { {copy.logs_conclusion} }
                }
                div { class: "logs-toolbar",
                    label { class: "logs-search",
                        Icon { name: IconName::Search, size: 14 }
                        input {
                            r#type: "search",
                            value: snapshot.search.clone(),
                            placeholder: copy.logs_search_placeholder,
                            aria_label: copy.logs_search_placeholder,
                            oninput: move |event| state.with_mut(|value| value.set_search(event.value())),
                        }
                    }
                    button {
                        r#type: "button",
                        class: "button button-secondary logs-refresh",
                        title: copy.logs_refresh,
                        aria_label: copy.logs_refresh,
                        onclick: move |_| state.with_mut(LogsState::refresh),
                        Icon { name: IconName::RefreshCw, size: 14 }
                    }
                }
                section { class: "panel logs-panel",
                    div { class: "logs-table-scroll",
                        table { class: "logs-table",
                            thead { tr {
                                th {}
                                th { {copy.logs_col_request_id} }
                                th { {copy.logs_col_model} }
                                th {}
                                th {}
                                th { {copy.logs_col_total_latency} }
                                th { {copy.logs_col_tokens} }
                                th { {copy.logs_col_cost} }
                                th {}
                            } }
                            tbody {
                                for entry in snapshot.entries().iter().copied() {
                                    tr {
                                        class: "logs-row",
                                        onclick: move |_| state.with_mut(|value| value.open_details(entry.request_id)),
                                        td { class: "logs-mono", {entry.timestamp} }
                                        td { class: "logs-mono logs-request-id", {entry.request_id} }
                                        td { class: "logs-model",
                                            strong { {entry.model} }
                                            small { {entry.tier} }
                                        }
                                        td { class: "logs-status", span { class: "badge-success", {entry.status} } }
                                        td { class: "logs-mono", {entry.ttft} }
                                        td { class: "logs-mono", {entry.total_latency} }
                                        td { class: "logs-mono", {entry.token_summary()} }
                                        td { class: "logs-mono logs-cost", {entry.cost} }
                                        td {
                                            button {
                                                r#type: "button",
                                                class: "logs-arrow",
                                                aria_label: copy.logs_request_prefix,
                                                onclick: move |event| {
                                                    event.stop_propagation();
                                                    state.with_mut(|value| value.open_details(entry.request_id));
                                                },
                                                "→"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(entry) = selected {
                {render_details(entry, copy, state)}
            }
        }
    }
}
