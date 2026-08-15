use std::{
    collections::{BTreeMap, BTreeSet},
    net::IpAddr,
    time::{SystemTime, UNIX_EPOCH},
};

use dioxus::prelude::*;

use crate::{
    backend::{use_auth, TokenDto, TokenService, User, UserService},
    components::Icon,
};

#[derive(Clone, PartialEq)]
struct SecretReveal {
    title: String,
    secret: String,
    detail: String,
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn short_management_ref(reference: &str) -> String {
    if reference.len() <= 22 {
        return reference.to_string();
    }
    format!("{}…{}", &reference[..12], &reference[reference.len() - 6..])
}

fn format_spend(nanodollars: i64) -> String {
    let dollars = nanodollars as f64 / 1_000_000_000.0;
    if nanodollars == 0 {
        "$0.00".to_string()
    } else if dollars.abs() >= 1.0 {
        format!("${dollars:.2}")
    } else {
        format!("${dollars:.6}")
    }
}

fn parse_spend_limit(input: &str) -> Result<Option<i64>, String> {
    let value = input.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.starts_with('-') {
        return Err("Spend limit cannot be negative. Leave it blank for unlimited.".to_string());
    }

    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some() || whole.is_empty() || !whole.chars().all(|c| c.is_ascii_digit()) {
        return Err("Enter a valid USD amount, for example 100 or 25.50.".to_string());
    }
    if fraction.len() > 9 || !fraction.chars().all(|c| c.is_ascii_digit()) {
        return Err("USD spend limit supports up to 9 decimal places.".to_string());
    }

    let whole: i128 = whole
        .parse()
        .map_err(|_| "USD spend limit is too large.".to_string())?;
    let mut fractional = fraction.to_string();
    while fractional.len() < 9 {
        fractional.push('0');
    }
    let fraction: i128 = if fractional.is_empty() {
        0
    } else {
        fractional
            .parse()
            .map_err(|_| "Enter a valid USD amount.".to_string())?
    };
    let total = whole
        .checked_mul(1_000_000_000)
        .and_then(|base| base.checked_add(fraction))
        .ok_or_else(|| "USD spend limit is too large.".to_string())?;
    i64::try_from(total)
        .map(Some)
        .map_err(|_| "USD spend limit is too large.".to_string())
}

fn relative_time(timestamp: i64, now: i64) -> String {
    if timestamp <= 0 {
        return "Unknown".to_string();
    }
    let seconds = now.saturating_sub(timestamp);
    if seconds < 60 {
        "Just now".to_string()
    } else if seconds < 3_600 {
        format!("{}m ago", seconds / 60)
    } else if seconds < 86_400 {
        format!("{}h ago", seconds / 3_600)
    } else {
        format!("{}d ago", seconds / 86_400)
    }
}

fn ip_entries(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
        .collect()
}

fn normalize_ip_rules(value: &str) -> Result<String, String> {
    let mut entries = BTreeSet::new();
    for raw in value.split(|c| c == ',' || c == '\n' || c == '\r') {
        let entry = raw.trim();
        if entry.is_empty() {
            continue;
        }
        if entry.contains('/') {
            return Err(
                "CIDR ranges are not supported by the current router. Enter exact IPv4 or IPv6 addresses only."
                    .to_string(),
            );
        }
        entry
            .parse::<IpAddr>()
            .map_err(|_| format!("'{entry}' is not a valid IPv4 or IPv6 address."))?;
        entries.insert(entry.to_string());
    }
    Ok(entries.into_iter().collect::<Vec<_>>().join(","))
}

fn needs_attention(token: &TokenDto, owners: &BTreeMap<String, User>) -> bool {
    token.status == "active"
        && (owners
            .get(&token.user_id)
            .map(|owner| owner.status != 1)
            .unwrap_or(true)
            || (token.quota_limit >= 0 && token.used_quota > token.quota_limit))
}

#[component]
pub fn APIKeys() -> Element {
    let auth = use_auth();
    let session_user_id = auth.user().map(|user| user.id).unwrap_or_default();
    let mut tokens_resource = use_resource(move || async move { TokenService::list().await });
    let mut users_resource = use_resource(move || async move { UserService::list().await });

    let mut create_open = use_signal(|| false);
    let mut network_target = use_signal(|| None::<TokenDto>);
    let mut rotate_target = use_signal(|| None::<TokenDto>);
    let mut status_target = use_signal(|| None::<TokenDto>);
    let mut delete_target = use_signal(|| None::<TokenDto>);
    let mut secret_reveal = use_signal(|| None::<SecretReveal>);

    let mut owner_id = use_signal(move || session_user_id);
    let mut spend_limit = use_signal(String::new);
    let mut whitelist = use_signal(String::new);
    let mut confirm_unrestricted = use_signal(|| false);
    let mut revoke_old_on_rotate = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(String::new);
    let mut error = use_signal(String::new);

    let token_snapshot = tokens_resource.read().clone();
    let user_snapshot = users_resource.read().clone();
    let tokens_loading = token_snapshot.is_none();
    let users_loading = user_snapshot.is_none();
    let load_error = token_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let users_error = user_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let tokens: Vec<TokenDto> = token_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let users: Vec<User> = user_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();

    let owners: BTreeMap<String, User> = users
        .iter()
        .cloned()
        .map(|user| (user.id.clone(), user))
        .collect();
    let eligible_users: Vec<User> = users
        .iter()
        .filter(|user| user.status == 1)
        .cloned()
        .collect();
    let preferred_owner = eligible_users
        .iter()
        .find(|user| user.id == owner_id())
        .or_else(|| eligible_users.first())
        .map(|user| user.id.clone())
        .unwrap_or_default();
    let can_create = !users_loading && users_error.is_none() && !eligible_users.is_empty();

    let total = tokens.len();
    let active = tokens
        .iter()
        .filter(|token| token.status == "active")
        .count();
    let restricted = tokens
        .iter()
        .filter(|token| !ip_entries(token.ip_whitelist.as_deref()).is_empty())
        .count();
    let attention = tokens
        .iter()
        .filter(|token| needs_attention(token, &owners))
        .count();
    let now = now_epoch();

    let (health_class, health_title, health_copy) = if attention > 0 {
        (
            "readiness-strip blocked api-key-health-strip",
            "API access needs attention",
            format!("{attention} active credential records have an owner or spend-limit condition that needs review."),
        )
    } else if active == 0 && total > 0 {
        (
            "readiness-strip blocked api-key-health-strip",
            "No active API access",
            "Credentials exist, but none are currently enabled for router traffic.".to_string(),
        )
    } else {
        (
            "readiness-strip ready api-key-health-strip",
            "API access is controlled",
            format!(
                "{active} active credential records • {restricted} restricted by exact source IP."
            ),
        )
    };

    let preferred_owner_for_header = preferred_owner.clone();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "API Keys" }
                    p { class: "page-subtitle", "Issue router credentials, control spend and network exposure, and rotate secrets without redisclosing them from management APIs." }
                }
                div { class: "header-actions",
                    button {
                        class: "button button-secondary",
                        disabled: tokens_loading || users_loading,
                        onclick: move |_| {
                            tokens_resource.restart();
                            users_resource.restart();
                        },
                        if tokens_loading || users_loading { "Refreshing…" } else { "Refresh" }
                    }
                    button {
                        class: "button button-primary",
                        disabled: !can_create,
                        onclick: move |_| {
                            owner_id.set(preferred_owner_for_header.clone());
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

            if tokens_loading {
                div { class: "card product-empty api-key-loading-state",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "key" } }
                        h3 { "Loading API access" }
                        p { "Reading credential state, spend controls, versions, and network restrictions from this BurnCloud environment." }
                    }
                }
            } else if load_error.is_none() {
                div { class: "metrics",
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Configured" } span { class: "metric-value", "{total}" } span { class: "metric-note", "credential records" } }
                        div { class: "metric-icon tone-gray", Icon { name: "key" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Active" } span { class: "metric-value", "{active}" } span { class: "metric-note", "available for traffic" } }
                        div { class: if active > 0 { "metric-icon tone-green" } else { "metric-icon tone-gray" }, Icon { name: "activity" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "IP Restricted" } span { class: "metric-value", "{restricted}" } span { class: "metric-note", "exact-address allowlists" } }
                        div { class: "metric-icon tone-gray", Icon { name: "shield" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Needs Attention" } span { class: "metric-value", "{attention}" } span { class: "metric-note", "active-key conditions" } }
                        div { class: if attention > 0 { "metric-icon tone-amber" } else { "metric-icon tone-gray" }, Icon { name: "routes" } }
                    }
                }

                if !tokens.is_empty() {
                    div { class: "{health_class}",
                        span { class: "readiness-dot" }
                        div { class: "api-key-health-copy",
                            strong { "{health_title}" }
                            span { class: "small muted", "{health_copy}" }
                        }
                        span { class: "badge badge-neutral api-key-health-meta", "Secrets never listed" }
                    }
                }
            }

            if !notice().is_empty() { div { class: "terminal auth-status", "{notice}" } }
            if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }

            if let Some(message) = users_error.clone() {
                div { class: "card card-pad stack api-key-owner-error",
                    strong { "New key creation is unavailable" }
                    p { class: "small muted", "BurnCloud could not verify the account directory, so it will not guess or accept a free-form owner ID. Existing credential lifecycle actions remain available." }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-secondary", onclick: move |_| users_resource.restart(), "Retry account directory" }
                }
            } else if !users_loading && eligible_users.is_empty() {
                div { class: "product-note", "No active owner accounts are available. Create or reactivate a customer account before issuing another credential." }
            }

            if let Some(message) = load_error.clone() {
                div { class: "card card-pad stack",
                    strong { class: "danger", "API keys could not be loaded" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| tokens_resource.restart(), "Retry" }
                }
            } else if !tokens_loading && tokens.is_empty() {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "key" } }
                        h3 { "Create the first client credential" }
                        p { "API keys authenticate Playground and external OpenAI-compatible clients to the BurnCloud router. The bearer secret is shown only when it is created or rotated." }
                        button {
                            class: "button button-primary",
                            disabled: !can_create,
                            onclick: move |_| {
                                owner_id.set(preferred_owner.clone());
                                spend_limit.set(String::new());
                                error.set(String::new());
                                create_open.set(true);
                            },
                            "Create API Key"
                        }
                    }
                }
            } else if !tokens_loading {
                div { class: "card table-card api-key-table-card",
                    div { class: "card-pad product-section-head",
                        div {
                            h3 { "Credential inventory" }
                            p { "The tok_… value is an opaque management reference, not a masked bearer secret and not a data-plane credential." }
                        }
                    }
                    div { class: "table-wrap",
                        table { class: "data-table api-key-table",
                            thead { tr {
                                th { "Credential" }
                                th { "Owner" }
                                th { "State" }
                                th { "Spend" }
                                th { "Network" }
                                th { "Lifecycle" }
                                th { class: "right", "Actions" }
                            } }
                            tbody {
                                for item in tokens.iter() {
                                    {
                                        let item = item.clone();
                                        let reference = item.token.clone();
                                        let management_label = short_management_ref(&reference);
                                        let owner = owners.get(&item.user_id).cloned();
                                        let owner_name = owner
                                            .as_ref()
                                            .map(|value| value.username.clone())
                                            .unwrap_or_else(|| "Owner unavailable".to_string());
                                        let owner_bad = owner.as_ref().map(|value| value.status != 1).unwrap_or(true);
                                        let over_limit = item.quota_limit >= 0 && item.used_quota > item.quota_limit;
                                        let row_attention = item.status == "active" && (owner_bad || over_limit);
                                        let status_label = if item.status == "active" { "Active" } else { "Disabled" };
                                        let status_class = if row_attention {
                                            "badge badge-warning"
                                        } else if item.status == "active" {
                                            "badge badge-success"
                                        } else {
                                            "badge badge-neutral"
                                        };
                                        let used_spend = format_spend(item.used_quota);
                                        let spend_primary = if item.quota_limit < 0 {
                                            format!("{used_spend} used")
                                        } else {
                                            format!("{used_spend} / {}", format_spend(item.quota_limit))
                                        };
                                        let spend_note = if item.quota_limit < 0 {
                                            "Unlimited spend limit".to_string()
                                        } else if over_limit {
                                            "Spend limit exceeded".to_string()
                                        } else {
                                            "Settled request cost".to_string()
                                        };
                                        let ips = ip_entries(item.ip_whitelist.as_deref());
                                        let network_primary = if ips.is_empty() {
                                            "Any source IP".to_string()
                                        } else if ips.len() == 1 {
                                            "1 exact IP".to_string()
                                        } else {
                                            format!("{} exact IPs", ips.len())
                                        };
                                        let network_note = if ips.is_empty() {
                                            "Unrestricted by source address".to_string()
                                        } else {
                                            ips.iter().take(2).cloned().collect::<Vec<_>>().join(", ")
                                        };
                                        let created_note = relative_time(item.created_at, now);
                                        let item_for_status = item.clone();
                                        let item_for_rotate = item.clone();
                                        let item_for_network = item.clone();
                                        let item_for_delete = item.clone();
                                        rsx! {
                                            tr { key: "{reference}",
                                                td {
                                                    div { class: "two-line api-key-identity",
                                                        strong { class: "mono table-primary", title: "{reference}", "{management_label}" }
                                                        small { class: "muted", "Opaque management reference" }
                                                    }
                                                }
                                                td {
                                                    div { class: "two-line",
                                                        strong { class: if owner_bad && item.status == "active" { "danger" } else { "" }, "{owner_name}" }
                                                        small { class: "mono muted", "{item.user_id}" }
                                                    }
                                                }
                                                td {
                                                    div { class: "two-line",
                                                        span { class: "{status_class}", "{status_label}" }
                                                        if owner_bad && item.status == "active" {
                                                            small { class: "danger", "Owner disabled or unavailable" }
                                                        } else if over_limit && item.status == "active" {
                                                            small { class: "danger", "Next quota check will fail closed" }
                                                        }
                                                    }
                                                }
                                                td {
                                                    div { class: "two-line",
                                                        strong { class: "small", "{spend_primary}" }
                                                        small { class: if over_limit { "danger" } else { "muted" }, "{spend_note}" }
                                                    }
                                                }
                                                td {
                                                    div { class: "two-line api-key-network-cell",
                                                        strong { class: "small", "{network_primary}" }
                                                        small { class: "mono muted", title: "{network_note}", "{network_note}" }
                                                    }
                                                }
                                                td {
                                                    div { class: "two-line",
                                                        strong { class: "small", "Version {item.key_version}" }
                                                        small { class: "muted", "Created {created_note}" }
                                                    }
                                                }
                                                td { class: "right",
                                                    div { class: "action-menu api-key-actions",
                                                        button {
                                                            class: "button button-ghost button-sm",
                                                            disabled: busy(),
                                                            onclick: move |_| {
                                                                error.set(String::new());
                                                                status_target.set(Some(item_for_status.clone()));
                                                            },
                                                            if item.status == "active" { "Disable" } else { "Enable" }
                                                        }
                                                        button {
                                                            class: "button button-ghost button-sm",
                                                            disabled: busy(),
                                                            onclick: move |_| {
                                                                revoke_old_on_rotate.set(false);
                                                                error.set(String::new());
                                                                rotate_target.set(Some(item_for_rotate.clone()));
                                                            },
                                                            "Rotate"
                                                        }
                                                        button {
                                                            class: "button button-ghost button-sm",
                                                            disabled: busy(),
                                                            onclick: move |_| {
                                                                whitelist.set(item_for_network.ip_whitelist.clone().unwrap_or_default());
                                                                confirm_unrestricted.set(false);
                                                                error.set(String::new());
                                                                network_target.set(Some(item_for_network.clone()));
                                                            },
                                                            "IP Rules"
                                                        }
                                                        button {
                                                            class: "button button-ghost button-sm danger",
                                                            disabled: busy(),
                                                            onclick: move |_| {
                                                                error.set(String::new());
                                                                delete_target.set(Some(item_for_delete.clone()));
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

            if create_open() {
                div { class: "drawer-backdrop", onclick: move |_| if !busy() { create_open.set(false) } }
                aside { class: "drawer",
                    div { class: "drawer-head",
                        div { h2 { "Create API Key" } p { class: "small muted", "Issue a new bearer credential to one verified active account." } }
                        button { class: "close-button", disabled: busy(), onclick: move |_| create_open.set(false), "×" }
                    }
                    div { class: "drawer-body stack-lg",
                        div { class: "form-section",
                            div { class: "form-section-head", strong { "Owner" } small { "Usage and billing attribution follow this BurnCloud account. Disabled accounts are excluded." } }
                            if users_loading {
                                div { class: "product-note", "Loading account directory before key creation can continue." }
                            } else if let Some(message) = users_error.clone() {
                                div { class: "terminal auth-status auth-status-error", "Account directory unavailable: {message}" }
                            } else if eligible_users.is_empty() {
                                div { class: "product-note", "No active owner account is available." }
                            } else {
                                div { class: "field",
                                    label { "Account" }
                                    select { class: "select", value: "{owner_id}", disabled: busy(), onchange: move |event| owner_id.set(event.value()),
                                        for user in eligible_users.iter() {
                                            option { value: "{user.id}", "{user.username} — {user.role}" }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "form-section",
                            div { class: "form-section-head", strong { "USD spend limit" } small { "This limits settled request cost for this credential. It is not a token-count limit." } }
                            div { class: "field",
                                label { "Spend limit (optional)" }
                                input { class: "input", value: "{spend_limit}", placeholder: "Unlimited — e.g. 100.00", disabled: busy(), oninput: move |event| spend_limit.set(event.value()) }
                                small { class: "muted", "Leave blank for unlimited. Values are stored exactly in nanodollars." }
                            }
                        }

                        if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }
                        div { class: "row customer-form-actions",
                            button { class: "button button-secondary", disabled: busy(), onclick: move |_| create_open.set(false), "Cancel" }
                            button {
                                class: "button button-primary",
                                disabled: busy() || !can_create,
                                onclick: move |_| {
                                    let uid = owner_id().trim().to_string();
                                    if uid.is_empty() || !eligible_users.iter().any(|user| user.id == uid) {
                                        error.set("Choose a verified active owner account.".to_string());
                                        return;
                                    }
                                    let quota_value = match parse_spend_limit(&spend_limit()) {
                                        Ok(value) => value,
                                        Err(message) => {
                                            error.set(message);
                                            return;
                                        }
                                    };
                                    busy.set(true);
                                    error.set(String::new());
                                    spawn(async move {
                                        match TokenService::create(&uid, quota_value).await {
                                            Ok(secret) if !secret.trim().is_empty() => {
                                                secret_reveal.set(Some(SecretReveal {
                                                    title: "API key created".to_string(),
                                                    secret,
                                                    detail: "Creation is a one-time disclosure point. This bearer secret will not appear in the credential list or detail API again.".to_string(),
                                                }));
                                                notice.set("API key created. Save the one-time credential before continuing.".to_string());
                                                create_open.set(false);
                                                tokens_resource.restart();
                                            }
                                            Ok(_) => error.set("Key creation returned without a bearer credential. Treat the result as ambiguous and refresh before retrying.".to_string()),
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

            if let Some(reveal) = secret_reveal() {
                div { class: "drawer-backdrop" }
                aside { class: "drawer",
                    div { class: "drawer-head", div { h2 { "{reveal.title}" } p { class: "small muted", "Save this credential before leaving this step." } } }
                    div { class: "drawer-body stack-lg",
                        div { class: "form-section",
                            strong { "One-time bearer secret" }
                            p { class: "small muted", "{reveal.detail}" }
                            code { class: "terminal", "{reveal.secret}" }
                        }
                        div { class: "product-note", "Store this value in a secret manager or environment variable. Do not paste it into source code, tickets, or chat logs." }
                        button { class: "button button-primary", onclick: move |_| secret_reveal.set(None), "I saved this credential" }
                    }
                }
            }

            if let Some(target) = status_target() {
                {
                    let reference = target.token.clone();
                    let label = short_management_ref(&reference);
                    let disabling = target.status == "active";
                    let next_status = if disabling { "disabled" } else { "active" };
                    rsx! {
                        div { class: "drawer-backdrop", onclick: move |_| if !busy() { status_target.set(None) } }
                        aside { class: "drawer",
                            div { class: "drawer-head",
                                h2 { if disabling { "Disable API Key" } else { "Enable API Key" } }
                                button { class: "close-button", disabled: busy(), onclick: move |_| status_target.set(None), "×" }
                            }
                            div { class: "drawer-body stack-lg",
                                div { class: "form-section",
                                    strong { if disabling { "Block traffic for {label}?" } else { "Allow traffic for {label}?" } }
                                    p { class: "small muted", if disabling { "Requests using this credential record will stop authenticating immediately." } else { "This re-enables the credential record. Spend and IP restrictions still apply independently." } }
                                }
                                if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }
                                div { class: "row customer-form-actions",
                                    button { class: "button button-secondary", disabled: busy(), onclick: move |_| status_target.set(None), "Cancel" }
                                    button {
                                        class: if disabling { "button button-danger" } else { "button button-primary" },
                                        disabled: busy(),
                                        onclick: move |_| {
                                            let reference = reference.clone();
                                            busy.set(true);
                                            error.set(String::new());
                                            spawn(async move {
                                                match TokenService::set_status(&reference, next_status).await {
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
                                        if busy() { "Saving…" } else if disabling { "Disable API Key" } else { "Enable API Key" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(target) = network_target() {
                {
                    let reference = target.token.clone();
                    let had_restriction = !ip_entries(target.ip_whitelist.as_deref()).is_empty();
                    let removing_restriction = had_restriction && whitelist().trim().is_empty();
                    rsx! {
                        div { class: "drawer-backdrop", onclick: move |_| if !busy() { network_target.set(None) } }
                        aside { class: "drawer",
                            div { class: "drawer-head",
                                div { h2 { "Network Access" } p { class: "small muted", "Restrict this credential by exact client source address." } }
                                button { class: "close-button", disabled: busy(), onclick: move |_| network_target.set(None), "×" }
                            }
                            div { class: "drawer-body stack-lg",
                                div { class: "form-section",
                                    div { class: "form-section-head", strong { "Exact IP allowlist" } small { "Comma or newline separated IPv4/IPv6 addresses. CIDR is rejected because the current router only performs exact matching." } }
                                    textarea { class: "textarea mono", rows: "7", value: "{whitelist}", disabled: busy(), oninput: move |event| { whitelist.set(event.value()); confirm_unrestricted.set(false); } }
                                }
                                if removing_restriction {
                                    label { class: "confirm-row",
                                        input { r#type: "checkbox", checked: confirm_unrestricted(), onclick: move |_| confirm_unrestricted.set(!confirm_unrestricted()) }
                                        span { "I understand that saving an empty allowlist broadens this credential to any source IP." }
                                    }
                                }
                                if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }
                                div { class: "row customer-form-actions",
                                    button { class: "button button-secondary", disabled: busy(), onclick: move |_| network_target.set(None), "Cancel" }
                                    button {
                                        class: "button button-primary",
                                        disabled: busy() || (removing_restriction && !confirm_unrestricted()),
                                        onclick: move |_| {
                                            let rules = match normalize_ip_rules(&whitelist()) {
                                                Ok(value) => value,
                                                Err(message) => {
                                                    error.set(message);
                                                    return;
                                                }
                                            };
                                            let reference = reference.clone();
                                            busy.set(true);
                                            error.set(String::new());
                                            spawn(async move {
                                                match TokenService::set_ip_whitelist(&reference, &rules).await {
                                                    Ok(()) => {
                                                        notice.set(if rules.is_empty() { "IP restriction removed; this key now accepts any source IP.".to_string() } else { "IP allowlist saved.".to_string() });
                                                        network_target.set(None);
                                                        tokens_resource.restart();
                                                    }
                                                    Err(message) => error.set(format!("Network rule update failed: {message}")),
                                                }
                                                busy.set(false);
                                            });
                                        },
                                        if busy() { "Saving…" } else { "Save Network Rules" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(target) = rotate_target() {
                {
                    let reference = target.token.clone();
                    let label = short_management_ref(&reference);
                    rsx! {
                        div { class: "drawer-backdrop", onclick: move |_| if !busy() { rotate_target.set(None) } }
                        aside { class: "drawer",
                            div { class: "drawer-head",
                                h2 { "Rotate API Key" }
                                button { class: "close-button", disabled: busy(), onclick: move |_| rotate_target.set(None), "×" }
                            }
                            div { class: "drawer-body stack-lg",
                                div { class: "form-section",
                                    strong { "Create a new secret for {label}?" }
                                    p { class: "small muted", "By default the previous secret remains valid for 24 hours so clients can move safely to the replacement." }
                                }
                                label { class: "confirm-row",
                                    input { r#type: "checkbox", checked: revoke_old_on_rotate(), onclick: move |_| revoke_old_on_rotate.set(!revoke_old_on_rotate()) }
                                    span { "Revoke the previous credential immediately instead of using the 24-hour transition." }
                                }
                                div { class: "product-note", "The replacement bearer secret is shown exactly once after rotation. The credential list continues to show only an opaque management reference." }
                                if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }
                                div { class: "row customer-form-actions",
                                    button { class: "button button-secondary", disabled: busy(), onclick: move |_| rotate_target.set(None), "Cancel" }
                                    button {
                                        class: "button button-primary",
                                        disabled: busy(),
                                        onclick: move |_| {
                                            let reference = reference.clone();
                                            let revoke_now = revoke_old_on_rotate();
                                            busy.set(true);
                                            error.set(String::new());
                                            spawn(async move {
                                                match TokenService::rotate(&reference, 24, revoke_now).await {
                                                    Ok(value) => {
                                                        let secret = value.get("new_token").and_then(|item| item.as_str()).unwrap_or_default().to_string();
                                                        let version = value.get("key_version").and_then(|item| item.as_i64()).unwrap_or_default();
                                                        let transition = value.get("transition_ends_at").and_then(|item| item.as_i64()).unwrap_or_default();
                                                        if secret.trim().is_empty() {
                                                            error.set("Rotation completed without a replacement bearer credential. Treat the result as ambiguous and refresh before retrying.".to_string());
                                                        } else {
                                                            let detail = if transition > 0 {
                                                                format!("Version {version} is now current. The previous bearer credential remains valid during the 24-hour transition unless it is revoked server-side sooner.")
                                                            } else {
                                                                format!("Version {version} is now current and the previous bearer credential was revoked immediately.")
                                                            };
                                                            secret_reveal.set(Some(SecretReveal { title: "API key rotated".to_string(), secret, detail }));
                                                            notice.set("API key rotated. Save the replacement credential before continuing.".to_string());
                                                            rotate_target.set(None);
                                                            tokens_resource.restart();
                                                        }
                                                    }
                                                    Err(message) => error.set(format!("Rotation failed: {message}")),
                                                }
                                                busy.set(false);
                                            });
                                        },
                                        if busy() { "Rotating…" } else { "Rotate API Key" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(target) = delete_target() {
                {
                    let reference = target.token.clone();
                    let label = short_management_ref(&reference);
                    let owner = owners.get(&target.user_id).map(|value| value.username.clone()).unwrap_or_else(|| target.user_id.clone());
                    rsx! {
                        div { class: "drawer-backdrop", onclick: move |_| if !busy() { delete_target.set(None) } }
                        aside { class: "drawer",
                            div { class: "drawer-head",
                                h2 { class: "danger", "Delete API Key" }
                                button { class: "close-button", disabled: busy(), onclick: move |_| delete_target.set(None), "×" }
                            }
                            div { class: "drawer-body stack-lg",
                                div { class: "form-section danger-zone",
                                    strong { "Delete {label}?" }
                                    p { class: "small muted", "This permanently removes the credential record owned by {owner}. Clients using its current or transition credential will stop authenticating." }
                                }
                                if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }
                                div { class: "row customer-form-actions",
                                    button { class: "button button-secondary", disabled: busy(), onclick: move |_| delete_target.set(None), "Cancel" }
                                    button {
                                        class: "button button-danger",
                                        disabled: busy(),
                                        onclick: move |_| {
                                            let reference = reference.clone();
                                            busy.set(true);
                                            error.set(String::new());
                                            spawn(async move {
                                                match TokenService::delete(&reference).await {
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
