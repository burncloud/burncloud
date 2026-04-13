// JSON Schema-driven UI — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::components::{
    BCBadge, BCButton, BadgeVariant, ButtonVariant, SchemaForm,
};
use burncloud_client_shared::schema::token_schema;
use burncloud_client_shared::{token_service::TokenService, use_auth, use_toast};
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
struct ApiKey {
    id: String,
    name: String,
    prefix: String,
    masked_key: String,
    status: &'static str,
    created_at: String,
    expires_at: String,
    quota_limit: Option<String>,
}

#[component]
pub fn AccessCredentialsPage() -> Element {
    let toast = use_toast();

    let mut show_create_modal = use_signal(|| false);
    let mut show_key_result_modal = use_signal(|| false);
    let mut new_full_key = use_signal(String::new);

    let auth = use_auth();
    let user = auth.current_user;

    let mut keys_resource =
        use_resource(move || async move { TokenService::list().await.unwrap_or_default() });

    let keys = use_memo(move || {
        keys_resource
            .read()
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|t| {
                let masked_key = if t.token.len() > 8 {
                    format!("{}...{}", &t.token[0..7], &t.token[t.token.len() - 4..])
                } else {
                    "********".to_string()
                };
                ApiKey {
                    id: t.token.clone(),
                    name: "API Key".to_string(),
                    prefix: "sk-burn".to_string(),
                    masked_key,
                    status: if t.status == "active" { "Active" } else { "Revoked" },
                    created_at: "N/A".to_string(),
                    expires_at: "Never".to_string(),
                    quota_limit: if t.quota_limit < 0 { None } else { Some(format!("${}", t.quota_limit)) },
                }
            })
            .collect::<Vec<_>>()
    });

    let handle_create_click = move |_| {
        show_create_modal.set(true);
    };

    // Schema for the token form
    let schema = token_schema();
    let mut form_data = use_signal(|| serde_json::json!({"user_id": "", "quota_limit": -1}));

    let handle_generate = move |value: serde_json::Value| {
        spawn(async move {
            let limit = value["quota_limit"].as_i64().and_then(|v| if v < 0 { None } else { Some(v) });

            let uid = user
                .read()
                .as_ref()
                .map(|u| u.id.clone())
                .unwrap_or_else(|| value["user_id"].as_str().unwrap_or("demo-user").to_string());

            match TokenService::create(&uid, limit).await {
                Ok(token) => {
                    new_full_key.set(token);
                    show_create_modal.set(false);
                    show_key_result_modal.set(true);
                    toast.success("访问凭证创建成功");
                    keys_resource.restart();
                }
                Err(e) => toast.error(&format!("创建失败: {}", e)),
            }
        });
    };

    let mut is_copied = use_signal(|| false);

    let copy_key = move |_| {
        let text = new_full_key();
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                if let Err(e) = clipboard.set_text(text) {
                    toast.error(&format!("复制失败: {}", e));
                } else {
                    is_copied.set(true);
                    spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        is_copied.set(false);
                    });
                }
            }
            Err(e) => toast.error(&format!("剪贴板不可用: {}", e)),
        }
    };

    // Delete Modal State
    let mut is_delete_modal_open = use_signal(|| false);
    let mut delete_token_id = use_signal(String::new);

    let mut open_delete_modal = move |token: String| {
        delete_token_id.set(token);
        is_delete_modal_open.set(true);
    };

    let handle_confirm_delete = move |_| {
        spawn(async move {
            let token = delete_token_id();
            match TokenService::delete(&token).await {
                Ok(_) => {
                    toast.success("凭证已删除");
                    is_delete_modal_open.set(false);
                    keys_resource.restart();
                }
                Err(e) => toast.error(&format!("删除失败: {}", e)),
            }
        });
    };

    let render_status_icon = |status: &str| {
        if status == "Active" {
            rsx! {
                svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z" }
                }
            }
        } else {
            rsx! {
                svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" }
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M21 12a9 9 0 11-18 0 9 9 0 0118 0z" }
                }
            }
        }
    };

    rsx! {
        div { class: "flex flex-col h-full gap-xl",
            // Header
            div { class: "flex justify-between items-end",
                div {
                    h1 { class: "text-title font-bold text-primary mb-xs tracking-tight", "访问凭证" }
                }
                BCButton {
                    class: "btn-neutral btn-sm px-lg text-white shadow-sm",
                    onclick: handle_create_click,
                    "创建新凭证"
                }
            }

            // Key List (card-based, custom UI)
            div { class: "flex-1 overflow-y-auto min-h-0",
                if keys.read().is_empty() {
                    div { class: "flex flex-col items-center justify-center h-full text-center pb-xxl",
                        div { class: "p-lg rounded-full", style: "background: var(--bc-bg-hover);",
                            svg { class: "w-12 h-12", style: "color: var(--bc-text-disabled);", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" }
                            }
                        }
                        h3 { class: "text-title font-bold text-primary mb-sm", "没有活跃的访问凭证" }
                        p { class: "text-body text-secondary max-w-sm mb-lg", "创建您的第一个 API Key 以开始集成 BurnCloud 服务。" }
                        BCButton {
                            variant: ButtonVariant::Primary,
                            onclick: handle_create_click,
                            "创建凭证"
                        }
                    }
                } else {
                    div { class: "grid gap-md",
                        {
                            keys().into_iter().map(|key| {
                                let status_id = key.id.clone();
                                let delete_id = key.id.clone();
                                let current_status = key.status;

                                rsx! {
                                    div { class: "bc-card-solid group relative flex items-center justify-between p-lg transition-all duration-200",
                                        style: "cursor: default;",
                                        // Left: Key Info
                                        div { class: "flex items-start gap-md",
                                            div { class: "p-md rounded-lg text-secondary",
                                                style: "background: var(--bc-bg-hover);",
                                                svg { class: "w-6 h-6", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" }
                                                }
                                            }
                                            div { class: "flex flex-col gap-xs",
                                                div { class: "flex items-center gap-sm",
                                                    span { class: "font-bold text-primary text-lg", "{key.name}" }
                                                    if key.status == "Active" {
                                                        BCBadge { variant: BadgeVariant::Success, dot: true, "使用中" }
                                                    } else {
                                                        BCBadge { variant: BadgeVariant::Neutral, dot: true, "已吊销" }
                                                    }
                                                }
                                                div { class: "flex items-center gap-md text-xs font-mono text-tertiary mt-xs",
                                                    span { class: "px-sm py-0.5 rounded text-secondary", style: "background: var(--bc-bg-hover);", "{key.masked_key}" }
                                                    span { "{key.created_at}" }
                                                    if key.expires_at != "Never" {
                                                        span { "Exp: {key.expires_at}" }
                                                    }
                                                }
                                            }
                                        }

                                        // Right: Actions
                                        div { class: "flex items-center gap-lg",
                                            if let Some(quota) = &key.quota_limit {
                                                 div { class: "badge badge-ghost font-mono text-xs", "{quota}" }
                                            }

                                            div { class: "flex gap-sm",
                                                button {
                                                    class: format!("btn btn-sm btn-ghost btn-square transition-colors {}",
                                                        if key.status == "Active" { "text-warning hover:bg-warning/10" } else { "text-success hover:bg-success/10" }
                                                    ),
                                                    title: if key.status == "Active" { "禁用凭证" } else { "启用凭证" },
                                                    onclick: move |_| {
                                                        let id = status_id.clone();
                                                        let status_val = current_status;
                                                        spawn(async move {
                                                            let new_status_str = if status_val == "Active" { "disabled" } else { "active" };
                                                            match TokenService::update_status(&id, new_status_str).await {
                                                                Ok(_) => {
                                                                    match new_status_str {
                                                                        "active" => toast.success("凭证已启用"),
                                                                        _ => toast.success("凭证已禁用"),
                                                                    }
                                                                    keys_resource.restart();
                                                                },
                                                                Err(e) => toast.error(&format!("操作失败: {}", e)),
                                                            }
                                                        });
                                                    },
                                                    {render_status_icon(key.status)}
                                                }

                                                button {
                                                    class: "btn btn-sm btn-ghost btn-square text-tertiary hover:text-error hover:bg-error/10 transition-colors",
                                                    onclick: move |_| open_delete_modal(delete_id.clone()),
                                                    svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            })
                        }
                    }
                }
            }

            // Create Modal - now uses SchemaForm from token_schema
            if show_create_modal() {
                div { class: "fixed inset-0 z-[9999] flex items-center justify-center p-0 sm:p-4",
                    div {
                        class: "absolute inset-0 transition-opacity",
                        style: "background: rgba(0,0,0,0.30); backdrop-filter: blur(5px); -webkit-backdrop-filter: blur(5px);",
                        onclick: move |_| show_create_modal.set(false)
                    }

                    div {
                        class: "relative w-full h-full sm:h-auto sm:max-h-[90vh] sm:max-w-lg flex flex-col overflow-hidden animate-scale-in pointer-events-auto overscroll-contain",
                        style: "background: var(--bc-bg-card-solid); border-radius: var(--bc-radius-lg); box-shadow: var(--bc-shadow-xl); border: 1px solid var(--bc-border);",
                        onclick: |e| e.stop_propagation(),

                        div { class: "flex justify-between items-center px-md py-sm sm:px-lg sm:py-md border-b shrink-0",
                            style: "background: var(--bc-bg-card-solid);",
                            div {
                                h3 { class: "text-subtitle font-bold text-primary tracking-tight", "创建访问凭证" }
                                p { class: "text-caption text-secondary font-medium hidden sm:block", "配置新的 API Key 以授权应用访问" }
                            }
                            button {
                                class: "btn btn-sm btn-circle btn-ghost text-secondary",
                                style: "background: transparent;",
                                onclick: move |_| show_create_modal.set(false),
                                "✕"
                            }
                        }

                        // SchemaForm body
                        div { class: "flex-1 overflow-y-auto p-md sm:p-lg min-h-0 overscroll-y-contain flex flex-col gap-md",
                            SchemaForm {
                                schema: schema.clone(),
                                data: form_data,
                                mode: burncloud_client_shared::components::FormMode::Create,
                                show_actions: false,
                            }

                            div { class: "alert alert-warning text-xs mt-sm py-sm",
                                style: "background: var(--bc-warning-light); border: 1px solid var(--bc-warning); color: var(--bc-warning);",
                                svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" }}
                                span { "创建后，完整的 API Key 仅会显示一次，请务必立即妥善保存。" }
                            }
                        }

                        div { class: "flex justify-end gap-md px-lg py-md border-t shrink-0",
                            style: "background: var(--bc-bg-hover);",
                            BCButton {
                                variant: ButtonVariant::Ghost,
                                onclick: move |_| show_create_modal.set(false),
                                "取消"
                            }
                            BCButton {
                                class: "btn-neutral text-white shadow-lg shadow-neutral/20",
                                onclick: move |_| {
                                    let data = form_data.read().clone();
                                    handle_generate(data);
                                },
                                "立即创建"
                            }
                        }
                    }
                }
            }

            // Key Result Modal (unchanged)
            if show_key_result_modal() {
                div { class: "fixed inset-0 z-[9999] flex items-center justify-center p-md",
                    div {
                        class: "absolute inset-0 transition-opacity",
                        style: "background: rgba(0,0,0,0.60); backdrop-filter: blur(8px);",
                        onclick: move |_| show_key_result_modal.set(false)
                    }
                    div { class: "relative w-full max-w-lg p-lg animate-scale-in",
                        style: "background: var(--bc-bg-card-solid); border-radius: var(--bc-radius-lg); box-shadow: var(--bc-shadow-xl);",
                        div { class: "flex flex-col items-center gap-md text-center",
                            div { class: "w-16 h-16 rounded-full flex items-center justify-center mb-sm",
                                style: "background: var(--bc-success-light);",
                                svg { class: "w-8 h-8", style: "color: var(--bc-success);", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M5 13l4 4L19 7" }
                                }
                            }
                            h3 { class: "text-2xl font-bold text-primary", "凭证已创建" }
                            p { class: "text-secondary", "请复制并保存您的 Secret Key，出于安全考虑，它将不会再次显示。" }

                            div { class: "w-full p-md rounded-lg flex items-center justify-between gap-md mt-md font-mono text-sm break-all",
                                style: "background: var(--bc-bg-hover);",
                                span { class: "text-primary select-all", "{new_full_key}" }
                                button {
                                    class: "btn btn-square btn-sm btn-ghost transition-all duration-200",
                                    class: if is_copied() { "text-success" } else { "" },
                                    style: if is_copied() { "background: var(--bc-success-light); transform: scale(1.1);" } else { "" },
                                    onclick: copy_key,
                                    if is_copied() {
                                        svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2.5", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M5 13l4 4L19 7" } }
                                    } else {
                                        svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" }}
                                    }
                                }
                            }

                            BCButton {
                                class: "btn-neutral w-full mt-md",
                                onclick: move |_| show_key_result_modal.set(false),
                                "我已保存"
                            }
                        }
                    }
                }
            }

            // Delete Confirmation Modal (unchanged)
            if is_delete_modal_open() {
                div { class: "fixed inset-0 z-[9999] flex items-center justify-center p-md",
                    div {
                        class: "absolute inset-0 transition-opacity",
                        style: "background: rgba(0,0,0,0.30); backdrop-filter: blur(5px); -webkit-backdrop-filter: blur(5px);",
                        onclick: move |_| is_delete_modal_open.set(false)
                    }

                    div {
                        class: "relative w-full max-w-md overflow-hidden animate-scale-in",
                        style: "background: var(--bc-bg-card-solid); border-radius: var(--bc-radius-lg); box-shadow: var(--bc-shadow-xl); border: 1px solid var(--bc-border);",
                        onclick: |e| e.stop_propagation(),

                        div { class: "flex items-center gap-md px-lg py-lg border-b",
                            style: "background: var(--bc-danger-light); border-color: var(--bc-danger-light);",
                            div { class: "w-12 h-12 rounded-full flex items-center justify-center",
                                style: "background: var(--bc-danger-light);",
                                svg { class: "w-6 h-6", style: "color: var(--bc-danger);", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" }
                                }
                            }
                            div { class: "flex-1",
                                h3 { class: "text-subtitle font-bold text-primary", "确认吊销" }
                                p { class: "text-body text-secondary mt-xs", "此操作无法撤销" }
                            }
                        }

                        div { class: "px-lg py-md",
                            p { class: "text-secondary",
                                "确定要吊销此访问凭证吗？"
                                br {}
                                "所有使用此凭证的应用将"
                                span { class: "font-bold", style: "color: var(--bc-danger);", "立即失去访问权限" }
                                "。"
                            }
                        }

                        div { class: "flex justify-end gap-md px-lg py-md border-t",
                            style: "background: var(--bc-bg-hover);",
                            BCButton {
                                variant: ButtonVariant::Ghost,
                                onclick: move |_| is_delete_modal_open.set(false),
                                "取消"
                            }
                            BCButton {
                                class: "btn-error text-white shadow-md",
                                onclick: handle_confirm_delete,
                                "确认吊销"
                            }
                        }
                    }
                }
            }
        }
    }
}
