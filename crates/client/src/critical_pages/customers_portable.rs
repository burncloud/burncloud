use dioxus::prelude::*;

use crate::{
    components::Icon,
    data::{Customer, CUSTOMERS, ROUTES},
};

#[component]
pub fn Customers() -> Element {
    let mut query = use_signal(String::new);
    let mut drawer_open = use_signal(|| false);
    let mut selected_index = use_signal(|| None::<usize>);
    let mut suspended = use_signal(Vec::<usize>::new);
    let mut pending_budget = use_signal(|| 5000u32);
    let mut pending_rps = use_signal(|| 50u32);
    let mut save_notice = use_signal(String::new);

    let query_text = query().to_lowercase();
    let visible: Vec<(usize, Customer)> = CUSTOMERS
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, customer)| {
            query_text.is_empty()
                || customer.name.to_lowercase().contains(&query_text)
                || customer.route.to_lowercase().contains(&query_text)
                || customer.environment.to_lowercase().contains(&query_text)
        })
        .collect();

    let total_spend: u32 = CUSTOMERS.iter().map(|customer| customer.spend).sum();
    let total_requests: u32 = CUSTOMERS.iter().map(|customer| customer.requests).sum();
    let alert_count = CUSTOMERS
        .iter()
        .filter(|customer| customer.budget > 0 && customer.spend as f64 / customer.budget as f64 >= 0.9)
        .count();
    let spend_text = format!("${:.2}K", total_spend as f64 / 1000.0);
    let requests_text = format!("{:.2}M Req", total_requests as f64 / 1_000_000.0);
    let visible_count = visible.len();
    let suspended_snapshot = suspended();
    let selected_customer = selected_index().and_then(|index| CUSTOMERS.get(index).copied());

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Customers" }
                    p { class: "page-subtitle",
                        "Manage tenant metadata, rate limits, budgets, route policies, API keys, and account status."
                    }
                }
                div { class: "header-actions",
                    button {
                        r#type: "button",
                        class: "button button-primary",
                        onclick: move |_| {
                            selected_index.set(None);
                            pending_budget.set(5000);
                            pending_rps.set(50);
                            save_notice.set(String::new());
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
                        span { class: "metric-value", "{CUSTOMERS.len()}" }
                    }
                    div { class: "metric-icon tone-gray", Icon { name: "users" } }
                }
                div { class: "card metric card-hover",
                    div { class: "metric-copy",
                        span { class: "metric-label", "Active Month Spend" }
                        span { class: "metric-value", "{spend_text}" }
                    }
                    div { class: "metric-icon tone-green", Icon { name: "dollar" } }
                }
                div { class: "card metric card-hover",
                    div { class: "metric-copy",
                        span { class: "metric-label", "Total Demands" }
                        span { class: "metric-value", "{requests_text}" }
                    }
                    div { class: "metric-icon tone-blue", Icon { name: "activity" } }
                }
                div { class: "card metric card-hover",
                    div { class: "metric-copy",
                        span { class: "metric-label", "Budget Alerts" }
                        span { class: "metric-value", "{alert_count} Critical" }
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
                            oninput: move |evt| query.set(evt.value()),
                        }
                    }
                    span { class: "small muted", "Showing {visible_count} of {CUSTOMERS.len()} tenants" }
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
                            for (index, customer) in visible {
                                {
                                    let is_suspended = suspended_snapshot.contains(&index);
                                    let ratio = if customer.budget == 0 {
                                        0.0
                                    } else {
                                        customer.spend as f64 / customer.budget as f64
                                    };
                                    let budget_state = if ratio >= 0.9 {
                                        "Critical"
                                    } else if ratio >= 0.6 {
                                        "Watch"
                                    } else {
                                        "Healthy"
                                    };
                                    rsx! {
                                        tr {
                                            td { class: "table-primary", "{customer.name}" }
                                            td {
                                                span {
                                                    class: if customer.environment == "Production" {
                                                        "badge badge-brand"
                                                    } else if customer.environment == "Staging" {
                                                        "badge badge-warning"
                                                    } else {
                                                        "badge badge-neutral"
                                                    },
                                                    "{customer.environment}"
                                                }
                                            }
                                            td {
                                                div { class: "two-line customer-cost",
                                                    span {
                                                        class: if ratio >= 0.9 { "strong danger" } else { "strong" },
                                                        "${customer.spend} / ${customer.budget}"
                                                    }
                                                    small { "Budget health: {budget_state}" }
                                                }
                                            }
                                            td { class: "right tabular", "{customer.rps} RPS" }
                                            td { class: "mono muted", "{customer.route}" }
                                            td { class: "center",
                                                span { class: "badge badge-neutral mono", "🔑 {customer.keys}" }
                                            }
                                            td { class: "center",
                                                button {
                                                    r#type: "button",
                                                    class: if is_suspended { "badge badge-neutral" } else { "badge badge-success" },
                                                    onclick: move |_| {
                                                        let mut next = suspended();
                                                        if let Some(position) = next.iter().position(|value| *value == index) {
                                                            next.remove(position);
                                                        } else {
                                                            next.push(index);
                                                        }
                                                        suspended.set(next);
                                                    },
                                                    if is_suspended { "Suspended" } else { "Active" }
                                                }
                                            }
                                            td {
                                                button {
                                                    r#type: "button",
                                                    class: "button button-ghost button-sm",
                                                    onclick: move |_| {
                                                        selected_index.set(Some(index));
                                                        pending_budget.set(customer.budget);
                                                        pending_rps.set(customer.rps);
                                                        save_notice.set(String::new());
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

            if drawer_open() {
                div {
                    class: "drawer-backdrop",
                    onclick: move |_| drawer_open.set(false),
                }
                aside { class: "drawer",
                    div { class: "drawer-head",
                        h2 {
                            if selected_customer.is_some() {
                                "Tenant Profile"
                            } else {
                                "Create Tenant Customer"
                            }
                        }
                        button {
                            r#type: "button",
                            class: "close-button",
                            onclick: move |_| drawer_open.set(false),
                            "×"
                        }
                    }
                    div { class: "drawer-body stack-lg",
                        if let Some(customer) = selected_customer {
                            div { class: "card card-pad stack",
                                div { class: "row between",
                                    span { class: "section-label", "Historical Demands" }
                                    span { class: "tiny mono subtle", "Tenant source: seeded workspace" }
                                }
                                div { class: "grid-2",
                                    div {
                                        span { class: "tiny subtle", "Customer" }
                                        div { class: "small strong", "{customer.name}" }
                                    }
                                    div {
                                        span { class: "tiny subtle", "Environment" }
                                        div { class: "small strong", "{customer.environment}" }
                                    }
                                    div {
                                        span { class: "tiny subtle", "Requests" }
                                        div { class: "small strong mono", "{customer.requests}" }
                                    }
                                    div {
                                        span { class: "tiny subtle", "Current spend" }
                                        div { class: "small strong mono", "${customer.spend}" }
                                    }
                                }
                            }
                        } else {
                            div { class: "card card-pad small muted",
                                "Create a tenant profile, then assign a route policy, monthly budget, RPS limit, and initial API-key policy."
                            }
                        }

                        div { class: "field",
                            label { "Tenant Customer Name" }
                            input {
                                class: "input",
                                placeholder: if selected_customer.is_some() { "Rename customer (optional)" } else { "e.g. AeroTech Corp" },
                            }
                        }

                        div { class: "grid-2",
                            div { class: "field",
                                label { "Environment" }
                                select { class: "select",
                                    option { "Production" }
                                    option { "Staging" }
                                    option { "Development" }
                                }
                            }
                            div { class: "field",
                                label { "Default Route Policy" }
                                select { class: "select",
                                    for route in ROUTES {
                                        option { "{route.name}" }
                                    }
                                }
                            }
                        }

                        div { class: "grid-2",
                            div { class: "field",
                                label { "Rate Limit Quota" }
                                input {
                                    class: "input",
                                    r#type: "number",
                                    min: "5",
                                    max: "500",
                                    placeholder: "50",
                                    oninput: move |evt| {
                                        if let Ok(value) = evt.value().parse::<u32>() {
                                            pending_rps.set(value.clamp(5, 500));
                                        }
                                    },
                                }
                                span { class: "tiny subtle", "Pending: {pending_rps} RPS" }
                            }
                            div { class: "field",
                                label { "Monthly Budget Threshold" }
                                input {
                                    class: "input",
                                    r#type: "number",
                                    min: "500",
                                    max: "50000",
                                    placeholder: "5000",
                                    oninput: move |evt| {
                                        if let Ok(value) = evt.value().parse::<u32>() {
                                            pending_budget.set(value.clamp(500, 50000));
                                        }
                                    },
                                }
                                span { class: "tiny subtle", "Pending: ${pending_budget}" }
                            }
                        }

                        if !save_notice().is_empty() {
                            div { class: "badge badge-success customer-form-error", "{save_notice}" }
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
                                    save_notice.set("Tenant controls validated. Save action is ready for backend binding.".to_string());
                                },
                                if selected_customer.is_some() { "Update Controls" } else { "Create Tenant" }
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
