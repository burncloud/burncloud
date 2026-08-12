use dioxus::prelude::*;

use crate::{
    backend::{User, UserService},
    components::Icon,
};

fn format_nano(nano: i64, symbol: &str) -> String {
    format!("{symbol}{:.2}", nano as f64 / 1_000_000_000.0)
}

fn is_staff_role(role: &str) -> bool {
    matches!(
        role.trim().to_ascii_lowercase().as_str(),
        "root" | "admin" | "administrator" | "operator" | "owner"
    )
}

#[component]
pub fn Customers() -> Element {
    let mut users = use_resource(move || async move { UserService::list().await });
    let mut query = use_signal(String::new);
    let mut active_only = use_signal(|| false);
    let mut create_open = use_signal(|| false);
    let mut topup_user = use_signal(|| None::<User>);
    let mut username = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut amount = use_signal(|| 0i64);
    let mut currency = use_signal(|| "CNY".to_string());
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(String::new);
    let mut error = use_signal(String::new);

    let resource = users.read().clone();
    let loading = resource.is_none();
    let load_error = resource.as_ref().and_then(|result| result.as_ref().err().cloned());
    let all_users = resource.and_then(Result::ok).unwrap_or_default();
    let customer_list: Vec<User> = all_users
        .into_iter()
        .filter(|user| !is_staff_role(&user.role))
        .collect();

    let search = query().to_lowercase();
    let filtered: Vec<User> = customer_list
        .iter()
        .filter(|user| {
            (!active_only() || user.status == 1)
                && (search.is_empty()
                    || user.username.to_lowercase().contains(&search)
                    || user.id.to_lowercase().contains(&search)
                    || user.email.as_deref().unwrap_or("").to_lowercase().contains(&search))
        })
        .cloned()
        .collect();

    let active_count = customer_list.iter().filter(|user| user.status == 1).count();
    let cny_total: i64 = customer_list.iter().map(|user| user.balance_cny).sum();
    let usd_total: i64 = customer_list.iter().map(|user| user.balance_usd).sum();
    let cny_total_text = format_nano(cny_total, "CNY ");
    let usd_total_text = format_nano(usd_total, "$");
    let filtered_count = filtered.len();
    let customer_count = customer_list.len();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Customers" }
                    p { class: "page-subtitle", "Manage business accounts, review wallet balances, and fund customer usage. Administrative staff are shown separately under Team." }
                }
                div { class: "header-actions",
                    button { class: "button button-secondary", onclick: move |_| users.restart(), "Refresh" }
                    button {
                        class: "button button-primary",
                        onclick: move |_| {
                            username.set(String::new());
                            email.set(String::new());
                            password.set(String::new());
                            notice.set(String::new());
                            error.set(String::new());
                            create_open.set(true);
                        },
                        Icon { name: "plus" }
                        "Create Customer"
                    }
                }
            }

            div { class: "metrics",
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Customers" } span { class: "metric-value", "{customer_count}" } span { class: "metric-note", "business accounts" } }
                    div { class: "metric-icon tone-gray", Icon { name: "users" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Active" } span { class: "metric-value", "{active_count}" } span { class: "metric-note", "enabled accounts" } }
                    div { class: "metric-icon tone-green", Icon { name: "activity" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "CNY Wallets" } span { class: "metric-value", "{cny_total_text}" } span { class: "metric-note", "aggregate balance" } }
                    div { class: "metric-icon tone-amber", Icon { name: "dollar" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "USD Wallets" } span { class: "metric-value", "{usd_total_text}" } span { class: "metric-note", "aggregate balance" } }
                    div { class: "metric-icon tone-blue", Icon { name: "dollar" } }
                }
            }

            if !notice().is_empty() { div { class: "terminal auth-status", "{notice}" } }
            if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }

            if loading {
                div { class: "card card-pad", "Loading customers…" }
            } else if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Customers could not be loaded" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| users.restart(), "Retry" }
                }
            } else if customer_list.is_empty() {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "users" } }
                        h3 { "Create the first customer account" }
                        p { "Customer accounts own wallet balances and can receive API keys for routed model usage. Team members are intentionally managed separately." }
                        button { class: "button button-primary", onclick: move |_| create_open.set(true), "Create Customer" }
                    }
                }
            } else {
                div { class: "card table-card",
                    div { class: "customer-toolbar",
                        div { class: "search-field customer-search",
                            Icon { name: "search" }
                            input {
                                class: "input",
                                placeholder: "Search customer, email or ID…",
                                value: "{query}",
                                oninput: move |event| query.set(event.value()),
                            }
                        }
                        label { class: "row gap-2 small muted",
                            input { r#type: "checkbox", checked: active_only(), onclick: move |_| active_only.set(!active_only()) }
                            "Active only"
                        }
                        span { class: "small muted", "Showing {filtered_count} of {customer_count}" }
                    }

                    if filtered.is_empty() {
                        div { class: "product-empty", style: "min-height:160px",
                            div { class: "product-empty-inner",
                                h3 { "No customers match this view" }
                                p { "Change the search text or clear the Active only filter." }
                            }
                        }
                    } else {
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr {
                                    th { "Customer" }
                                    th { "Status" }
                                    th { "Wallet" }
                                    th { "Preferred Currency" }
                                    th { class: "right", "Actions" }
                                } }
                                tbody {
                                    for user in filtered {
                                        {
                                            let email_text = user.email.clone().unwrap_or_else(|| "No email".to_string());
                                            let status_text = if user.status == 1 { "Active" } else { "Disabled" };
                                            let cny_text = format_nano(user.balance_cny, "CNY ");
                                            let usd_text = format_nano(user.balance_usd, "$");
                                            let preferred = user.preferred_currency.clone().unwrap_or_else(|| "Not set".to_string());
                                            rsx! {
                                                tr { key: "{user.id}",
                                                    td {
                                                        div { class: "two-line",
                                                            strong { class: "table-primary", "{user.username}" }
                                                            small { class: "muted", "{email_text} • {user.id}" }
                                                        }
                                                    }
                                                    td { span { class: if user.status == 1 { "badge badge-success" } else { "badge badge-neutral" }, "{status_text}" } }
                                                    td {
                                                        div { class: "two-line",
                                                            strong { class: "tabular", "{cny_text}" }
                                                            small { class: "tabular muted", "{usd_text}" }
                                                        }
                                                    }
                                                    td { "{preferred}" }
                                                    td { class: "right",
                                                        button {
                                                            class: "button button-secondary button-sm",
                                                            onclick: move |_| {
                                                                amount.set(0);
                                                                currency.set(user.preferred_currency.clone().filter(|value| value == "USD" || value == "CNY").unwrap_or_else(|| "CNY".to_string()));
                                                                notice.set(String::new());
                                                                error.set(String::new());
                                                                topup_user.set(Some(user.clone()));
                                                            },
                                                            "Add Funds"
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
                    div { class: "card-pad product-note",
                        "Account status is currently read-only because the BurnCloud server does not yet expose a safe suspend/reactivate endpoint. The UI does not simulate that action."
                    }
                }
            }

            if create_open() {
                div { class: "drawer-backdrop", onclick: move |_| create_open.set(false) }
                aside { class: "drawer",
                    div { class: "drawer-head",
                        div { h2 { "Create Customer" } p { class: "small muted", "Create a business account that can own wallet balance and API access." } }
                        button { class: "close-button", onclick: move |_| create_open.set(false), "×" }
                    }
                    div { class: "drawer-body stack-lg",
                        div { class: "form-section",
                            div { class: "form-section-head", strong { "Account identity" } small { "The username is the customer's BurnCloud login identity." } }
                            div { class: "field", label { "Username" } input { class: "input", value: "{username}", disabled: busy(), oninput: move |event| username.set(event.value()) } }
                            div { class: "field", label { "Email (optional)" } input { class: "input", r#type: "email", value: "{email}", disabled: busy(), oninput: move |event| email.set(event.value()) } }
                            div { class: "field", label { "Temporary password" } input { class: "input", r#type: "password", value: "{password}", disabled: busy(), placeholder: "At least 8 characters", oninput: move |event| password.set(event.value()) } }
                        }
                        if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }
                        div { class: "row customer-form-actions",
                            button { class: "button button-secondary", disabled: busy(), onclick: move |_| create_open.set(false), "Cancel" }
                            button {
                                class: "button button-primary",
                                disabled: busy(),
                                onclick: move |_| {
                                    let username_value = username().trim().to_string();
                                    let email_value = email().trim().to_string();
                                    let password_value = password();
                                    if username_value.is_empty() || password_value.len() < 8 {
                                        error.set("Username is required and the password must contain at least 8 characters.".to_string());
                                        return;
                                    }
                                    busy.set(true);
                                    error.set(String::new());
                                    spawn(async move {
                                        let email_arg = if email_value.is_empty() { None } else { Some(email_value.as_str()) };
                                        match UserService::create(&username_value, &password_value, email_arg).await {
                                            Ok(_) => {
                                                notice.set(format!("Customer {username_value} created."));
                                                create_open.set(false);
                                                users.restart();
                                            }
                                            Err(message) => error.set(format!("Create customer failed: {message}")),
                                        }
                                        busy.set(false);
                                    });
                                },
                                if busy() { "Creating…" } else { "Create Customer" }
                            }
                        }
                    }
                }
            }

            if let Some(user) = topup_user() {
                {
                    let current_cny = format_nano(user.balance_cny, "CNY ");
                    let current_usd = format_nano(user.balance_usd, "$");
                    rsx! {
                        div { class: "drawer-backdrop", onclick: move |_| topup_user.set(None) }
                        aside { class: "drawer",
                            div { class: "drawer-head",
                                div { h2 { "Add Funds" } p { class: "small muted", "Credit the customer's BurnCloud wallet." } }
                                button { class: "close-button", onclick: move |_| topup_user.set(None), "×" }
                            }
                            div { class: "drawer-body stack-lg",
                                div { class: "form-section",
                                    div { class: "form-section-head", strong { "{user.username}" } small { "Current balances: {current_cny} • {current_usd}" } }
                                    div { class: "grid-2",
                                        div { class: "field",
                                            label { "Currency" }
                                            select { class: "select", value: "{currency}", disabled: busy(), onchange: move |event| currency.set(event.value()),
                                                option { value: "CNY", "CNY" }
                                                option { value: "USD", "USD" }
                                            }
                                        }
                                        div { class: "field",
                                            label { "Amount" }
                                            input { class: "input", r#type: "number", min: "1", value: "{amount}", disabled: busy(), oninput: move |event| amount.set(event.value().parse::<i64>().unwrap_or(0)) }
                                        }
                                    }
                                    div { class: "row gap-2",
                                        button { class: "button button-secondary button-sm", onclick: move |_| amount.set(100), "100" }
                                        button { class: "button button-secondary button-sm", onclick: move |_| amount.set(500), "500" }
                                        button { class: "button button-secondary button-sm", onclick: move |_| amount.set(1000), "1,000" }
                                    }
                                }
                                if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }
                                if !notice().is_empty() { div { class: "terminal auth-status", "{notice}" } }
                                div { class: "row customer-form-actions",
                                    button { class: "button button-secondary", disabled: busy(), onclick: move |_| topup_user.set(None), "Cancel" }
                                    button {
                                        class: "button button-primary",
                                        disabled: busy(),
                                        onclick: move |_| {
                                            let value = amount();
                                            let selected_currency = currency();
                                            let user_id = user.id.clone();
                                            let user_name = user.username.clone();
                                            if value <= 0 {
                                                error.set("Funding amount must be greater than zero.".to_string());
                                                return;
                                            }
                                            busy.set(true);
                                            error.set(String::new());
                                            spawn(async move {
                                                let amount_nano = value.saturating_mul(1_000_000_000);
                                                match UserService::topup(&user_id, amount_nano, &selected_currency).await {
                                                    Ok(_) => {
                                                        notice.set(format!("Added {value} {selected_currency} to {user_name}."));
                                                        topup_user.set(None);
                                                        users.restart();
                                                    }
                                                    Err(message) => error.set(format!("Add funds failed: {message}")),
                                                }
                                                busy.set(false);
                                            });
                                        },
                                        if busy() { "Adding…" } else { "Confirm Funding" }
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
pub fn Users() -> Element {
    rsx! { Customers {} }
}
