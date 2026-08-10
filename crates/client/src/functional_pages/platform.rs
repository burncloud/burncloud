use std::collections::{BTreeMap, BTreeSet};

use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{Channel, ChannelService},
    components::Icon,
};

fn channel_status(channel: &Channel) -> &'static str {
    if channel.status == 1 { "Active" } else { "Down" }
}

#[component]
pub fn Providers() -> Element {
    let mut channels = use_resource(move || async move { ChannelService::list(100).await });
    let mut editor_open = use_signal(|| false);
    let mut editing_id = use_signal(|| 0i32);
    let mut name = use_signal(String::new);
    let mut provider_type = use_signal(|| 1i32);
    let mut key = use_signal(String::new);
    let mut base_url = use_signal(String::new);
    let mut models = use_signal(String::new);
    let mut group = use_signal(|| "default".to_string());
    let mut weight = use_signal(|| 100i32);
    let mut priority = use_signal(|| 0i64);
    let mut rpm_cap = use_signal(String::new);
    let mut tpm_cap = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(String::new);
    let mut error = use_signal(String::new);

    let result = channels.read().clone();
    let loading = result.is_none();
    let load_error = result.as_ref().and_then(|r| r.as_ref().err().cloned());
    let list = result.and_then(Result::ok).unwrap_or_default();
    let active = list.iter().filter(|c| c.status == 1).count();

    let reset_form = move || {
        editing_id.set(0);
        name.set(String::new());
        provider_type.set(1);
        key.set(String::new());
        base_url.set(String::new());
        models.set(String::new());
        group.set("default".to_string());
        weight.set(100);
        priority.set(0);
        rpm_cap.set(String::new());
        tpm_cap.set(String::new());
    };

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Providers" }
                    p { class: "page-subtitle", "Backed by BurnCloud ChannelService. Provider credentials, models, groups, priority and capacity limits are real server configuration." }
                }
                div { class: "header-actions",
                    button { class: "button button-secondary", onclick: move |_| channels.restart(), "Refresh" }
                    button {
                        class: "button button-primary",
                        onclick: move |_| {
                            reset_form();
                            error.set(String::new());
                            notice.set(String::new());
                            editor_open.set(true);
                        },
                        Icon { name: "plus" }
                        "Add Provider"
                    }
                }
            }

            div { class: "metrics",
                div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Providers" } span { class: "metric-value", "{list.len()}" } } div { class: "metric-icon tone-blue", Icon { name: "providers" } } }
                div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Active" } span { class: "metric-value", "{active}" } } div { class: "metric-icon tone-green", Icon { name: "activity" } } }
            }

            if !notice().is_empty() { div { class: "terminal auth-status", "{notice}" } }
            if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }

            if loading {
                div { class: "card card-pad", "Loading providers…" }
            } else if let Some(message) = load_error {
                div { class: "card card-pad stack", strong { class: "danger", "Unable to load channels" } code { class: "terminal", "{message}" } }
            } else {
                div { class: "card table-card",
                    if list.is_empty() {
                        div { class: "card-pad small muted", "No provider channels are configured." }
                    } else {
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr {
                                    th { "Provider" }
                                    th { "Type" }
                                    th { "Models" }
                                    th { "Group" }
                                    th { class: "right", "Priority" }
                                    th { class: "right", "Weight" }
                                    th { "Capacity" }
                                    th { "Status" }
                                    th { "" }
                                } }
                                tbody {
                                    for channel in list {
                                        {
                                            let status = channel_status(&channel);
                                            let capacity = format!("RPM {} • TPM {}", channel.rpm_cap.map(|v| v.to_string()).unwrap_or_else(|| "∞".to_string()), channel.tpm_cap.map(|v| v.to_string()).unwrap_or_else(|| "∞".to_string()));
                                            rsx! {
                                                tr { key: "{channel.id}",
                                                    td {
                                                        div { class: "two-line", span { class: "table-primary", "{channel.name}" } small { class: "mono", "{channel.base_url.clone().unwrap_or_else(|| "default base URL".to_string())}" } }
                                                    }
                                                    td { class: "mono", "{channel.type_}" }
                                                    td { class: "mono muted", style: "max-width:320px", "{channel.models}" }
                                                    td { "{channel.group}" }
                                                    td { class: "right tabular", "{channel.priority}" }
                                                    td { class: "right tabular", "{channel.weight}" }
                                                    td { class: "tiny muted", "{capacity}" }
                                                    td { span { class: if channel.status == 1 { "badge badge-success" } else { "badge badge-error" }, "{status}" } }
                                                    td {
                                                        div { class: "row gap-2",
                                                            button {
                                                                class: "button button-ghost button-sm",
                                                                onclick: move |_| {
                                                                    editing_id.set(channel.id);
                                                                    name.set(channel.name.clone());
                                                                    provider_type.set(channel.type_);
                                                                    key.set(channel.key.clone());
                                                                    base_url.set(channel.base_url.clone().unwrap_or_default());
                                                                    models.set(channel.models.clone());
                                                                    group.set(channel.group.clone());
                                                                    weight.set(channel.weight);
                                                                    priority.set(channel.priority);
                                                                    rpm_cap.set(channel.rpm_cap.map(|v| v.to_string()).unwrap_or_default());
                                                                    tpm_cap.set(channel.tpm_cap.map(|v| v.to_string()).unwrap_or_default());
                                                                    error.set(String::new());
                                                                    editor_open.set(true);
                                                                },
                                                                "Edit"
                                                            }
                                                            button {
                                                                class: "button button-ghost button-sm danger",
                                                                disabled: busy(),
                                                                onclick: move |_| {
                                                                    let id = channel.id;
                                                                    busy.set(true);
                                                                    error.set(String::new());
                                                                    spawn(async move {
                                                                        match ChannelService::delete(id).await {
                                                                            Ok(()) => { notice.set(format!("Provider channel {id} deleted.")); channels.restart(); }
                                                                            Err(message) => error.set(format!("Delete provider failed: {message}")),
                                                                        }
                                                                        busy.set(false);
                                                                    });
                                                                },
                                                                "Delete"
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

            if editor_open() {
                div { class: "drawer-backdrop", onclick: move |_| editor_open.set(false) }
                aside { class: "drawer",
                    div { class: "drawer-head",
                        h2 { if editing_id() > 0 { "Edit Provider" } else { "Add Provider" } }
                        button { class: "close-button", onclick: move |_| editor_open.set(false), "×" }
                    }
                    div { class: "drawer-body stack-lg",
                        div { class: "grid-2",
                            div { class: "field", label { "Name" } input { class: "input", value: "{name}", oninput: move |evt| name.set(evt.value()) } }
                            div { class: "field", label { "Provider Type ID" } input { class: "input", r#type: "number", value: "{provider_type}", oninput: move |evt| provider_type.set(evt.value().parse().unwrap_or(1)) } }
                        }
                        div { class: "field", label { "API Key / Credential" } input { class: "input mono", r#type: "password", value: "{key}", oninput: move |evt| key.set(evt.value()) } }
                        div { class: "field", label { "Base URL (optional)" } input { class: "input mono", value: "{base_url}", oninput: move |evt| base_url.set(evt.value()) } }
                        div { class: "field", label { "Models (comma-separated)" } textarea { class: "textarea mono", rows: "4", value: "{models}", oninput: move |evt| models.set(evt.value()) } }
                        div { class: "grid-2",
                            div { class: "field", label { "Group" } input { class: "input", value: "{group}", oninput: move |evt| group.set(evt.value()) } }
                            div { class: "field", label { "Weight" } input { class: "input", r#type: "number", value: "{weight}", oninput: move |evt| weight.set(evt.value().parse().unwrap_or(100)) } }
                            div { class: "field", label { "Priority" } input { class: "input", r#type: "number", value: "{priority}", oninput: move |evt| priority.set(evt.value().parse().unwrap_or(0)) } }
                            div { class: "field", label { "RPM Cap" } input { class: "input", r#type: "number", value: "{rpm_cap}", oninput: move |evt| rpm_cap.set(evt.value()) } }
                            div { class: "field", label { "TPM Cap" } input { class: "input", r#type: "number", value: "{tpm_cap}", oninput: move |evt| tpm_cap.set(evt.value()) } }
                        }
                        p { class: "tiny subtle", "Provider Type is the integer enum used by BurnCloud Channel. This page intentionally does not invent a second provider schema." }
                        if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }
                        button {
                            class: "button button-primary",
                            disabled: busy(),
                            onclick: move |_| {
                                let channel = Channel {
                                    id: editing_id(),
                                    type_: provider_type(),
                                    key: key(),
                                    name: name().trim().to_string(),
                                    base_url: if base_url().trim().is_empty() { None } else { Some(base_url().trim().to_string()) },
                                    models: models().trim().to_string(),
                                    group: group().trim().to_string(),
                                    status: 1,
                                    weight: weight(),
                                    priority: priority(),
                                    param_override: None,
                                    header_override: None,
                                    api_version: None,
                                    model_mapping: None,
                                    rpm_cap: rpm_cap().trim().parse().ok(),
                                    tpm_cap: tpm_cap().trim().parse().ok(),
                                };
                                if channel.name.is_empty() || channel.key.is_empty() || channel.models.is_empty() {
                                    error.set("Name, credential and at least one model are required.".to_string());
                                    return;
                                }
                                busy.set(true);
                                error.set(String::new());
                                let is_update = channel.id > 0;
                                spawn(async move {
                                    let result = if is_update { ChannelService::update(&channel).await } else { ChannelService::create(&channel).await };
                                    match result {
                                        Ok(()) => { notice.set(if is_update { "Provider updated.".to_string() } else { "Provider created.".to_string() }); editor_open.set(false); channels.restart(); }
                                        Err(message) => error.set(format!("Provider save failed: {message}")),
                                    }
                                    busy.set(false);
                                });
                            },
                            if busy() { "Saving…" } else if editing_id() > 0 { "Save Provider" } else { "Create Provider" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Models() -> Element {
    let mut channels = use_resource(move || async move { ChannelService::list(100).await });
    let result = channels.read().clone();
    let load_error = result.as_ref().and_then(|r| r.as_ref().err().cloned());
    let list = result.and_then(Result::ok).unwrap_or_default();
    let mut models: BTreeMap<String, (usize, usize, BTreeSet<String>)> = BTreeMap::new();
    for channel in &list {
        for model in channel.models.split(',').map(str::trim).filter(|m| !m.is_empty()) {
            let entry = models.entry(model.to_string()).or_insert((0, 0, BTreeSet::new()));
            entry.0 += 1;
            if channel.status == 1 { entry.1 += 1; }
            entry.2.insert(channel.name.clone());
        }
    }

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div { h2 { class: "page-title", "Models" } p { class: "page-subtitle", "Derived from the real model lists on configured provider channels. BurnCloud currently has no independent model CRUD API." } }
                button { class: "button button-secondary", onclick: move |_| channels.restart(), "Refresh" }
            }
            if let Some(message) = load_error {
                div { class: "terminal auth-status auth-status-error", "{message}" }
            } else {
                div { class: "card table-card",
                    div { class: "table-wrap",
                        table { class: "data-table",
                            thead { tr { th { "Model" } th { class: "right", "Providers" } th { class: "right", "Active" } th { "Source Channels" } th { "Management" } } }
                            tbody {
                                for (model, (providers, active, sources)) in models {
                                    {
                                        let source_text = sources.into_iter().collect::<Vec<_>>().join(", ");
                                        rsx! {
                                            tr { key: "{model}",
                                                td { class: "table-primary mono", "{model}" }
                                                td { class: "right tabular", "{providers}" }
                                                td { class: "right tabular", "{active}" }
                                                td { class: "muted", "{source_text}" }
                                                td { Link { class: "button button-ghost button-sm", to: Route::Providers {}, "Edit Providers" } }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if list.is_empty() { div { class: "card-pad small muted", "No channels are configured, so no models can be derived." } }
                }
            }
        }
    }
}

#[component]
pub fn Routes() -> Element {
    let mut channels = use_resource(move || async move { ChannelService::list(100).await });
    let result = channels.read().clone();
    let load_error = result.as_ref().and_then(|r| r.as_ref().err().cloned());
    let list = result.and_then(Result::ok).unwrap_or_default();
    let mut groups: BTreeMap<String, Vec<Channel>> = BTreeMap::new();
    for channel in list {
        groups.entry(channel.group.clone()).or_default().push(channel);
    }
    for entries in groups.values_mut() {
        entries.sort_by_key(|c| (c.priority, -c.weight));
    }

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div { h2 { class: "page-title", "Routes" } p { class: "page-subtitle", "Routing groups are derived from Channel.group, priority and weight — the fields the router actually uses. There is no separate route CRUD API." } }
                button { class: "button button-secondary", onclick: move |_| channels.restart(), "Refresh" }
            }
            if let Some(message) = load_error {
                div { class: "terminal auth-status auth-status-error", "{message}" }
            } else if groups.is_empty() {
                div { class: "card card-pad small muted", "No routing groups can be derived until provider channels are configured." }
            } else {
                div { class: "stack-lg",
                    for (group_name, entries) in groups {
                        div { class: "card card-pad stack",
                            div { class: "row between",
                                div { h3 { "{group_name}" } span { class: "small muted", "{entries.len()} channel candidates" } }
                                Link { to: Route::Providers {}, class: "button button-secondary button-sm", "Manage Channels" }
                            }
                            div { class: "table-wrap",
                                table { class: "data-table",
                                    thead { tr { th { "Order" } th { "Provider" } th { "Models" } th { class: "right", "Priority" } th { class: "right", "Weight" } th { "Status" } } }
                                    tbody {
                                        for (index, channel) in entries.iter().enumerate() {
                                            {
                                                let order = index + 1;
                                                let status = channel_status(channel);
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
