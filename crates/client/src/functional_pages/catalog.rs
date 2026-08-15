use std::collections::{BTreeMap, BTreeSet};

use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{Channel, ChannelService},
    components::Icon,
};

fn status_label(channel: &Channel) -> &'static str {
    if channel.status == 1 {
        "Active"
    } else {
        "Down"
    }
}

#[derive(Default)]
struct ModelAvailability {
    providers: BTreeSet<String>,
    active_providers: BTreeSet<String>,
    groups: BTreeSet<String>,
}

#[derive(Default)]
struct RouteGroupHealth {
    active_candidates: usize,
    configured_models: usize,
    available_models: usize,
    redundant_models: usize,
    unavailable_models: usize,
    single_upstream_models: usize,
}

impl RouteGroupHealth {
    fn fully_available(&self) -> bool {
        self.configured_models > 0 && self.unavailable_models == 0
    }

    fn fully_redundant(&self) -> bool {
        self.fully_available() && self.single_upstream_models == 0
    }
}

fn route_group_health(rows: &[Channel]) -> RouteGroupHealth {
    let active_candidates = rows.iter().filter(|channel| channel.status == 1).count();
    let mut configured_models = BTreeSet::new();
    let mut active_model_upstreams: BTreeMap<String, usize> = BTreeMap::new();

    for channel in rows {
        for model in channel
            .models
            .split(',')
            .map(str::trim)
            .filter(|model| !model.is_empty())
        {
            configured_models.insert(model.to_string());
            if channel.status == 1 {
                *active_model_upstreams.entry(model.to_string()).or_default() += 1;
            }
        }
    }

    let available_models = configured_models
        .iter()
        .filter(|model| active_model_upstreams.get(*model).copied().unwrap_or(0) > 0)
        .count();
    let redundant_models = configured_models
        .iter()
        .filter(|model| active_model_upstreams.get(*model).copied().unwrap_or(0) >= 2)
        .count();
    let unavailable_models = configured_models.len().saturating_sub(available_models);
    let single_upstream_models = available_models.saturating_sub(redundant_models);

    RouteGroupHealth {
        active_candidates,
        configured_models: configured_models.len(),
        available_models,
        redundant_models,
        unavailable_models,
        single_upstream_models,
    }
}

#[component]
pub fn Models() -> Element {
    let mut resource = use_resource(move || async move { ChannelService::list(100).await });
    let snapshot = resource.read().clone();
    let is_loading = snapshot.is_none();
    let load_error = snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let has_load_error = load_error.is_some();
    let channels = snapshot
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();

    let mut model_map: BTreeMap<String, ModelAvailability> = BTreeMap::new();
    for channel in &channels {
        for model in channel
            .models
            .split(',')
            .map(str::trim)
            .filter(|model| !model.is_empty())
        {
            let entry = model_map.entry(model.to_string()).or_default();
            entry.providers.insert(channel.name.clone());
            if channel.status == 1 {
                entry.active_providers.insert(channel.name.clone());
            }
            for group in channel
                .group
                .split(',')
                .map(str::trim)
                .filter(|group| !group.is_empty())
            {
                entry.groups.insert(group.to_string());
            }
        }
    }

    let total_models = model_map.len();
    let available_models = model_map
        .values()
        .filter(|model| !model.active_providers.is_empty())
        .count();
    let redundant_models = model_map
        .values()
        .filter(|model| model.active_providers.len() >= 2)
        .count();
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
    let is_loading = snapshot.is_none();
    let load_error = snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let has_load_error = load_error.is_some();
    let channels = snapshot
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();

    let mut groups: BTreeMap<String, Vec<Channel>> = BTreeMap::new();
    for channel in channels {
        for group in channel
            .group
            .split(',')
            .map(str::trim)
            .filter(|group| !group.is_empty())
        {
            groups
                .entry(group.to_string())
                .or_default()
                .push(channel.clone());
        }
    }
    for rows in groups.values_mut() {
        rows.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| right.weight.cmp(&left.weight))
        });
    }

    let route_groups = groups.len();
    let group_health = groups
        .values()
        .map(|rows| route_group_health(rows))
        .collect::<Vec<_>>();
    let fully_available_groups = group_health
        .iter()
        .filter(|health| health.fully_available())
        .count();
    let redundant_groups = group_health
        .iter()
        .filter(|health| health.fully_redundant())
        .count();
    let unavailable_groups = group_health
        .iter()
        .filter(|health| health.available_models == 0)
        .count();
    let partial_groups = route_groups
        .saturating_sub(fully_available_groups)
        .saturating_sub(unavailable_groups);
    let single_upstream_groups = fully_available_groups.saturating_sub(redundant_groups);
    let attention_groups = route_groups.saturating_sub(redundant_groups);
    let health_class = if attention_groups > 0 {
        "readiness-strip blocked route-health-strip"
    } else {
        "readiness-strip ready route-health-strip"
    };
    let health_title = if unavailable_groups > 0 || partial_groups > 0 {
        "Some routes cannot serve their full model set"
    } else if single_upstream_groups > 0 {
        "Routes are available, but failover is incomplete"
    } else {
        "Routing groups are resilient"
    };
    let health_copy = if unavailable_groups > 0 || partial_groups > 0 {
        format!("{unavailable_groups} routing groups are unavailable and {partial_groups} have partial model coverage. Fix provider health or overlap before production traffic relies on them.")
    } else if single_upstream_groups > 0 {
        format!("{single_upstream_groups} routing groups still contain model IDs that rely on a single active upstream.")
    } else {
        format!("All {route_groups} routing groups have active model coverage with failover redundancy.")
    };
    let redundancy_label = format!("{redundant_groups} redundant");
    let attention_note = format!("{single_upstream_groups} single • {partial_groups} partial • {unavailable_groups} unavailable");

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Routes" }
                    p { class: "page-subtitle", "Understand how traffic groups choose providers and whether every routed model has an active failover path." }
                }
                div { class: "header-actions",
                    button {
                        class: "button button-secondary",
                        disabled: is_loading,
                        onclick: move |_| resource.restart(),
                        if is_loading { "Refreshing…" } else { "Refresh" }
                    }
                    Link { class: "button button-primary", to: Route::Providers {}, "Manage Routing Inputs" }
                }
            }

            if is_loading {
                div { class: "card product-empty route-loading-state",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "routes" } }
                        h3 { "Evaluating routing groups" }
                        p { "Reading provider status and model overlap before calculating route availability and failover coverage." }
                    }
                }
            } else if !has_load_error {
                div { class: "metrics",
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Routing Groups" } span { class: "metric-value", "{route_groups}" } span { class: "metric-note", "traffic policies" } }
                        div { class: "metric-icon tone-gray", Icon { name: "routes" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Fully Available" } span { class: "metric-value", "{fully_available_groups}" } span { class: "metric-note", "every configured model has an active upstream" } }
                        div { class: if fully_available_groups > 0 { "metric-icon tone-green" } else { "metric-icon tone-gray" }, Icon { name: "activity" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Redundant" } span { class: "metric-value", "{redundant_groups}" } span { class: "metric-note", "every model has 2+ active upstreams" } }
                        div { class: "metric-icon tone-gray", Icon { name: "shield" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Needs Attention" } span { class: "metric-value", "{attention_groups}" } span { class: "metric-note", "{attention_note}" } }
                        div { class: if attention_groups > 0 { "metric-icon tone-amber" } else { "metric-icon tone-gray" }, Icon { name: "providers" } }
                    }
                }

                if !groups.is_empty() {
                    div { class: "{health_class}",
                        span { class: "readiness-dot" }
                        div { class: "route-health-copy",
                            strong { "{health_title}" }
                            span { class: "small muted", "{health_copy}" }
                        }
                        span { class: "badge badge-neutral route-health-meta", "{redundancy_label}" }
                    }
                }
            }

            if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Routes could not be derived" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| resource.restart(), "Retry" }
                }
            } else if !is_loading && groups.is_empty() {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "routes" } }
                        h3 { "No routing groups yet" }
                        p { "Routing groups come from provider configuration. Connect a provider and assign it to a group before traffic can be evaluated here." }
                        Link { class: "button button-primary", to: Route::Providers {}, "Configure Providers" }
                    }
                }
            } else if !is_loading {
                div { class: "stack-lg",
                    for (group_name, rows) in groups {
                        {
                            let health = route_group_health(&rows);
                            let active_count = health.active_candidates;
                            let configured_model_count = health.configured_models;
                            let available_model_count = health.available_models;
                            let redundant_model_count = health.redundant_models;
                            let unavailable_model_count = health.unavailable_models;
                            let single_model_count = health.single_upstream_models;
                            let (group_health_class, group_health_text) = if available_model_count == 0 {
                                ("badge badge-error", "Unavailable")
                            } else if unavailable_model_count > 0 {
                                ("badge badge-warning", "Partial coverage")
                            } else if single_model_count > 0 {
                                ("badge badge-warning", "Single upstream")
                            } else {
                                ("badge badge-success", "Redundant")
                            };
                            let model_summary = format!("{available_model_count}/{configured_model_count} model IDs available • {redundant_model_count} redundant");
                            rsx! {
                                div { class: "card card-pad stack route-group-card",
                                    div { class: "product-section-head",
                                        div { class: "route-group-heading",
                                            div { class: "row gap-2",
                                                h3 { title: "{group_name}", "{group_name}" }
                                                span { class: "{group_health_class}", "{group_health_text}" }
                                            }
                                            p { "{active_count} active of {rows.len()} candidates • {model_summary}" }
                                        }
                                        if available_model_count == 0 || unavailable_model_count > 0 {
                                            Link { class: "button button-secondary button-sm", to: Route::Providers {}, "Fix Providers" }
                                        } else if single_model_count > 0 {
                                            Link { class: "button button-secondary button-sm", to: Route::Providers {}, "Add Failover" }
                                        } else {
                                            Link { class: "button button-secondary button-sm", to: Route::Playground {}, "Test Route" }
                                        }
                                    }
                                    div { class: "table-wrap",
                                        table { class: "data-table route-candidate-table",
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
                                                        let model_count_text = if model_count == 1 { "1 model ID".to_string() } else { format!("{model_count} model IDs") };
                                                        rsx! {
                                                            tr { key: "{channel.id}",
                                                                td { class: "mono", "#{preference}" }
                                                                td { class: "table-primary route-provider-name", title: "{channel.name}", "{channel.name}" }
                                                                td { span { class: if channel.status == 1 { "badge badge-success" } else { "badge badge-error" }, "{status}" } }
                                                                td { "{model_count_text}" }
                                                                td { class: "right tabular", "{channel.priority}" }
                                                                td { class: "right tabular", "{channel.weight}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if unavailable_model_count > 0 {
                                        div { class: "product-note route-risk-note",
                                            "{unavailable_model_count} model IDs in this routing group currently have no active upstream. {single_model_count} additional model IDs have only one active upstream."
                                        }
                                    } else if single_model_count > 0 {
                                        div { class: "product-note route-risk-note",
                                            "{single_model_count} model IDs in this routing group still rely on a single active upstream. Add overlapping provider coverage to create failover."
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
