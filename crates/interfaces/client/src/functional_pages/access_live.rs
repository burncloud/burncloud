use dioxus::prelude::*;

use crate::{
    backend::{use_auth, User, UserService},
    components::Icon,
};

fn is_staff_role(role: &str) -> bool {
    matches!(
        role.trim().to_ascii_lowercase().as_str(),
        "root" | "admin" | "administrator" | "operator" | "owner"
    )
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

    let username = session
        .as_ref()
        .map(|user| user.username.clone())
        .unwrap_or_else(|| "-".to_string());
    let user_id = session
        .as_ref()
        .map(|user| user.id.clone())
        .unwrap_or_else(|| "-".to_string());
    let roles = session
        .as_ref()
        .map(|user| user.roles.join(", "))
        .unwrap_or_else(|| "-".to_string());

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
