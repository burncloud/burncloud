use burncloud_client_shared::components::{
    BCBadge, BCButton, BCInput, BadgeVariant, ButtonVariant,
};
use burncloud_client_shared::{token_service::TokenService, use_auth, use_toast};
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
struct ApiKey {
    id: String,
    name: String,
    prefix: String,
    masked_key: String,
    status: &'static str, // "Active", "Revoked", "Expired"
    created_at: String,
    expires_at: String,
    quota_limit: Option<String>,
}

#[component]
pub fn AccessCredentialsPage() -> Element {
    let toast = use_toast();

    // State management
    let mut show_create_modal = use_signal(|| false);
    let mut show_key_result_modal = use_signal(|| false);
    let mut new_full_key = use_signal(String::new);

    let auth = use_auth();
    let user = auth.current_user;

    // Resource for fetching keys
    let mut keys_resource =
        use_resource(move || async move { TokenService::list().await.unwrap_or_default() });

    // Computed keys for UI (mapping logic)
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
                    status: if t.status == "active" {
                        "Active"
                    } else {
                        "Revoked"
                    },
                    created_at: "N/A".to_string(),
                    expires_at: "Never".to_string(),
                    quota_limit: if t.quota_limit < 0 {
                        None
                    } else {
                        Some(format!("${}", t.quota_limit))
                    },
                }
            })
            .collect::<Vec<_>>()
    });

    // Form State
    let mut form_name = use_signal(String::new);
    let mut form_expiry = use_signal(|| "Never".to_string());
    let mut form_quota = use_signal(String::new);

    let handle_create_click = move |_| {
        form_name.set(String::new());
        form_expiry.set("Never".to_string());
        form_quota.set(String::new());
        show_create_modal.set(true);
    };

    let handle_generate = move |_| {
        spawn(async move {
            let limit = if form_quota().is_empty() {
                None
            } else {
                let s = form_quota().replace("$", "");
                s.parse::<i64>().ok()
            };

            let uid = user
                .read()
                .as_ref()
                .map(|u| u.id.clone())
                .unwrap_or("demo-user".to_string());

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
                    // toast.success("已复制到剪贴板"); // Removed according to Jobs: redundant if UI reacts
                    is_copied.set(true);

                    // Reset after 2 seconds
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
        div { class: "flex flex-col h-full gap-8",
            // Header
            div { class: "flex justify-between items-end",
                div {
                    h1 { class: "text-2xl font-bold text-base-content mb-1 tracking-tight", "访问凭证" }
                    // Removed verbose description
                }
                BCButton {
                    class: "btn-neutral btn-sm px-6 text-white shadow-sm",
                    onclick: handle_create_click,
                    "创建新凭证"
                }
            }

            // Key List
            div { class: "flex-1 overflow-y-auto min-h-0",
                if keys.read().is_empty() {
                    div { class: "flex flex-col items-center justify-center h-full text-center pb-20",
                        div { class: "p-6 rounded-full bg-base-200/50 mb-6",
                            svg { class: "w-12 h-12 text-base-content/20", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" }
                            }
                        }
                        h3 { class: "text-xl font-bold text-base-content mb-2", "没有活跃的访问凭证" }
                        p { class: "text-base text-base-content/60 max-w-sm mb-6", "创建您的第一个 API Key 以开始集成 BurnCloud 服务。" }
                        BCButton {
                            variant: ButtonVariant::Primary,
                            onclick: handle_create_click,
                            "创建凭证"
                        }
                    }
                } else {
                    div { class: "grid gap-4",
                        {
                            keys().into_iter().map(|key| {
                                // Clone IDs for closures to avoid move errors
                                let status_id = key.id.clone();
                                let delete_id = key.id.clone();
                                let current_status = key.status;

                                rsx! {
                                    div { class: "group relative flex items-center justify-between p-5 bg-base-100 rounded-xl border border-base-200 hover:border-base-300 transition-all duration-200",
                                        // Left: Key Info
                                        div { class: "flex items-start gap-4",
                                            div { class: "p-3 rounded-lg bg-base-200/50 text-base-content/70",
                                                svg { class: "w-6 h-6", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" }
                                                }
                                            }
                                            div { class: "flex flex-col gap-1",
                                                div { class: "flex items-center gap-2",
                                                    span { class: "font-bold text-base-content text-lg", "{key.name}" }
                                                    if key.status == "Active" {
                                                        BCBadge { variant: BadgeVariant::Success, dot: true, "使用中" }
                                                    } else {
                                                        BCBadge { variant: BadgeVariant::Neutral, dot: true, "已吊销" }
                                                    }
                                                }
                                                div { class: "flex items-center gap-4 text-xs font-mono text-base-content/40 mt-1",
                                                    span { class: "bg-base-200/50 px-1.5 py-0.5 rounded text-base-content/70", "{key.masked_key}" }
                                                    span { "{key.created_at}" }
                                                    if key.expires_at != "Never" {
                                                        span { "Exp: {key.expires_at}" }
                                                    }
                                                }
                                            }
                                        }

                                        // Right: Actions & Scopes
                                        div { class: "flex items-center gap-6",

                                            // Quota display - Minimalist
                                            if let Some(quota) = &key.quota_limit {
                                                 div { class: "badge badge-ghost font-mono text-xs", "{quota}" }
                                            }

                                            // Action Buttons
                                            div { class: "flex gap-2",
                                                // Status Toggle Button
                                                button {
                                                    class: format!("btn btn-sm btn-ghost btn-square transition-colors {}",
                                                        if key.status == "Active" { "text-warning hover:bg-warning/10" } else { "text-success hover:bg-success/10" }
                                                    ),
                                                    title: if key.status == "Active" { "禁用凭证" } else { "启用凭证" },
                                                    onclick: move |_| {
                                                        // Capture the cloned values
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
                                                    class: "btn btn-sm btn-ghost btn-square text-base-content/40 hover:text-error hover:bg-error/10 transition-colors",
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

            // Custom Create Modal
            if show_create_modal() {
                div { class: "fixed inset-0 z-[9999] flex items-center justify-center p-0 sm:p-4",
                    // Backdrop
                    div {
                        class: "absolute inset-0 bg-black/30 transition-opacity",
                        style: "backdrop-filter: blur(5px); -webkit-backdrop-filter: blur(5px);",
                        onclick: move |_| show_create_modal.set(false)
                    }

                    // Modal Content
                    div {
                        // Exact styles from models.rs for consistency
                        class: "relative w-full h-full sm:h-auto sm:max-h-[90vh] sm:max-w-lg bg-base-100 sm:rounded-2xl shadow-2xl border-0 sm:border border-base-200 flex flex-col overflow-hidden animate-[scale-in_0.2s_ease-out] pointer-events-auto overscroll-contain",
                        onclick: |e| e.stop_propagation(),

                        // Header
                        div { class: "flex justify-between items-center px-4 py-3 sm:px-6 sm:py-4 border-b border-base-200 shrink-0 bg-base-100",
                            div {
                                h3 { class: "text-lg font-bold text-base-content tracking-tight", "创建访问凭证" }
                                p { class: "text-xs text-base-content/60 font-medium hidden sm:block", "配置新的 API Key 以授权应用访问" }
                            }
                            button {
                                class: "btn btn-sm btn-circle btn-ghost text-base-content/50 hover:bg-base-200",
                                onclick: move |_| show_create_modal.set(false),
                                "✕"
                            }
                        }

                        // Body
                        div { class: "flex-1 overflow-y-auto p-4 sm:p-6 min-h-0 overscroll-y-contain flex flex-col gap-4",
                            BCInput {
                                label: Some("凭证名称 (Description)".to_string()),
                                value: "{form_name}",
                                placeholder: "e.g. My Chatbot Production".to_string(),
                                oninput: move |e: FormEvent| form_name.set(e.value())
                            }



                            div { class: "grid grid-cols-2 gap-4",
                                div { class: "flex flex-col gap-1.5",
                                    label { class: "text-sm font-medium text-base-content/80", "过期时间" }
                                    select { class: "select select-bordered w-full select-sm bg-base-100",
                                        value: "{form_expiry}",
                                        onchange: move |e: FormEvent| form_expiry.set(e.value()),
                                        option { value: "Never", "永不过期" }
                                        option { value: "30 Days", "30 天后" }
                                        option { value: "7 Days", "7 天后" }
                                        option { value: "1 Day", "24 小时后" }
                                    }
                                }
                                BCInput {
                                    label: Some("预算上限 (可选)".to_string()),
                                    value: "{form_quota}",
                                    placeholder: "e.g. $100.00".to_string(),
                                    oninput: move |e: FormEvent| form_quota.set(e.value())
                                }
                            }

                            div { class: "alert alert-warning text-xs mt-2 py-2 bg-warning/10 border-warning/20 text-warning-content",
                                svg { class: "w-4 h-4 text-warning", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" }}
                                span { "创建后，完整的 API Key 仅会显示一次，请务必立即妥善保存。" }
                            }
                        }

                        // Footer
                        div { class: "flex justify-end gap-3 px-6 py-4 border-t border-base-200 bg-base-50/50 shrink-0",
                            BCButton {
                                variant: ButtonVariant::Ghost,
                                onclick: move |_| show_create_modal.set(false),
                                "取消"
                            }
                            BCButton {
                                class: "btn-neutral text-white shadow-lg shadow-neutral/20",
                                onclick: handle_generate,
                                "立即创建"
                            }
                        }
                    }
                }
            }

            // Key Result Modal (Show full key)
            if show_key_result_modal() {
                div { class: "fixed inset-0 z-[9999] flex items-center justify-center p-4",
                    div {
                        class: "absolute inset-0 bg-black/60 transition-opacity backdrop-blur-sm",
                        onclick: move |_| show_key_result_modal.set(false)
                    }
                    div { class: "relative w-full max-w-lg bg-base-100 rounded-2xl shadow-2xl p-6 animate-[scale-in_0.2s_ease-out]",
                        div { class: "flex flex-col items-center gap-4 text-center",
                            div { class: "w-16 h-16 rounded-full bg-success/10 flex items-center justify-center mb-2",
                                svg { class: "w-8 h-8 text-success", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M5 13l4 4L19 7" }
                                }
                            }
                            h3 { class: "text-2xl font-bold", "凭证已创建" }
                            p { class: "text-base-content/60", "请复制并保存您的 Secret Key，出于安全考虑，它将不会再次显示。" }

                            div { class: "w-full p-4 bg-base-200 rounded-lg flex items-center justify-between gap-3 mt-4 font-mono text-sm break-all",
                                span { class: "text-base-content select-all", "{new_full_key}" }
                                button {
                                    class: "btn btn-square btn-sm btn-ghost transition-all duration-200",
                                    class: if is_copied() { "text-success bg-success/10 scale-110" } else { "" },
                                    onclick: copy_key,
                                    if is_copied() {
                                        svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2.5", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M5 13l4 4L19 7" } }
                                    } else {
                                        svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" }}
                                    }
                                }
                            }

                            BCButton {
                                class: "btn-neutral w-full mt-4",
                                onclick: move |_| show_key_result_modal.set(false),
                                "我已保存"
                            }
                        }
                    }
                }
            }

            // Delete Confirmation Modal
            if is_delete_modal_open() {
                div { class: "fixed inset-0 z-[9999] flex items-center justify-center p-4",
                    // Backdrop
                    div {
                        class: "absolute inset-0 bg-black/30 transition-opacity",
                        style: "backdrop-filter: blur(5px); -webkit-backdrop-filter: blur(5px);",
                        onclick: move |_| is_delete_modal_open.set(false)
                    }

                    // Modal Content
                    div {
                        class: "relative w-full max-w-md bg-base-100 rounded-2xl shadow-2xl border border-base-200 overflow-hidden animate-[scale-in_0.2s_ease-out]",
                        onclick: |e| e.stop_propagation(),

                        // Header with Warning Icon
                        div { class: "flex items-center gap-4 px-6 py-5 bg-error/5 border-b border-error/10",
                            div { class: "w-12 h-12 rounded-full bg-error/10 flex items-center justify-center",
                                svg { class: "w-6 h-6 text-error", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" }
                                }
                            }
                            div { class: "flex-1",
                                h3 { class: "text-lg font-bold text-base-content", "确认吊销" }
                                p { class: "text-sm text-base-content/60 mt-1", "此操作无法撤销" }
                            }
                        }

                        // Message
                        div { class: "px-6 py-4",
                            p { class: "text-base-content/80",
                                "确定要吊销此访问凭证吗？"
                                br {}
                                "所有使用此凭证的应用将"
                                span { class: "font-bold text-error", "立即失去访问权限" }
                                "。"
                            }
                        }

                        // Footer
                        div { class: "flex justify-end gap-3 px-6 py-4 bg-base-50/50 border-t border-base-200",
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
