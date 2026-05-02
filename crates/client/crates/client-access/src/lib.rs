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

fn format_quota(cents: i64) -> String {
    let dollars = cents / 100;
    format!("${dollars}")
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

        div { class: "page-content flex flex-col gap-lg",
            if loading {
                SkeletonCard { variant: Some(SkeletonVariant::Row) }
                SkeletonCard { variant: Some(SkeletonVariant::Row) }
                SkeletonCard { variant: Some(SkeletonVariant::Row) }
            } else if token_list.is_empty() {
                EmptyState {
                    icon: rsx! { span { class: "text-display", "🔑" } },
                    title: "没有活跃的访问凭证".to_string(),
                    description: Some("创建您的第一个 API Key 以开始集成 BurnCloud 服务。".to_string()),
                    cta: Some(rsx! {
                        BCButton {
                            class: "btn-primary",
                            onclick: move |_| show_create.set(true),
                            "创建凭证"
                        }
                    }),
                }
            } else {
                div { class: "grid gap-md",
                    for tk in token_list {
                        {
                            let tk_id = tk.token.clone();
                            let tk_id_for_del = tk.token.clone();
                            let _tk_id_for_copy = tk.token.clone();
                            rsx! {
                                div { key: "{tk_id}", class: "row-card p-xl",
                                    div { class: "flex items-start justify-between gap-lg",
                                        div { class: "flex items-start gap-lg",
                                            div { class: "bc-icon-box",
                                                span { class: "text-title", "🔑" }
                                            }
                                            div { class: "flex flex-col gap-xs",
                                                div { class: "flex items-center gap-sm",
                                                    span { class: "text-subtitle font-bold", "API Key" }
                                                    StatusPill {
                                                        value: if tk.status == "active" { "ok".to_string() } else { "neutral".to_string() },
                                                        label: if tk.status == "active" { Some("使用中".to_string()) } else { Some("已吊销".to_string()) },
                                                    }
                                                }
                                                div { class: "mono bc-key-row",
                                                    span { class: "bc-key-display", "{mask_key(&tk.token)}" }
                                                    if tk.quota_limit > 0 {
                                                        span { class: "pill neutral mono bc-text-11px", "{format_quota(tk.quota_limit)}" }
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "flex items-center gap-xs",
                                            button {
                                                class: "btn-icon text-tertiary",
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
                div { class: "bc-hint-text mt-xs", "配置新的 API Key 以授权应用访问" }

                SchemaForm {
                    schema: schema.clone(),
                    data: form_data,
                    mode: FormMode::Create,
                    show_actions: false,
                    on_submit: handle_create,
                }

                div { class: "bc-warning-banner",
                    span { class: "bc-hint-text", "创建后，完整的 API Key 仅会显示一次，请务必立即妥善保存。" }
                }

                div { class: "flex justify-end gap-md mt-md",
                    BCButton {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| show_create.set(false),
                        "取消"
                    }
                    BCButton {
                        variant: ButtonVariant::Black,
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

            div { class: "bc-result-modal-body",
                div { class: "bc-success-circle",
                    "✓"
                }
                h3 { class: "bc-heading-22px", "凭证已创建" }
                p { class: "bc-body-13px text-secondary",
                    "请复制并保存您的 Secret Key，"
                    br {}
                    "出于安全考虑，它将不会再次显示。"
                }

                div { class: "bc-key-result-box",
                    span { class: "bc-selectable flex-1 text-left", "{new_full_key}" }
                    button {
                        class: "btn-icon",
                        onclick: move |_| {
                            let key = new_full_key();
                            spawn(async move {
                                if let Ok(mut cb) = arboard::Clipboard::new() {
                                    if cb.set_text(&key).is_ok() {
                                        toast.success("已复制到剪贴板");
                                    }
                                }
                            });
                        },
                        "📋"
                    }
                }

                BCButton {
                    variant: ButtonVariant::Black,
                    class: "width-full mt-md",
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

            div { class: "flex flex-col",
                div { class: "bc-danger-banner",
                    div { class: "bc-danger-circle",
                        "🛡"
                    }
                    div {
                        div { class: "text-subtitle font-bold", "确认吊销" }
                        div { class: "bc-body-13px text-secondary mt-xs", "此操作无法撤销" }
                    }
                }
                div { class: "bc-delete-warning-body",
                    "确定要吊销此访问凭证吗？"
                    br {}
                    "所有使用此凭证的应用将"
                    span { class: "text-danger font-bold", "立即失去访问权限" }
                    "。"
                }
                div { class: "flex justify-end gap-md p-md",
                    BCButton {
                        variant: ButtonVariant::Ghost,
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
