use burncloud_client_shared::components::{
    BCBadge, BCButton, BCInput, BCModal, BadgeVariant, ButtonVariant,
};
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
    let mut topup_amount = use_signal(|| 0.0);
    let mut is_loading = use_signal(|| false);
    let toast = use_toast();

    // Mock Stats
    let total_users = 1248;
    let active_today = 842;
    let total_balance_held = "¥ 452,000.00";

    let handle_confirm_topup = move |_| {
        is_loading.set(true);
        let amount_dollars = *topup_amount.read();
        let amount_nano = burncloud_common::dollars_to_nano(amount_dollars);
        spawn(async move {
            match UserService::topup(&selected_user_id(), amount_nano, Some("CNY")).await {
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
                // Total Users
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
                // Active Users
                div { class: "p-lg bc-card-solid flex flex-col gap-xs",
                    span { class: "text-xxs font-semibold uppercase tracking-wider text-tertiary", "今日活跃" }
                    div { class: "flex items-baseline gap-sm",
                        span { class: "text-3xl font-bold text-primary tracking-tight", "{active_today}" }
                        span { class: "text-xs font-medium text-tertiary", "67% 活跃率" }
                    }
                }
                // Total Funds
                div { class: "p-lg bc-card-solid flex flex-col gap-xs",
                    span { class: "text-xxs font-semibold uppercase tracking-wider text-tertiary", "用户资金池" }
                    div { class: "flex items-baseline gap-sm",
                        span { class: "text-3xl font-bold text-primary tracking-tight", "{total_balance_held}" }
                    }
                }
            }

            // Client Table
            div { class: "flex flex-col gap-md",
                div { class: "flex items-center justify-between border-b pb-sm",
                    h3 { class: "text-caption font-medium text-secondary", "客户明细" }
                    // Tabs
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
                        button {
                            class: format!("text-caption font-medium transition-colors pb-sm border-b-2 -mb-sm {}",
                                if *active_tab.read() == "new" { "text-primary" } else { "text-tertiary border-transparent" }),
                            style: if *active_tab.read() == "new" { "border-color: var(--bc-text-primary);" } else { "border-color: transparent;" },
                            onclick: move |_| active_tab.set("new".to_string()),
                            "新注册"
                        }
                        button {
                            class: format!("text-caption font-medium transition-colors pb-sm border-b-2 -mb-sm {}",
                                if *active_tab.read() == "churn" { "text-primary" } else { "text-tertiary border-transparent" }),
                            style: if *active_tab.read() == "churn" { "border-color: var(--bc-text-primary);" } else { "border-color: transparent;" },
                            onclick: move |_| active_tab.set("churn".to_string()),
                            "流失预警"
                        }
                    }
                }

                div { class: "overflow-x-auto bc-card-solid",
                    table { class: "table w-full text-caption",
                        thead {
                            style: "background: var(--bc-bg-hover);",
                            tr {
                                th { class: "font-medium text-secondary", "客户信息" }
                                th { class: "font-medium text-secondary", "角色 / 分组" }
                                th { class: "font-medium text-secondary", "账户余额" }
                                th { class: "font-medium text-secondary", "历史消费 (LTV)" }
                                th { class: "font-medium text-secondary", "最后活跃" }
                                th { class: "font-medium text-secondary", "状态" }
                                th { class: "text-right font-medium text-secondary", "操作" }
                            }
                        }
                        tbody {
                            match users_data {
                                Some(list) if !list.is_empty() => rsx! {
                                    for user in list {
                                        tr { class: "transition-colors group",
                                            style: "cursor: default;",
                                            td {
                                                div { class: "flex items-center gap-md",
                                                    div { class: "w-9 h-9 rounded-full flex items-center justify-center text-white font-bold text-caption",
                                                        style: "background: linear-gradient(135deg, var(--bc-primary), var(--bc-primary-dark)); box-shadow: var(--bc-shadow-xs);",
                                                        "{user.username.chars().next().unwrap_or('?')}"
                                                    }
                                                    div { class: "flex flex-col",
                                                        span { class: "font-semibold text-primary", "{user.username}" }
                                                        span { class: "text-xxs text-tertiary", "ID: {user.id}" }
                                                    }
                                                }
                                            }
                                            td {
                                                div { class: "flex flex-col gap-xs",
                                                    span { class: "text-xxs font-medium px-sm py-0.5 rounded w-fit text-secondary",
                                                        style: "background: var(--bc-bg-hover);",
                                                        "{user.role}"
                                                    }
                                                    span { class: "text-xxs text-tertiary", "Group: {user.group}" }
                                                }
                                            }
                                            td { class: "font-mono font-medium", style: "color: var(--bc-success);", "¥ {nano_to_dollars(user.balance_cny):.2}" }
                                            td { class: "font-mono text-secondary", "¥ 1,240.00" } // Mock LTV
                                            td { class: "text-xxs text-secondary", "2 mins ago" }   // Mock Last Seen
                                            td {
                                                if user.status == 1 {
                                                    BCBadge { variant: BadgeVariant::Success, dot: true, "正常" }
                                                } else {
                                                    BCBadge { variant: BadgeVariant::Neutral, dot: true, "已禁用" }
                                                }
                                            }
                                            td { class: "text-right",
                                                div { class: "flex justify-end gap-sm",
                                                    button {
                                                        class: "btn btn-xs btn-neutral text-white",
                                                        onclick: move |_| {
                                                            selected_user_id.set(user.id.clone());
                                                            selected_username.set(user.username.clone());
                                                            topup_amount.set(0.0);
                                                            is_topup_open.set(true);
                                                        },
                                                        "充值"
                                                    }
                                                    button {
                                                        class: "btn btn-xs btn-ghost text-tertiary group-hover:text-primary transition-colors",
                                                        "管理"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                },
                                Some(_) => rsx! { tr { td { colspan: "7", class: "p-xl text-center text-tertiary", "暂无客户数据" } } },
                                None => rsx! { tr { td { colspan: "7", class: "p-xl text-center text-tertiary", "加载客户列表中..." } } }
                            }
                        }
                    }
                }
            }

            // Topup Modal
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

                    BCInput {
                        label: Some("充值金额 (¥)".to_string()),
                        value: "{topup_amount}",
                        placeholder: "0.00".to_string(),
                        oninput: move |e: FormEvent| topup_amount.set(e.value().parse().unwrap_or(0.0))
                    }

                    div { class: "flex gap-sm",
                        button { class: "btn btn-xs btn-outline flex-1", onclick: move |_| topup_amount.set(100.0), "¥100" }
                        button { class: "btn btn-xs btn-outline flex-1", onclick: move |_| topup_amount.set(500.0), "¥500" }
                        button { class: "btn btn-xs btn-outline flex-1", onclick: move |_| topup_amount.set(1000.0), "¥1000" }
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
                        onclick: handle_confirm_topup,
                        "确认充值"
                    }
                }
            }
        }
    }
}
