use dioxus::prelude::*;

use crate::{
    backend::{use_auth, User, UserService},
    components::Icon,
};

fn is_console_admin_role(role: &str) -> bool {
    role.trim().eq_ignore_ascii_case("admin")
}

fn status_metadata(status: i32) -> String {
    if status == 1 {
        "Default (1)".to_string()
    } else {
        format!("Status {status}")
    }
}

#[component]
pub fn Team() -> Element {
    let auth = use_auth();
    let session = auth.user();
    let mut resource = use_resource(move || async move { UserService::list().await });
    let snapshot = resource.read().clone();
    let loading = snapshot.is_none();
    let load_error = snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let all_users = snapshot.and_then(Result::ok).unwrap_or_default();
    let admins: Vec<User> = all_users
        .iter()
        .filter(|user| is_console_admin_role(&user.role))
        .cloned()
        .collect();

    let session_username = session
        .as_ref()
        .map(|user| user.username.clone())
        .unwrap_or_else(|| "-".to_string());
    let session_user_id = session
        .as_ref()
        .map(|user| user.id.clone())
        .unwrap_or_else(|| "-".to_string());
    let session_roles = session
        .as_ref()
        .map(|user| user.roles.clone())
        .unwrap_or_default();
    let session_roles_text = if session_roles.is_empty() {
        "No roles returned".to_string()
    } else {
        session_roles.join(", ")
    };
    let session_is_admin = session_roles
        .iter()
        .any(|role| role.eq_ignore_ascii_case("admin"));

    let admin_count = admins.len();
    let status_attention_count = admins.iter().filter(|user| user.status != 1).count();
    let inventory_class = if admin_count > 0 && status_attention_count == 0 {
        "readiness-strip ready"
    } else {
        "readiness-strip blocked"
    };
    let inventory_title = if admin_count == 0 {
        "No Console administrators returned"
    } else if status_attention_count > 0 {
        "Administrator records need review"
    } else {
        "Console administrator inventory loaded"
    };
    let inventory_copy = if admin_count == 0 {
        "The account list did not return an effective admin role. BurnCloud currently grants Console administration only through the admin role."
            .to_string()
    } else if status_attention_count > 0 {
        format!(
            "{status_attention_count} administrator record(s) carry non-default account status metadata. The current login path does not prove that this flag blocks access, so Team does not label those accounts disabled."
        )
    } else {
        "Every administrator returned by the current account list uses the default account status metadata. Role membership remains read-only in this Console."
            .to_string()
    };

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Team" }
                    p { class: "page-subtitle", "Review identities that currently satisfy BurnCloud's Console administrator role. Customer accounts remain under Customers." }
                }
                button {
                    class: "button button-secondary",
                    disabled: loading,
                    onclick: move |_| resource.restart(),
                    if loading { "Refreshing…" } else { "Refresh" }
                }
            }

            if loading {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "users" } }
                        h3 { "Loading Console administrators" }
                        p { "Reading account-role summaries before showing administrator inventory or status conclusions." }
                    }
                }
            } else if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Team could not be loaded" }
                    p { class: "small muted", "The administrator inventory is unavailable, so Team will not infer membership from the current session alone." }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| resource.restart(), "Retry" }
                }
            } else {
                div { class: "metrics",
                    div { class: "card metric",
                        div { class: "metric-copy",
                            span { class: "metric-label", "Console Admins" }
                            span { class: "metric-value", "{admin_count}" }
                            span { class: "metric-note", "effective admin role" }
                        }
                        div { class: "metric-icon tone-gray", Icon { name: "users" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy",
                            span { class: "metric-label", "Current Session" }
                            span { class: "metric-value", if session_is_admin { "Admin" } else { "Not Admin" } }
                            span { class: "metric-note", "from authenticated roles" }
                        }
                        div { class: if session_is_admin { "metric-icon tone-green" } else { "metric-icon tone-gray" }, Icon { name: "shield" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy",
                            span { class: "metric-label", "Role Changes" }
                            span { class: "metric-value", "Read-only" }
                            span { class: "metric-note", "no mutation endpoint" }
                        }
                        div { class: "metric-icon tone-gray", Icon { name: "lock" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy",
                            span { class: "metric-label", "Status Metadata" }
                            span { class: "metric-value", "{status_attention_count}" }
                            span { class: "metric-note", "non-default records" }
                        }
                        div { class: if status_attention_count > 0 { "metric-icon tone-amber" } else { "metric-icon tone-gray" }, Icon { name: "activity" } }
                    }
                }

                div { class: "{inventory_class}",
                    span { class: "readiness-dot" }
                    div {
                        strong { "{inventory_title}" }
                        div { class: "small muted", "{inventory_copy}" }
                    }
                }

                div { class: "grid-2",
                    div { class: "card card-pad stack",
                        div { class: "product-section-head",
                            div {
                                h3 { "Your session" }
                                p { "The identity currently operating this Console." }
                            }
                        }
                        div { class: "receipt-row", label { "User" } strong { "{session_username}" } }
                        div { class: "receipt-row", label { "User ID" } strong { class: "mono", "{session_user_id}" } }
                        div { class: "receipt-row", label { "Authenticated roles" } strong { "{session_roles_text}" } }
                        div { class: "receipt-row",
                            label { "Console admin authorization" }
                            strong { if session_is_admin { "Granted by admin role" } else { "Not granted by admin role" } }
                        }
                    }
                    div { class: "card card-pad stack",
                        div { class: "product-section-head",
                            div {
                                h3 { "Role management" }
                                p { "Why Team is read-only today." }
                            }
                        }
                        p { class: "small muted", "BurnCloud's current Console authorization checks the admin role from the database. The server can read account roles, but it does not expose authenticated endpoints to invite an administrator, grant or revoke admin, or remove a Team member." }
                        div { class: "product-note", "Team is read-only until the server exposes explicit role-management endpoints with authorization and audit semantics." }
                    }
                }

                if admins.is_empty() {
                    div { class: "card product-empty",
                        div { class: "product-empty-inner",
                            div { class: "product-empty-icon", Icon { name: "users" } }
                            h3 { "No Console administrators in the loaded account list" }
                            p { "Team intentionally does not reinterpret root, owner, operator, enterprise, or other labels as Console admin access. The current authorization boundary recognizes admin." }
                        }
                    }
                } else {
                    div { class: "card table-card",
                        div { class: "card-pad product-section-head",
                            div {
                                h3 { "Console administrators" }
                                p { "Accounts whose effective role summary is admin. Membership is informational until role-management endpoints exist." }
                            }
                        }
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr {
                                    th { "Administrator" }
                                    th { "Email" }
                                    th { "Console Role" }
                                    th { "Account Status Metadata" }
                                } }
                                tbody {
                                    for user in admins {
                                        {
                                            let email = user.email.clone().unwrap_or_else(|| "No email".to_string());
                                            let status = status_metadata(user.status);
                                            let is_current = user.id == session_user_id;
                                            rsx! {
                                                tr { key: "{user.id}",
                                                    td {
                                                        div { class: "two-line",
                                                            strong { class: "table-primary", "{user.username}" }
                                                            small { class: "mono muted",
                                                                "{user.id}"
                                                                if is_current { " • Current session" }
                                                            }
                                                        }
                                                    }
                                                    td { "{email}" }
                                                    td { span { class: "badge badge-neutral", "admin" } }
                                                    td {
                                                        div { class: "two-line",
                                                            strong { "{status}" }
                                                            small { class: "muted", "Metadata only; Team does not infer login blocking." }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "card-pad product-note", "No Invite, Change Role, Remove Member, or Suspend controls are shown because the current server does not expose those management operations." }
                    }
                }
            }
        }
    }
}
