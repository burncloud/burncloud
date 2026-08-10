use dioxus::prelude::*;

use crate::{
    backend::{use_auth, TokenDto, TokenService, UserService},
    components::Icon,
};

fn masked(token: &str) -> String {
    if token.len() <= 16 { return "••••••••".to_string(); }
    format!("{}••••••••{}", &token[..8], &token[token.len()-6..])
}

#[component]
pub fn APIKeys() -> Element {
    let auth = use_auth();
    let default_user = auth.user().map(|u| u.id).unwrap_or_default();
    let mut resource = use_resource(move || async move { TokenService::list().await });
    let mut create_open = use_signal(|| false);
    let mut whitelist_target = use_signal(|| None::<TokenDto>);
    let mut user_id = use_signal(move || default_user);
    let mut quota = use_signal(String::new);
    let mut whitelist = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(String::new);
    let mut error = use_signal(String::new);

    let snapshot = resource.read().clone();
    let load_error = snapshot.as_ref().and_then(|r| r.as_ref().err().cloned());
    let list = snapshot.and_then(Result::ok).unwrap_or_default();
    let active = list.iter().filter(|t| t.status == "active").count();
    let total = list.len();

    rsx! {
        div { class:"page",
            div { class:"page-header",
                div { h2 { class:"page-title", "API Keys" } p { class:"page-subtitle", "Real BurnCloud router tokens: create, enable/disable, rotate, restrict by IP, and delete." } }
                div { class:"header-actions",
                    button { class:"button button-secondary", onclick:move |_| resource.restart(), "Refresh" }
                    button { class:"button button-primary", onclick:move |_| { error.set(String::new()); create_open.set(true); }, Icon { name:"plus" } "Create API Key" }
                }
            }
            div { class:"metrics",
                div { class:"card metric", div { class:"metric-copy", span { class:"metric-label", "Keys" } span { class:"metric-value", "{total}" } } div { class:"metric-icon tone-blue", Icon { name:"key" } } }
                div { class:"card metric", div { class:"metric-copy", span { class:"metric-label", "Active" } span { class:"metric-value", "{active}" } } div { class:"metric-icon tone-green", Icon { name:"activity" } } }
            }
            if !notice().is_empty() { div { class:"terminal auth-status", "{notice}" } }
            if !error().is_empty() { div { class:"terminal auth-status auth-status-error", "{error}" } }
            if let Some(message) = load_error {
                div { class:"card card-pad stack", strong { class:"danger", "Unable to load API keys" } code { class:"terminal", "{message}" } }
            } else {
                div { class:"card table-card",
                    if list.is_empty() { div { class:"card-pad small muted", "No API keys exist yet." } }
                    else {
                        div { class:"table-wrap",
                            table { class:"data-table",
                                thead { tr { th { "Key" } th { "User" } th { "Status" } th { class:"right", "Quota" } th { class:"right", "Used" } th { "IP Rules" } th { "Version" } th { "Actions" } } }
                                tbody {
                                    for item in list {
                                        {
                                            let row_key = item.token.clone();
                                            let token_toggle = item.token.clone();
                                            let token_rotate = item.token.clone();
                                            let token_delete = item.token.clone();
                                            let item_for_whitelist = item.clone();
                                            let label = masked(&item.token);
                                            let next_status = if item.status == "active" { "disabled" } else { "active" };
                                            let toggle_label = if item.status == "active" { "Disable" } else { "Enable" };
                                            let quota_text = if item.quota_limit < 0 { "Unlimited".to_string() } else { item.quota_limit.to_string() };
                                            let ip_text = item.ip_whitelist.clone().unwrap_or_else(|| "Any IP".to_string());
                                            rsx! {
                                                tr { key:"{row_key}",
                                                    td { class:"mono table-primary", "{label}" }
                                                    td { class:"mono muted", "{item.user_id}" }
                                                    td { span { class:if item.status == "active" { "badge badge-success" } else { "badge badge-neutral" }, "{item.status}" } }
                                                    td { class:"right tabular", "{quota_text}" }
                                                    td { class:"right tabular", "{item.used_quota}" }
                                                    td { class:"mono muted", "{ip_text}" }
                                                    td { class:"mono", "v{item.key_version}" }
                                                    td { div { class:"row gap-2", style:"flex-wrap:wrap",
                                                        button { class:"button button-ghost button-sm", disabled:busy(), onclick:move |_| {
                                                            let token = token_toggle.clone(); busy.set(true); error.set(String::new());
                                                            spawn(async move { let r = TokenService::set_status(&token, next_status).await; match r { Ok(()) => { notice.set(format!("Key status changed to {next_status}.")); resource.restart(); }, Err(e) => error.set(format!("Status update failed: {e}")) } busy.set(false); });
                                                        }, "{toggle_label}" }
                                                        button { class:"button button-ghost button-sm", disabled:busy(), onclick:move |_| {
                                                            let token = token_rotate.clone(); busy.set(true); error.set(String::new());
                                                            spawn(async move { let r = TokenService::rotate(&token, 24, false).await; match r { Ok(v) => { notice.set(format!("Rotation result: {v}")); resource.restart(); }, Err(e) => error.set(format!("Rotation failed: {e}")) } busy.set(false); });
                                                        }, "Rotate" }
                                                        button { class:"button button-ghost button-sm", onclick:move |_| { whitelist.set(item_for_whitelist.ip_whitelist.clone().unwrap_or_default()); whitelist_target.set(Some(item_for_whitelist.clone())); }, "IP Rules" }
                                                        button { class:"button button-ghost button-sm danger", disabled:busy(), onclick:move |_| {
                                                            let token = token_delete.clone(); busy.set(true); error.set(String::new());
                                                            spawn(async move { let r = TokenService::delete(&token).await; match r { Ok(()) => { notice.set("API key deleted.".to_string()); resource.restart(); }, Err(e) => error.set(format!("Delete failed: {e}")) } busy.set(false); });
                                                        }, "Delete" }
                                                    } }
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
                div { class:"drawer-backdrop", onclick:move |_| create_open.set(false) }
                aside { class:"drawer",
                    div { class:"drawer-head", h2 { "Create API Key" } button { class:"close-button", onclick:move |_| create_open.set(false), "×" } }
                    div { class:"drawer-body stack-lg",
                        div { class:"field", label { "User ID" } input { class:"input mono", value:"{user_id}", oninput:move |e| user_id.set(e.value()) } }
                        div { class:"field", label { "Quota (optional raw quota)" } input { class:"input", r#type:"number", value:"{quota}", oninput:move |e| quota.set(e.value()) } }
                        button { class:"button button-primary", disabled:busy(), onclick:move |_| {
                            let uid=user_id().trim().to_string(); let q=quota().trim().parse::<i64>().ok();
                            if uid.is_empty() { error.set("User ID is required.".to_string()); return; }
                            busy.set(true); error.set(String::new());
                            spawn(async move { match TokenService::create(&uid,q).await { Ok(token)=>{ notice.set(format!("Created API key: {token} — copy it now.")); create_open.set(false); resource.restart(); }, Err(e)=>error.set(format!("Create key failed: {e}")) } busy.set(false); });
                        }, if busy() { "Creating…" } else { "Create Key" } }
                    }
                }
            }
            if let Some(target) = whitelist_target() {
                div { class:"drawer-backdrop", onclick:move |_| whitelist_target.set(None) }
                aside { class:"drawer",
                    div { class:"drawer-head", h2 { "IP Whitelist" } button { class:"close-button", onclick:move |_| whitelist_target.set(None), "×" } }
                    div { class:"drawer-body stack-lg",
                        p { class:"small muted", "Empty means unrestricted. The value is sent directly to BurnCloud token IP whitelist configuration." }
                        textarea { class:"textarea mono", rows:"6", value:"{whitelist}", oninput:move |e| whitelist.set(e.value()) }
                        button { class:"button button-primary", disabled:busy(), onclick:move |_| {
                            let token=target.token.clone(); let rules=whitelist(); busy.set(true); error.set(String::new());
                            spawn(async move { match TokenService::set_ip_whitelist(&token,&rules).await { Ok(())=>{ notice.set("IP whitelist saved.".to_string()); whitelist_target.set(None); resource.restart(); }, Err(e)=>error.set(format!("Whitelist failed: {e}")) } busy.set(false); });
                        }, "Save IP Rules" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Team() -> Element {
    let auth=use_auth();
    let session=auth.user();
    let mut resource=use_resource(move || async move { UserService::list().await });
    let snapshot=resource.read().clone();
    let error=snapshot.as_ref().and_then(|r| r.as_ref().err().cloned());
    let users=snapshot.and_then(Result::ok).unwrap_or_default();
    let username=session.as_ref().map(|u|u.username.clone()).unwrap_or_else(||"-".to_string());
    let user_id=session.as_ref().map(|u|u.id.clone()).unwrap_or_else(||"-".to_string());
    let roles=session.as_ref().map(|u|u.roles.join(", ")).unwrap_or_else(||"-".to_string());
    rsx! {
        div { class:"page",
            div { class:"page-header", div { h2 { class:"page-title", "Team" } p { class:"page-subtitle", "Live BurnCloud accounts. Account creation is managed from Customers / Users." } } button { class:"button button-secondary", onclick:move |_| resource.restart(), "Refresh" } }
            div { class:"card card-pad stack", span { class:"section-label", "Current Session" } strong { "{username}" } code { "{user_id}" } span { class:"small muted", "Roles: {roles}" } }
            if let Some(message)=error { div { class:"terminal auth-status auth-status-error", "{message}" } }
            else { div { class:"card table-card", div { class:"table-wrap", table { class:"data-table",
                thead { tr { th { "Username" } th { "Email" } th { "Role" } th { "Group" } th { "Status" } } }
                tbody { for user in users { { let email=user.email.clone().unwrap_or_else(||"-".to_string()); let status=if user.status==1 {"Active"} else {"Disabled"}; rsx!{ tr { key:"{user.id}", td { class:"table-primary", "{user.username}" } td { "{email}" } td { "{user.role}" } td { "{user.group}" } td { span { class:if user.status==1 {"badge badge-success"} else {"badge badge-neutral"}, "{status}" } } } } } } }
            } } } }
        }
    }
}
