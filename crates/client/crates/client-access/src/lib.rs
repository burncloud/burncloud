// JSON Schema-driven UI — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::components::{
    BCButton, BCModal, ButtonVariant,
    FormMode, PageHeader, SchemaForm, StatusPill,
    EmptyState, SkeletonCard, SkeletonVariant,
};
use burncloud_client_shared::i18n::{t, t_fmt};
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
    let i18n = burncloud_client_shared::i18n::use_i18n();
    let lang = i18n.language;
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
                Ok(_key) => {
                    new_full_key.set(_key);
                    show_create.set(false);
                    show_result.set(true);
                    tokens.restart();
                    form_data.set(serde_json::json!({}));
                    toast.success(t(*lang.read(), "access.token_created"));
                }
                Err(e) => toast.error(&t_fmt(*lang.read(), "access.create_failed", &[("error", &e.to_string())])),
            }
        });
    };

    let schema = token_schema();

    rsx! {
        PageHeader {
            title: t(*lang.read(), "access.title"),
            actions: rsx! {
                BCButton {
                    class: "btn-black",
                    onclick: move |_| show_create.set(true),
                    {t(*lang.read(), "access.create_new")}
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
                    title: t(*lang.read(), "access.empty_title").to_string(),
                    description: Some(t(*lang.read(), "access.empty_desc").to_string()),
                    cta: Some(rsx! {
                        BCButton {
                            class: "btn-primary",
                            onclick: move |_| show_create.set(true),
                            {t(*lang.read(), "access.create_first")}
                        }
                    }),
                }
            } else {
                div { style: "display:grid; gap:12px",
                    for tk in token_list {
                        {
                            let tk_id = tk.token.clone();
                            let tk_id_for_del = tk.token.clone();
                            let _tk_id_for_copy = tk.token.clone();
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
                                                        label: if tk.status == "active" { Some(t(*lang.read(), "access.status.active").to_string()) } else { Some(t(*lang.read(), "access.status.revoked").to_string()) },
                                                    }
                                                }
                                                div { class: "mono", style: "display:flex; align-items:center; gap:16px; font-size:11px; color:var(--bc-text-tertiary); margin-top:4px",
                                                    span { style: "padding:2px 8px; border-radius:4px; background:var(--bc-bg-hover); color:var(--bc-text-secondary)", "{mask_key(&tk.token)}" }
                                                    if tk.quota_limit > 0 {
                                                        span { class: "pill neutral mono", style: "font-size:11px", "{format_quota(tk.quota_limit)}" }
                                                    }
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
            title: t(*lang.read(), "access.create_modal.title").to_string(),
            open: show_create(),
            onclose: move |_| show_create.set(false),

            div { class: "flex flex-col gap-lg p-lg",
                div { style: "font-size:12px; color:var(--bc-text-secondary); margin-top:4px", {t(*lang.read(), "access.create_modal.desc")} }

                SchemaForm {
                    schema: schema.clone(),
                    data: form_data,
                    mode: FormMode::Create,
                    show_actions: false,
                    on_submit: handle_create,
                }

                div { style: "display:flex; align-items:flex-start; gap:12px; padding:12px; background:var(--bc-warning-light, #fef3cd); border-radius:8px; color:var(--bc-warning); border:1px solid var(--bc-warning)",
                    span { style: "font-size:12px; line-height:1.5", {t(*lang.read(), "access.create_modal.warning")} }
                }

                div { class: "flex justify-end gap-md mt-md",
                    BCButton {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| show_create.set(false),
                        {t(*lang.read(), "common.cancel")}
                    }
                    BCButton {
                        variant: ButtonVariant::Black,
                        onclick: move |_| {
                            let data = form_data.read().clone();
                            handle_create(data);
                        },
                        {t(*lang.read(), "access.create_modal.submit")}
                    }
                }
            }
        }

        // Key result modal
        BCModal {
            title: t(*lang.read(), "access.result_modal.title").to_string(),
            open: show_result(),
            onclose: move |_| show_result.set(false),

            div { style: "padding:24px 24px 16px; text-align:center",
                div { style: "width:64px; height:64px; margin:0 auto 12px; border-radius:99px; background:var(--bc-success-light, #d1fae5); color:var(--bc-success); display:flex; align-items:center; justify-content:center; font-size:28px",
                    "✓"
                }
                h3 { style: "font-size:22px; font-weight:700; margin:0", {t(*lang.read(), "access.result_modal.heading")} }
                p { style: "font-size:13px; color:var(--bc-text-secondary); margin:8px 0 16px; line-height:1.5",
                    {t(*lang.read(), "access.result_modal.copy_prompt_1")}
                    br {}
                    {t(*lang.read(), "access.result_modal.copy_prompt_2")}
                }

                div { style: "display:flex; align-items:center; justify-content:space-between; gap:12px; padding:12px 16px; background:var(--bc-bg-hover); border-radius:8px; font-family:var(--bc-font-mono); font-size:13px; word-break:break-all",
                    span { style: "user-select:all; text-align:left; flex:1", "{new_full_key}" }
                    button {
                        class: "btn-icon",
                        onclick: move |_| {
                            let key = new_full_key();
                            spawn(async move {
                                if let Ok(mut cb) = arboard::Clipboard::new() {
                                    if cb.set_text(&key).is_ok() {
                                        toast.success(t(*lang.read(), "access.result_modal.copied"));
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
                    {t(*lang.read(), "access.result_modal.saved")}
                }
            }
        }

        // Delete confirmation modal
        BCModal {
            title: t(*lang.read(), "access.delete_modal.title").to_string(),
            open: show_delete(),
            onclose: move |_| show_delete.set(false),

            div { style: "display:flex; flex-direction:column",
                div { style: "display:flex; align-items:center; gap:12px; padding:24px; background:var(--bc-danger-light, #fee2e2)",
                    div { style: "width:48px; height:48px; border-radius:99px; background:var(--bc-danger-light, #fee2e2); color:var(--bc-danger); display:flex; align-items:center; justify-content:center; font-size:20px",
                        "🛡"
                    }
                    div {
                        div { style: "font-size:17px; font-weight:700", {t(*lang.read(), "access.delete_modal.heading")} }
                        div { style: "font-size:13px; color:var(--bc-text-secondary); margin-top:2px", {t(*lang.read(), "access.delete_modal.cannot_undo")} }
                    }
                }
                div { style: "padding:16px 24px; font-size:13px; color:var(--bc-text-secondary); line-height:1.6",
                    {t(*lang.read(), "access.delete_modal.confirm_msg")}
                    br {}
                    {t(*lang.read(), "access.delete_modal.impact_prefix")}
                    span { style: "color:var(--bc-danger); font-weight:700", {t(*lang.read(), "access.delete_modal.impact_highlight")} }
                    {t(*lang.read(), "access.delete_modal.impact_suffix")}
                }
                div { class: "flex justify-end gap-md p-md",
                    BCButton {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| show_delete.set(false),
                        {t(*lang.read(), "common.cancel")}
                    }
                    BCButton {
                        class: "btn btn-danger",
                        onclick: move |_| {
                            let id = delete_token_id();
                            spawn(async move {
                                match TokenService::delete(&id).await {
                                    Ok(_) => {
                                        tokens.restart();
                                        toast.success(t(*lang.read(), "access.delete_modal.revoked"));
                                    }
                                    Err(e) => toast.error(&t_fmt(*lang.read(), "access.delete_modal.revoke_failed", &[("error", &e.to_string())])),
                                }
                            });
                            show_delete.set(false);
                        },
                        {t(*lang.read(), "access.delete_modal.confirm")}
                    }
                }
            }
        }
    }
}
