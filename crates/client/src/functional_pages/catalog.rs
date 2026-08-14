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
    let is_loading = snapshot.is_none();
    let load_error = snapshot.as_ref().and_then(|result| result.as_ref().err().cloned());
    let has_load_error = load_error.is_some();
    let channels = snapshot
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();

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
    let redundant_models = model_map.values().filter(|model| model.active_providers.len() >= 2).count();
    let unavailable_models = total_models.saturating_sub(available_models);
    let single_upstream_models = available_models.saturating_sub(redundant_models);
    let health_class = if unavailable_models > 0 || single_upstream_models > 0 {
        "readiness-strip blocked model-health-strip"
    } else {
        "readiness-strip ready model-health-strip"
    };
    let health_title = if unavailable_models > 0 {
        "Some models are unavailable"
    } else if single_upstream_models > 0 {
        "Model supply is available, but not fully redundant"
    } else {
        "Model supply is resilient"
    };
    let health_copy = if unavailable_models > 0 {
        format!("{unavailable_models} model IDs have no active upstream. Restore provider health before relying on the full catalog.")
    } else if single_upstream_models > 0 {
        format!("{single_upstream_models} of {available_models} available model IDs still rely on one active upstream.")
    } else {
        format!("All {available_models} available model IDs have at least two active upstreams.")
    };
    let redundancy_label = format!("{redundant_models} redundant");

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Models" }
                    p { class: "page-subtitle", "See which model IDs BurnCloud can actually serve and whether each model has upstream redundancy." }
                }
                div { class: "header-actions",
                    button {
                        class: "button button-secondary",
                        disabled: is_loading,
                        onclick: move |_| resource.restart(),
                        if is_loading { "Refreshing…" } else { "Refresh" }
                    }
                    Link { class: "button button-primary", to: Route::Providers {}, "Manage Providers" }
                }
            }

            if is_loading {
                div { class: "card product-empty model-loading-state",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "models" } }
                        h3 { "Building model availability" }
                        p { "Reading provider model IDs and active upstream state before deriving the catalog." }
                    }
                }
            } else if !has_load_error {
                div { class: "metrics",
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Models" } span { class: "metric-value", "{total_models}" } span { class: "metric-note", "derived from providers" } }
                        div { class: "metric-icon tone-gray", Icon { name: "models" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Available" } span { class: "metric-value", "{available_models}" } span { class: "metric-note", "at least one active upstream" } }
                        div { class: if available_models > 0 { "metric-icon tone-green" } else { "metric-icon tone-gray" }, Icon { name: "activity" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Redundant" } span { class: "metric-value", "{redundant_models}" } span { class: "metric-note", "2+ active upstreams" } }
                        div { class: "metric-icon tone-gray", Icon { name: "routes" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Unavailable" } span { class: "metric-value", "{unavailable_models}" } span { class: "metric-note", "no active upstream" } }
                        div { class: if unavailable_models > 0 { "metric-icon tone-red" } else { "metric-icon tone-gray" }, Icon { name: "shield" } }
                    }
                }

                if !model_map.is_empty() {
                    div { class: "{health_class}",
                        span { class: "readiness-dot" }
                        div { class: "model-health-copy",
                            strong { "{health_title}" }
                            span { class: "small muted", "{health_copy}" }
                        }
                        span { class: "badge badge-neutral model-health-meta", "{redundancy_label}" }
                    }
                }
            }

            if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Model catalog could not be built" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| resource.restart(), "Retry" }
                }
            } else if !is_loading && model_map.is_empty() {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "models" } }
                        h3 { "No models are exposed yet" }
                        p { "BurnCloud derives the model catalog from provider configuration. Add a provider or add model IDs to an existing provider." }
                        Link { class: "button button-primary", to: Route::Providers {}, "Configure Providers" }
                    }
                }
            } else if !is_loading {
                div { class: "card table-card",
                    div { class: "card-pad product-section-head",
                        div {
                            h3 { "Model availability" }
                            p { "Availability reflects active providers now; redundancy highlights models that can survive one upstream failure." }
                        }
                    }
                    div { class: "table-wrap",
                        table { class: "data-table model-availability-table",
                            thead { tr {
                                th { "Model" }
                                th { "Availability" }
                                th { "Active Upstreams" }
                                th { "Routing Groups" }
                                th { class: "right", "Action" }
                            } }
                            tbody {
                                for (model_name, availability) in model_map {
                                    {
                                        let active_count = availability.active_providers.len();
                                        let total_count = availability.providers.len();
                                        let active_preview = availability.active_providers.iter().take(2).cloned().collect::<Vec<_>>().join(", ");
                                        let active_text = if active_count == 0 {
                                            "No active upstream".to_string()
                                        } else if active_count > 2 {
                                            format!("{active_preview} +{} more", active_count - 2)
                                        } else {
                                            active_preview
                                        };
                                        let active_count_text = if active_count == 1 { "1 active".to_string() } else { format!("{active_count} active") };
                                        let configured_text = if total_count == 1 { "1 configured provider".to_string() } else { format!("{total_count} configured providers") };
                                        let group_count = availability.groups.len();
                                        let group_preview = availability.groups.iter().take(2).cloned().collect::<Vec<_>>().join(", ");
                                        let group_text = if group_count == 0 {
                                            "No routing group".to_string()
                                        } else if group_count > 2 {
                                            format!("{group_preview} +{} more", group_count - 2)
                                        } else {
                                            group_preview
                                        };
                                        let group_count_text = if group_count == 1 { "1 group".to_string() } else { format!("{group_count} groups") };
                                        let (badge_class, badge_text, note) = if active_count == 0 {
                                            ("badge badge-error", "Unavailable", "No active upstream")
                                        } else if active_count == 1 {
                                            ("badge badge-warning", "Single upstream", "No failover redundancy")
                                        } else {
                                            ("badge badge-success", "Redundant", "Failover available")
                                        };
                                        rsx! {
                                            tr { key: "{model_name}",
                                                td {
                                                    div { class: "two-line model-name-cell",
                                                        strong { class: "table-primary mono", title: "{model_name}", "{model_name}" }
                                                        small { class: "muted", "{configured_text}" }
                                                    }
                                                }
                                                td {
                                                    div { class: "two-line",
                                                        span { class: "{badge_class}", "{badge_text}" }
                                                        small { class: "muted", "{note}" }
                                                    }
                                                }
                                                td {
                                                    div { class: "two-line",
                                                        strong { class: "small", "{active_count_text}" }
                                                        small { class: "muted model-upstream-preview", title: "{active_text}", "{active_text}" }
                                                    }
                                                }
                                                td {
                                                    div { class: "two-line",
                                                        strong { class: "small", "{group_count_text}" }
                                                        small { class: "mono muted model-group-preview", title: "{group_text}", "{group_text}" }
                                                    }
                                                }
                                                td { class: "right",
                                                    if active_count == 0 {
                                                        Link { class: "button button-ghost button-sm", to: Route::Providers {}, "Fix Provider" }
                                                    } else if active_count == 1 {
                                                        Link { class: "button button-ghost button-sm", to: Route::Providers {}, "Add Failover" }
                                                    } else {
                                                        Link { class: "button button-ghost button-sm", to: Route::Playground {}, "Test" }
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
    let redundant_groups = groups
        .values()
        .filter(|rows| rows.iter().filter(|channel| channel.status == 1).count() >= 2)
        .count();
    let unavailable_groups = route_groups.saturating_sub(healthy_groups);

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Routes" }
                    p { class: "page-subtitle", "Understand how traffic groups choose providers and where a routing group still depends on a single upstream." }
                }
                div { class: "header-actions",
                    button { class: "button button-secondary", onclick: move |_| resource.restart(), "Refresh" }
                    Link { class: "button button-primary", to: Route::Providers {}, "Manage Routing Inputs" }
                }
            }

            div { class: "metrics",
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Routing Groups" } span { class: "metric-value", "{route_groups}" } span { class: "metric-note", "traffic policies" } }
                    div { class: "metric-icon tone-blue", Icon { name: "routes" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Available" } span { class: "metric-value", "{healthy_groups}" } span { class: "metric-note", "has an active provider" } }
                    div { class: "metric-icon tone-green", Icon { name: "activity" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Redundant" } span { class: "metric-value", "{redundant_groups}" } span { class: "metric-note", "2+ active candidates" } }
                    div { class: "metric-icon tone-purple", Icon { name: "shield" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Unavailable" } span { class: "metric-value", "{unavailable_groups}" } span { class: "metric-note", "no active candidate" } }
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
                div { class: "stack-lg",
                    for (group_name, rows) in groups {
                        {
                            let active_count = rows.iter().filter(|channel| channel.status == 1).count();
                            let model_count = rows
                                .iter()
                                .flat_map(|channel| channel.models.split(',').map(str::trim).filter(|model| !model.is_empty()).map(str::to_string))
                                .collect::<BTreeSet<_>>()
                                .len();
                            let (health_class, health_text) = if active_count == 0 {
                                ("badge badge-error", "Unavailable")
                            } else if active_count == 1 {
                                ("badge badge-warning", "Single upstream")
                            } else {
                                ("badge badge-success", "Redundant")
                            };
                            rsx! {
                                div { class: "card card-pad stack",
                                    div { class: "product-section-head",
                                        div {
                                            div { class: "row gap-2",
                                                h3 { "{group_name}" }
                                                span { class: "{health_class}", "{health_text}" }
                                            }
                                            p { "{active_count} active of {rows.len()} candidates • {model_count} model IDs available" }
                                        }
                                        Link { class: "button button-secondary button-sm", to: Route::Providers {}, "Edit Providers" }
                                    }
                                    div { class: "table-wrap",
                                        table { class: "data-table",
                                            thead { tr {
                                                th { "Preference" }
                                                th { "Provider" }
                                                th { "Status" }
                                                th { "Models" }
                                                th { class: "right", "Priority" }
                                                th { class: "right", "Weight" }
                                            } }
                                            tbody {
                                                for (index, channel) in rows.iter().enumerate() {
                                                    {
                                                        let preference = index + 1;
                                                        let status = status_label(channel);
                                                        let model_count = channel.models.split(',').map(str::trim).filter(|model| !model.is_empty()).count();
                                                        rsx! {
                                                            tr { key: "{channel.id}",
                                                                td { class: "mono", "#{preference}" }
                                                                td { class: "table-primary", "{channel.name}" }
                                                                td { span { class: if channel.status == 1 { "badge badge-success" } else { "badge badge-error" }, "{status}" } }
                                                                td { "{model_count} models" }
                                                                td { class: "right tabular", "{channel.priority}" }
                                                                td { class: "right tabular", "{channel.weight}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if active_count == 1 {
                                        div { class: "product-note", "This routing group currently has a single active upstream. Adding a second provider with overlapping model coverage improves failover resilience." }
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
