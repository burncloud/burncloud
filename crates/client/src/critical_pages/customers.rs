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

    let tenant_snapshot = tenants();
    let query_value = query().to_lowercase();
    let visible: Vec<Tenant> = tenant_snapshot
        .iter()
        .filter(|tenant| {
            query_value.is_empty()
                || tenant.name.to_lowercase().contains(&query_value)
                || tenant.route.to_lowercase().contains(&query_value)
        })
        .cloned()
        .collect();
    let visible_count = visible.len();
    let total_count = tenant_snapshot.len();
    let total_spend: u32 = tenant_snapshot.iter().map(|tenant| tenant.spend).sum();
    let total_requests: u32 = tenant_snapshot.iter().map(|tenant| tenant.requests).sum();
    let critical = tenant_snapshot
        .iter()
        .filter(|tenant| tenant.budget > 0 && (tenant.spend as f64 / tenant.budget as f64) >= 0.9)
        .count();
    let selected = edit_id()
        .and_then(|id| tenant_snapshot.iter().find(|tenant| tenant.id == id).cloned());

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Customers" }
                    p { class: "page-subtitle", "Manage tenant metadata, dynamic rate limits, model access budgets, and route policy mapping." }
                }
                div { class: "header-actions",
                    button {
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
                TenantMetric { label: "Total Tenants", value: total_count.to_string(), icon: "users", tone: "tone-gray" }
                TenantMetric { label: "Active Month Spend", value: format!("${:.2}K", total_spend as f64 / 1000.0), icon: "dollar", tone: "tone-green" }
                TenantMetric { label: "Total Demands", value: format!("{:.2}M Req", total_requests as f64 / 1_000_000.0), icon: "activity", tone: "tone-blue" }
                TenantMetric { label: "Budget Alerts", value: format!("{} Critical", critical), icon: "shield", tone: "tone-amber" }
            }

            div { class: "card table-card",
                div { style: "padding:20px;border-bottom:1px solid #f3f4f6;display:flex;align-items:center;gap:16px",
                    div { class: "search-field", style: "max-width:420px;flex:1",
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
                                    let name_for_edit = tenant.name.clone();
                                    let env_for_edit = tenant.environment.clone();
                                    let route_for_edit = tenant.route.clone();
                                    let budget_for_edit = tenant.budget;
                                    let rps_for_edit = tenant.rps;
                                    let id_for_toggle = tenant.id.clone();
                                    let ratio = if tenant.budget == 0 { 0.0 } else { tenant.spend as f64 / tenant.budget as f64 };
                                    let width = format!("width:{:.0}%", (ratio * 100.0).min(100.0));
                                    let progress_tone = if ratio >= 0.9 {
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
                                                        span { style: "width:6px;height:6px;border-radius:50%;background:#ef4444" }
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
                                                div { class: "two-line", style: "min-width:150px",
                                                    span { class: if ratio >= 0.9 { "strong danger" } else { "strong" }, "${tenant.spend} / ${tenant.budget}" }
                                                    div { class: "progress tenant-progress", span { style: "{width};{progress_tone}" } }
                                                }
                                            }
                                            td { class: "right tabular", "{tenant.rps} RPS" }
                                            td { class: "mono muted", "{tenant.route}" }
                                            td { class: "center", span { class: "badge badge-neutral mono", "🔑 {tenant.keys}" } }
                                            td { class: "center",
                                                button {
                                                    r#type: "button",
                                                    class: if tenant.status == "Active" { "badge badge-success" } else { "badge badge-neutral" },
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
                                                        form_name.set(name_for_edit.clone());
                                                        form_env.set(env_for_edit.clone());
                                                        form_budget.set(budget_for_edit);
                                                        form_rps.set(rps_for_edit);
                                                        form_route.set(route_for_edit.clone());
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
                        div { class: "card card-pad stack",
                            div { class: "row between",
                                span { class: "section-label", "Historical Demands" }
                                span { class: "tiny mono subtle", "ID: {tenant.id}" }
                            }
                            div { class: "grid-2",
                                DetailStat { label: "Total API Demands", value: format!("{} requests", tenant.requests) }
                                DetailStat { label: "Spend Velocity", value: format!("${:.2} / day", tenant.spend as f64 / 30.0) }
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
                        span { class: "tiny subtle", "Throttle traffic automatically when this tenant exceeds the requests-per-second quota." }
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
                        span { class: "tiny subtle", "Notifications or model failovers trigger once cumulative tenant spend crosses this threshold." }
                    }

                    if let Some(ref tenant) = selected {
                        div { class: "card card-pad stack",
                            span { class: "section-label", "Client Health Status Actions" }
                            button {
                                r#type: "button",
                                class: "button button-secondary",
                                onclick: {
                                    let id = tenant.id.clone();
                                    move |_| {
                                        let mut next = tenants();
                                        if let Some(item) = next.iter_mut().find(|item| item.id == id) {
                                            item.status = if item.status == "Active" {
                                                "Suspended".to_string()
                                            } else {
                                                "Active".to_string()
                                            };
                                        }
                                        tenants.set(next);
                                    }
                                },
                                if tenant.status == "Active" { "Suspend Tenant Access" } else { "Reactivate Tenant Access" }
                            }
                        }
                    }

                    if !save_error().is_empty() {
                        div { class: "badge badge-error", style: "padding:10px", "{save_error}" }
                    }

                    div { class: "row", style: "justify-content:flex-end",
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
                            if edit_id().is_some() { "Update Controls" } else { "Create Tenant" }
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

#[component]
fn TenantMetric(label: &'static str, value: String, icon: &'static str, tone: &'static str) -> Element {
    rsx! {
        div { class: "card metric card-hover",
            div { class: "metric-copy",
                span { class: "metric-label", "{label}" }
                span { class: "metric-value", "{value}" }
            }
            div { class: "metric-icon {tone}", Icon { name: icon } }
        }
    }
}

#[component]
fn DetailStat(label: &'static str, value: String) -> Element {
    rsx! {
        div {
            span { class: "tiny subtle", "{label}" }
            div { class: "small strong mono", "{value}" }
        }
    }
}
