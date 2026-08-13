use std::{
    collections::{BTreeMap, BTreeSet},
    net::IpAddr,
};

use dioxus::prelude::*;

use crate::{
    backend::{use_auth, TokenDto, TokenService, User, UserService},
    components::Icon,
};

#[derive(Clone, PartialEq)]
struct RevealedCredential {
    title: String,
    token: String,
    detail: String,
}

fn masked(token: &str) -> String {
    if token.len() <= 16 {
        return "••••••••".to_string();
    }
    format!("{}••••••••{}", &token[..8], &token[token.len() - 6..])
}

fn status_label(status: &str) -> String {
    match status {
        "active" => "Active".to_string(),
        "disabled" => "Disabled".to_string(),
        other if other.trim().is_empty() => "Unknown".to_string(),
        other => format!("Unknown ({other})"),
    }
}

fn format_usd_nano(nano: i64) -> String {
    let negative = nano < 0;
    let absolute = nano.unsigned_abs();
    let whole = absolute / 1_000_000_000;
    let fraction = absolute % 1_000_000_000;
    let mut fraction_text = format!("{fraction:09}");
    while fraction_text.len() > 2 && fraction_text.ends_with('0') {
        fraction_text.pop();
    }
    format!("{}${whole}.{fraction_text}", if negative { "-" } else { "" })
}

fn parse_spend_limit_usd(input: &str) -> Result<Option<i64>, String> {
    let value = input.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.starts_with('-') || value.starts_with('+') {
        return Err("Enter a positive USD amount, or leave the field blank for unlimited.".to_string());
    }

    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some() {
        return Err("Enter one decimal USD amount, for example 25 or 25.50.".to_string());
    }
    if whole.is_empty() && fraction.is_empty() {
        return Err("Enter a USD amount, or leave the field blank for unlimited.".to_string());
    }
    if (!whole.is_empty() && !whole.chars().all(|ch| ch.is_ascii_digit()))
        || (!fraction.is_empty() && !fraction.chars().all(|ch| ch.is_ascii_digit()))
    {
        return Err("Spend limit must contain digits and an optional decimal point only.".to_string());
    }
    if fraction.len() > 9 {
        return Err("Spend limit supports at most 9 decimal places (nano-USD precision).".to_string());
    }

    let whole_value = if whole.is_empty() {
        0i64
    } else {
        whole
            .parse::<i64>()
            .map_err(|_| "Spend limit is too large.".to_string())?
    };
    let mut fraction_text = fraction.to_string();
    while fraction_text.len() < 9 {
        fraction_text.push('0');
    }
    let fraction_nano = if fraction_text.is_empty() {
        0i64
    } else {
        fraction_text
            .parse::<i64>()
            .map_err(|_| "Spend limit is invalid.".to_string())?
    };
    let nano = whole_value
        .checked_mul(1_000_000_000)
        .and_then(|base| base.checked_add(fraction_nano))
        .ok_or_else(|| "Spend limit is too large.".to_string())?;
    if nano <= 0 {
        return Err("Spend limit must be greater than $0. Use Disable Key when traffic must stop.".to_string());
    }
    Ok(Some(nano))
}

fn normalize_ip_whitelist(input: &str) -> Result<String, String> {
    if input.trim().is_empty() {
        return Ok(String::new());
    }

    let mut addresses = BTreeSet::new();
    for entry in input.split(|ch| matches!(ch, ',' | '\n' | '\r')) {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let parsed = entry.parse::<IpAddr>().map_err(|_| {
            format!(
                "'{entry}' is not an exact IPv4 or IPv6 address. The current server does not support CIDR ranges."
            )
        })?;
        addresses.insert(parsed.to_string());
    }

    Ok(addresses.into_iter().collect::<Vec<_>>().join(","))
}

#[component]
pub fn APIKeys() -> Element {
    let auth = use_auth();
    let current_user_id = auth.user().map(|user| user.id).unwrap_or_default();
    let current_user_for_signal = current_user_id.clone();

    let mut tokens_resource = use_resource(move || async move { TokenService::list().await });
    let mut users_resource = use_resource(move || async move { UserService::list().await });

    let mut create_open = use_signal(|| false);
    let mut manage_target = use_signal(|| None::<TokenDto>);
    let mut status_target = use_signal(|| None::<TokenDto>);
    let mut whitelist_target = use_signal(|| None::<TokenDto>);
    let mut rotate_target = use_signal(|| None::<TokenDto>);
    let mut delete_target = use_signal(|| None::<TokenDto>);
    let mut revealed_key = use_signal(|| None::<RevealedCredential>);
    let mut user_id = use_signal(move || current_user_for_signal);
    let mut spend_limit = use_signal(String::new);
    let mut whitelist = use_signal(String::new);
    let mut confirm_unrestricted = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(String::new);
    let mut error = use_signal(String::new);

    let token_snapshot = tokens_resource.read().clone();
    let user_snapshot = users_resource.read().clone();
    let token_loading = token_snapshot.is_none();
    let owner_loading = user_snapshot.is_none();
    let load_error = token_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let users_error = user_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let token_ready = token_snapshot.as_ref().is_some_and(Result::is_ok);
    let owner_directory_ready = user_snapshot.as_ref().is_some_and(Result::is_ok);
    let tokens = token_snapshot.clone().and_then(Result::ok).unwrap_or_default();
    let users: Vec<User> = user_snapshot.clone().and_then(Result::ok).unwrap_or_default();
    let active_users: Vec<User> = users
        .iter()
        .filter(|user| user.status == 1)
        .cloned()
        .collect();

    let owner_accounts: BTreeMap<String, (String, i32)> = users
        .iter()
        .map(|user| (user.id.clone(), (user.username.clone(), user.status)))
        .collect();

    let active = tokens.iter().filter(|token| token.status == "active").count();
    let disabled = tokens.iter().filter(|token| token.status == "disabled").count();
    let unknown_status = tokens
        .len()
        .saturating_sub(active)
        .saturating_sub(disabled);
    let spend_limited = tokens.iter().filter(|token| token.quota_limit >= 0).count();
    let restricted = tokens
        .iter()
        .filter(|token| {
            token
                .ip_whitelist
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
        })
        .count();
    let owner_attention = if owner_directory_ready {
        tokens
            .iter()
            .filter(|token| {
                token.status == "active"
                    && owner_accounts
                        .get(&token.user_id)
                        .map(|(_, status)| *status != 1)
                        .unwrap_or(true)
            })
            .count()
    } else {
        0
    };
    let attention = unknown_status + owner_attention;

    let active_text = if token_ready {
        active.to_string()
    } else {
        "—".to_string()
    };
    let limited_text = if token_ready {
        spend_limited.to_string()
    } else {
        "—".to_string()
    };
    let restricted_text = if token_ready {
        restricted.to_string()
    } else {
        "—".to_string()
    };
    let attention_text = if token_ready && owner_directory_ready {
        attention.to_string()
    } else {
        "—".to_string()
    };

    let create_ready = owner_directory_ready && !active_users.is_empty();
    let preferred_owner = if active_users
        .iter()
        .any(|user| user.id == current_user_id)
    {
        current_user_id.clone()
    } else {
        active_users
            .first()
            .map(|user| user.id.clone())
            .unwrap_or_default()
    };
    let preferred_owner_header = preferred_owner.clone();
    let preferred_owner_empty = preferred_owner.clone();
    let create_disabled_title = if owner_loading {
        "Loading active owner accounts"
    } else if users_error.is_some() {
        "Owner directory unavailable — retry before creating a credential"
    } else if active_users.is_empty() {
        "No active owner account is available"
    } else {
        "Create a router credential"
    };

    let spend_validation = parse_spend_limit_usd(&spend_limit());
    let spend_error = spend_validation.as_ref().err().cloned();
    let spend_preview = match &spend_validation {
        Ok(Some(value)) => format_usd_nano(*value),
        Ok(None) => "Unlimited".to_string(),
        Err(_) => "Invalid".to_string(),
    };

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "API Keys" }
                    p { class: "page-subtitle", "Control who can send routed traffic, how much a credential may spend, where it may connect from, and how its lifecycle is changed." }
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
                        disabled: !create_ready,
                        title: "{create_disabled_title}",
                        onclick: move |_| {
                            user_id.set(preferred_owner_header.clone());
                            spend_limit.set(String::new());
                            notice.set(String::new());
                            error.set(String::new());
                            create_open.set(true);
                        },
                        Icon { name: "plus" }
                        "Create API Key"
                    }
                }
            }

            div { class: "product-note",
                strong { "Spend quota semantics: " }
                "router credential quota is charged from calculated request cost in nano-USD. BurnCloud converts those integer values to normal USD amounts in this page; blank spend limit means unlimited."
            }

            div { class: "metrics",
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Active Keys" } span { class: "metric-value", "{active_text}" } span { class: "metric-note", if token_ready { "{disabled} disabled" } else { "credential inventory unavailable" } } }
                    div { class: "metric-icon tone-green", Icon { name: "key" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Spend Limited" } span { class: "metric-value", "{limited_text}" } span { class: "metric-note", "credentials with a finite USD cost ceiling" } }
                    div { class: "metric-icon tone-amber", Icon { name: "dollar" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "IP Restricted" } span { class: "metric-value", "{restricted_text}" } span { class: "metric-note", "keys limited to exact source IPs" } }
                    div { class: "metric-icon tone-purple", Icon { name: "shield" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Needs Attention" } span { class: "metric-value", "{attention_text}" } span { class: "metric-note", "unknown state or active key with inactive/missing owner" } }
                    div { class: "metric-icon", Icon { name: "activity" } }
                }
            }

            if !notice().is_empty() { div { class: "terminal auth-status", "{notice}" } }
            if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }

            if owner_loading {
                div { class: "product-note", "Loading owner accounts. New API-key creation stays disabled until attribution can be verified." }
            } else if let Some(message) = users_error.clone() {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Owner directory unavailable" }
                    p { class: "small muted", "Existing credentials can still be inspected by owner ID, but BurnCloud will not fall back to free-form ownership when the account directory failed to load." }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-secondary button-sm", onclick: move |_| users_resource.restart(), "Retry owner directory" }
                }
            } else if active_users.is_empty() {
                div { class: "readiness-strip blocked",
                    span { class: "readiness-dot" }
                    strong { "No active account can own a new credential" }
                    span { class: "muted", "Create or reactivate the intended account before issuing API access." }
                }
            }

            if owner_directory_ready && owner_attention > 0 {
                div { class: "card card-pad stack",
                    strong { class: "danger", "{owner_attention} active key(s) belong to an inactive or missing account" }
                    p { class: "small muted", "This is an access-review warning. The current legacy router-token path does not prove that disabling an account revokes its router token; server-side enforcement is tracked separately. Disable affected keys directly if traffic must stop now." }
                }
            }

            if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "API keys could not be loaded" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| tokens_resource.restart(), "Retry" }
                }
            } else if token_loading {
                div { class: "card card-pad", "Loading API-key inventory…" }
            } else if tokens.is_empty() {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "key" } }
                        h3 { "Create the first client credential" }
                        p { "API keys are how Playground and external OpenAI-compatible clients authenticate to the BurnCloud router." }
                        button {
                            class: "button button-primary",
                            disabled: !create_ready,
                            title: "{create_disabled_title}",
                            onclick: move |_| {
                                user_id.set(preferred_owner_empty.clone());
                                spend_limit.set(String::new());
                                error.set(String::new());
                                create_open.set(true);
                            },
                            "Create API Key"
                        }
                    }
                }
            } else {
                div { class: "card table-card",
                    div { class: "card-pad product-section-head",
                        div {
                            h3 { "Credentials" }
                            p { "Ownership, spend ceiling, network restriction, and status are visible before lifecycle actions." }
                        }
                    }
                    div { class: "table-wrap",
                        table { class: "data-table",
                            thead { tr {
                                th { "Key" }
                                th { "Owner" }
                                th { "Status" }
                                th { "Spend Used / Limit" }
                                th { "Network" }
                                th { class: "right", "Action" }
                            } }
                            tbody {
                                for item in tokens {
                                    {
                                        let row_key = item.token.clone();
                                        let item_for_manage = item.clone();
                                        let label = masked(&item.token);
                                        let owner_info = owner_accounts.get(&item.user_id).cloned();
                                        let owner_name = owner_info
                                            .as_ref()
                                            .map(|(name, _)| name.clone())
                                            .unwrap_or_else(|| item.user_id.clone());
                                        let owner_inactive = owner_directory_ready
                                            && owner_info.as_ref().is_some_and(|(_, status)| *status != 1);
                                        let owner_missing = owner_directory_ready && owner_info.is_none();
                                        let state = status_label(&item.status);
                                        let spend_text = if item.quota_limit < 0 {
                                            format!("{} / Unlimited", format_usd_nano(item.used_quota))
                                        } else {
                                            format!(
                                                "{} / {}",
                                                format_usd_nano(item.used_quota),
                                                format_usd_nano(item.quota_limit)
                                            )
                                        };
                                        let network_restricted = item
                                            .ip_whitelist
                                            .as_deref()
                                            .is_some_and(|value| !value.trim().is_empty());
                                        rsx! {
                                            tr { key: "{row_key}",
                                                td { class: "mono table-primary", "{label}" }
                                                td {
                                                    div { class: "two-line",
                                                        div { class: "row gap-2",
                                                            strong { "{owner_name}" }
                                                            if owner_inactive { span { class: "badge badge-error", "Owner disabled" } }
                                                            if owner_missing { span { class: "badge badge-warning", "Owner missing" } }
                                                        }
                                                        small { class: "mono muted", "{item.user_id}" }
                                                    }
                                                }
                                                td {
                                                    span {
                                                        class: if item.status == "active" { "badge badge-success" } else if item.status == "disabled" { "badge badge-neutral" } else { "badge badge-warning" },
                                                        "{state}"
                                                    }
                                                }
                                                td {
                                                    div { class: "two-line",
                                                        strong { class: "small tabular", "{spend_text}" }
                                                        small { class: "muted", "calculated USD cost" }
                                                    }
                                                }
                                                td {
                                                    span { class: if network_restricted { "badge badge-neutral" } else { "small muted" },
                                                        if network_restricted { "Exact IP allowlist" } else { "Any IP" }
                                                    }
                                                }
                                                td { class: "right",
                                                    button {
                                                        class: "button button-secondary button-sm",
                                                        onclick: move |_| {
                                                            notice.set(String::new());
                                                            error.set(String::new());
                                                            manage_target.set(Some(item_for_manage.clone()));
                                                        },
                                                        "Manage"
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
                        div { h2 { "Create API Key" } p { class: "small muted", "Choose an active owner and an optional USD spend ceiling." } }
                        button { class: "close-button", onclick: move |_| create_open.set(false), "×" }
                    }
                    div { class: "drawer-body stack-lg",
                        div { class: "form-section",
                            div { class: "form-section-head", strong { "Owner" } small { "Usage and billing attribution follow the selected account." } }
                            if !owner_directory_ready {
                                div { class: "readiness-strip blocked",
                                    span { class: "readiness-dot" }
                                    strong { "Owner directory is not ready" }
                                    span { class: "muted", "Creation is blocked rather than accepting an unverified free-form user ID." }
                                }
                            } else if active_users.is_empty() {
                                div { class: "readiness-strip blocked", span { class: "readiness-dot" } strong { "No active owner account" } }
                            } else {
                                div { class: "field",
                                    label { "Active account" }
                                    select { class: "select", value: "{user_id}", onchange: move |event| user_id.set(event.value()),
                                        for user in active_users.iter() {
                                            option { value: "{user.id}", "{user.username} — {user.role}" }
                                        }
                                    }
                                    span { class: "tiny subtle", "Disabled accounts are intentionally excluded from new credential ownership." }
                                }
                            }
                        }

                        div { class: "form-section",
                            div { class: "form-section-head", strong { "Spend limit" } small { "Optional ceiling on calculated routed-request cost. Blank means unlimited." } }
                            div { class: "field",
                                label { "USD spend limit (optional)" }
                                input {
                                    class: "input mono",
                                    r#type: "number",
                                    min: "0",
                                    step: "0.01",
                                    value: "{spend_limit}",
                                    placeholder: "Unlimited",
                                    oninput: move |event| spend_limit.set(event.value()),
                                }
                                span { class: "tiny subtle", "Stored as nano-USD (1 USD = 1,000,000,000 quota units). Up to 9 decimal places are accepted." }
                            }
                            div { class: "receipt-row", label { "Resulting limit" } strong { class: "mono", "{spend_preview}" } }
                            if let Some(message) = spend_error.clone() {
                                div { class: "terminal auth-status auth-status-error", "{message}" }
                            }
                        }

                        if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }
                        div { class: "row customer-form-actions",
                            button { class: "button button-secondary", disabled: busy(), onclick: move |_| create_open.set(false), "Cancel" }
                            button {
                                class: "button button-primary",
                                disabled: busy() || !create_ready || spend_validation.is_err(),
                                onclick: move |_| {
                                    let uid = user_id().trim().to_string();
                                    if !active_users.iter().any(|user| user.id == uid && user.status == 1) {
                                        error.set("Choose an active owner returned by the current account directory.".to_string());
                                        return;
                                    }
                                    let quota_value = match parse_spend_limit_usd(&spend_limit()) {
                                        Ok(value) => value,
                                        Err(message) => {
                                            error.set(message);
                                            return;
                                        }
                                    };
                                    let limit_label = quota_value
                                        .map(format_usd_nano)
                                        .unwrap_or_else(|| "Unlimited".to_string());
                                    busy.set(true);
                                    error.set(String::new());
                                    spawn(async move {
                                        match TokenService::create(&uid, quota_value).await {
                                            Ok(token) => {
                                                revealed_key.set(Some(RevealedCredential {
                                                    title: "API Key Created".to_string(),
                                                    token,
                                                    detail: format!("Owner: {uid} • Spend limit: {limit_label}"),
                                                }));
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

            if let Some(target) = manage_target() {
                {
                    let target_for_status = target.clone();
                    let target_for_network = target.clone();
                    let target_for_rotate = target.clone();
                    let target_for_delete = target.clone();
                    let label = masked(&target.token);
                    let owner = owner_accounts
                        .get(&target.user_id)
                        .map(|(name, _)| name.clone())
                        .unwrap_or_else(|| target.user_id.clone());
                    let state = status_label(&target.status);
                    let network_restricted = target
                        .ip_whitelist
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty());
                    let spend_text = if target.quota_limit < 0 {
                        format!("{} used • Unlimited", format_usd_nano(target.used_quota))
                    } else {
                        format!(
                            "{} used of {}",
                            format_usd_nano(target.used_quota),
                            format_usd_nano(target.quota_limit)
                        )
                    };
                    rsx! {
                        div { class: "drawer-backdrop", onclick: move |_| manage_target.set(None) }
                        aside { class: "drawer",
                            div { class: "drawer-head",
                                div { h2 { "Manage API Key" } p { class: "small muted mono", "{label}" } }
                                button { class: "close-button", onclick: move |_| manage_target.set(None), "×" }
                            }
                            div { class: "drawer-body stack-lg",
                                div { class: "form-section",
                                    div { class: "form-section-head", strong { "Access summary" } small { "Verify ownership and current access state before changing the credential." } }
                                    div { class: "receipt-row", label { "Owner" } strong { "{owner}" } }
                                    div { class: "receipt-row", label { "Status" } strong { "{state}" } }
                                    div { class: "receipt-row", label { "Spend used / limit" } strong { class: "mono", "{spend_text}" } }
                                    div { class: "receipt-row", label { "Network" } strong { if network_restricted { "Exact IP allowlist" } else { "Any IP" } } }
                                    details {
                                        summary { class: "small strong", style: "cursor:pointer", "Technical details" }
                                        div { class: "stack", style: "margin-top:12px",
                                            div { class: "receipt-row", label { "Owner ID" } strong { class: "mono", "{target.user_id}" } }
                                            div { class: "receipt-row", label { "Key version" } strong { class: "mono", "v{target.key_version}" } }
                                            div { class: "receipt-row", label { "Raw quota used" } strong { class: "mono", "{target.used_quota} nano-USD" } }
                                            div { class: "receipt-row", label { "Raw quota limit" } strong { class: "mono", if target.quota_limit < 0 { "unlimited" } else { "{target.quota_limit} nano-USD" } } }
                                        }
                                    }
                                }

                                div { class: "form-section",
                                    div { class: "form-section-head", strong { "Credential lifecycle" } small { "Status changes get their own confirmation. Rotation is preferred for normal credential replacement." } }
                                    div { class: "row gap-2", style: "flex-wrap:wrap",
                                        button {
                                            class: if target.status == "active" { "button button-secondary" } else { "button button-primary" },
                                            disabled: busy() || !matches!(target.status.as_str(), "active" | "disabled"),
                                            onclick: move |_| {
                                                manage_target.set(None);
                                                status_target.set(Some(target_for_status.clone()));
                                            },
                                            if target.status == "active" { "Disable Key" } else if target.status == "disabled" { "Enable Key" } else { "Unknown Status" }
                                        }
                                        button {
                                            class: "button button-secondary",
                                            disabled: target.status != "active",
                                            onclick: move |_| {
                                                manage_target.set(None);
                                                rotate_target.set(Some(target_for_rotate.clone()));
                                            },
                                            "Rotate Key"
                                        }
                                        button {
                                            class: "button button-secondary",
                                            onclick: move |_| {
                                                whitelist.set(target_for_network.ip_whitelist.clone().unwrap_or_default());
                                                confirm_unrestricted.set(false);
                                                manage_target.set(None);
                                                whitelist_target.set(Some(target_for_network.clone()));
                                            },
                                            "Network Access"
                                        }
                                    }
                                }

                                div { class: "form-section danger-zone",
                                    div { class: "form-section-head", strong { class: "danger", "Delete credential" } small { "Deletion is permanent and immediately breaks any client still using this key." } }
                                    button {
                                        class: "button button-secondary danger",
                                        onclick: move |_| {
                                            manage_target.set(None);
                                            delete_target.set(Some(target_for_delete.clone()));
                                        },
                                        "Delete API Key"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(target) = status_target() {
                {
                    let token = target.token.clone();
                    let label = masked(&target.token);
                    let disabling = target.status == "active";
                    let next_status = if disabling { "disabled" } else { "active" };
                    let action = if disabling { "Disable Key" } else { "Enable Key" };
                    rsx! {
                        div { class: "drawer-backdrop", onclick: move |_| status_target.set(None) }
                        aside { class: "drawer",
                            div { class: "drawer-head",
                                div { h2 { "{action}" } p { class: "small muted mono", "{label}" } }
                                button { class: "close-button", onclick: move |_| status_target.set(None), "×" }
                            }
                            div { class: "drawer-body stack-lg",
                                if disabling {
                                    div { class: "form-section danger-zone",
                                        strong { class: "danger", "Stop this credential from authenticating?" }
                                        p { class: "small muted", "Clients using this key will fail immediately. Use rotation instead when you only need to replace a credential without an abrupt cutover." }
                                    }
                                } else {
                                    div { class: "product-note", "Enabling this key restores its ability to authenticate routed requests, subject to spend and network restrictions." }
                                }
                                div { class: "row customer-form-actions",
                                    button { class: "button button-secondary", disabled: busy(), onclick: move |_| status_target.set(None), "Cancel" }
                                    button {
                                        class: "button button-primary",
                                        disabled: busy(),
                                        onclick: move |_| {
                                            let token = token.clone();
                                            busy.set(true);
                                            error.set(String::new());
                                            spawn(async move {
                                                match TokenService::set_status(&token, next_status).await {
                                                    Ok(()) => {
                                                        notice.set(format!("API key {next_status}."));
                                                        status_target.set(None);
                                                        tokens_resource.restart();
                                                    }
                                                    Err(message) => error.set(format!("Status update failed: {message}")),
                                                }
                                                busy.set(false);
                                            });
                                        },
                                        if busy() { "Saving…" } else { "{action}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(target) = whitelist_target() {
                {
                    let had_restriction = target
                        .ip_whitelist
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty());
                    let whitelist_validation = normalize_ip_whitelist(&whitelist());
                    let whitelist_error = whitelist_validation.as_ref().err().cloned();
                    let will_be_unrestricted = whitelist_validation
                        .as_ref()
                        .is_ok_and(|value| value.is_empty());
                    let broadening_access = had_restriction && will_be_unrestricted;
                    rsx! {
                        div { class: "drawer-backdrop", onclick: move |_| whitelist_target.set(None) }
                        aside { class: "drawer",
                            div { class: "drawer-head",
                                div { h2 { "Network Access" } p { class: "small muted", "Exact source-IP allowlist for this credential." } }
                                button { class: "close-button", onclick: move |_| whitelist_target.set(None), "×" }
                            }
                            div { class: "drawer-body stack-lg",
                                div { class: "form-section",
                                    div { class: "form-section-head", strong { "Allowed source IPs" } small { "Enter exact IPv4 or IPv6 addresses separated by commas or new lines. BurnCloud normalizes them to the server's comma-separated format." } }
                                    textarea {
                                        class: "textarea mono",
                                        rows: "7",
                                        value: "{whitelist}",
                                        placeholder: "203.0.113.10\n2001:db8::10",
                                        oninput: move |event| {
                                            whitelist.set(event.value());
                                            confirm_unrestricted.set(false);
                                        }
                                    }
                                    div { class: "product-note", "CIDR ranges such as 10.0.0.0/8 are not supported by the current server and will be rejected instead of pretending the rule works." }
                                    if let Some(message) = whitelist_error.clone() {
                                        div { class: "terminal auth-status auth-status-error", "{message}" }
                                    }
                                }
                                if broadening_access {
                                    div { class: "form-section danger-zone",
                                        strong { class: "danger", "This removes the current IP restriction" }
                                        p { class: "small muted", "Saving an empty allowlist makes the credential usable from any source IP." }
                                        label { class: "row gap-2 small", style: "align-items:flex-start",
                                            input { r#type: "checkbox", checked: confirm_unrestricted(), onchange: move |_| confirm_unrestricted.set(!confirm_unrestricted()) }
                                            span { "I understand this broadens network access for the credential." }
                                        }
                                    }
                                }
                                div { class: "row customer-form-actions",
                                    button { class: "button button-secondary", disabled: busy(), onclick: move |_| whitelist_target.set(None), "Cancel" }
                                    button {
                                        class: "button button-primary",
                                        disabled: busy() || whitelist_validation.is_err() || (broadening_access && !confirm_unrestricted()),
                                        onclick: move |_| {
                                            let token = target.token.clone();
                                            let rules = match normalize_ip_whitelist(&whitelist()) {
                                                Ok(value) => value,
                                                Err(message) => {
                                                    error.set(message);
                                                    return;
                                                }
                                            };
                                            busy.set(true);
                                            error.set(String::new());
                                            spawn(async move {
                                                match TokenService::set_ip_whitelist(&token, &rules).await {
                                                    Ok(()) => {
                                                        notice.set(if rules.is_empty() {
                                                            "IP restriction removed; this key now accepts any source IP.".to_string()
                                                        } else {
                                                            "Exact IP allowlist saved.".to_string()
                                                        });
                                                        whitelist_target.set(None);
                                                        confirm_unrestricted.set(false);
                                                        tokens_resource.restart();
                                                    }
                                                    Err(message) => error.set(format!("Network access update failed: {message}")),
                                                }
                                                busy.set(false);
                                            });
                                        },
                                        "Save Network Access"
                                    }
                                }
                            }
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
                            div { class: "drawer-head",
                                div { h2 { "Rotate API Key" } p { class: "small muted mono", "{label}" } }
                                button { class: "close-button", onclick: move |_| rotate_target.set(None), "×" }
                            }
                            div { class: "drawer-body stack-lg",
                                div { class: "form-section",
                                    strong { "Create a replacement credential?" }
                                    p { class: "small muted", "BurnCloud will replace the stored current key and keep the old credential valid for a 24-hour transition period." }
                                }
                                div { class: "product-note", "The replacement key is a one-time secret. The next panel will stay open until you explicitly confirm that you saved it." }
                                div { class: "row customer-form-actions",
                                    button { class: "button button-secondary", disabled: busy(), onclick: move |_| rotate_target.set(None), "Cancel" }
                                    button {
                                        class: "button button-primary",
                                        disabled: busy(),
                                        onclick: move |_| {
                                            let token = token.clone();
                                            busy.set(true);
                                            error.set(String::new());
                                            spawn(async move {
                                                match TokenService::rotate(&token, 24, false).await {
                                                    Ok(value) => {
                                                        let new_token = value
                                                            .get("new_token")
                                                            .and_then(|value| value.as_str())
                                                            .map(str::to_string);
                                                        rotate_target.set(None);
                                                        tokens_resource.restart();
                                                        if let Some(new_token) = new_token {
                                                            let version = value
                                                                .get("key_version")
                                                                .and_then(|value| value.as_i64())
                                                                .map(|version| format!(" • New version: v{version}"))
                                                                .unwrap_or_default();
                                                            revealed_key.set(Some(RevealedCredential {
                                                                title: "API Key Rotated".to_string(),
                                                                token: new_token,
                                                                detail: format!("Old credential remains valid for the requested 24-hour transition period{version}."),
                                                            }));
                                                        } else {
                                                            error.set("Rotation may have succeeded, but the server response did not include the replacement credential. Do not rotate again until server state is verified; the old key may still be in its transition period.".to_string());
                                                        }
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
                    let owner = owner_accounts
                        .get(&target.user_id)
                        .map(|(name, _)| name.clone())
                        .unwrap_or_else(|| target.user_id.clone());
                    rsx! {
                        div { class: "drawer-backdrop", onclick: move |_| delete_target.set(None) }
                        aside { class: "drawer",
                            div { class: "drawer-head",
                                div { h2 { class: "danger", "Delete API Key" } p { class: "small muted mono", "{label}" } }
                                button { class: "close-button", onclick: move |_| delete_target.set(None), "×" }
                            }
                            div { class: "drawer-body stack-lg",
                                div { class: "form-section danger-zone",
                                    strong { "Delete this credential permanently?" }
                                    p { class: "small muted", "This immediately removes the credential owned by {owner}. Any client still using it will stop authenticating." }
                                }
                                div { class: "row customer-form-actions",
                                    button { class: "button button-secondary", disabled: busy(), onclick: move |_| delete_target.set(None), "Cancel" }
                                    button {
                                        class: "button button-primary",
                                        disabled: busy(),
                                        onclick: move |_| {
                                            let token = token.clone();
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

            if let Some(revealed) = revealed_key() {
                div { class: "drawer-backdrop" }
                aside { class: "drawer",
                    div { class: "drawer-head",
                        div {
                            h2 { "{revealed.title}" }
                            p { class: "small muted", "One-time credential reveal — save it before leaving this panel." }
                        }
                    }
                    div { class: "drawer-body stack-lg",
                        div { class: "form-section",
                            strong { "Copy this credential now" }
                            code { class: "terminal", style: "word-break:break-all;user-select:all", "{revealed.token}" }
                            p { class: "small muted", "{revealed.detail}" }
                        }
                        div { class: "product-note", "The backdrop and close button are intentionally disabled for one-time secrets. Store the key in a secret manager or environment variable; do not paste it into source code, tickets, or chat logs." }
                        button { class: "button button-primary", onclick: move |_| revealed_key.set(None), "I saved this credential" }
                    }
                }
            }
        }
    }
}
