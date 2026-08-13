use std::collections::BTreeSet;

use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{use_auth, User, UserService},
    components::Icon,
    role_access::is_staff_role,
};

fn role_label(role: &str) -> String {
    let trimmed = role.trim();
    if trimmed.is_empty() {
        "Unknown role".to_string()
    } else {
        trimmed.to_string()
    }
}

#[component]
pub fn Team() -> Element {
    let auth = use_auth();
    let session = auth.user();
    let mut resource = use_resource(move || async move { UserService::list().await });

    let snapshot = resource.read().clone();
    let loading = snapshot.is_none();
    let load_error = snapshot.as_ref().and_then(|result| result.as_ref().err().cloned());
    let staff: Option<Vec<User>> = snapshot.clone().and_then(Result::ok).map(|users| {
        users
            .into_iter()
            .filter(|user| is_staff_role(&user.role))
            .collect()
    });

    let session_username = session
        .as_ref()
        .map(|user| user.username.clone())
        .unwrap_or_else(|| "Unknown session".to_string());
    let session_id = session
        .as_ref()
        .map(|user| user.id.clone())
        .unwrap_or_default();
    let session_roles = session
        .as_ref()
        .map(|user| user.roles.join(", "))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "No role claims returned".to_string());

    let operator_count = staff.as_ref().map(|users| users.len());
    let active_count = staff
        .as_ref()
        .map(|users| users.iter().filter(|user| user.status == 1).count());
    let disabled_count = staff
        .as_ref()
        .map(|users| users.iter().filter(|user| user.status != 1).count());
    let role_set: BTreeSet<String> = staff
        .as_ref()
        .map(|users| users.iter().map(|user| role_label(&user.role)).collect())
        .unwrap_or_default();
    let session_in_directory = staff
        .as_ref()
        .is_some_and(|users| users.iter().any(|user| user.id == session_id));

    let operator_text = operator_count.map(|value| value.to_string()).unwrap_or_else(|| "—".to_string());
    let active_text = active_count.map(|value| value.to_string()).unwrap_or_else(|| "—".to_string());
    let disabled_text = disabled_count.map(|value| value.to_string()).unwrap_or_else(|| "—".to_string());
    let roles_text = if staff.is_some() { role_set.len().to_string() } else { "—".to_string() };

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Team" }
                    p { class: "page-subtitle", "Review admin identities that can operate this BurnCloud environment and spot disabled or unexpected access." }
                }
                button { class: "button button-secondary", onclick: move |_| resource.restart(), "Refresh" }
            }

            div { class: "metrics",
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Admins" } span { class: "metric-value", "{operator_text}" } span { class: "metric-note", "accounts with the current admin role" } }
                    div { class: "metric-icon tone-blue", Icon { name: "users" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Active" } span { class: "metric-value", "{active_text}" } span { class: "metric-note", "currently enabled admin accounts" } }
                    div { class: "metric-icon tone-green", Icon { name: "activity" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Disabled" } span { class: "metric-value", "{disabled_text}" } span { class: "metric-note", "admin accounts not currently enabled" } }
                    div { class: "metric-icon tone-gray", Icon { name: "lock" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Roles Observed" } span { class: "metric-value", "{roles_text}" } span { class: "metric-note", "admin role labels returned here" } }
                    div { class: "metric-icon tone-purple", Icon { name: "shield" } }
                }
            }

            if let Some(message) = load_error.clone() {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Team directory could not be loaded" }
                    p { class: "small muted", "BurnCloud will not infer admin counts or access state when the account list is unavailable." }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary button-sm", onclick: move |_| resource.restart(), "Retry" }
                }
            }

            div { class: "grid-2",
                div { class: "card card-pad stack-lg",
                    div { class: "product-section-head",
                        div {
                            h3 { "Your session" }
                            p { "Confirm which identity is currently making administrative changes." }
                        }
                    }
                    div { class: "receipt-row", label { "Signed in as" } strong { "{session_username}" } }
                    div { class: "receipt-row", label { "Role claims" } strong { "{session_roles}" } }
                    details {
                        summary { class: "small strong", style: "cursor:pointer", "Technical identity" }
                        div { class: "receipt-row", style: "margin-top:12px", label { "User ID" } strong { class: "mono", if session_id.is_empty() { "-" } else { "{session_id}" } } }
                    }
                    if staff.is_some() && !session_id.is_empty() {
                        if session_in_directory {
                            div { class: "readiness-strip ready",
                                span { class: "readiness-dot" }
                                strong { "Current session appears in the admin directory" }
                            }
                        } else {
                            div { class: "readiness-strip blocked",
                                span { class: "readiness-dot" }
                                strong { "Current admin session is not present in the returned admin directory" }
                                span { class: "muted", "The session can still be authenticated, but the account list and session claims do not line up. Review server role data before relying on this directory." }
                            }
                        }
                    }
                }

                div { class: "card card-pad stack-lg",
                    div { class: "product-section-head",
                        div {
                            h3 { "Access control scope" }
                            p { "What this page can and cannot safely manage today." }
                        }
                    }
                    div { class: "product-note",
                        "This page is an access inventory, not a fake staff-management screen. The current persisted role model exposes admin and user roles, but the server does not expose explicit endpoints to invite admins, change roles, disable an admin account, or revoke admin access safely from here."
                    }
                    if !role_set.is_empty() {
                        div { class: "stack",
                            strong { class: "small", "Roles currently observed" }
                            div { class: "row gap-2", style: "flex-wrap:wrap",
                                for role in role_set.iter() {
                                    span { class: "badge badge-neutral", "{role}" }
                                }
                            }
                        }
                    }
                    div { class: "row gap-2", style: "flex-wrap:wrap",
                        Link { class: "button button-secondary button-sm", to: Route::Customers {}, "Customer accounts" }
                        Link { class: "button button-secondary button-sm", to: Route::APIKeys {}, "API access" }
                    }
                }
            }

            if loading {
                div { class: "card card-pad", "Loading admin directory…" }
            } else if load_error.is_none() {
                if let Some(users) = staff.clone() {
                    if users.is_empty() {
                        div { class: "card product-empty",
                            div { class: "product-empty-inner",
                                div { class: "product-empty-icon", Icon { name: "users" } }
                                h3 { "No admin-role accounts returned" }
                                p { "The account list did not return an admin-role account. This does not prove that no privileged session exists; compare it with the current session above and review server role data." }
                            }
                        }
                    } else {
                        div { class: "card table-card",
                            div { class: "card-pad product-section-head",
                                div {
                                    h3 { "Admin directory" }
                                    p { "Administrative identities are separated from customer accounts so access review stays focused." }
                                }
                                span { class: "small muted", "{users.len()} admins" }
                            }
                            div { class: "table-wrap",
                                table { class: "data-table",
                                    thead { tr { th { "Admin" } th { "Role" } th { "Status" } th { "Contact" } } }
                                    tbody {
                                        for user in users {
                                            {
                                                let email = user.email.clone().filter(|value| !value.trim().is_empty()).unwrap_or_else(|| "No email".to_string());
                                                let status = if user.status == 1 { "Active" } else { "Disabled" };
                                                let is_current = user.id == session_id;
                                                rsx! {
                                                    tr { key: "{user.id}",
                                                        td {
                                                            div { class: "two-line",
                                                                div { class: "row gap-2",
                                                                    strong { class: "table-primary", "{user.username}" }
                                                                    if is_current { span { class: "badge badge-brand", "Current session" } }
                                                                }
                                                                small { class: "mono muted", "{user.id}" }
                                                            }
                                                        }
                                                        td { span { class: "badge badge-neutral", "{user.role}" } }
                                                        td { span { class: if user.status == 1 { "badge badge-success" } else { "badge badge-neutral" }, "{status}" } }
                                                        td { class: "muted", "{email}" }
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
}