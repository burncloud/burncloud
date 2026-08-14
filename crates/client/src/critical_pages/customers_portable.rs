use dioxus::prelude::*;

use crate::{
    backend::{User, UserService},
    components::Icon,
};

const NANO_PER_UNIT: i64 = 1_000_000_000;
const NANO_PER_CENT: i64 = 10_000_000;

fn format_nano(nano: i64, symbol: &str) -> String {
    let value = nano as f64 / NANO_PER_UNIT as f64;
    if nano != 0 && value.abs() < 1.0 {
        format!("{symbol}{value:.6}")
    } else {
        format!("{symbol}{value:.2}")
    }
}

fn format_currency_nano(nano: i64, currency: &str) -> String {
    if currency == "USD" {
        format_nano(nano, "$")
    } else {
        format_nano(nano, "CNY ")
    }
}

fn parse_positive_amount_nano(raw: &str) -> Result<i64, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("Enter a funding amount.".to_string());
    }
    if raw.starts_with('-') {
        return Err("Funding amount must be greater than zero.".to_string());
    }

    let mut parts = raw.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    if parts.next().is_some() {
        return Err("Enter a valid amount with at most one decimal point.".to_string());
    }

    let whole_value = if whole.is_empty() {
        0
    } else {
        if !whole.chars().all(|ch| ch.is_ascii_digit()) {
            return Err("Enter a valid positive amount.".to_string());
        }
        whole
            .parse::<i64>()
            .map_err(|_| "Funding amount is too large.".to_string())?
    };

    let cents = match fraction {
        None => 0,
        Some(value) if value.is_empty() => 0,
        Some(value) => {
            if value.len() > 2 || !value.chars().all(|ch| ch.is_ascii_digit()) {
                return Err("Use no more than two decimal places.".to_string());
            }
            let parsed = value
                .parse::<i64>()
                .map_err(|_| "Enter a valid funding amount.".to_string())?;
            if value.len() == 1 {
                parsed * 10
            } else {
                parsed
            }
        }
    };

    let nano = whole_value
        .checked_mul(NANO_PER_UNIT)
        .and_then(|value| value.checked_add(cents * NANO_PER_CENT))
        .ok_or_else(|| "Funding amount is too large.".to_string())?;

    if nano <= 0 {
        return Err("Funding amount must be greater than zero.".to_string());
    }
    Ok(nano)
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
    let mut default_status_only = use_signal(|| false);
    let mut create_open = use_signal(|| false);
    let mut topup_user = use_signal(|| None::<User>);
    let mut username = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut amount = use_signal(|| "100".to_string());
    let mut currency = use_signal(|| "CNY".to_string());
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(String::new);
    let mut form_error = use_signal(String::new);

    let resource = users.read().clone();
    let loading = resource.is_none();
    let load_error = resource
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let all_users = resource.and_then(Result::ok).unwrap_or_default();
    let customer_list: Vec<User> = all_users
        .into_iter()
        .filter(|user| !is_staff_role(&user.role))
        .collect();

    let search = query().trim().to_lowercase();
    let filtered: Vec<User> = customer_list
        .iter()
        .filter(|user| {
            (!default_status_only() || user.status == 1)
                && (search.is_empty()
                    || user.username.to_lowercase().contains(&search)
                    || user.id.to_lowercase().contains(&search)
                    || user
                        .email
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&search))
        })
        .cloned()
        .collect();

    let default_status_count = customer_list.iter().filter(|user| user.status == 1).count();
    let non_default_status_count = customer_list.len().saturating_sub(default_status_count);
    let cny_total: i64 = customer_list.iter().map(|user| user.balance_cny).sum();
    let usd_total: i64 = customer_list.iter().map(|user| user.balance_usd).sum();
    let cny_total_text = format_nano(cny_total, "CNY ");
    let usd_total_text = format_nano(usd_total, "$");
    let filtered_count = filtered.len();
    let customer_count = customer_list.len();
    let amount_preview = parse_positive_amount_nano(&amount()).ok();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Customers" }
                    p { class: "page-subtitle", "Manage business accounts, review wallet balances, and fund customer usage. Administrative staff are shown separately under Team." }
                }
                div { class: "header-actions",
                    button {
                        class: "button button-secondary",
                        disabled: loading,
                        onclick: move |_| users.restart(),
                        if loading { "Refreshing…" } else { "Refresh" }
                    }
                    button {
                        class: "button button-primary",
                        onclick: move |_| {
                            username.set(String::new());
                            email.set(String::new());
                            password.set(String::new());
                            notice.set(String::new());
                            form_error.set(String::new());
                            create_open.set(true);
                        },
                        Icon { name: "plus" }
                        "Create Customer"
                    }
                }
            }

            if !notice().is_empty() {
                div { class: "terminal auth-status", "{notice}" }
            }

            if loading {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "users" } }
                        h3 { "Loading customer accounts" }
                        p { "Reading business accounts and wallet balances before showing customer totals or account-state conclusions." }
                    }
                }
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
                        button {
                            class: "button button-primary",
                            onclick: move |_| {
                                username.set(String::new());
                                email.set(String::new());
                                password.set(String::new());
                                form_error.set(String::new());
                                create_open.set(true);
                            },
                            "Create Customer"
                        }
                    }
                }
            } else {
                div { class: "metrics",
                    div { class: "card metric",
                        div { class: "metric-copy",
                            span { class: "metric-label", "Customers" }
                            span { class: "metric-value", "{customer_count}" }
                            span { class: "metric-note", "business accounts" }
                        }
                        div { class: "metric-icon tone-gray", Icon { name: "users" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy",
                            span { class: "metric-label", "Default Status" }
                            span { class: "metric-value", "{default_status_count}" }
                            span { class: "metric-note", "records reporting status = 1" }
                        }
                        div { class: "metric-icon tone-gray", Icon { name: "activity" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy",
                            span { class: "metric-label", "CNY Wallets" }
                            span { class: "metric-value", "{cny_total_text}" }
                            span { class: "metric-note", "aggregate balance" }
                        }
                        div { class: "metric-icon tone-gray", Icon { name: "dollar" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy",
                            span { class: "metric-label", "USD Wallets" }
                            span { class: "metric-value", "{usd_total_text}" }
                            span { class: "metric-note", "aggregate balance" }
                        }
                        div { class: "metric-icon tone-gray", Icon { name: "dollar" } }
                    }
                }

                if non_default_status_count > 0 {
                    div { class: "readiness-strip blocked",
                        span { class: "readiness-dot" }
                        div {
                            strong { "{non_default_status_count} customer records have a non-default status flag" }
                            span { class: "small muted", "Treat status as server metadata here: this console has no suspend/reactivate endpoint and does not infer login or API enforcement from non-default values." }
                        }
                        span { class: "badge badge-neutral", "Status review" }
                    }
                } else {
                    div { class: "readiness-strip ready",
                        span { class: "readiness-dot" }
                        div {
                            strong { "Customer records use the default status flag" }
                            span { class: "small muted", "All loaded business accounts report status 1. Wallet funding remains an explicit operator action." }
                        }
                        span { class: "badge badge-neutral", "{customer_count} loaded" }
                    }
                }

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
                            input {
                                r#type: "checkbox",
                                checked: default_status_only(),
                                onclick: move |_| default_status_only.set(!default_status_only()),
                            }
                            "Default status only"
                        }
                        span { class: "small muted", "Showing {filtered_count} of {customer_count}" }
                    }

                    if filtered.is_empty() {
                        div { class: "product-empty",
                            div { class: "product-empty-inner",
                                h3 { "No customers match this view" }
                                p { "Change the search text or clear the Default status only filter." }
                                button {
                                    class: "button button-secondary",
                                    onclick: move |_| {
                                        query.set(String::new());
                                        default_status_only.set(false);
                                    },
                                    "Clear filters"
                                }
                            }
                        }
                    } else {
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr {
                                    th { "Customer" }
                                    th { "Status Metadata" }
                                    th { "Wallet" }
                                    th { "Preferred Currency" }
                                    th { class: "right", "Actions" }
                                } }
                                tbody {
                                    for user in filtered {
                                        {
                                            let email_text = user.email.clone().unwrap_or_else(|| "No email".to_string());
                                            let status_text = if user.status == 1 {
                                                "Active flag".to_string()
                                            } else {
                                                format!("Status {}", user.status)
                                            };
                                            let cny_text = format_nano(user.balance_cny, "CNY ");
                                            let usd_text = format_nano(user.balance_usd, "$");
                                            let preferred = user.preferred_currency.clone().unwrap_or_else(|| "Not set".to_string());
                                            rsx! {
                                                tr { key: "{user.id}",
                                                    td {
                                                        div { class: "two-line",
                                                            strong { class: "table-primary", title: "{user.username}", "{user.username}" }
                                                            small { class: "muted", title: "{email_text} • {user.id}", "{email_text} • {user.id}" }
                                                        }
                                                    }
                                                    td {
                                                        span { class: if user.status == 1 { "badge badge-success" } else { "badge badge-neutral" }, "{status_text}" }
                                                    }
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
                                                                amount.set("100".to_string());
                                                                currency.set(
                                                                    user.preferred_currency
                                                                        .clone()
                                                                        .filter(|value| value == "USD" || value == "CNY")
                                                                        .unwrap_or_else(|| "CNY".to_string()),
                                                                );
                                                                notice.set(String::new());
                                                                form_error.set(String::new());
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
                        "Status is server metadata. BurnCloud does not currently expose an administrator suspend/reactivate endpoint here, so this page does not simulate access-control changes or treat non-default values as proof that sign-in or API traffic is blocked."
                    }
                }
            }

            if create_open() {
                div {
                    class: "drawer-backdrop",
                    onclick: move |_| if !busy() { create_open.set(false) },
                }
                aside { class: "drawer",
                    div { class: "drawer-head",
                        div {
                            h2 { "Create Customer" }
                            p { class: "small muted", "Create a business account that can own wallet balance and API access." }
                        }
                        button {
                            class: "close-button",
                            disabled: busy(),
                            onclick: move |_| create_open.set(false),
                            "×"
                        }
                    }
                    div { class: "drawer-body stack-lg",
                        div { class: "form-section",
                            div { class: "form-section-head",
                                strong { "Account identity" }
                                small { "The username becomes the customer's BurnCloud login identity. Creation uses the server's default starting wallet; add any additional balance separately." }
                            }
                            div { class: "field",
                                label { "Username" }
                                input {
                                    class: "input",
                                    value: "{username}",
                                    disabled: busy(),
                                    autocomplete: "off",
                                    oninput: move |event| {
                                        username.set(event.value());
                                        form_error.set(String::new());
                                    },
                                }
                            }
                            div { class: "field",
                                label { "Email (optional)" }
                                input {
                                    class: "input",
                                    r#type: "email",
                                    value: "{email}",
                                    disabled: busy(),
                                    oninput: move |event| email.set(event.value()),
                                }
                            }
                            div { class: "field",
                                label { "Temporary password" }
                                input {
                                    class: "input",
                                    r#type: "password",
                                    value: "{password}",
                                    disabled: busy(),
                                    placeholder: "At least 8 characters",
                                    oninput: move |event| {
                                        password.set(event.value());
                                        form_error.set(String::new());
                                    },
                                }
                                small { class: "muted", "Share this password with the customer through an appropriate secure channel; the admin console does not display it again after creation." }
                            }
                        }
                        if !form_error().is_empty() {
                            div { class: "terminal auth-status auth-status-error", "{form_error}" }
                        }
                        div { class: "row customer-form-actions",
                            button {
                                class: "button button-secondary",
                                disabled: busy(),
                                onclick: move |_| create_open.set(false),
                                "Cancel"
                            }
                            button {
                                class: "button button-primary",
                                disabled: busy() || username().trim().is_empty() || password().len() < 8,
                                onclick: move |_| {
                                    let username_value = username().trim().to_string();
                                    let email_value = email().trim().to_string();
                                    let password_value = password();
                                    if username_value.is_empty() || password_value.len() < 8 {
                                        form_error.set("Username is required and the password must contain at least 8 characters.".to_string());
                                        return;
                                    }
                                    busy.set(true);
                                    form_error.set(String::new());
                                    spawn(async move {
                                        let email_arg = if email_value.is_empty() {
                                            None
                                        } else {
                                            Some(email_value.as_str())
                                        };
                                        match UserService::create(&username_value, &password_value, email_arg).await {
                                            Ok(_) => {
                                                notice.set(format!("Customer {username_value} created. The server applied its default starting wallet."));
                                                create_open.set(false);
                                                users.restart();
                                            }
                                            Err(message) => form_error.set(format!("Create customer failed: {message}")),
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
                    let preview_text = amount_preview.map(|nano| format_currency_nano(nano, &currency()));
                    let funding_button_label = if busy() {
                        "Adding…".to_string()
                    } else if let Some(preview) = preview_text.as_ref() {
                        format!("Add {preview}")
                    } else {
                        "Confirm Funding".to_string()
                    };
                    rsx! {
                        div {
                            class: "drawer-backdrop",
                            onclick: move |_| if !busy() { topup_user.set(None) },
                        }
                        aside { class: "drawer",
                            div { class: "drawer-head",
                                div {
                                    h2 { "Add Funds" }
                                    p { class: "small muted", "Credit the customer's BurnCloud wallet immediately." }
                                }
                                button {
                                    class: "close-button",
                                    disabled: busy(),
                                    onclick: move |_| topup_user.set(None),
                                    "×"
                                }
                            }
                            div { class: "drawer-body stack-lg",
                                div { class: "form-section",
                                    div { class: "form-section-head",
                                        strong { "{user.username}" }
                                        small { "Current balances: {current_cny} • {current_usd}" }
                                    }
                                    div { class: "grid-2",
                                        div { class: "field",
                                            label { "Currency" }
                                            select {
                                                class: "select",
                                                value: "{currency}",
                                                disabled: busy(),
                                                onchange: move |event| {
                                                    currency.set(event.value());
                                                    form_error.set(String::new());
                                                },
                                                option { value: "CNY", "CNY" }
                                                option { value: "USD", "USD" }
                                            }
                                        }
                                        div { class: "field",
                                            label { "Amount" }
                                            input {
                                                class: "input",
                                                r#type: "number",
                                                min: "0.01",
                                                step: "0.01",
                                                value: "{amount}",
                                                disabled: busy(),
                                                oninput: move |event| {
                                                    amount.set(event.value());
                                                    form_error.set(String::new());
                                                },
                                            }
                                            small { class: "muted", "Up to two decimal places. The client converts this exactly to the server's nanodollar balance contract." }
                                        }
                                    }
                                    div { class: "row gap-2",
                                        button { class: "button button-secondary button-sm", disabled: busy(), onclick: move |_| amount.set("100".to_string()), "100" }
                                        button { class: "button button-secondary button-sm", disabled: busy(), onclick: move |_| amount.set("500".to_string()), "500" }
                                        button { class: "button button-secondary button-sm", disabled: busy(), onclick: move |_| amount.set("1000".to_string()), "1,000" }
                                    }
                                }

                                if let Some(preview) = preview_text {
                                    div { class: "product-note",
                                        strong { "Funding review" }
                                        span { "Add {preview} to {user.username}. This immediately creates a wallet credit through the real top-up endpoint." }
                                    }
                                }

                                if !form_error().is_empty() {
                                    div { class: "terminal auth-status auth-status-error", "{form_error}" }
                                }

                                div { class: "row customer-form-actions",
                                    button {
                                        class: "button button-secondary",
                                        disabled: busy(),
                                        onclick: move |_| topup_user.set(None),
                                        "Cancel"
                                    }
                                    button {
                                        class: "button button-primary",
                                        disabled: busy() || amount_preview.is_none(),
                                        onclick: move |_| {
                                            let selected_currency = currency();
                                            let user_id = user.id.clone();
                                            let user_name = user.username.clone();
                                            let amount_nano = match parse_positive_amount_nano(&amount()) {
                                                Ok(value) => value,
                                                Err(message) => {
                                                    form_error.set(message);
                                                    return;
                                                }
                                            };
                                            let amount_text = format_currency_nano(amount_nano, &selected_currency);
                                            busy.set(true);
                                            form_error.set(String::new());
                                            spawn(async move {
                                                match UserService::topup(&user_id, amount_nano, &selected_currency).await {
                                                    Ok(new_balance) => {
                                                        let new_balance_text = format_currency_nano(new_balance, &selected_currency);
                                                        notice.set(format!("Added {amount_text} to {user_name}. New {selected_currency} balance: {new_balance_text}."));
                                                        topup_user.set(None);
                                                        users.restart();
                                                    }
                                                    Err(message) => form_error.set(format!("Add funds failed: {message}")),
                                                }
                                                busy.set(false);
                                            });
                                        },
                                        "{funding_button_label}"
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
