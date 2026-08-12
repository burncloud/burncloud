use dioxus::prelude::*;

use crate::{
    backend::{Channel, ChannelService},
    components::Icon,
    functional_api::update_channel_preserving_reservations,
};

#[component]
pub fn Providers() -> Element {
    let mut resource = use_resource(move || async move { ChannelService::list(100).await });
    let mut editing = use_signal(|| None::<Channel>);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(String::new);
    let mut error = use_signal(String::new);

    let mut name = use_signal(String::new);
    let mut provider_type = use_signal(|| 1i32);
    let mut credential = use_signal(String::new);
    let mut base_url = use_signal(String::new);
    let mut models = use_signal(String::new);
    let mut group = use_signal(|| "default".to_string());
    let mut weight = use_signal(|| 100i32);
    let mut priority = use_signal(|| 0i64);
    let mut rpm_cap = use_signal(String::new);
    let mut tpm_cap = use_signal(String::new);

    let snapshot = resource.read().clone();
    let load_error = snapshot.as_ref().and_then(|r| r.as_ref().err().cloned());
    let channels = snapshot.and_then(Result::ok).unwrap_or_default();
    let total = channels.len();
    let active = channels.iter().filter(|c| c.status == 1).count();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Providers" }
                    p { class: "page-subtitle", "Direct Channel CRUD. Edits preserve existing L2 reservation thresholds even though those advanced fields are not exposed in this form." }
                }
                div { class: "header-actions",
                    button { class: "button button-secondary", onclick: move |_| resource.restart(), "Refresh" }
                    button {
                        class: "button button-primary",
                        onclick: move |_| {
                            editing.set(Some(Channel::default()));
                            name.set(String::new());
                            provider_type.set(1);
                            credential.set(String::new());
                            base_url.set(String::new());
                            models.set(String::new());
                            group.set("default".to_string());
                            weight.set(100);
                            priority.set(0);
                            rpm_cap.set(String::new());
                            tpm_cap.set(String::new());
                            error.set(String::new());
                        },
                        Icon { name: "plus" }
                        "Add Provider"
                    }
                }
            }

            div { class: "metrics",
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Providers" } span { class: "metric-value", "{total}" } }
                    div { class: "metric-icon tone-blue", Icon { name: "providers" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Active" } span { class: "metric-value", "{active}" } }
                    div { class: "metric-icon tone-green", Icon { name: "activity" } }
                }
            }

            if !notice().is_empty() { div { class: "terminal auth-status", "{notice}" } }
            if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }

            if let Some(message) = load_error {
                div { class: "terminal auth-status auth-status-error", "{message}" }
            } else {
                div { class: "card table-card",
                    if channels.is_empty() {
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
                                    th { "Actions" }
                                } }
                                tbody {
                                    for channel in channels {
                                        {
                                            let edit_channel = channel.clone();
                                            let delete_id = channel.id;
                                            let base = channel.base_url.clone().unwrap_or_else(|| "default base URL".to_string());
                                            let rpm = channel.rpm_cap.map(|v| v.to_string()).unwrap_or_else(|| "unlimited".to_string());
                                            let tpm = channel.tpm_cap.map(|v| v.to_string()).unwrap_or_else(|| "unlimited".to_string());
                                            let capacity = format!("RPM {rpm} • TPM {tpm}");
                                            let status = if channel.status == 1 { "Active" } else { "Down" };
                                            rsx! {
                                                tr { key: "{channel.id}",
                                                    td {
                                                        div { class: "two-line",
                                                            span { class: "table-primary", "{channel.name}" }
                                                            small { class: "mono", "{base}" }
                                                        }
                                                    }
                                                    td { class: "mono", "{channel.type_}" }
                                                    td { class: "mono muted", "{channel.models}" }
                                                    td { "{channel.group}" }
                                                    td { class: "right tabular", "{channel.priority}" }
                                                    td { class: "right tabular", "{channel.weight}" }
                                                    td { class: "small muted", "{capacity}" }
                                                    td { span { class: if channel.status == 1 { "badge badge-success" } else { "badge badge-error" }, "{status}" } }
                                                    td {
                                                        div { class: "row gap-2",
                                                            button {
                                                                class: "button button-ghost button-sm",
                                                                onclick: move |_| {
                                                                    name.set(edit_channel.name.clone());
                                                                    provider_type.set(edit_channel.type_);
                                                                    credential.set(edit_channel.key.clone());
                                                                    base_url.set(edit_channel.base_url.clone().unwrap_or_default());
                                                                    models.set(edit_channel.models.clone());
                                                                    group.set(edit_channel.group.clone());
                                                                    weight.set(edit_channel.weight);
                                                                    priority.set(edit_channel.priority);
                                                                    rpm_cap.set(edit_channel.rpm_cap.map(|v| v.to_string()).unwrap_or_default());
                                                                    tpm_cap.set(edit_channel.tpm_cap.map(|v| v.to_string()).unwrap_or_default());
                                                                    error.set(String::new());
                                                                    editing.set(Some(edit_channel.clone()));
                                                                },
                                                                "Edit"
                                                            }
                                                            button {
                                                                class: "button button-ghost button-sm danger",
                                                                disabled: busy(),
                                                                onclick: move |_| {
                                                                    busy.set(true);
                                                                    error.set(String::new());
                                                                    spawn(async move {
                                                                        match ChannelService::delete(delete_id).await {
                                                                            Ok(()) => {
                                                                                notice.set("Provider deleted.".to_string());
                                                                                resource.restart();
                                                                            }
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

            if let Some(current) = editing() {
                {
                    let current_id = current.id;
                    let current_status = current.status;
                    let current_param_override = current.param_override.clone();
                    let current_header_override = current.header_override.clone();
                    let current_api_version = current.api_version.clone();
                    let current_model_mapping = current.model_mapping.clone();
                    rsx! {
                        div { class: "drawer-backdrop", onclick: move |_| editing.set(None) }
                        aside { class: "drawer",
                            div { class: "drawer-head",
                                h2 { if current_id > 0 { "Edit Provider" } else { "Add Provider" } }
                                button { class: "close-button", onclick: move |_| editing.set(None), "×" }
                            }
                            div { class: "drawer-body stack-lg",
                                div { class: "grid-2",
                                    div { class: "field", label { "Name" } input { class: "input", value: "{name}", oninput: move |e| name.set(e.value()) } }
                                    div { class: "field", label { "Provider Type ID" } input { class: "input", r#type: "number", value: "{provider_type}", oninput: move |e| provider_type.set(e.value().parse().unwrap_or(1)) } }
                                }
                                div { class: "field", label { "API Key / Credential" } input { class: "input mono", r#type: "password", value: "{credential}", oninput: move |e| credential.set(e.value()) } }
                                div { class: "field", label { "Base URL" } input { class: "input mono", value: "{base_url}", oninput: move |e| base_url.set(e.value()) } }
                                div { class: "field", label { "Models (comma-separated)" } textarea { class: "textarea mono", rows: "4", value: "{models}", oninput: move |e| models.set(e.value()) } }
                                div { class: "grid-2",
                                    div { class: "field", label { "Group" } input { class: "input", value: "{group}", oninput: move |e| group.set(e.value()) } }
                                    div { class: "field", label { "Weight" } input { class: "input", r#type: "number", value: "{weight}", oninput: move |e| weight.set(e.value().parse().unwrap_or(100)) } }
                                    div { class: "field", label { "Priority" } input { class: "input", r#type: "number", value: "{priority}", oninput: move |e| priority.set(e.value().parse().unwrap_or(0)) } }
                                    div { class: "field", label { "RPM Cap" } input { class: "input", r#type: "number", value: "{rpm_cap}", oninput: move |e| rpm_cap.set(e.value()) } }
                                    div { class: "field", label { "TPM Cap" } input { class: "input", r#type: "number", value: "{tpm_cap}", oninput: move |e| tpm_cap.set(e.value()) } }
                                }
                                p { class: "tiny subtle", "Existing reservation_green / reservation_yellow / reservation_red values are fetched from the server and preserved during edits." }
                                button {
                                    class: "button button-primary",
                                    disabled: busy(),
                                    onclick: move |_| {
                                        let item = Channel {
                                            id: current_id,
                                            type_: provider_type(),
                                            key: credential(),
                                            name: name().trim().to_string(),
                                            base_url: if base_url().trim().is_empty() { None } else { Some(base_url().trim().to_string()) },
                                            models: models().trim().to_string(),
                                            group: group().trim().to_string(),
                                            status: if current_id > 0 { current_status } else { 1 },
                                            weight: weight(),
                                            priority: priority(),
                                            param_override: current_param_override.clone(),
                                            header_override: current_header_override.clone(),
                                            api_version: current_api_version.clone(),
                                            model_mapping: current_model_mapping.clone(),
                                            rpm_cap: rpm_cap().trim().parse().ok(),
                                            tpm_cap: tpm_cap().trim().parse().ok(),
                                        };
                                        if item.name.is_empty() || item.key.is_empty() || item.models.is_empty() {
                                            error.set("Name, credential and models are required.".to_string());
                                            return;
                                        }
                                        let updating = item.id > 0;
                                        busy.set(true);
                                        error.set(String::new());
                                        spawn(async move {
                                            let result = if updating {
                                                update_channel_preserving_reservations(&item).await
                                            } else {
                                                ChannelService::create(&item).await
                                            };
                                            match result {
                                                Ok(()) => {
                                                    notice.set(if updating { "Provider updated without clearing L2 reservations.".to_string() } else { "Provider created.".to_string() });
                                                    editing.set(None);
                                                    resource.restart();
                                                }
                                                Err(message) => error.set(format!("Provider save failed: {message}")),
                                            }
                                            busy.set(false);
                                        });
                                    },
                                    if busy() { "Saving…" } else { "Save Provider" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
