use std::collections::{BTreeMap, BTreeSet};

use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{Channel, ChannelService},
};

fn status_label(channel: &Channel) -> &'static str {
    if channel.status == 1 { "Active" } else { "Down" }
}

#[component]
pub fn Models() -> Element {
    let mut resource = use_resource(move || async move { ChannelService::list(100).await });
    let snapshot = resource.read().clone();
    let error = snapshot.as_ref().and_then(|r| r.as_ref().err().cloned());
    let channels = snapshot.and_then(Result::ok).unwrap_or_default();
    let no_channels = channels.is_empty();

    let mut model_map: BTreeMap<String, (usize, usize, BTreeSet<String>)> = BTreeMap::new();
    for channel in &channels {
        for model in channel.models.split(',').map(str::trim).filter(|m| !m.is_empty()) {
            let entry = model_map
                .entry(model.to_string())
                .or_insert((0, 0, BTreeSet::new()));
            entry.0 += 1;
            if channel.status == 1 {
                entry.1 += 1;
            }
            entry.2.insert(channel.name.clone());
        }
    }

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Models" }
                    p { class: "page-subtitle", "Derived from real Channel.models. The server has no independent model CRUD API." }
                }
                button { class: "button button-secondary", onclick: move |_| resource.restart(), "Refresh" }
            }

            if let Some(message) = error {
                div { class: "terminal auth-status auth-status-error", "{message}" }
            } else {
                div { class: "card table-card",
                    if no_channels {
                        div { class: "card-pad small muted", "No channels configured." }
                    } else {
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr {
                                    th { "Model" }
                                    th { class: "right", "Providers" }
                                    th { class: "right", "Active" }
                                    th { "Source Channels" }
                                    th { "Management" }
                                } }
                                tbody {
                                    for (model, (providers, active, sources)) in model_map {
                                        {
                                            let source_text = sources.into_iter().collect::<Vec<_>>().join(", ");
                                            rsx! {
                                                tr { key: "{model}",
                                                    td { class: "table-primary mono", "{model}" }
                                                    td { class: "right tabular", "{providers}" }
                                                    td { class: "right tabular", "{active}" }
                                                    td { "{source_text}" }
                                                    td { Link { class: "button button-ghost button-sm", to: Route::Providers {}, "Edit Providers" } }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Routes() -> Element {
    let mut resource = use_resource(move || async move { ChannelService::list(100).await });
    let snapshot = resource.read().clone();
    let error = snapshot.as_ref().and_then(|r| r.as_ref().err().cloned());
    let channels = snapshot.and_then(Result::ok).unwrap_or_default();

    let mut groups: BTreeMap<String, Vec<Channel>> = BTreeMap::new();
    for channel in channels {
        groups.entry(channel.group.clone()).or_default().push(channel);
    }
    for rows in groups.values_mut() {
        rows.sort_by_key(|channel| (channel.priority, -channel.weight));
    }

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Routes" }
                    p { class: "page-subtitle", "Derived from Channel.group, priority and weight — the routing configuration BurnCloud actually stores." }
                }
                button { class: "button button-secondary", onclick: move |_| resource.restart(), "Refresh" }
            }

            if let Some(message) = error {
                div { class: "terminal auth-status auth-status-error", "{message}" }
            } else if groups.is_empty() {
                div { class: "card card-pad small muted", "No routing groups can be derived until provider channels are configured." }
            } else {
                div { class: "stack-lg",
                    for (group_name, rows) in groups {
                        div { class: "card card-pad stack",
                            div { class: "row between",
                                div {
                                    h3 { "{group_name}" }
                                    span { class: "small muted", "{rows.len()} channel candidates" }
                                }
                                Link { class: "button button-secondary button-sm", to: Route::Providers {}, "Manage Channels" }
                            }
                            div { class: "table-wrap",
                                table { class: "data-table",
                                    thead { tr {
                                        th { "Order" }
                                        th { "Provider" }
                                        th { "Models" }
                                        th { class: "right", "Priority" }
                                        th { class: "right", "Weight" }
                                        th { "Status" }
                                    } }
                                    tbody {
                                        for (index, channel) in rows.iter().enumerate() {
                                            {
                                                let order = index + 1;
                                                let status = status_label(channel);
                                                rsx! {
                                                    tr { key: "{channel.id}",
                                                        td { class: "mono", "#{order}" }
                                                        td { class: "table-primary", "{channel.name}" }
                                                        td { class: "mono muted", "{channel.models}" }
                                                        td { class: "right tabular", "{channel.priority}" }
                                                        td { class: "right tabular", "{channel.weight}" }
                                                        td { span { class: if channel.status == 1 { "badge badge-success" } else { "badge badge-error" }, "{status}" } }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
