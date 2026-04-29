// JSON Schema-driven UI — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::components::{
    BCButton, BCModal, ButtonVariant,
    FormMode, PageHeader, SchemaForm, StatusPill,
    EmptyState, SkeletonCard, SkeletonVariant,
};
use burncloud_client_shared::schema::token_schema;
use burncloud_client_shared::services::token_service::TokenService;
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

fn mask_key(key: &str) -> String {
    if key.len() > 8 {
        format!("{}...{}", &key[..7], &key[key.len()-4..])
    } else {
        "********".to_string()
    }
}

#[component]
pub fn AccessPage() -> Element {
    let mut show_create = use_signal(|| false);
    let mut show_result = use_signal(|| false);
    let mut show_delete = use_signal(|| false);
    let mut delete_token_id = use_signal(String::new);
    let mut new_full_key = use_signal(String::new);
    let mut form_data = use_signal(|| serde_json::json!({}));
    let toast = use_toast();

    let mut tokens = use_resource(move || async move {
        TokenService::list().await.unwrap_or_default()
    });

    let token_list = tokens.read().clone().unwrap_or_default();
    let loading = tokens.read().is_none();

    let handle_create = move |value: serde_json::Value| {
        let name = value["name"].as_str().unwrap_or("").to_string();
        if name.is_empty() { return; }

        spawn(async move {
            match TokenService::create(&name, None).await {
                Ok(key) => {
                    new_full_key.set(key);
                    show_create.set(false);
                    show_result.set(true);
                    tokens.restart();
                    form_data.set(serde_json::json!({}));
                    toast.success("凭证已创建");
                }
                Err(e) => toast.error(&format!("创建失败: {}", e)),
            }
        });
    };

    let schema = token_schema();

    rsx! {
        PageHeader {
            title: "访问凭证",
            actions: rsx! {
                BCButton {
                    class: "btn-black",
                    onclick: move |_| show_create.set(true),
                    "创建新凭证"
                }
            },
        }

        div { class: "page-content", style: "display:flex; flex-direction:column; gap:16px",
            if loading {
                SkeletonCard { variant: Some(SkeletonVariant::Row) }
                SkeletonCard { variant: Some(SkeletonVariant::Row) }
                SkeletonCard { variant: Some(SkeletonVariant::Row) }
            } else if token_list.is_empty() {
                EmptyState {
                    icon: rsx! { span { style: "font-size:40px", "🔑" } },
                    title: "没有活跃的访问凭证".to_string(),
                    description: Some("创建您的第一个 API Key 以开始集成 BurnCloud 服务。".to_string()),
                    cta: Some(rsx! {
                        BCButton {
                            class: "btn-black",
                            onclick: move |_| show_create.set(true),
                            "创建凭证"
                        }
                    }),
                }
            } else {
                div { style: "display:grid; gap:12px",
                    for tk in token_list {
                        {
                            let tk_id = tk.token.clone();
                            let tk_id_for_del = tk.token.clone();
                            let tk_id_for_copy = tk.token.clone();
                            rsx! {
                                div { key: "{tk_id}", class: "row-card", style: "padding:20px",
                                    div { style: "display:flex; align-items:flex-start; justify-content:space-between; gap:16px",
                                        div { style: "display:flex; align-items:flex-start; gap:16px",
                                            div { style: "padding:12px; border-radius:8px; background:var(--bc-bg-hover); color:var(--bc-text-secondary); display:flex; align-items:center; justify-content:center",
                                                span { style: "font-size:20px", "🔑" }
                                            }
                                            div { style: "display:flex; flex-direction:column; gap:4px",
                                                div { style: "display:flex; align-items:center; gap:8px",
                                                    span { style: "font-size:16px; font-weight:700", "API Key" }
                                                    StatusPill {
                                                        value: if tk.status == "active" { "ok".to_string() } else { "neutral".to_string() },
                                                        label: if tk.status == "active" { Some("使用中".to_string()) } else { Some("已吊销".to_string()) },
                                                    }
                                                }
                                                div { class: "mono", style: "display:flex; align-items:center; gap:16px; font-size:11px; color:var(--bc-text-tertiary); margin-top:4px",
                                                    span { style: "padding:2px 8px; border-radius:4px; background:var(--bc-bg-hover); color:var(--bc-text-secondary)", "{mask_key(&tk.token)}" }
                                                }
                                            }
                                        }
                                        div { style: "display:flex; align-items:center; gap:4px",
                                            button {
                                                class: "btn-icon",
                                                style: "color:var(--bc-text-tertiary)",
                                                onclick: move |_| {
                                                    let id = tk_id_for_del.clone();
                                                    delete_token_id.set(id);
                                                    show_delete.set(true);
                                                },
                                                "🗑"
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

        // Create modal
        BCModal {
            title: "创建访问凭证".to_string(),
            open: show_create(),
            onclose: move |_| show_create.set(false),

            div { class: "flex flex-col gap-lg p-lg",
                SchemaForm {
                    schema: schema.clone(),
                    data: form_data,
                    mode: FormMode::Create,
                    show_actions: false,
                    on_submit: handle_create,
                }

                div { style: "display:flex; align-items:flex-start; gap:12px; padding:12px; background:var(--bc-warning-light, #fef3cd); border-radius:8px; color:var(--bc-warning); border:1px solid var(--bc-warning)",
                    span { style: "font-size:12px; line-height:1.5", "创建后，完整的 API Key 仅会显示一次，请务必立即妥善保存。" }
                }

                div { class: "flex justify-end gap-md mt-md",
                    BCButton {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| show_create.set(false),
                        "取消"
                    }
                    BCButton {
                        variant: ButtonVariant::Primary,
                        onclick: move |_| {
                            let data = form_data.read().clone();
                            handle_create(data);
                        },
                        "立即创建"
                    }
                }
            }
        }

        // Key result modal
        BCModal {
            title: "凭证已创建".to_string(),
            open: show_result(),
            onclose: move |_| show_result.set(false),

            div { style: "padding:24px 24px 16px; text-align:center",
                div { style: "width:64px; height:64px; margin:0 auto 12px; border-radius:99px; background:var(--bc-success-light, #d1fae5); color:var(--bc-success); display:flex; align-items:center; justify-content:center; font-size:28px",
                    "✓"
                }
                h3 { style: "font-size:22px; font-weight:700; margin:0", "凭证已创建" }
                p { style: "font-size:13px; color:var(--bc-text-secondary); margin:8px 0 16px; line-height:1.5",
                    "请复制并保存您的 Secret Key，"
                    br {}
                    "出于安全考虑，它将不会再次显示。"
                }

                div { style: "display:flex; align-items:center; justify-content:space-between; gap:12px; padding:12px 16px; background:var(--bc-bg-hover); border-radius:8px; font-family:var(--bc-font-mono); font-size:13px; word-break:break-all",
                    span { style: "user-select:all; text-align:left; flex:1", "{new_full_key}" }
                    button {
                        class: "btn-icon",
                        onclick: move |_| {
                            let key = new_full_key();
                            spawn(async move {
                                match arboard::Clipboard::new() {
                                    Ok(mut cb) => {
                                        if cb.set_text(&key).is_ok() {
                                            toast.success("已复制到剪贴板");
                                        }
                                    }
                                    _ => {}
                                }
                            });
                        },
                        "📋"
                    }
                }

                BCButton {
                    class: "btn-black width-full mt-md",
                    onclick: move |_| show_result.set(false),
                    "我已保存"
                }
            }
        }

        // Delete confirmation modal
        BCModal {
            title: "确认吊销".to_string(),
            open: show_delete(),
            onclose: move |_| show_delete.set(false),

            div { style: "display:flex; flex-direction:column",
                div { style: "display:flex; align-items:center; gap:12px; padding:24px; background:var(--bc-danger-light, #fee2e2)",
                    div { style: "width:48px; height:48px; border-radius:99px; background:var(--bc-danger-light, #fee2e2); color:var(--bc-danger); display:flex; align-items:center; justify-content:center; font-size:20px",
                        "🛡"
                    }
                    div {
                        div { style: "font-size:17px; font-weight:700", "确认吊销" }
                        div { style: "font-size:13px; color:var(--bc-text-secondary); margin-top:2px", "此操作无法撤销" }
                    }
                }
                div { style: "padding:16px 24px; font-size:13px; color:var(--bc-text-secondary); line-height:1.6",
                    "确定要吊销此访问凭证吗？"
                    br {}
                    "所有使用此凭证的应用将"
                    span { style: "color:var(--bc-danger); font-weight:700", "立即失去访问权限" }
                    "。"
                }
                div { class: "flex justify-end gap-md p-md",
                    BCButton {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| show_delete.set(false),
                        "取消"
                    }
                    BCButton {
                        class: "btn btn-danger",
                        onclick: move |_| {
                            let id = delete_token_id();
                            spawn(async move {
                                match TokenService::delete(&id).await {
                                    Ok(_) => {
                                        tokens.restart();
                                        toast.success("凭证已吊销");
                                    }
                                    Err(e) => toast.error(&format!("吊销失败: {}", e)),
                                }
                            });
                            show_delete.set(false);
                        },
                        "确认吊销"
                    }
                }
            }
        }
    }
}
