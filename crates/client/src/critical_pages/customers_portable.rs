use dioxus::prelude::*;

use crate::{
    backend::{User, UserService},
    components::Icon,
};

fn format_nano(nano: i64, symbol: &str) -> String {
    format!("{symbol}{:.2}", nano as f64 / 1_000_000_000.0)
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
    let load_error = resource.as_ref().and_then(|r| r.as_ref().err().cloned());
    let user_list = resource.and_then(Result::ok).unwrap_or_default();
    let q = query().to_lowercase();
    let filtered: Vec<User> = user_list
        .iter()
        .filter(|user| {
            (!active_only() || user.status == 1)
                && (q.is_empty()
                    || user.username.to_lowercase().contains(&q)
                    || user.id.to_lowercase().contains(&q)
                    || user.email.as_deref().unwrap_or("").to_lowercase().contains(&q)
                    || user.role.to_lowercase().contains(&q))
        })
        .cloned()
        .collect();

    let active_count = user_list.iter().filter(|u| u.status == 1).count();
    let cny_total: i64 = user_list.iter().map(|u| u.balance_cny).sum();
    let usd_total: i64 = user_list.iter().map(|u| u.balance_usd).sum();
    let cny_total_text = format_nano(cny_total, "CNY ");
    let usd_total_text = format_nano(usd_total, "$");
    let filtered_count = filtered.len();
    let user_count = user_list.len();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Customers / Users" }
                    p { class: "page-subtitle", "Real BurnCloud accounts from /console/api/list_users. Account creation and balance top-up are connected to the server." }
                }
                div { class: "header-actions",
                    button {
                        class: "button button-secondary",
                        onclick: move |_| users.restart(),
                        "Refresh"
                    }
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
                        "Create User"
                    }
                }
            }

            div { class: "metrics",
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Total Accounts" } span { class: "metric-value", "{user_count}" } }
                    div { class: "metric-icon tone-gray", Icon { name: "users" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Active Accounts" } span { class: "metric-value", "{active_count}" } }
                    div { class: "metric-icon tone-green", Icon { name: "activity" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "CNY Balance Pool" } span { class: "metric-value", "{cny_total_text}" } }
                    div { class: "metric-icon tone-amber", Icon { name: "dollar" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "USD Balance Pool" } span { class: "metric-value", "{usd_total_text}" } }
                    div { class: "metric-icon tone-blue", Icon { name: "dollar" } }
                }
            }

            if loading {
                div { class: "card card-pad", "Loading accounts…" }
            } else if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Unable to load users" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| users.restart(), "Retry" }
                }
            } else {
                div { class: "card table-card",
                    div { class: "customer-toolbar",
                        div { class: "search-field customer-search",
                            Icon { name: "search" }
                            input {
                                class: "input",
                                placeholder: "Search username, ID, email or role…",
                                value: "{query}",
                                oninput: move |evt| query.set(evt.value()),
                            }
                        }
                        label { class: "row gap-2 small muted",
                            input {
                                r#type: "checkbox",
                                checked: active_only(),
                                onclick: move |_| active_only.set(!active_only()),
                            }
                            "Active only"
                        }
                        span { class: "small muted", "Showing {filtered_count} of {user_count} accounts" }
                    }

                    if filtered.is_empty() {
                        div { class: "card-pad small muted", "No accounts match the current filter." }
                    } else {
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr {
                                    th { "Username" }
                                    th { "User ID" }
                                    th { "Email" }
                                    th { "Role" }
                                    th { "Group" }
                                    th { class: "right", "CNY Balance" }
                                    th { class: "right", "USD Balance" }
                                    th { class: "center", "Status" }
                                    th { "" }
                                } }
                                tbody {
                                    for user in filtered {
                                        {
                                            let email_text = user.email.clone().unwrap_or_else(|| "-".to_string());
                                            let status_text = if user.status == 1 { "Active" } else { "Disabled" };
                                            let status_class = if user.status == 1 { "badge badge-success" } else { "badge badge-neutral" };
                                            let cny_text = format_nano(user.balance_cny, "CNY ");
                                            let usd_text = format_nano(user.balance_usd, "$");
                                            rsx! {
                                                tr { key: "{user.id}",
                                                    td { class: "table-primary", "{user.username}" }
                                                    td { class: "mono muted", "{user.id}" }
                                                    td { class: "muted", "{email_text}" }
                                                    td { span { class: "badge badge-neutral", "{user.role}" } }
                                                    td { "{user.group}" }
                                                    td { class: "right tabular", "{cny_text}" }
                                                    td { class: "right tabular", "{usd_text}" }
                                                    td { class: "center", span { class: "{status_class}", "{status_text}" } }
                                                    td {
                                                        button {
                                                            class: "button button-ghost button-sm",
                                                            onclick: move |_| {
                                                                amount.set(0);
                                                                currency.set("CNY".to_string());
                                                                notice.set(String::new());
                                                                error.set(String::new());
                                                                topup_user.set(Some(user.clone()));
                                                            },
                                                            "Top Up"
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
                    div { class: "card-pad tiny subtle",
                        "User status is read-only because the current server does not expose an account suspend/reactivate endpoint. The previous fake suspend toggle has been removed."
                    }
                }
            }

            if create_open() {
                div { class: "drawer-backdrop", onclick: move |_| create_open.set(false) }
                aside { class: "drawer",
                    div { class: "drawer-head",
                        h2 { "Create BurnCloud User" }
                        button { class: "close-button", onclick: move |_| create_open.set(false), "×" }
                    }
                    div { class: "drawer-body stack-lg",
                        div { class: "field",
                            label { "Username" }
                            input { class: "input", value: "{username}", disabled: busy(), oninput: move |evt| username.set(evt.value()) }
                        }
                        div { class: "field",
                            label { "Email (optional)" }
                            input { class: "input", r#type: "email", value: "{email}", disabled: busy(), oninput: move |evt| email.set(evt.value()) }
                        }
                        div { class: "field",
                            label { "Password" }
                            input { class: "input", r#type: "password", value: "{password}", disabled: busy(), placeholder: "At least 8 characters", oninput: move |evt| password.set(evt.value()) }
                        }
                        if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }
                        if !notice().is_empty() { div { class: "badge badge-success", "{notice}" } }
                        div { class: "row customer-form-actions",
                            button { class: "button button-secondary", disabled: busy(), onclick: move |_| create_open.set(false), "Cancel" }
                            button {
                                class: "button button-primary",
                                disabled: busy(),
                                onclick: move |_| {
                                    let u = username().trim().to_string();
                                    let e = email().trim().to_string();
                                    let p = password();
                                    if u.is_empty() || p.len() < 8 {
                                        error.set("Username is required and password must contain at least 8 characters.".to_string());
                                        return;
                                    }
                                    busy.set(true);
                                    error.set(String::new());
                                    notice.set("Creating account…".to_string());
                                    spawn(async move {
                                        let email_arg = if e.is_empty() { None } else { Some(e.as_str()) };
                                        match UserService::create(&u, &p, email_arg).await {
                                            Ok(_) => {
                                                busy.set(false);
                                                create_open.set(false);
                                                notice.set(String::new());
                                                users.restart();
                                            }
                                            Err(message) => {
                                                busy.set(false);
                                                notice.set(String::new());
                                                error.set(format!("Create user failed: {message}"));
                                            }
                                        }
                                    });
                                },
                                if busy() { "Creating…" } else { "Create User" }
                            }
                        }
                    }
                }
            }

            if let Some(user) = topup_user() {
                div { class: "drawer-backdrop", onclick: move |_| topup_user.set(None) }
                aside { class: "drawer",
                    div { class: "drawer-head",
                        h2 { "Top Up Balance" }
                        button { class: "close-button", onclick: move |_| topup_user.set(None), "×" }
                    }
                    div { class: "drawer-body stack-lg",
                        div { class: "card card-pad stack",
                            span { class: "section-label", "Target account" }
                            strong { "{user.username}" }
                            code { class: "tiny muted", "{user.id}" }
                        }
                        div { class: "grid-2",
                            div { class: "field",
                                label { "Currency" }
                                select { class: "select", value: "{currency}", disabled: busy(), onchange: move |evt| currency.set(evt.value()),
                                    option { value: "CNY", "CNY" }
                                    option { value: "USD", "USD" }
                                }
                            }
                            div { class: "field",
                                label { "Amount" }
                                input {
                                    class: "input",
                                    r#type: "number",
                                    min: "1",
                                    value: "{amount}",
                                    disabled: busy(),
                                    oninput: move |evt| amount.set(evt.value().parse::<i64>().unwrap_or(0)),
                                }
                            }
                        }
                        div { class: "row gap-2",
                            button { class: "button button-secondary button-sm", onclick: move |_| amount.set(100), "100" }
                            button { class: "button button-secondary button-sm", onclick: move |_| amount.set(500), "500" }
                            button { class: "button button-secondary button-sm", onclick: move |_| amount.set(1000), "1000" }
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
                                    let uid = user.id.clone();
                                    if value <= 0 {
                                        error.set("Top-up amount must be greater than zero.".to_string());
                                        return;
                                    }
                                    busy.set(true);
                                    error.set(String::new());
                                    notice.set("Submitting balance top-up…".to_string());
                                    let amount_nano = value.saturating_mul(1_000_000_000);
                                    spawn(async move {
                                        match UserService::topup(&uid, amount_nano, &selected_currency).await {
                                            Ok(new_balance) => {
                                                busy.set(false);
                                                notice.set(format!("Top-up confirmed. New raw balance: {new_balance}"));
                                                users.restart();
                                            }
                                            Err(message) => {
                                                busy.set(false);
                                                notice.set(String::new());
                                                error.set(format!("Top-up failed: {message}"));
                                            }
                                        }
                                    });
                                },
                                if busy() { "Submitting…" } else { "Confirm Top Up" }
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
