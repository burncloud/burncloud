// JSON Schema-driven UI — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::components::{
    ActionDef, ActionEvent, BCButton, BCModal, ButtonVariant, FormMode, SchemaForm, SchemaTable,
};
use burncloud_client_shared::schema::{topup_schema, user_schema};
use burncloud_client_shared::use_toast;
use burncloud_client_shared::user_service::UserService;
use burncloud_common::nano_to_dollars;
use dioxus::prelude::*;

#[component]
pub fn UserPage() -> Element {
    let mut users =
        use_resource(move || async move { UserService::list().await.unwrap_or(vec![]) });

    let mut is_topup_open = use_signal(|| false);
    let mut selected_user_id = use_signal(String::new);
    let mut selected_username = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let toast = use_toast();

    // Stats
    let total_users = 1248;
    let active_today = 842;
    let total_balance_held = "¥ 452,000.00";

    // Topup form data
    let mut topup_data = use_signal(|| serde_json::json!({"user_id": "", "amount": 0}));
    let topup_schema_val = topup_schema();

    let mut handle_topup_open = move |user_id: String, username: String| {
        selected_user_id.set(user_id.clone());
        selected_username.set(username);
        topup_data.set(serde_json::json!({"user_id": user_id, "amount": 0}));
        is_topup_open.set(true);
    };

    let mut handle_confirm_topup = move |value: serde_json::Value| {
        is_loading.set(true);
        let amount_dollars = value["amount"].as_f64().unwrap_or(0.0);
        let uid = selected_user_id();
        let amount_nano = burncloud_common::dollars_to_nano(amount_dollars);
        spawn(async move {
            match UserService::topup(&uid, amount_nano, Some("CNY")).await {
                Ok(new_balance_nano) => {
                    let new_balance = nano_to_dollars(new_balance_nano);
                    toast.success(&format!("充值成功，当前余额: ¥ {:.2}", new_balance));
                    is_topup_open.set(false);
                    users.restart();
                }
                Err(e) => toast.error(&format!("充值失败: {}", e)),
            }
            is_loading.set(false);
        });
    };

    let users_data = users.read().clone();
    let mut active_tab = use_signal(|| "all".to_string());

    // Convert users to serde_json::Value for SchemaTable
    let schema = user_schema();
    let is_loading_data = users_data.is_none();
    let table_data: Vec<serde_json::Value> = match users_data {
        Some(list) => list
            .iter()
            .map(|u| {
                serde_json::json!({
                    "id": u.id,
                    "username": u.username,
                    "role": u.role,
                    "balance_cny": format!("¥ {:.2}", nano_to_dollars(u.balance_cny)),
                    "group": u.group,
                    "status": u.status
                })
            })
            .collect(),
        None => vec![],
    };

    // Actions for user table
    let _uid_for_topup = selected_user_id();
    let _uname_for_topup = selected_username();
    let actions = vec![ActionDef {
        action_id: "topup".to_string(),
        label: "充值".to_string(),
        color: "var(--bc-primary)".to_string(),
    }];

    let handle_action = move |event: ActionEvent| {
        if event.action_id == "topup" {
            let uid = event.row["id"].as_str().unwrap_or("").to_string();
            let uname = event.row["username"].as_str().unwrap_or("").to_string();
            handle_topup_open(uid, uname);
        }
    };

    rsx! {
        div { class: "flex flex-col h-full gap-xl",
            // Header
            div { class: "flex justify-between items-end",
                div {
                    h1 { class: "text-title font-semibold text-primary mb-xs tracking-tight", "客户列表" }
                    p { class: "text-caption text-secondary font-medium", "用户增长与留存管理" }
                }
                BCButton {
                    class: "btn-neutral btn-sm px-lg text-white shadow-sm",
                    "邀请新用户"
                }
            }

            // Stats Bar
            div { class: "grid grid-cols-3 gap-lg",
                div { class: "p-lg bc-card-solid flex flex-col gap-xs",
                    span { class: "text-xxs font-semibold uppercase tracking-wider text-tertiary", "总用户数" }
                    div { class: "flex items-baseline gap-sm",
                        span { class: "text-3xl font-bold text-primary tracking-tight", "{total_users}" }
                        span { class: "text-xs font-medium px-sm py-0.5 rounded",
                            style: "color: var(--bc-success); background: var(--bc-success-light);",
                            "+24 This Week"
                        }
                    }
                }
                div { class: "p-lg bc-card-solid flex flex-col gap-xs",
                    span { class: "text-xxs font-semibold uppercase tracking-wider text-tertiary", "今日活跃" }
                    div { class: "flex items-baseline gap-sm",
                        span { class: "text-3xl font-bold text-primary tracking-tight", "{active_today}" }
                        span { class: "text-xs font-medium text-tertiary", "67% 活跃率" }
                    }
                }
                div { class: "p-lg bc-card-solid flex flex-col gap-xs",
                    span { class: "text-xxs font-semibold uppercase tracking-wider text-tertiary", "用户资金池" }
                    div { class: "flex items-baseline gap-sm",
                        span { class: "text-3xl font-bold text-primary tracking-tight", "{total_balance_held}" }
                    }
                }
            }

            // Client Table via SchemaTable
            div { class: "flex flex-col gap-md",
                div { class: "flex items-center justify-between border-b pb-sm",
                    h3 { class: "text-caption font-medium text-secondary", "客户明细" }
                    div { class: "flex gap-md",
                        button {
                            class: format!("text-caption font-medium transition-colors pb-sm border-b-2 -mb-sm {}",
                                if *active_tab.read() == "all" { "text-primary" } else { "text-tertiary border-transparent" }),
                            style: if *active_tab.read() == "all" { "border-color: var(--bc-text-primary);" } else { "border-color: transparent;" },
                            onclick: move |_| active_tab.set("all".to_string()),
                            "全部客户"
                        }
                        button {
                            class: format!("text-caption font-medium transition-colors pb-sm border-b-2 -mb-sm {}",
                                if *active_tab.read() == "vip" { "text-primary" } else { "text-tertiary border-transparent" }),
                            style: if *active_tab.read() == "vip" { "border-color: var(--bc-text-primary);" } else { "border-color: transparent;" },
                            onclick: move |_| active_tab.set("vip".to_string()),
                            "VIP客户"
                        }
                    }
                }

                div { class: "overflow-x-auto bc-card-solid",
                    SchemaTable {
                        schema: schema.clone(),
                        data: table_data,
                        loading: is_loading_data,
                        actions: actions,
                        on_action: handle_action,
                        on_row_click: move |_| {},
                    }
                }
            }

            // Topup Modal via SchemaForm
            BCModal {
                open: is_topup_open(),
                title: "账户充值".to_string(),
                onclose: move |_| is_topup_open.set(false),

                div { class: "flex flex-col gap-md py-sm",
                    div { class: "p-md rounded-lg flex items-center justify-between",
                        style: "background: var(--bc-bg-hover);",
                        span { class: "text-caption text-secondary", "目标账户" }
                        span { class: "font-semibold text-primary", "{selected_username}" }
                    }

                    SchemaForm {
                        schema: topup_schema_val.clone(),
                        data: topup_data,
                        mode: FormMode::Create,
                        show_actions: false,
                        on_submit: handle_confirm_topup,
                    }

                    div { class: "flex gap-sm",
                        button { class: "btn btn-xs btn-secondary flex-1",
                            onclick: move |_| {
                                let mut d = topup_data.write();
                                d.as_object_mut().map(|m| m.insert("amount".to_string(), serde_json::json!(100)));
                            },
                            "¥100"
                        }
                        button { class: "btn btn-xs btn-secondary flex-1",
                            onclick: move |_| {
                                let mut d = topup_data.write();
                                d.as_object_mut().map(|m| m.insert("amount".to_string(), serde_json::json!(500)));
                            },
                            "¥500"
                        }
                        button { class: "btn btn-xs btn-secondary flex-1",
                            onclick: move |_| {
                                let mut d = topup_data.write();
                                d.as_object_mut().map(|m| m.insert("amount".to_string(), serde_json::json!(1000)));
                            },
                            "¥1000"
                        }
                    }
                }

                div { class: "flex justify-end gap-md mt-xl",
                    BCButton {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| is_topup_open.set(false),
                        "取消"
                    }
                    BCButton {
                        class: "btn-neutral text-white px-lg",
                        loading: is_loading(),
                        onclick: move |_| {
                            let data = topup_data.read().clone();
                            handle_confirm_topup(data);
                        },
                        "确认充值"
                    }
                }
            }
        }
    }
}
