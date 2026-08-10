use dioxus::prelude::*;

use crate::{
    components::{Drawer, Icon},
    data::{Customer, CUSTOMERS, ROUTES},
};

#[derive(Clone, PartialEq)]
struct Tenant {
    id: String,
    name: String,
    environment: String,
    spend: u32,
    budget: u32,
    rps: u32,
    route: String,
    status: String,
    keys: u32,
    requests: u32,
}

impl Tenant {
    fn from_customer(index: usize, customer: Customer) -> Self {
        Self {
            id: format!("c{}", index + 1),
            name: customer.name.to_string(),
            environment: customer.environment.to_string(),
            spend: customer.spend,
            budget: customer.budget,
            rps: customer.rps,
            route: customer.route.to_string(),
            status: "Active".to_string(),
            keys: customer.keys,
            requests: customer.requests,
        }
    }
}

#[component]
pub fn Customers() -> Element {
    let mut tenants = use_signal(|| {
        CUSTOMERS
            .iter()
            .copied()
            .enumerate()
            .map(|(index, customer)| Tenant::from_customer(index, customer))
            .collect::<Vec<_>>()
    });
    let mut query = use_signal(String::new);
    let mut drawer_open = use_signal(|| false);
    let mut edit_id = use_signal(|| None::<String>);
    let mut form_name = use_signal(String::new);
    let mut form_env = use_signal(|| "Production".to_string());
    let mut form_budget = use_signal(|| 5000u32);
    let mut form_rps = use_signal(|| 50u32);
    let mut form_route = use_signal(|| "production-chat-default".to_string());
    let mut save_error = use_signal(String::new);

    let snapshot = tenants();
    let query_text = query().to_lowercase();
    let visible: Vec<Tenant> = snapshot
        .iter()
        .filter(|tenant| {
            query_text.is_empty()
                || tenant.name.to_lowercase().contains(&query_text)
                || tenant.route.to_lowercase().contains(&query_text)
                || tenant.environment.to_lowercase().contains(&query_text)
        })
        .cloned()
        .collect();
    let visible_count = visible.len();
    let total_count = snapshot.len();
    let total_spend: u32 = snapshot.iter().map(|tenant| tenant.spend).sum();
    let total_requests: u32 = snapshot.iter().map(|tenant| tenant.requests).sum();
    let budget_alerts = snapshot
        .iter()
        .filter(|tenant| tenant.budget > 0 && tenant.spend as f64 / tenant.budget as f64 >= 0.9)
        .count();
    let total_spend_text = format!("${:.2}K", total_spend as f64 / 1000.0);
    let total_requests_text = format!("{:.2}M Req", total_requests as f64 / 1_000_000.0);
    let alert_text = format!("{budget_alerts} Critical");
    let selected = edit_id()
        .and_then(|id| snapshot.iter().find(|tenant| tenant.id == id).cloned());

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Customers" }
                    p { class: "page-subtitle",
                        "Manage tenant metadata, dynamic rate limits, model access budgets, and route policy mapping."
                    }
                }
                div { class: "header-actions",
                    button {
                        r#type: "button",
                        class: "button button-primary",
                        onclick: move |_| {
                            edit_id.set(None);
                            form_name.set(String::new());
                            form_env.set("Production".to_string());
                            form_budget.set(5000);
                            form_rps.set(50);
                            form_route.set("production-chat-default".to_string());
                            save_error.set(String::new());
                            drawer_open.set(true);
                        },
                        Icon { name: "plus" }
                        "Add Tenant Customer"
                    }
                }
            }

            div { class: "metrics",
                div { class: "card metric card-hover",
                    div { class: "metric-copy",
                        span { class: "metric-label", "Total Tenants" }
                        span { class: "metric-value", "{total_count}" }
                    }
                    div { class: "metric-icon tone-gray", Icon { name: "users" } }
                }
                div { class: "card metric card-hover",
                    div { class: "metric-copy",
                        span { class: "metric-label", "Active Month Spend" }
                        span { class: "metric-value", "{total_spend_text}" }
                    }
                    div { class: "metric-icon tone-green", Icon { name: "dollar" } }
                }
                div { class: "card metric card-hover",
                    div { class: "metric-copy",
                        span { class: "metric-label", "Total Demands" }
                        span { class: "metric-value", "{total_requests_text}" }
                    }
                    div { class: "metric-icon tone-blue", Icon { name: "activity" } }
                }
                div { class: "card metric card-hover",
                    div { class: "metric-copy",
                        span { class: "metric-label", "Budget Alerts" }
                        span { class: "metric-value", "{alert_text}" }
                    }
                    div { class: "metric-icon tone-amber", Icon { name: "shield" } }
                }
            }

            div { class: "card table-card",
                div { class: "customer-toolbar",
                    div { class: "search-field customer-search",
                        Icon { name: "search" }
                        input {
                            class: "input",
                            placeholder: "Search tenants or active policies...",
                            value: "{query}",
                            oninput: move |evt| query.set(evt.value()),
                        }
                    }
                    span { class: "small muted", "Showing {visible_count} of {total_count} tenants" }
                }

                div { class: "table-wrap",
                    table { class: "data-table",
                        thead { tr {
                            th { "Tenant Name" }
                            th { "Environment" }
                            th { "Monthly Cost Track" }
                            th { class: "right", "Quota Rate Limit" }
                            th { "Assigned Default Route" }
                            th { class: "center", "API Keys" }
                            th { class: "center", "Status" }
                            th { "" }
                        } }
                        tbody {
                            for tenant in visible {
                                {
                                    let id_for_edit = tenant.id.clone();
                                    let id_for_toggle = tenant.id.clone();
                                    let edit_name = tenant.name.clone();
                                    let edit_environment = tenant.environment.clone();
                                    let edit_route = tenant.route.clone();
                                    let edit_budget = tenant.budget;
                                    let edit_rps = tenant.rps;
                                    let ratio = if tenant.budget == 0 {
                                        0.0
                                    } else {
                                        tenant.spend as f64 / tenant.budget as f64
                                    };
                                    let progress_width = format!("width:{:.0}%", (ratio * 100.0).min(100.0));
                                    let progress_color = if ratio >= 0.9 {
                                        "background:#ef4444"
                                    } else if ratio >= 0.6 {
                                        "background:#f59e0b"
                                    } else {
                                        "background:#10b981"
                                    };
                                    rsx! {
                                        tr {
                                            td { class: "table-primary",
                                                div { class: "row gap-2",
                                                    "{tenant.name}"
                                                    if ratio >= 0.9 {
                                                        span { class: "customer-alert-dot" }
                                                    }
                                                }
                                            }
                                            td {
                                                span {
                                                    class: if tenant.environment == "Production" {
                                                        "badge badge-brand"
                                                    } else if tenant.environment == "Staging" {
                                                        "badge badge-warning"
                                                    } else {
                                                        "badge badge-neutral"
                                                    },
                                                    "{tenant.environment}"
                                                }
                                            }
                                            td {
                                                div { class: "two-line customer-cost",
                                                    span {
                                                        class: if ratio >= 0.9 { "strong danger" } else { "strong" },
                                                        "${tenant.spend} / ${tenant.budget}"
                                                    }
                                                    div { class: "progress tenant-progress",
                                                        span { style: "{progress_width};{progress_color}" }
                                                    }
                                                }
                                            }
                                            td { class: "right tabular", "{tenant.rps} RPS" }
                                            td { class: "mono muted", "{tenant.route}" }
                                            td { class: "center",
                                                span { class: "badge badge-neutral mono", "🔑 {tenant.keys}" }
                                            }
                                            td { class: "center",
                                                button {
                                                    r#type: "button",
                                                    class: if tenant.status == "Active" {
                                                        "badge badge-success"
                                                    } else {
                                                        "badge badge-neutral"
                                                    },
                                                    onclick: move |_| {
                                                        let mut next = tenants();
                                                        if let Some(item) = next.iter_mut().find(|item| item.id == id_for_toggle) {
                                                            item.status = if item.status == "Active" {
                                                                "Suspended".to_string()
                                                            } else {
                                                                "Active".to_string()
                                                            };
                                                        }
                                                        tenants.set(next);
                                                    },
                                                    "{tenant.status}"
                                                }
                                            }
                                            td {
                                                button {
                                                    r#type: "button",
                                                    class: "button button-ghost button-sm",
                                                    onclick: move |_| {
                                                        edit_id.set(Some(id_for_edit.clone()));
                                                        form_name.set(edit_name.clone());
                                                        form_env.set(edit_environment.clone());
                                                        form_budget.set(edit_budget);
                                                        form_rps.set(edit_rps);
                                                        form_route.set(edit_route.clone());
                                                        save_error.set(String::new());
                                                        drawer_open.set(true);
                                                    },
                                                    "Edit"
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

            Drawer {
                title: if edit_id().is_some() { "Tenant Profile" } else { "Create Tenant Customer" },
                open: drawer_open(),
                on_close: move |_| drawer_open.set(false),
                div { class: "stack-lg",
                    if let Some(ref tenant) = selected {
                        {
                            let request_text = format!("{} requests", tenant.requests);
                            let velocity_text = format!("${:.2} / day", tenant.spend as f64 / 30.0);
                            rsx! {
                                div { class: "card card-pad stack",
                                    div { class: "row between",
                                        span { class: "section-label", "Historical Demands" }
                                        span { class: "tiny mono subtle", "ID: {tenant.id}" }
                                    }
                                    div { class: "grid-2",
                                        div {
                                            span { class: "tiny subtle", "Total API Demands" }
                                            div { class: "small strong mono", "{request_text}" }
                                        }
                                        div {
                                            span { class: "tiny subtle", "Spend Velocity" }
                                            div { class: "small strong mono", "{velocity_text}" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "field",
                        label { "Tenant Customer Name" }
                        input {
                            class: "input",
                            value: "{form_name}",
                            placeholder: "e.g. AeroTech Corp",
                            oninput: move |evt| form_name.set(evt.value()),
                        }
                    }

                    div { class: "grid-2",
                        div { class: "field",
                            label { "Environment" }
                            select {
                                class: "select",
                                value: "{form_env}",
                                oninput: move |evt| form_env.set(evt.value()),
                                option { value: "Production", "Production" }
                                option { value: "Staging", "Staging" }
                                option { value: "Development", "Development" }
                            }
                        }
                        div { class: "field",
                            label { "Default Route Policy" }
                            select {
                                class: "select",
                                value: "{form_route}",
                                oninput: move |evt| form_route.set(evt.value()),
                                for route in ROUTES {
                                    option { value: route.name, "{route.name}" }
                                }
                            }
                        }
                    }

                    div { class: "field",
                        div { class: "row between",
                            label { "Rate Limit Quota" }
                            strong { class: "mono small", "{form_rps} RPS" }
                        }
                        input {
                            r#type: "range",
                            min: "5",
                            max: "500",
                            step: "5",
                            value: "{form_rps}",
                            oninput: move |evt| {
                                if let Ok(value) = evt.value().parse::<u32>() {
                                    form_rps.set(value);
                                }
                            },
                        }
                        span { class: "tiny subtle",
                            "Throttle traffic automatically when this tenant exceeds the requests-per-second quota."
                        }
                    }

                    div { class: "field",
                        div { class: "row between",
                            label { "Monthly Budget Threshold" }
                            strong { class: "mono small", "${form_budget}" }
                        }
                        input {
                            r#type: "range",
                            min: "500",
                            max: "50000",
                            step: "500",
                            value: "{form_budget}",
                            oninput: move |evt| {
                                if let Ok(value) = evt.value().parse::<u32>() {
                                    form_budget.set(value);
                                }
                            },
                        }
                        span { class: "tiny subtle",
                            "Notifications or model failovers trigger once cumulative tenant spend crosses this threshold."
                        }
                    }

                    if let Some(ref tenant) = selected {
                        {
                            let status_id = tenant.id.clone();
                            rsx! {
                                div { class: "card card-pad stack",
                                    span { class: "section-label", "Client Health Status Actions" }
                                    button {
                                        r#type: "button",
                                        class: "button button-secondary",
                                        onclick: move |_| {
                                            let mut next = tenants();
                                            if let Some(item) = next.iter_mut().find(|item| item.id == status_id) {
                                                item.status = if item.status == "Active" {
                                                    "Suspended".to_string()
                                                } else {
                                                    "Active".to_string()
                                                };
                                            }
                                            tenants.set(next);
                                        },
                                        if tenant.status == "Active" {
                                            "Suspend Tenant Access"
                                        } else {
                                            "Reactivate Tenant Access"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if !save_error().is_empty() {
                        div { class: "badge badge-error customer-form-error", "{save_error}" }
                    }

                    div { class: "row customer-form-actions",
                        button {
                            r#type: "button",
                            class: "button button-secondary",
                            onclick: move |_| drawer_open.set(false),
                            "Cancel"
                        }
                        button {
                            r#type: "button",
                            class: "button button-primary",
                            onclick: move |_| {
                                let name = form_name().trim().to_string();
                                if name.is_empty() {
                                    save_error.set("Tenant name is required.".to_string());
                                    return;
                                }

                                let mut next = tenants();
                                if let Some(id) = edit_id() {
                                    if let Some(item) = next.iter_mut().find(|item| item.id == id) {
                                        item.name = name;
                                        item.environment = form_env();
                                        item.budget = form_budget();
                                        item.rps = form_rps();
                                        item.route = form_route();
                                    }
                                } else {
                                    next.insert(0, Tenant {
                                        id: format!("c{}", next.len() + 1),
                                        name,
                                        environment: form_env(),
                                        spend: 0,
                                        budget: form_budget(),
                                        rps: form_rps(),
                                        route: form_route(),
                                        status: "Active".to_string(),
                                        keys: 1,
                                        requests: 0,
                                    });
                                }
                                tenants.set(next);
                                drawer_open.set(false);
                            },
                            if edit_id().is_some() {
                                "Update Controls"
                            } else {
                                "Create Tenant"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Users() -> Element {
    rsx! { Customers {} }
}
