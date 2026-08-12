use std::collections::BTreeMap;

use dioxus::prelude::*;

use crate::{
    backend::{use_auth, TokenDto, TokenService, User, UserService},
    components::Icon,
};

fn masked(token: &str) -> String {
    if token.len() <= 16 {
        return "••••••••".to_string();
    }
    format!("{}••••••••{}", &token[..8], &token[token.len() - 6..])
}

fn is_staff_role(role: &str) -> bool {
    matches!(
        role.trim().to_ascii_lowercase().as_str(),
        "root" | "admin" | "administrator" | "operator" | "owner"
    )
}

#[component]
pub fn APIKeys() -> Element {
    let auth = use_auth();
    let default_user = auth.user().map(|user| user.id).unwrap_or_default();
    let mut tokens_resource = use_resource(move || async move { TokenService::list().await });
    let mut users_resource = use_resource(move || async move { UserService::list().await });

    let mut create_open = use_signal(|| false);
    let mut whitelist_target = use_signal(|| None::<TokenDto>);
    let mut rotate_target = use_signal(|| None::<TokenDto>);
    let mut delete_target = use_signal(|| None::<TokenDto>);
    let mut new_key = use_signal(|| None::<String>);
    let mut user_id = use_signal(move || default_user);
    let mut quota = use_signal(String::new);
    let mut whitelist = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(String::new);
    let mut error = use_signal(String::new);

    let token_snapshot = tokens_resource.read().clone();
    let user_snapshot = users_resource.read().clone();
    let load_error = token_snapshot.as_ref().and_then(|result| result.as_ref().err().cloned());
    let users_error = user_snapshot.as_ref().and_then(|result| result.as_ref().err().cloned());
    let tokens = token_snapshot.and_then(Result::ok).unwrap_or_default();
    let users: Vec<User> = user_snapshot.and_then(Result::ok).unwrap_or_default();

    let owner_names: BTreeMap<String, String> = users
        .iter()
        .map(|user| (user.id.clone(), user.username.clone()))
        .collect();
    let active = tokens.iter().filter(|token| token.status == "active").count();
    let disabled = tokens.len().saturating_sub(active);
    let restricted = tokens
        .iter()
        .filter(|token| token.ip_whitelist.as_deref().is_some_and(|value| !value.trim().is_empty()))
        .count();
    let total = tokens.len();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "API Keys" }
                    p { class: "page-subtitle", "Control which BurnCloud account can send router traffic and protect credentials over their lifecycle." }
                }
                div { class: "header-actions",
                    button {
                        class: "button button-secondary",
                        onclick: move |_| {
                            tokens_resource.restart();
                            users_resource.restart();
                        },
                        "Refresh"
                    }
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
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Total Keys" } span { class: "metric-value", "{total}" } span { class: "metric-note", "router credentials" } }
                    div { class: "metric-icon tone-blue", Icon { name: "key" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Active" } span { class: "metric-value", "{active}" } span { class: "metric-note", "can send traffic" } }
                    div { class: "metric-icon tone-green", Icon { name: "activity" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Disabled" } span { class: "metric-value", "{disabled}" } span { class: "metric-note", "blocked credentials" } }
                    div { class: "metric-icon tone-gray", Icon { name: "lock" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "IP Restricted" } span { class: "metric-value", "{restricted}" } span { class: "metric-note", "keys with allowlists" } }
                    div { class: "metric-icon tone-purple", Icon { name: "shield" } }
                }
            }

            if !notice().is_empty() { div { class: "terminal auth-status", "{notice}" } }
            if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }
            if let Some(message) = users_error {
                div { class: "product-note", "Owner names could not be loaded ({message}). Existing keys still work; owner IDs will be shown where necessary." }
            }

            if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "API keys could not be loaded" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| tokens_resource.restart(), "Retry" }
                }
            } else if tokens.is_empty() {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "key" } }
                        h3 { "Create the first client credential" }
                        p { "API keys are how Playground and external OpenAI-compatible clients authenticate to the BurnCloud router." }
                        button { class: "button button-primary", onclick: move |_| create_open.set(true), "Create API Key" }
                    }
                }
            } else {
                div { class: "card table-card",
                    div { class: "card-pad product-section-head",
                        div {
                            h3 { "Credentials" }
                            p { "Keys are masked after creation. Rotate credentials instead of sharing or recreating them casually." }
                        }
                    }
                    div { class: "table-wrap",
                        table { class: "data-table",
                            thead { tr {
                                th { "Key" }
                                th { "Owner" }
                                th { "Status" }
                                th { "Quota" }
                                th { "Network Access" }
                                th { "Version" }
                                th { class: "right", "Actions" }
                            } }
                            tbody {
                                for item in tokens {
                                    {
                                        let row_key = item.token.clone();
                                        let token_toggle = item.token.clone();
                                        let item_for_whitelist = item.clone();
                                        let item_for_rotate = item.clone();
                                        let item_for_delete = item.clone();
                                        let label = masked(&item.token);
                                        let next_status = if item.status == "active" { "disabled" } else { "active" };
                                        let toggle_label = if item.status == "active" { "Disable" } else { "Enable" };
                                        let quota_text = if item.quota_limit < 0 { "Unlimited".to_string() } else { item.quota_limit.to_string() };
                                        let ip_text = item
                                            .ip_whitelist
                                            .clone()
                                            .filter(|value| !value.trim().is_empty())
                                            .unwrap_or_else(|| "Any IP".to_string());
                                        let owner_name = owner_names.get(&item.user_id).cloned().unwrap_or_else(|| item.user_id.clone());
                                        rsx! {
                                            tr { key: "{row_key}",
                                                td { class: "mono table-primary", "{label}" }
                                                td {
                                                    div { class: "two-line",
                                                        strong { "{owner_name}" }
                                                        small { class: "mono muted", "{item.user_id}" }
                                                    }
                                                }
                                                td { span { class: if item.status == "active" { "badge badge-success" } else { "badge badge-neutral" }, "{item.status}" } }
                                                td {
                                                    div { class: "two-line",
                                                        strong { class: "small", "{quota_text}" }
                                                        small { class: "muted", "used {item.used_quota}" }
                                                    }
                                                }
                                                td { class: "mono muted", "{ip_text}" }
                                                td { class: "mono", "v{item.key_version}" }
                                                td { class: "right",
                                                    div { class: "action-menu",
                                                        button {
                                                            class: "button button-ghost button-sm",
                                                            disabled: busy(),
                                                            onclick: move |_| {
                                                                let token = token_toggle.clone();
                                                                busy.set(true);
                                                                error.set(String::new());
                                                                spawn(async move {
                                                                    match TokenService::set_status(&token, next_status).await {
                                                                        Ok(()) => {
                                                                            notice.set(format!("API key {next_status}."));
                                                                            tokens_resource.restart();
                                                                        }
                                                                        Err(message) => error.set(format!("Status update failed: {message}")),
                                                                    }
                                                                    busy.set(false);
                                                                });
                                                            },
                                                            "{toggle_label}"
                                                        }
                                                        button { class: "button button-ghost button-sm", onclick: move |_| rotate_target.set(Some(item_for_rotate.clone())), "Rotate" }
                                                        button {
                                                            class: "button button-ghost button-sm",
                                                            onclick: move |_| {
                                                                whitelist.set(item_for_whitelist.ip_whitelist.clone().unwrap_or_default());
                                                                whitelist_target.set(Some(item_for_whitelist.clone()));
                                                            },
                                                            "IP Rules"
                                                        }
                                                        button { class: "button button-ghost button-sm danger", onclick: move |_| delete_target.set(Some(item_for_delete.clone())), "Delete" }
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
                    div { class: "drawer-head",
                        div { h2 { "Create API Key" } p { class: "small muted", "Choose which account will own this router credential." } }
                        button { class: "close-button", onclick: move |_| create_open.set(false), "×" }
                    }
                    div { class: "drawer-body stack-lg",
                        div { class: "form-section",
                            div { class: "form-section-head", strong { "Owner" } small { "Usage and billing attribution follow the selected BurnCloud account." } }
                            if users.is_empty() {
                                div { class: "field",
                                    label { "Owner user ID" }
                                    input { class: "input mono", value: "{user_id}", oninput: move |event| user_id.set(event.value()) }
                                }
                            } else {
                                div { class: "field",
                                    label { "Account" }
                                    select { class: "select", value: "{user_id}", onchange: move |event| user_id.set(event.value()),
                                        for user in users.iter() {
                                            option { value: "{user.id}", "{user.username} — {user.role}" }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "form-section",
                            div { class: "form-section-head", strong { "Quota" } small { "Leave blank for unlimited. The server currently exposes a single quota limit rather than named scopes." } }
                            div { class: "field",
                                label { "Quota limit (optional)" }
                                input { class: "input", r#type: "number", value: "{quota}", placeholder: "Unlimited", oninput: move |event| quota.set(event.value()) }
                            }
                        }

                        if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }
                        div { class: "row customer-form-actions",
                            button { class: "button button-secondary", disabled: busy(), onclick: move |_| create_open.set(false), "Cancel" }
                            button {
                                class: "button button-primary",
                                disabled: busy(),
                                onclick: move |_| {
                                    let uid = user_id().trim().to_string();
                                    let quota_value = quota().trim().parse::<i64>().ok();
                                    if uid.is_empty() {
                                        error.set("Choose an owner for this API key.".to_string());
                                        return;
                                    }
                                    busy.set(true);
                                    error.set(String::new());
                                    spawn(async move {
                                        match TokenService::create(&uid, quota_value).await {
                                            Ok(token) => {
                                                new_key.set(Some(token));
                                                create_open.set(false);
                                                tokens_resource.restart();
                                            }
                                            Err(message) => error.set(format!("Create key failed: {message}")),
                                        }
                                        busy.set(false);
                                    });
                                },
                                if busy() { "Creating…" } else { "Create API Key" }
                            }
                        }
                    }
                }
            }

            if let Some(created_key) = new_key() {
                div { class: "drawer-backdrop", onclick: move |_| new_key.set(None) }
                aside { class: "drawer",
                    div { class: "drawer-head",
                        h2 { "API Key Created" }
                        button { class: "close-button", onclick: move |_| new_key.set(None), "×" }
                    }
                    div { class: "drawer-body stack-lg",
                        div { class: "form-section",
                            strong { "Copy this credential now" }
                            p { class: "small muted", "The table will only show a masked version after you close this panel." }
                            code { class: "terminal", style: "word-break:break-all;user-select:all", "{created_key}" }
                        }
                        div { class: "product-note", "Store API keys in a secret manager or environment variable. Do not paste them into source code, tickets, or chat logs." }
                        button { class: "button button-primary", onclick: move |_| new_key.set(None), "I saved the key" }
                    }
                }
            }

            if let Some(target) = whitelist_target() {
                div { class: "drawer-backdrop", onclick: move |_| whitelist_target.set(None) }
                aside { class: "drawer",
                    div { class: "drawer-head", h2 { "Network Access" } button { class: "close-button", onclick: move |_| whitelist_target.set(None), "×" } }
                    div { class: "drawer-body stack-lg",
                        div { class: "form-section",
                            div { class: "form-section-head", strong { "IP whitelist" } small { "Empty means unrestricted. Only add rules when the calling network has stable source addresses." } }
                            textarea { class: "textarea mono", rows: "6", value: "{whitelist}", oninput: move |event| whitelist.set(event.value()) }
                        }
                        button {
                            class: "button button-primary",
                            disabled: busy(),
                            onclick: move |_| {
                                let token = target.token.clone();
                                let rules = whitelist();
                                busy.set(true);
                                error.set(String::new());
                                spawn(async move {
                                    match TokenService::set_ip_whitelist(&token, &rules).await {
                                        Ok(()) => {
                                            notice.set("IP whitelist saved.".to_string());
                                            whitelist_target.set(None);
                                            tokens_resource.restart();
                                        }
                                        Err(message) => error.set(format!("Whitelist update failed: {message}")),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Save Network Rules"
                        }
                    }
                }
            }

            if let Some(target) = rotate_target() {
                {
                    let token = target.token.clone();
                    let label = masked(&target.token);
                    rsx! {
                        div { class: "drawer-backdrop", onclick: move |_| rotate_target.set(None) }
                        aside { class: "drawer",
                            div { class: "drawer-head", h2 { "Rotate API Key" } button { class: "close-button", onclick: move |_| rotate_target.set(None), "×" } }
                            div { class: "drawer-body stack-lg",
                                div { class: "form-section",
                                    strong { "Rotate {label}?" }
                                    p { class: "small muted", "BurnCloud will create a new key version and keep the old credential valid for the configured 24-hour transition period." }
                                }
                                div { class: "product-note", "Update clients to the new credential before the transition period ends. Rotation is safer than deleting an in-use key." }
                                div { class: "row customer-form-actions",
                                    button { class: "button button-secondary", disabled: busy(), onclick: move |_| rotate_target.set(None), "Cancel" }
                                    button {
                                        class: "button button-primary",
                                        disabled: busy(),
                                        onclick: move |_| {
                                            busy.set(true);
                                            error.set(String::new());
                                            spawn(async move {
                                                match TokenService::rotate(&token, 24, false).await {
                                                    Ok(value) => {
                                                        notice.set(format!("Key rotation accepted: {value}"));
                                                        rotate_target.set(None);
                                                        tokens_resource.restart();
                                                    }
                                                    Err(message) => error.set(format!("Rotation failed: {message}")),
                                                }
                                                busy.set(false);
                                            });
                                        },
                                        if busy() { "Rotating…" } else { "Rotate Key" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(target) = delete_target() {
                {
                    let token = target.token.clone();
                    let label = masked(&target.token);
                    let owner = owner_names.get(&target.user_id).cloned().unwrap_or_else(|| target.user_id.clone());
                    rsx! {
                        div { class: "drawer-backdrop", onclick: move |_| delete_target.set(None) }
                        aside { class: "drawer",
                            div { class: "drawer-head", h2 { class: "danger", "Delete API Key" } button { class: "close-button", onclick: move |_| delete_target.set(None), "×" } }
                            div { class: "drawer-body stack-lg",
                                div { class: "form-section danger-zone",
                                    strong { "Delete {label}?" }
                                    p { class: "small muted", "This immediately removes the credential owned by {owner}. Any client still using it will stop authenticating." }
                                }
                                div { class: "row customer-form-actions",
                                    button { class: "button button-secondary", disabled: busy(), onclick: move |_| delete_target.set(None), "Cancel" }
                                    button {
                                        class: "button button-primary",
                                        disabled: busy(),
                                        onclick: move |_| {
                                            busy.set(true);
                                            error.set(String::new());
                                            spawn(async move {
                                                match TokenService::delete(&token).await {
                                                    Ok(()) => {
                                                        notice.set("API key deleted.".to_string());
                                                        delete_target.set(None);
                                                        tokens_resource.restart();
                                                    }
                                                    Err(message) => error.set(format!("Delete failed: {message}")),
                                                }
                                                busy.set(false);
                                            });
                                        },
                                        if busy() { "Deleting…" } else { "Delete API Key" }
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
pub fn Team() -> Element {
    let auth = use_auth();
    let session = auth.user();
    let mut resource = use_resource(move || async move { UserService::list().await });
    let snapshot = resource.read().clone();
    let load_error = snapshot.as_ref().and_then(|result| result.as_ref().err().cloned());
    let all_users = snapshot.and_then(Result::ok).unwrap_or_default();
    let staff: Vec<User> = all_users
        .iter()
        .filter(|user| is_staff_role(&user.role))
        .cloned()
        .collect();

    let username = session.as_ref().map(|user| user.username.clone()).unwrap_or_else(|| "-".to_string());
    let user_id = session.as_ref().map(|user| user.id.clone()).unwrap_or_else(|| "-".to_string());
    let roles = session.as_ref().map(|user| user.roles.join(", ")).unwrap_or_else(|| "-".to_string());

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Team" }
                    p { class: "page-subtitle", "People with administrative or operator roles who manage this BurnCloud environment." }
                }
                button { class: "button button-secondary", onclick: move |_| resource.restart(), "Refresh" }
            }

            div { class: "grid-2",
                div { class: "card card-pad stack",
                    div { class: "product-section-head", div { h3 { "Your session" } p { "The identity currently operating this console." } } }
                    div { class: "receipt-row", label { "User" } strong { "{username}" } }
                    div { class: "receipt-row", label { "User ID" } strong { class: "mono", "{user_id}" } }
                    div { class: "receipt-row", label { "Roles" } strong { "{roles}" } }
                }
                div { class: "card card-pad stack",
                    div { class: "product-section-head", div { h3 { "Role management" } p { "Why this page is read-only today." } } }
                    p { class: "small muted", "The current BurnCloud server exposes user roles when listing accounts, but it does not expose an API to invite a staff member or change roles safely." }
                    div { class: "product-note", "Customer account creation remains under Customers. Team will become editable only when the backend has explicit role-management endpoints." }
                }
            }

            if let Some(message) = load_error {
                div { class: "terminal auth-status auth-status-error", "{message}" }
            } else if staff.is_empty() {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "users" } }
                        h3 { "No staff roles returned" }
                        p { "The current session may still be authorized, but the user list did not return accounts with admin, root, owner, or operator roles." }
                    }
                }
            } else {
                div { class: "card table-card",
                    div { class: "card-pad product-section-head", div { h3 { "Environment operators" } p { "Administrative identities are separated from customer accounts." } } }
                    div { class: "table-wrap",
                        table { class: "data-table",
                            thead { tr { th { "Member" } th { "Email" } th { "Role" } th { "Status" } } }
                            tbody {
                                for user in staff {
                                    {
                                        let email = user.email.clone().unwrap_or_else(|| "-".to_string());
                                        let status = if user.status == 1 { "Active" } else { "Disabled" };
                                        rsx! {
                                            tr { key: "{user.id}",
                                                td {
                                                    div { class: "two-line",
                                                        strong { class: "table-primary", "{user.username}" }
                                                        small { class: "mono muted", "{user.id}" }
                                                    }
                                                }
                                                td { "{email}" }
                                                td { span { class: "badge badge-neutral", "{user.role}" } }
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
