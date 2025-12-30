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
    let mut new_full_key = use_signal(|| String::new());

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

    let copy_key = move |_| {
        let text = new_full_key();
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                if let Err(e) = clipboard.set_text(text) {
                    toast.error(&format!("复制失败: {}", e));
                } else {
                    toast.success("已复制到剪贴板");
                }
            }
            Err(e) => toast.error(&format!("剪贴板不可用: {}", e)),
        }
    };

    let mut toggle_status = move |token: String| {
        spawn(async move {
            match TokenService::delete(&token).await {
                Ok(_) => {
                    toast.success("凭证已删除");
                    keys_resource.restart();
                }
                Err(e) => toast.error(&format!("删除失败: {}", e)),
            }
        });
    };

    rsx! {
        div { class: "flex flex-col h-full gap-8",
            // Header
            div { class: "flex justify-between items-end",
                div {
                    h1 { class: "text-2xl font-semibold text-base-content mb-1 tracking-tight", "访问凭证 (Keymaster)" }
                    p { class: "text-sm text-base-content/60 font-medium", "管理用于访问 BurnCloud 服务的 API 密钥" }
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
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11.536 17 9.229 17 9.229 14.771 9.229 17 6.914 17 4.607 17a2.001 2.001 0 01-1.996-1.854L2.61 7.427A6 6 0 019.229 4.607L11.536 7H15z" }
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
                        for (idx, key) in keys().into_iter().enumerate() {
                            div { class: "group relative flex items-center justify-between p-5 bg-base-100 rounded-xl border border-base-200 hover:border-base-300 transition-all duration-200",
                                // Left: Key Info
                                div { class: "flex items-start gap-4",
                                    div { class: "p-3 rounded-lg bg-base-200/50 text-base-content/70",
                                        svg { class: "w-6 h-6", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11.536 17 9.229 17 9.229 14.771 9.229 17 6.914 17 4.607 17a2.001 2.001 0 01-1.996-1.854L2.61 7.427A6 6 0 019.229 4.607L11.536 7H15z" }
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
                                        div { class: "flex items-center gap-3 text-sm font-mono text-base-content/60",
                                            span { "{key.masked_key}" }
                                            span { class: "text-base-content/30", "|" }
                                            span { "Created: {key.created_at}" }
                                            span { class: "text-base-content/30", "|" }
                                            span { "Expires: {key.expires_at}" }
                                        }
                                    }
                                }

                                // Right: Actions & Scopes
                                div { class: "flex items-center gap-6",

                                    // Quota display
                                    if let Some(quota) = &key.quota_limit {
                                         div { class: "text-right",
                                            div { class: "text-xs font-semibold text-base-content/40 uppercase tracking-wide", "预算上限" }
                                            div { class: "text-sm font-mono font-medium", "{quota}" }
                                         }
                                    }

                                    // Action Buttons
                                    div { class: "flex gap-2",
                                        button {
                                            class: "btn btn-sm btn-ghost text-base-content/40 hover:text-base-content",
                                            onclick: move |_| toggle_status(key.id.clone()),
                                            "删除"
                                        }
                                    }
                                }
                            }
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
                                    class: "btn btn-square btn-sm btn-ghost",
                                    onclick: copy_key,
                                    svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" }}
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
        }
    }
}
