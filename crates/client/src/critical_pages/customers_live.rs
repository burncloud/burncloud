use dioxus::prelude::*;

use crate::{
    backend::{User, UserService},
    components::Icon,
    role_access::{is_staff_role, is_staff_roles},
};

fn format_money_nano(nano: i64, currency: &str) -> String {
    let negative = nano < 0;
    let absolute = nano.unsigned_abs();
    let rounded_cents = absolute.saturating_add(5_000_000) / 10_000_000;
    let whole = rounded_cents / 100;
    let cents = rounded_cents % 100;
    let symbol = if currency == "USD" { "$" } else { "CNY " };
    format!("{}{symbol}{whole}.{cents:02}", if negative { "-" } else { "" })
}

fn format_total_nano(nano: i128, currency: &str) -> String {
    let negative = nano < 0;
    let absolute = nano.unsigned_abs();
    let rounded_cents = absolute.saturating_add(5_000_000) / 10_000_000;
    let whole = rounded_cents / 100;
    let cents = rounded_cents % 100;
    let symbol = if currency == "USD" { "$" } else { "CNY " };
    format!("{}{symbol}{whole}.{cents:02}", if negative { "-" } else { "" })
}

fn parse_currency_amount_nano(input: &str) -> Result<i64, String> {
    let value = input.trim();
    if value.is_empty() {
        return Err("Enter an amount.".to_string());
    }
    if value.starts_with('-') || value.starts_with('+') {
        return Err("Funding amount must be greater than zero.".to_string());
    }

    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some() || fraction.len() > 2 {
        return Err("Use a currency amount with no more than 2 decimal places.".to_string());
    }
    if whole.is_empty() && fraction.is_empty() {
        return Err("Enter a valid amount.".to_string());
    }
    if (!whole.is_empty() && !whole.chars().all(|ch| ch.is_ascii_digit()))
        || (!fraction.is_empty() && !fraction.chars().all(|ch| ch.is_ascii_digit()))
    {
        return Err("Enter a valid currency amount.".to_string());
    }

    let whole_value = if whole.is_empty() {
        0i64
    } else {
        whole
            .parse::<i64>()
            .map_err(|_| "Amount is too large.".to_string())?
    };
    let fraction_value = if fraction.is_empty() {
        0i64
    } else {
        fraction
            .parse::<i64>()
            .map_err(|_| "Enter a valid currency amount.".to_string())?
    };
    let fraction_nano = match fraction.len() {
        0 => 0,
        1 => fraction_value
            .checked_mul(100_000_000)
            .ok_or_else(|| "Amount is too large.".to_string())?,
        2 => fraction_value
            .checked_mul(10_000_000)
            .ok_or_else(|| "Amount is too large.".to_string())?,
        _ => return Err("Use a currency amount with no more than 2 decimal places.".to_string()),
    };
    let nano = whole_value
        .checked_mul(1_000_000_000)
        .and_then(|base| base.checked_add(fraction_nano))
        .ok_or_else(|| "Amount is too large.".to_string())?;
    if nano <= 0 {
        return Err("Funding amount must be greater than zero.".to_string());
    }
    Ok(nano)
}

#[component]
pub fn Customers() -> Element {
    let mut users_resource = use_resource(move || async move { UserService::list().await });
    let mut query = use_signal(String::new);
    let mut active_only = use_signal(|| false);
    let mut create_open = use_signal(|| false);
    let mut topup_user = use_signal(|| None::<User>);
    let mut username = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut amount = use_signal(String::new);
    let mut currency = use_signal(|| "USD".to_string());
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(String::new);
    let mut error = use_signal(String::new);

    let snapshot = users_resource.read().clone();
    let loading = snapshot.is_none();
    let load_error = snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let directory_ready = snapshot.as_ref().is_some_and(Result::is_ok);
    let all_users = snapshot.clone().and_then(Result::ok).unwrap_or_default();
    let customer_list: Vec<User> = all_users
        .into_iter()
        .filter(|user| !is_staff_role(&user.role))
        .collect();

    let search = query().trim().to_lowercase();
    let filtered: Vec<User> = customer_list
        .iter()
        .filter(|user| {
            (!active_only() || user.status == 1)
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

    let customer_count = customer_list.len();
    let active_count = customer_list.iter().filter(|user| user.status == 1).count();
    let disabled_count = customer_list.iter().filter(|user| user.status != 1).count();
    let cny_total = customer_list
        .iter()
        .fold(0i128, |total, user| total + user.balance_cny as i128);
    let usd_total = customer_list
        .iter()
        .fold(0i128, |total, user| total + user.balance_usd as i128);

    let customer_count_text = if directory_ready {
        customer_count.to_string()
    } else {
        "—".to_string()
    };
    let active_count_text = if directory_ready {
        active_count.to_string()
    } else {
        "—".to_string()
    };
    let cny_total_text = if directory_ready {
        format_total_nano(cny_total, "CNY")
    } else {
        "—".to_string()
    };
    let usd_total_text = if directory_ready {
        format_total_nano(usd_total, "USD")
    } else {
        "—".to_string()
    };
    let filtered_count = filtered.len();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Customers" }
                    p { class: "page-subtitle", "Manage customer accounts, understand their real wallet state, and fund active accounts with an explicit financial confirmation." }
                }
                div { class: "header-actions",
                    button { class: "button button-secondary", onclick: move |_| users_resource.restart(), "Refresh" }
                    button {
                        class: "button button-primary",
                        disabled: !directory_ready,
                        title: if directory_ready { "Create a customer account" } else { "Load the customer directory before creating another account" },
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

            div { class: "product-note",
                strong { "Current account defaults: " }
                "the server creates new users as Active and currently grants a $10.00 USD signup wallet credit. In an initialized environment, additional accounts receive the user role; the returned roles are checked after creation."
            }

            div { class: "metrics",
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Customers" } span { class: "metric-value", "{customer_count_text}" } span { class: "metric-note", "non-admin accounts" } }
                    div { class: "metric-icon tone-gray", Icon { name: "users" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Active" } span { class: "metric-value", "{active_count_text}" } span { class: "metric-note", if directory_ready { "{disabled_count} disabled" } else { "customer directory unavailable" } } }
                    div { class: "metric-icon tone-green", Icon { name: "activity" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "CNY Wallets" } span { class: "metric-value", "{cny_total_text}" } span { class: "metric-note", "aggregate customer balance" } }
                    div { class: "metric-icon tone-amber", Icon { name: "dollar" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "USD Wallets" } span { class: "metric-value", "{usd_total_text}" } span { class: "metric-note", "aggregate customer balance" } }
                    div { class: "metric-icon tone-blue", Icon { name: "dollar" } }
                }
            }

            if !notice().is_empty() { div { class: "terminal auth-status", "{notice}" } }
            if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }

            if loading {
                div { class: "card card-pad", "Loading customer directory…" }
            } else if let Some(message) = load_error.clone() {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Customers could not be loaded" }
                    p { class: "small muted", "Counts and wallet totals remain unavailable rather than falling back to zero. Account creation is also blocked until the directory is known." }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| users_resource.restart(), "Retry" }
                }
            } else if customer_list.is_empty() {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "users" } }
                        h3 { "Create the first customer account" }
                        p { "A new account will be Active and will receive the server's current $10.00 USD signup credit." }
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
                                    th { "Wallet Balance" }
                                    th { "Preferred Currency" }
                                    th { class: "right", "Action" }
                                } }
                                tbody {
                                    for user in filtered {
                                        {
                                            let email_text = user
                                                .email
                                                .clone()
                                                .filter(|value| !value.trim().is_empty())
                                                .unwrap_or_else(|| "No email".to_string());
                                            let status_text = if user.status == 1 { "Active" } else { "Disabled" };
                                            let cny_text = format_money_nano(user.balance_cny, "CNY");
                                            let usd_text = format_money_nano(user.balance_usd, "USD");
                                            let preferred = user
                                                .preferred_currency
                                                .clone()
                                                .filter(|value| matches!(value.as_str(), "USD" | "CNY"))
                                                .unwrap_or_else(|| "Not set".to_string());
                                            let user_for_topup = user.clone();
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
                                                            disabled: user.status != 1,
                                                            title: if user.status == 1 { "Credit this customer's wallet" } else { "Disabled accounts cannot be funded from the console" },
                                                            onclick: move |_| {
                                                                if user_for_topup.status != 1 {
                                                                    return;
                                                                }
                                                                amount.set(String::new());
                                                                currency.set(
                                                                    user_for_topup
                                                                        .preferred_currency
                                                                        .clone()
                                                                        .filter(|value| matches!(value.as_str(), "USD" | "CNY"))
                                                                        .unwrap_or_else(|| "USD".to_string()),
                                                                );
                                                                notice.set(String::new());
                                                                error.set(String::new());
                                                                topup_user.set(Some(user_for_topup.clone()));
                                                            },
                                                            if user.status == 1 { "Add Funds" } else { "Account Disabled" }
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
                        "Disabled accounts are visible for access review, but BurnCloud does not fund them from this page. Account status remains read-only because the current server has no explicit safe suspend/reactivate endpoint in the console API."
                    }
                }
            }

            if create_open() {
                div { class: "drawer-backdrop", onclick: move |_| create_open.set(false) }
                aside { class: "drawer",
                    div { class: "drawer-head",
                        div { h2 { "Create Customer" } p { class: "small muted", "Create an active customer login with the server's current signup wallet credit." } }
                        button { class: "close-button", onclick: move |_| create_open.set(false), "×" }
                    }
                    div { class: "drawer-body stack-lg",
                        div { class: "form-section",
                            div { class: "form-section-head", strong { "Account identity" } small { "The username is the customer's BurnCloud login identity." } }
                            div { class: "field", label { "Username" } input { class: "input", value: "{username}", disabled: busy(), oninput: move |event| username.set(event.value()) } }
                            div { class: "field", label { "Email (optional)" } input { class: "input", r#type: "email", value: "{email}", disabled: busy(), oninput: move |event| email.set(event.value()) } }
                            div { class: "field", label { "Temporary password" } input { class: "input", r#type: "password", value: "{password}", disabled: busy(), placeholder: "At least 8 characters", oninput: move |event| password.set(event.value()) } }
                        }
                        div { class: "form-section",
                            div { class: "form-section-head", strong { "Account created with" } small { "These values come from the current server registration behavior." } }
                            div { class: "receipt-row", label { "Status" } strong { "Active" } }
                            div { class: "receipt-row", label { "Signup wallet credit" } strong { class: "tabular", "$10.00 USD" } }
                            div { class: "receipt-row", label { "Expected role" } strong { "user" } }
                            div { class: "product-note", "Role is verified from the server response after creation. If the server unexpectedly returns an admin role, BurnCloud will surface a high-priority warning instead of calling the result a normal customer." }
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
                                    if username_value.is_empty() {
                                        error.set("Username is required.".to_string());
                                        return;
                                    }
                                    if password_value.len() < 8 {
                                        error.set("Temporary password must contain at least 8 characters.".to_string());
                                        return;
                                    }
                                    if !email_value.is_empty() && !email_value.contains('@') {
                                        error.set("Enter a valid account email or leave the email blank.".to_string());
                                        return;
                                    }
                                    busy.set(true);
                                    error.set(String::new());
                                    spawn(async move {
                                        let email_arg = if email_value.is_empty() {
                                            None
                                        } else {
                                            Some(email_value.as_str())
                                        };
                                        match UserService::create(&username_value, &password_value, email_arg).await {
                                            Ok(created) => {
                                                if is_staff_roles(&created.roles) {
                                                    error.set(format!(
                                                        "Account {username_value} was created, but the server returned an admin role. Treat this as a privileged-account anomaly and review Team immediately."
                                                    ));
                                                } else {
                                                    notice.set(format!(
                                                        "Customer {username_value} created Active with the server's $10.00 USD signup credit."
                                                    ));
                                                }
                                                create_open.set(false);
                                                users_resource.restart();
                                            }
                                            Err(message) => error.set(format!("Create customer failed: {message}")),
                                        }
                                        busy.set(false);
                                    });
                                },
                                if busy() { "Creating…" } else { "Create Active Customer + $10 Credit" }
                            }
                        }
                    }
                }
            }

            if let Some(user) = topup_user() {
                {
                    let selected_currency = currency();
                    let current_balance_nano = if selected_currency == "USD" {
                        user.balance_usd
                    } else {
                        user.balance_cny
                    };
                    let parsed_amount = parse_currency_amount_nano(&amount());
                    let amount_nano = parsed_amount.clone().unwrap_or(0);
                    let funding_text = if amount_nano > 0 {
                        format_money_nano(amount_nano, &selected_currency)
                    } else {
                        "—".to_string()
                    };
                    let after_text = if amount_nano > 0 {
                        current_balance_nano
                            .checked_add(amount_nano)
                            .map(|balance| format_money_nano(balance, &selected_currency))
                            .unwrap_or_else(|| "Amount too large".to_string())
                    } else {
                        format_money_nano(current_balance_nano, &selected_currency)
                    };
                    let confirm_label = if amount_nano > 0 {
                        format!("Add {funding_text}")
                    } else {
                        "Add Funds".to_string()
                    };
                    let account_active = user.status == 1;
                    rsx! {
                        div { class: "drawer-backdrop", onclick: move |_| topup_user.set(None) }
                        aside { class: "drawer",
                            div { class: "drawer-head",
                                div { h2 { "Add Funds" } p { class: "small muted", "Credit an active customer's BurnCloud wallet." } }
                                button { class: "close-button", onclick: move |_| topup_user.set(None), "×" }
                            }
                            div { class: "drawer-body stack-lg",
                                if !account_active {
                                    div { class: "readiness-strip blocked",
                                        span { class: "readiness-dot" }
                                        strong { "This customer is disabled" }
                                        span { class: "muted", "Funding is blocked until account status is intentionally restored outside this page." }
                                    }
                                }
                                div { class: "form-section",
                                    div { class: "form-section-head", strong { "{user.username}" } small { "Current balances: {format_money_nano(user.balance_cny, "CNY")} • {format_money_nano(user.balance_usd, "USD")}" } }
                                    div { class: "grid-2",
                                        div { class: "field",
                                            label { "Currency" }
                                            select { class: "select", value: "{currency}", disabled: busy() || !account_active, onchange: move |event| currency.set(event.value()),
                                                option { value: "USD", "USD" }
                                                option { value: "CNY", "CNY" }
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
                                                placeholder: "0.00",
                                                disabled: busy() || !account_active,
                                                oninput: move |event| amount.set(event.value()),
                                            }
                                        }
                                    }
                                    div { class: "row gap-2",
                                        button { class: "button button-secondary button-sm", disabled: !account_active, onclick: move |_| amount.set("100".to_string()), "100" }
                                        button { class: "button button-secondary button-sm", disabled: !account_active, onclick: move |_| amount.set("500".to_string()), "500" }
                                        button { class: "button button-secondary button-sm", disabled: !account_active, onclick: move |_| amount.set("1000".to_string()), "1,000" }
                                    }
                                }

                                div { class: "form-section",
                                    div { class: "form-section-head", strong { "Review wallet change" } small { "Funding is posted immediately and recorded as a recharge by the server." } }
                                    div { class: "receipt-row", label { "Customer" } strong { "{user.username}" } }
                                    div { class: "receipt-row", label { "Current balance" } strong { class: "tabular", "{format_money_nano(current_balance_nano, &selected_currency)}" } }
                                    div { class: "receipt-row", label { "Add" } strong { class: "tabular", "{funding_text}" } }
                                    div { class: "receipt-row", label { "Expected balance after" } strong { class: "tabular", "{after_text}" } }
                                    if let Err(message) = parsed_amount.clone() {
                                        if !amount().trim().is_empty() {
                                            div { class: "terminal auth-status auth-status-error", "{message}" }
                                        }
                                    }
                                }

                                if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }
                                div { class: "row customer-form-actions",
                                    button { class: "button button-secondary", disabled: busy(), onclick: move |_| topup_user.set(None), "Cancel" }
                                    button {
                                        class: "button button-primary",
                                        disabled: busy() || !account_active || parsed_amount.is_err(),
                                        onclick: move |_| {
                                            if user.status != 1 {
                                                error.set("Disabled customer accounts cannot be funded from this page.".to_string());
                                                return;
                                            }
                                            let amount_nano = match parse_currency_amount_nano(&amount()) {
                                                Ok(value) => value,
                                                Err(message) => {
                                                    error.set(message);
                                                    return;
                                                }
                                            };
                                            let selected_currency = currency();
                                            let user_id = user.id.clone();
                                            let user_name = user.username.clone();
                                            let display_amount = format_money_nano(amount_nano, &selected_currency);
                                            busy.set(true);
                                            error.set(String::new());
                                            spawn(async move {
                                                match UserService::topup(&user_id, amount_nano, &selected_currency).await {
                                                    Ok(server_balance) => {
                                                        let balance_text = format_money_nano(server_balance, &selected_currency);
                                                        notice.set(format!(
                                                            "Added {display_amount} to {user_name}. Server-confirmed {selected_currency} balance: {balance_text}."
                                                        ));
                                                        topup_user.set(None);
                                                        users_resource.restart();
                                                    }
                                                    Err(message) => error.set(format!("Add funds failed: {message}")),
                                                }
                                                busy.set(false);
                                            });
                                        },
                                        if busy() { "Adding…" } else { "{confirm_label}" }
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
