use std::collections::{BTreeMap, BTreeSet};

use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{Channel, ChannelService},
    components::Icon,
};

fn status_label(channel: &Channel) -> &'static str {
    if channel.status == 1 { "Active" } else { "Down" }
}

#[derive(Default)]
struct ModelAvailability {
    providers: BTreeSet<String>,
    active_providers: BTreeSet<String>,
    groups: BTreeSet<String>,
}

#[component]
pub fn Models() -> Element {
    let mut resource = use_resource(move || async move { ChannelService::list(100).await });
    let snapshot = resource.read().clone();
    let load_error = snapshot.as_ref().and_then(|result| result.as_ref().err().cloned());
    let channels = snapshot.and_then(Result::ok).unwrap_or_default();

    let mut model_map: BTreeMap<String, ModelAvailability> = BTreeMap::new();
    for channel in &channels {
        for model in channel.models.split(',').map(str::trim).filter(|model| !model.is_empty()) {
            let entry = model_map.entry(model.to_string()).or_default();
            entry.providers.insert(channel.name.clone());
            if channel.status == 1 {
                entry.active_providers.insert(channel.name.clone());
            }
            for group in channel.group.split(',').map(str::trim).filter(|group| !group.is_empty()) {
                entry.groups.insert(group.to_string());
            }
        }
    }

    let total_models = model_map.len();
    let available_models = model_map.values().filter(|model| !model.active_providers.is_empty()).count();
    let protected_models = model_map.values().filter(|model| model.active_providers.len() >= 2).count();
    let unavailable_models = total_models.saturating_sub(available_models);
    let needs_backup_models = model_map.values().filter(|model| model.active_providers.len() == 1).count();
    let attention_models = needs_backup_models + unavailable_models;

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Models" }
                    p { class: "page-subtitle", "See what BurnCloud can serve now and which models would stop working if one provider fails." }
                }
                div { class: "header-actions",
                    button { class: "button button-secondary", onclick: move |_| resource.restart(), "Refresh" }
                    Link { class: "button button-primary", to: Route::Providers {}, "Manage Providers" }
                }
            }

            div { class: "metrics",
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Models" } span { class: "metric-value", "{total_models}" } span { class: "metric-note", "configured model IDs" } }
                    div { class: "metric-icon tone-blue", Icon { name: "models" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Usable Now" } span { class: "metric-value", "{available_models}" } span { class: "metric-note", "has an active provider" } }
                    div { class: "metric-icon tone-green", Icon { name: "activity" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Protected" } span { class: "metric-value", "{protected_models}" } span { class: "metric-note", "2+ active providers" } }
                    div { class: "metric-icon tone-purple", Icon { name: "shield" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Needs Attention" } span { class: "metric-value", "{attention_models}" } span { class: "metric-note", "no backup or unavailable" } }
                    div { class: "metric-icon tone-amber", Icon { name: "routes" } }
                }
            }

            if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Model catalog could not be built" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| resource.restart(), "Retry" }
                }
            } else if model_map.is_empty() {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "models" } }
                        h3 { "No models are exposed yet" }
                        p { "BurnCloud builds this catalog from provider configuration. Add a provider, then list the model IDs that provider can serve." }
                        Link { class: "button button-primary", to: Route::Providers {}, "Configure Providers" }
                    }
                }
            } else {
                if attention_models > 0 {
                    div { class: "readiness-strip blocked",
                        span { class: "readiness-dot" }
                        strong { "{attention_models} model(s) need attention" }
                        span { class: "muted", "Add overlapping providers for important models so one upstream failure does not interrupt service." }
                    }
                }

                div { class: "card table-card",
                    div { class: "card-pad product-section-head",
                        div {
                            h3 { "Model availability" }
                            p { "The status is based on active providers right now, not just configured entries." }
                        }
                    }
                    div { class: "table-wrap",
                        table { class: "data-table",
                            thead { tr {
                                th { "Model" }
                                th { "Resilience" }
                                th { "Active Providers" }
                                th { "Routing Groups" }
                                th { class: "right", "Next Step" }
                            } }
                            tbody {
                                for (model_name, availability) in model_map {
                                    {
                                        let active_count = availability.active_providers.len();
                                        let total_count = availability.providers.len();
                                        let provider_word = if total_count == 1 { "provider" } else { "providers" };
                                        let active_text = availability.active_providers.iter().cloned().collect::<Vec<_>>().join(", ");
                                        let group_text = availability.groups.iter().cloned().collect::<Vec<_>>().join(", ");
                                        let (badge_class, badge_text, note) = if active_count == 0 {
                                            ("badge badge-error", "Unavailable", "No active provider")
                                        } else if active_count == 1 {
                                            ("badge badge-warning", "Needs backup", "Only one active provider")
                                        } else {
                                            ("badge badge-success", "Protected", "Failover available")
                                        };
                                        rsx! {
                                            tr { key: "{model_name}",
                                                td {
                                                    div { class: "two-line",
                                                        strong { class: "table-primary mono", "{model_name}" }
                                                        small { class: "muted", "{total_count} configured {provider_word}" }
                                                    }
                                                }
                                                td {
                                                    div { class: "two-line",
                                                        span { class: "{badge_class}", "{badge_text}" }
                                                        small { class: "muted", "{note}" }
                                                    }
                                                }
                                                td { class: "small", if active_text.is_empty() { "—" } else { "{active_text}" } }
                                                td { class: "mono muted", if group_text.is_empty() { "—" } else { "{group_text}" } }
                                                td { class: "right",
                                                    if active_count > 0 {
                                                        Link { class: "button button-ghost button-sm", to: Route::Playground {}, "Open Playground" }
                                                    } else {
                                                        Link { class: "button button-ghost button-sm", to: Route::Providers {}, "Fix Provider" }
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

#[component]
pub fn Routes() -> Element {
    let mut resource = use_resource(move || async move { ChannelService::list(100).await });
    let snapshot = resource.read().clone();
    let load_error = snapshot.as_ref().and_then(|result| result.as_ref().err().cloned());
    let channels = snapshot.and_then(Result::ok).unwrap_or_default();

    let mut groups: BTreeMap<String, Vec<Channel>> = BTreeMap::new();
    for channel in channels {
        for group in channel.group.split(',').map(str::trim).filter(|group| !group.is_empty()) {
            groups.entry(group.to_string()).or_default().push(channel.clone());
        }
    }
    for rows in groups.values_mut() {
        rows.sort_by_key(|channel| (channel.priority, -channel.weight));
    }

    let route_groups = groups.len();
    let healthy_groups = groups
        .values()
        .filter(|rows| rows.iter().any(|channel| channel.status == 1))
        .count();
    let protected_groups = groups
        .values()
        .filter(|rows| rows.iter().filter(|channel| channel.status == 1).count() >= 2)
        .count();
    let unavailable_groups = route_groups.saturating_sub(healthy_groups);
    let needs_backup_groups = groups
        .values()
        .filter(|rows| rows.iter().filter(|channel| channel.status == 1).count() == 1)
        .count();
    let attention_groups = needs_backup_groups + unavailable_groups;

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Routes" }
                    p { class: "page-subtitle", "See which providers will receive traffic, which path is preferred, and where failover is still missing." }
                }
                div { class: "header-actions",
                    button { class: "button button-secondary", onclick: move |_| resource.restart(), "Refresh" }
                    Link { class: "button button-primary", to: Route::Providers {}, "Manage Providers" }
                }
            }

            div { class: "metrics",
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Routing Groups" } span { class: "metric-value", "{route_groups}" } span { class: "metric-note", "traffic destinations" } }
                    div { class: "metric-icon tone-blue", Icon { name: "routes" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Serving" } span { class: "metric-value", "{healthy_groups}" } span { class: "metric-note", "has an active provider" } }
                    div { class: "metric-icon tone-green", Icon { name: "activity" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Protected" } span { class: "metric-value", "{protected_groups}" } span { class: "metric-note", "2+ active providers" } }
                    div { class: "metric-icon tone-purple", Icon { name: "shield" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Needs Attention" } span { class: "metric-value", "{attention_groups}" } span { class: "metric-note", "no backup or unavailable" } }
                    div { class: "metric-icon tone-amber", Icon { name: "providers" } }
                }
            }

            if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Routes could not be derived" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| resource.restart(), "Retry" }
                }
            } else if groups.is_empty() {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "routes" } }
                        h3 { "No routing groups yet" }
                        p { "Routing groups come from provider configuration. Connect a provider and assign it to a group before traffic can be evaluated here." }
                        Link { class: "button button-primary", to: Route::Providers {}, "Configure Providers" }
                    }
                }
            } else {
                if attention_groups > 0 {
                    div { class: "readiness-strip blocked",
                        span { class: "readiness-dot" }
                        strong { "{attention_groups} routing group(s) need attention" }
                        span { class: "muted", "A production route should normally have more than one active provider for the models it serves." }
                    }
                }

                div { class: "stack-lg",
                    for (group_name, rows) in groups {
                        {
                            let active_count = rows.iter().filter(|channel| channel.status == 1).count();
                            let model_count = rows
                                .iter()
                                .flat_map(|channel| channel.models.split(',').map(str::trim).filter(|model| !model.is_empty()).map(str::to_string))
                                .collect::<BTreeSet<_>>()
                                .len();
                            let candidate_word = if rows.len() == 1 { "provider" } else { "providers" };
                            let model_word = if model_count == 1 { "model" } else { "models" };
                            let (health_class, health_text) = if active_count == 0 {
                                ("badge badge-error", "Unavailable")
                            } else if active_count == 1 {
                                ("badge badge-warning", "Needs backup")
                            } else {
                                ("badge badge-success", "Protected")
                            };
                            rsx! {
                                div { class: "card card-pad stack",
                                    div { class: "product-section-head",
                                        div {
                                            div { class: "row gap-2",
                                                h3 { "{group_name}" }
                                                span { class: "{health_class}", "{health_text}" }
                                            }
                                            p { "{active_count} active of {rows.len()} {candidate_word} • {model_count} {model_word} available" }
                                        }
                                        Link {
                                            class: "button button-secondary button-sm",
                                            to: Route::Providers {},
                                            if active_count < 2 { "Add backup provider" } else { "Manage providers" }
                                        }
                                    }
                                    div { class: "table-wrap",
                                        table { class: "data-table",
                                            thead { tr {
                                                th { "Order" }
                                                th { "Provider" }
                                                th { "Status" }
                                                th { "Models" }
                                                th { class: "right", "Routing Policy" }
                                            } }
                                            tbody {
                                                for (index, channel) in rows.iter().enumerate() {
                                                    {
                                                        let preference = index + 1;
                                                        let status = status_label(channel);
                                                        let channel_model_count = channel.models.split(',').map(str::trim).filter(|model| !model.is_empty()).count();
                                                        let channel_model_word = if channel_model_count == 1 { "model" } else { "models" };
                                                        let policy = format!("Priority {} • Weight {}", channel.priority, channel.weight);
                                                        rsx! {
                                                            tr { key: "{channel.id}",
                                                                td { class: "mono", "#{preference}" }
                                                                td { class: "table-primary", "{channel.name}" }
                                                                td { span { class: if channel.status == 1 { "badge badge-success" } else { "badge badge-error" }, "{status}" } }
                                                                td { "{channel_model_count} {channel_model_word}" }
                                                                td { class: "right mono muted", "{policy}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    div { class: "product-note",
                                        if active_count == 0 {
                                            "No active provider can currently serve this route. Restore a provider before sending traffic to this group."
                                        } else if active_count == 1 {
                                            "Only one active provider can serve this route. Add another provider with overlapping model coverage so one upstream failure does not stop traffic."
                                        } else {
                                            "Lower priority values are preferred first. Weight distributes traffic between providers that share the same priority."
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
