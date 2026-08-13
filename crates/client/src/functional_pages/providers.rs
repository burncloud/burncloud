use std::collections::BTreeSet;

use dioxus::prelude::*;

use crate::{
    backend::{Channel, ChannelService},
    components::Icon,
    functional_api::{repair_channel_and_reactivate, update_channel_preserving_reservations},
};

const PROVIDER_TYPES: &[(i32, &str)] = &[
    (1, "OpenAI"),
    (3, "Azure OpenAI"),
    (4, "Ollama"),
    (8, "OpenAI-Compatible / Custom"),
    (14, "Anthropic"),
    (20, "OpenRouter"),
    (24, "Google Gemini"),
    (25, "Moonshot / Kimi"),
    (33, "AWS"),
    (40, "SiliconFlow"),
    (41, "Google Vertex AI"),
    (42, "Mistral"),
    (43, "DeepSeek"),
    (45, "VolcEngine"),
    (48, "xAI"),
    (58, "New API Compatible"),
];

fn provider_label(type_id: i32) -> String {
    PROVIDER_TYPES
        .iter()
        .find(|(id, _)| *id == type_id)
        .map(|(_, label)| (*label).to_string())
        .unwrap_or_else(|| format!("Provider type {type_id}"))
}

fn known_provider_type(type_id: i32) -> bool {
    PROVIDER_TYPES.iter().any(|(id, _)| *id == type_id)
}

fn provider_mark(label: &str) -> String {
    label
        .split(|c: char| c.is_whitespace() || c == '-' || c == '/')
        .filter(|part| !part.is_empty())
        .take(2)
        .filter_map(|part| part.chars().next())
        .collect::<String>()
        .to_ascii_uppercase()
}

fn model_list(channel: &Channel) -> Vec<String> {
    channel
        .models
        .split(',')
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(str::to_string)
        .collect()
}

fn model_count_text(count: usize) -> String {
    format!("{count} {}", if count == 1 { "model" } else { "models" })
}

#[component]
pub fn Providers() -> Element {
    let mut resource = use_resource(move || async move { ChannelService::list(100).await });
    let mut editing = use_signal(|| None::<Channel>);
    let mut pending_delete = use_signal(|| None::<Channel>);
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
    let load_error = snapshot.as_ref().and_then(|result| result.as_ref().err().cloned());
    let channels = snapshot.and_then(Result::ok).unwrap_or_default();
    let total = channels.len();
    let active = channels.iter().filter(|channel| channel.status == 1).count();
    let attention = total.saturating_sub(active);
    let mut unique_models = BTreeSet::new();
    let mut routing_groups = BTreeSet::new();
    for channel in &channels {
        for model in model_list(channel) {
            unique_models.insert(model);
        }
        for route_group in channel.group.split(',').map(str::trim).filter(|group| !group.is_empty()) {
            routing_groups.insert(route_group.to_string());
        }
    }
    let model_count = unique_models.len();
    let group_count = routing_groups.len();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Providers" }
                    p { class: "page-subtitle", "Manage upstream supply, model coverage, routing preference, and capacity from one place." }
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
                            notice.set(String::new());
                            error.set(String::new());
                        },
                        Icon { name: "plus" }
                        "Add Provider"
                    }
                }
            }

            div { class: "metrics",
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Active Providers" } span { class: "metric-value", "{active}" } span { class: "metric-note", "{total} configured" } }
                    div { class: "metric-icon tone-green", Icon { name: "activity" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Models" } span { class: "metric-value", "{model_count}" } span { class: "metric-note", "unique model IDs" } }
                    div { class: "metric-icon tone-purple", Icon { name: "models" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Routing Groups" } span { class: "metric-value", "{group_count}" } span { class: "metric-note", "traffic destinations" } }
                    div { class: "metric-icon tone-blue", Icon { name: "routes" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Needs Attention" } span { class: "metric-value", "{attention}" } span { class: "metric-note", "inactive or down providers" } }
                    div { class: "metric-icon tone-amber", Icon { name: "shield" } }
                }
            }

            if !notice().is_empty() { div { class: "terminal auth-status", "{notice}" } }
            if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }

            if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Providers could not be loaded" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| resource.restart(), "Retry" }
                }
            } else if channels.is_empty() {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "providers" } }
                        h3 { "Connect your first upstream provider" }
                        p { "A provider supplies the model endpoint and credentials BurnCloud needs before Models, Routes, Playground, and real traffic can work." }
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
                            "Add first provider"
                        }
                    }
                }
            } else {
                div { class: "card table-card",
                    div { class: "card-pad product-section-head",
                        div {
                            h3 { "Provider inventory" }
                            p { "See service state and coverage first. Connection details and routing numbers stay secondary." }
                        }
                    }
                    div { class: "table-wrap",
                        table { class: "data-table",
                            thead { tr {
                                th { "Provider" }
                                th { "Status" }
                                th { "Models" }
                                th { "Routing" }
                                th { "Capacity" }
                                th { class: "right", "Actions" }
                            } }
                            tbody {
                                for channel in channels {
                                    {
                                        let edit_channel = channel.clone();
                                        let delete_channel = channel.clone();
                                        let type_label = provider_label(channel.type_);
                                        let mark = provider_mark(&type_label);
                                        let base = channel.base_url.clone().unwrap_or_else(|| "Default endpoint".to_string());
                                        let served_models = model_list(&channel);
                                        let served_count = served_models.len();
                                        let served_count_text = model_count_text(served_count);
                                        let model_preview = served_models.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
                                        let model_text = if served_count > 3 { format!("{model_preview} +{} more", served_count - 3) } else { model_preview };
                                        let routing_text = format!("Priority {} • Weight {}", channel.priority, channel.weight);
                                        let capacity = match (channel.rpm_cap, channel.tpm_cap) {
                                            (None, None) => "No limits".to_string(),
                                            (rpm, tpm) => {
                                                let rpm_text = rpm.map(|value| format!("{value} RPM")).unwrap_or_else(|| "No RPM limit".to_string());
                                                let tpm_text = tpm.map(|value| format!("{value} TPM")).unwrap_or_else(|| "No TPM limit".to_string());
                                                format!("{rpm_text} • {tpm_text}")
                                            }
                                        };
                                        let status = if channel.status == 1 { "Active" } else { "Down" };
                                        let action_label = if channel.status == 1 { "Edit" } else { "Repair" };
                                        rsx! {
                                            tr { key: "{channel.id}",
                                                td {
                                                    div { class: "product-table-primary",
                                                        div { class: "provider-mark", "{mark}" }
                                                        div { class: "provider-name-block",
                                                            strong { "{channel.name}" }
                                                            small { "{type_label} • {base}" }
                                                        }
                                                    }
                                                }
                                                td {
                                                    span { class: if channel.status == 1 { "badge badge-success" } else { "badge badge-error" }, "{status}" }
                                                }
                                                td {
                                                    div { class: "two-line",
                                                        strong { class: "small", "{served_count_text}" }
                                                        small { class: "mono muted", if model_text.is_empty() { "No models configured" } else { "{model_text}" } }
                                                    }
                                                }
                                                td {
                                                    div { class: "two-line",
                                                        strong { class: "small", "{channel.group}" }
                                                        small { class: "mono muted", "{routing_text}" }
                                                    }
                                                }
                                                td { class: "small muted", "{capacity}" }
                                                td { class: "right",
                                                    div { class: "action-menu",
                                                        button {
                                                            class: "button button-ghost button-sm",
                                                            title: if channel.status == 1 { "Edit provider" } else { "Repair this provider; saving changes will reactivate routing" },
                                                            onclick: move |_| {
                                                                name.set(edit_channel.name.clone());
                                                                provider_type.set(edit_channel.type_);
                                                                credential.set(String::new());
                                                                base_url.set(edit_channel.base_url.clone().unwrap_or_default());
                                                                models.set(edit_channel.models.clone());
                                                                group.set(edit_channel.group.clone());
                                                                weight.set(edit_channel.weight);
                                                                priority.set(edit_channel.priority);
                                                                rpm_cap.set(edit_channel.rpm_cap.map(|value| value.to_string()).unwrap_or_default());
                                                                tpm_cap.set(edit_channel.tpm_cap.map(|value| value.to_string()).unwrap_or_default());
                                                                notice.set(String::new());
                                                                error.set(String::new());
                                                                editing.set(Some(edit_channel.clone()));
                                                            },
                                                            "{action_label}"
                                                        }
                                                        button {
                                                            class: "button button-ghost button-sm danger",
                                                            disabled: busy(),
                                                            onclick: move |_| pending_delete.set(Some(delete_channel.clone())),
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

            if let Some(current) = editing() {
                {
                    let current_id = current.id;
                    let current_status = current.status;
                    let current_key = current.key.clone();
                    let current_param_override = current.param_override.clone();
                    let current_header_override = current.header_override.clone();
                    let current_api_version = current.api_version.clone();
                    let current_model_mapping = current.model_mapping.clone();
                    let current_type_known = known_provider_type(provider_type());
                    let current_type_label = provider_label(provider_type());
                    let is_new = current_id == 0;
                    let is_repair = current_id > 0 && current_status != 1;
                    rsx! {
                        div { class: "drawer-backdrop", onclick: move |_| editing.set(None) }
                        aside { class: "drawer",
                            div { class: "drawer-head",
                                div {
                                    h2 { if is_new { "Add Provider" } else if is_repair { "Repair Provider" } else { "Edit Provider" } }
                                    p { class: "small muted",
                                        if is_new {
                                            "Connect an upstream and define the models it can serve."
                                        } else if is_repair {
                                            "Fix connection or configuration problems, then explicitly reactivate this provider."
                                        } else {
                                            "Update connection, model coverage, or routing policy."
                                        }
                                    }
                                }
                                button { class: "close-button", onclick: move |_| editing.set(None), "×" }
                            }
                            div { class: "drawer-body stack-lg",
                                if is_repair {
                                    div { class: "readiness-strip blocked",
                                        span { class: "readiness-dot" }
                                        strong { "Saving this repair will reactivate routing" }
                                        span { class: "muted", "Review credentials, endpoint, models, and routing policy before continuing." }
                                    }
                                }

                                div { class: "form-section",
                                    div { class: "form-section-head",
                                        strong { "Provider" }
                                        small { "Name the upstream and select the adapter BurnCloud should use." }
                                    }
                                    div { class: "grid-2",
                                        div { class: "field",
                                            label { "Display name" }
                                            input { class: "input", value: "{name}", placeholder: "e.g. Anthropic Production", oninput: move |event| name.set(event.value()) }
                                        }
                                        div { class: "field",
                                            label { "Provider type" }
                                            select { class: "select", value: "{provider_type}", onchange: move |event| provider_type.set(event.value().parse().unwrap_or(8)),
                                                if !current_type_known {
                                                    option { value: "{provider_type}", "{current_type_label}" }
                                                }
                                                for (type_id, label) in PROVIDER_TYPES {
                                                    option { value: "{type_id}", "{label}" }
                                                }
                                            }
                                        }
                                    }
                                }

                                div { class: "form-section",
                                    div { class: "form-section-head",
                                        strong { "Connection" }
                                        small { if is_new { "Provide the credential BurnCloud will use to call this upstream." } else { "The stored credential is never shown here. Leave the field blank to keep it unchanged." } }
                                    }
                                    div { class: "field",
                                        label { if is_new { "API key / credential" } else { "Replace credential (optional)" } }
                                        input {
                                            class: "input mono",
                                            r#type: "password",
                                            value: "{credential}",
                                            placeholder: if is_new { "Paste provider credential" } else { "Leave blank to keep stored credential" },
                                            oninput: move |event| credential.set(event.value()),
                                        }
                                    }
                                    div { class: "field",
                                        label { "Base URL (optional)" }
                                        input { class: "input mono", value: "{base_url}", placeholder: "Use provider default when blank", oninput: move |event| base_url.set(event.value()) }
                                    }
                                }

                                div { class: "form-section",
                                    div { class: "form-section-head",
                                        strong { "Models served" }
                                        small { "These model IDs become available to Models, Routes, and Playground." }
                                    }
                                    textarea { class: "textarea mono", rows: "5", value: "{models}", placeholder: "claude-sonnet-4-5, claude-opus-4-1", oninput: move |event| models.set(event.value()) }
                                    div { class: "product-note", "Use exact upstream model IDs separated by commas. BurnCloud derives its model catalog from this list." }
                                }

                                details { class: "form-section",
                                    summary { class: "strong", style: "cursor:pointer", "Advanced routing & capacity" }
                                    div { class: "form-section-head", style: "margin-top:12px",
                                        small { "Most providers can keep the defaults. Change these only when you intentionally control route preference or capacity." }
                                    }
                                    div { class: "grid-2",
                                        div { class: "field", label { "Routing group" } input { class: "input", value: "{group}", oninput: move |event| group.set(event.value()) } }
                                        div { class: "field", label { "Weight" } input { class: "input", r#type: "number", value: "{weight}", oninput: move |event| weight.set(event.value().parse().unwrap_or(100)) } }
                                        div { class: "field", label { "Priority" } input { class: "input", r#type: "number", value: "{priority}", oninput: move |event| priority.set(event.value().parse().unwrap_or(0)) } }
                                        div { class: "field", label { "RPM limit" } input { class: "input", r#type: "number", value: "{rpm_cap}", placeholder: "Unlimited", oninput: move |event| rpm_cap.set(event.value()) } }
                                        div { class: "field", label { "TPM limit" } input { class: "input", r#type: "number", value: "{tpm_cap}", placeholder: "Unlimited", oninput: move |event| tpm_cap.set(event.value()) } }
                                    }
                                    if !is_new {
                                        div { class: "product-note", "Existing green / yellow / red reservation thresholds are preserved automatically when this provider is saved." }
                                    }
                                }

                                if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }

                                div { class: "row customer-form-actions",
                                    button { class: "button button-secondary", disabled: busy(), onclick: move |_| editing.set(None), "Cancel" }
                                    button {
                                        class: "button button-primary",
                                        disabled: busy(),
                                        onclick: move |_| {
                                            let entered_credential = credential();
                                            let effective_credential = if current_id > 0 && entered_credential.trim().is_empty() {
                                                current_key.clone()
                                            } else {
                                                entered_credential
                                            };
                                            let item = Channel {
                                                id: current_id,
                                                type_: provider_type(),
                                                key: effective_credential,
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
                                            if item.name.is_empty() {
                                                error.set("Provider name is required.".to_string());
                                                return;
                                            }
                                            if item.key.trim().is_empty() {
                                                error.set("A provider credential is required.".to_string());
                                                return;
                                            }
                                            if item.models.is_empty() {
                                                error.set("Add at least one model this provider can serve.".to_string());
                                                return;
                                            }
                                            let updating = item.id > 0;
                                            let repairing = updating && current_status != 1;
                                            busy.set(true);
                                            error.set(String::new());
                                            spawn(async move {
                                                let result = if repairing {
                                                    repair_channel_and_reactivate(&item).await
                                                } else if updating {
                                                    update_channel_preserving_reservations(&item).await
                                                } else {
                                                    ChannelService::create(&item).await
                                                };
                                                match result {
                                                    Ok(()) => {
                                                        let message = if repairing {
                                                            "Provider repaired and reactivated. Run a Playground test before relying on production traffic."
                                                        } else if updating {
                                                            "Provider updated."
                                                        } else {
                                                            "Provider connected. Review Models and Routes before sending traffic."
                                                        };
                                                        notice.set(message.to_string());
                                                        editing.set(None);
                                                        resource.restart();
                                                    }
                                                    Err(message) => error.set(format!("Provider save failed: {message}")),
                                                }
                                                busy.set(false);
                                            });
                                        },
                                        if busy() {
                                            "Saving…"
                                        } else if is_new {
                                            "Connect Provider"
                                        } else if is_repair {
                                            "Save & Reactivate"
                                        } else {
                                            "Save Changes"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(target) = pending_delete() {
                {
                    let target_id = target.id;
                    let target_name = target.name.clone();
                    let target_models = model_list(&target).len();
                    let target_models_text = model_count_text(target_models);
                    rsx! {
                        div { class: "drawer-backdrop", onclick: move |_| pending_delete.set(None) }
                        aside { class: "drawer",
                            div { class: "drawer-head",
                                h2 { class: "danger", "Delete Provider" }
                                button { class: "close-button", onclick: move |_| pending_delete.set(None), "×" }
                            }
                            div { class: "drawer-body stack-lg",
                                div { class: "form-section danger-zone",
                                    strong { "Delete {target_name}?" }
                                    p { class: "small muted", "This provider currently exposes {target_models_text}. Deleting it removes this upstream from routing immediately and cannot be undone from the console." }
                                }
                                div { class: "product-note", "Delete is for permanent removal. If this provider is down, use Repair after fixing its connection instead of deleting and recreating it." }
                                div { class: "row customer-form-actions",
                                    button { class: "button button-secondary", disabled: busy(), onclick: move |_| pending_delete.set(None), "Cancel" }
                                    button {
                                        class: "button button-primary danger-zone",
                                        disabled: busy(),
                                        onclick: move |_| {
                                            let deleted_name = target_name.clone();
                                            busy.set(true);
                                            error.set(String::new());
                                            spawn(async move {
                                                match ChannelService::delete(target_id).await {
                                                    Ok(()) => {
                                                        notice.set(format!("Provider {deleted_name} deleted."));
                                                        pending_delete.set(None);
                                                        resource.restart();
                                                    }
                                                    Err(message) => error.set(format!("Delete provider failed: {message}")),
                                                }
                                                busy.set(false);
                                            });
                                        },
                                        if busy() { "Deleting…" } else { "Delete Provider" }
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
