use dioxus::prelude::*;

use crate::{
    backend::{use_auth, TokenDto, TokenService, UserService},
    components::Icon,
};

fn masked(token: &str) -> String {
    if token.len() <= 16 {
        return "••••••••".to_string();
    }
    format!("{}••••••••{}", &token[..8], &token[token.len() - 6..])
}

#[component]
pub fn APIKeys() -> Element {
    let auth = use_auth();
    let current_user_id = auth.user().map(|u| u.id).unwrap_or_default();
    let mut tokens = use_resource(move || async move { TokenService::list().await });
    let mut create_open = use_signal(|| false);
    let mut whitelist_token = use_signal(|| None::<TokenDto>);
    let mut user_id = use_signal(move || current_user_id);
    let mut quota = use_signal(String::new);
    let mut whitelist = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(String::new);
    let mut error = use_signal(String::new);

    let result = tokens.read().clone();
    let loading = result.is_none();
    let load_error = result.as_ref().and_then(|r| r.as_ref().err().cloned());
    let list = result.and_then(Result::ok).unwrap_or_default();
    let active_count = list.iter().filter(|t| t.status == "active").count();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "API Keys" }
                    p { class: "page-subtitle", "Real router tokens. Create, activate/deactivate, rotate, restrict by IP, or delete keys through /console/api/tokens." }
                }
                div { class: "header-actions",
                    button { class: "button button-secondary", onclick: move |_| tokens.restart(), "Refresh" }
                    button {
                        class: "button button-primary",
                        onclick: move |_| {
                            quota.set(String::new());
                            notice.set(String::new());
                            error.set(String::new());
                            create_open.set(true);
                        },
                        Icon { name: "plus" }
                        "Create API Key"
                    }
                }
            }

            div { class: "metrics",
                div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Keys" } span { class: "metric-value", "{list.len()}" } } div { class: "metric-icon tone-blue", Icon { name: "key" } } }
                div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Active" } span { class: "metric-value", "{active_count}" } } div { class: "metric-icon tone-green", Icon { name: "activity" } } }
            }

            if !notice().is_empty() { div { class: "terminal auth-status", "{notice}" } }
            if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }

            if loading {
                div { class: "card card-pad", "Loading API keys…" }
            } else if let Some(message) = load_error {
                div { class: "card card-pad stack", strong { class: "danger", "Unable to load API keys" } code { class: "terminal", "{message}" } }
            } else {
                div { class: "card table-card",
                    if list.is_empty() {
                        div { class: "card-pad small muted", "No API keys exist yet. Create one to use Playground or external OpenAI-compatible clients." }
                    } else {
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr {
                                    th { "Key" }
                                    th { "User" }
                                    th { "Status" }
                                    th { class: "right", "Quota" }
                                    th { class: "right", "Used" }
                                    th { "IP Whitelist" }
                                    th { "Version" }
                                    th { "Actions" }
                                } }
                                tbody {
                                    for item in list {
                                        {
                                            let token_value = item.token.clone();
                                            let token_label = masked(&item.token);
                                            let next_status = if item.status == "active" { "disabled" } else { "active" };
                                            let toggle_label = if item.status == "active" { "Disable" } else { "Enable" };
                                            let whitelist_text = item.ip_whitelist.clone().unwrap_or_else(|| "Any IP".to_string());
                                            let quota_text = if item.quota_limit < 0 { "Unlimited".to_string() } else { item.quota_limit.to_string() };
                                            rsx! {
                                                tr { key: "{item.token}",
                                                    td { class: "mono table-primary", "{token_label}" }
                                                    td { class: "mono muted", "{item.user_id}" }
                                                    td { span { class: if item.status == "active" { "badge badge-success" } else { "badge badge-neutral" }, "{item.status}" } }
                                                    td { class: "right tabular", "{quota_text}" }
                                                    td { class: "right tabular", "{item.used_quota}" }
                                                    td { class: "mono muted", "{whitelist_text}" }
                                                    td { class: "mono", "v{item.key_version}" }
                                                    td {
                                                        div { class: "row gap-2", style: "flex-wrap:wrap",
                                                            button {
                                                                class: "button button-ghost button-sm",
                                                                disabled: busy(),
                                                                onclick: move |_| {
                                                                    let token = token_value.clone();
                                                                    busy.set(true);
                                                                    error.set(String::new());
                                                                    spawn(async move {
                                                                        match TokenService::set_status(&token, next_status).await {
                                                                            Ok(()) => { notice.set(format!("Key status changed to {next_status}.")); tokens.restart(); }
                                                                            Err(message) => error.set(format!("Key status update failed: {message}")),
                                                                        }
                                                                        busy.set(false);
                                                                    });
                                                                },
                                                                "{toggle_label}"
                                                            }
                                                            button {
                                                                class: "button button-ghost button-sm",
                                                                disabled: busy(),
                                                                onclick: move |_| {
                                                                    let token = item.token.clone();
                                                                    busy.set(true);
                                                                    error.set(String::new());
                                                                    spawn(async move {
                                                                        match TokenService::rotate(&token, 24, false).await {
                                                                            Ok(value) => { notice.set(format!("Rotation result: {value}")); tokens.restart(); }
                                                                            Err(message) => error.set(format!("Key rotation failed: {message}")),
                                                                        }
                                                                        busy.set(false);
                                                                    });
                                                                },
                                                                "Rotate"
                                                            }
                                                            button {
                                                                class: "button button-ghost button-sm",
                                                                onclick: move |_| {
                                                                    whitelist.set(item.ip_whitelist.clone().unwrap_or_default());
                                                                    error.set(String::new());
                                                                    whitelist_token.set(Some(item.clone()));
                                                                },
                                                                "IP Rules"
                                                            }
                                                            button {
                                                                class: "button button-ghost button-sm danger",
                                                                disabled: busy(),
                                                                onclick: move |_| {
                                                                    let token = item.token.clone();
                                                                    busy.set(true);
                                                                    error.set(String::new());
                                                                    spawn(async move {
                                                                        match TokenService::delete(&token).await {
                                                                            Ok(()) => { notice.set("API key deleted.".to_string()); tokens.restart(); }
                                                                            Err(message) => error.set(format!("Delete failed: {message}")),
                                                                        }
                                                                        busy.set(false);
                                                                    });
                                                                },
                                                                "Delete"
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

            if create_open() {
                div { class: "drawer-backdrop", onclick: move |_| create_open.set(false) }
                aside { class: "drawer",
                    div { class: "drawer-head", h2 { "Create API Key" } button { class: "close-button", onclick: move |_| create_open.set(false), "×" } }
                    div { class: "drawer-body stack-lg",
                        div { class: "field", label { "User ID" } input { class: "input mono", value: "{user_id}", oninput: move |evt| user_id.set(evt.value()) } }
                        div { class: "field", label { "Quota limit (optional raw quota)" } input { class: "input", r#type: "number", value: "{quota}", oninput: move |evt| quota.set(evt.value()) } }
                        button {
                            class: "button button-primary",
                            disabled: busy(),
                            onclick: move |_| {
                                let uid = user_id().trim().to_string();
                                let quota_value = quota().trim().parse::<i64>().ok();
                                if uid.is_empty() { error.set("User ID is required.".to_string()); return; }
                                busy.set(true);
                                error.set(String::new());
                                spawn(async move {
                                    match TokenService::create(&uid, quota_value).await {
                                        Ok(token) => { notice.set(format!("Created API key: {token} — copy it now.")); create_open.set(false); tokens.restart(); }
                                        Err(message) => error.set(format!("Create key failed: {message}")),
                                    }
                                    busy.set(false);
                                });
                            },
                            if busy() { "Creating…" } else { "Create Key" }
                        }
                    }
                }
            }

            if let Some(item) = whitelist_token() {
                div { class: "drawer-backdrop", onclick: move |_| whitelist_token.set(None) }
                aside { class: "drawer",
                    div { class: "drawer-head", h2 { "IP Whitelist" } button { class: "close-button", onclick: move |_| whitelist_token.set(None), "×" } }
                    div { class: "drawer-body stack-lg",
                        p { class: "small muted", "Enter the exact whitelist syntax accepted by the router token service. Empty value removes the restriction." }
                        textarea { class: "textarea mono", rows: "6", value: "{whitelist}", oninput: move |evt| whitelist.set(evt.value()) }
                        button {
                            class: "button button-primary",
                            disabled: busy(),
                            onclick: move |_| {
                                let token = item.token.clone();
                                let rules = whitelist();
                                busy.set(true);
                                error.set(String::new());
                                spawn(async move {
                                    match TokenService::set_ip_whitelist(&token, &rules).await {
                                        Ok(()) => { notice.set("IP whitelist saved.".to_string()); whitelist_token.set(None); tokens.restart(); }
                                        Err(message) => error.set(format!("Whitelist update failed: {message}")),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Save IP Rules"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Team() -> Element {
    let auth = use_auth();
    let current = auth.user();
    let mut users = use_resource(move || async move { UserService::list().await });
    let result = users.read().clone();
    let load_error = result.as_ref().and_then(|r| r.as_ref().err().cloned());
    let list = result.and_then(Result::ok).unwrap_or_default();
    let role_text = current.as_ref().map(|u| u.roles.join(", ")).unwrap_or_default();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Team" }
                    p { class: "page-subtitle", "Team membership reflects actual BurnCloud user accounts. Create/invite accounts from Customers / Users." }
                }
                button { class: "button button-secondary", onclick: move |_| users.restart(), "Refresh" }
            }
            if let Some(user) = current {
                div { class: "card card-pad stack",
                    span { class: "section-label", "Current Session" }
                    strong { "{user.username}" }
                    code { class: "small muted", "{user.id}" }
                    span { class: "small muted", "Roles: {role_text}" }
                }
            }
            if let Some(message) = load_error {
                div { class: "terminal auth-status auth-status-error", "{message}" }
            } else {
                div { class: "card table-card",
                    div { class: "table-wrap",
                        table { class: "data-table",
                            thead { tr { th { "Username" } th { "Email" } th { "Role" } th { "Group" } th { "Status" } } }
                            tbody {
                                for user in list {
                                    {
                                        let email = user.email.clone().unwrap_or_else(|| "-".to_string());
                                        let status = if user.status == 1 { "Active" } else { "Disabled" };
                                        rsx! {
                                            tr { key: "{user.id}",
                                                td { class: "table-primary", "{user.username}" }
                                                td { "{email}" }
                                                td { "{user.role}" }
                                                td { "{user.group}" }
                                                td { span { class: if user.status == 1 { "badge badge-success" } else { "badge badge-neutral" }, "{status}" } }
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
