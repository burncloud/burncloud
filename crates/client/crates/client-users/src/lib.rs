use burncloud_client_shared::components::{
    BCButton, BCInput, BCModal, ButtonVariant, PageHeader, StatusPill,
    EmptyState, SkeletonCard, SkeletonVariant,
};
use burncloud_client_shared::services::user_service::UserService;
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

fn format_nano_to_cny(nano: i64) -> String {
    let yuan = nano as f64 / 1_000_000_000.0;
    format!("¥ {yuan:.2}")
}

#[component]
pub fn UsersPage() -> Element {
    let mut active_tab = use_signal(|| "all".to_string());
    let mut show_topup = use_signal(|| None::<String>);
    let mut topup_amount = use_signal(|| 0i64);
    let mut topup_username = use_signal(String::new);
    let mut topup_loading = use_signal(|| false);
    let mut show_invite = use_signal(|| false);
    let mut invite_username = use_signal(String::new);
    let mut invite_password = use_signal(String::new);
    let mut invite_loading = use_signal(|| false);
    let toast = use_toast();

    let mut users = use_resource(move || async move {
        UserService::list().await.unwrap_or_default()
    });

    let user_list = users.read().clone().unwrap_or_default();
    let loading = users.read().is_none();

    let total = user_list.len();
    let active_count = user_list.iter().filter(|u| u.status == 1).count();

    let filtered: Vec<_> = user_list.iter().filter(|&u| {
        match active_tab().as_str() {
            "vip" => u.group == "VIP",
            _ => true,
        }
    }).cloned().collect();

    rsx! {
        PageHeader {
            title: "客户列表",
            subtitle: Some("用户增长与留存管理".to_string()),
            actions: rsx! {
                BCButton {
                    class: "btn-black",
                    onclick: move |_| {
                        invite_username.set(String::new());
                        invite_password.set(String::new());
                        show_invite.set(true);
                    },
                    "邀请新用户"
                }
            },
        }

        div { class: "page-content", style: "display:flex; flex-direction:column; gap:24px",
            // KPI strip
            div { class: "stats-grid",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "总用户数" }
                        div { class: "stat-value",
                            "{total}"
                        }
                    }
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "今日活跃" }
                        div { class: "stat-value",
                            "{active_count}"
                        }
                    }
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "用户资金池" }
                        div { class: "stat-value", "{format_nano_to_cny(user_list.iter().map(|u| u.balance_cny).sum::<i64>())}" }
                    }
                }
            }

            // Tabs + table
            div {
                div { class: "section-h row", style: "margin-bottom:0",
                    span { class: "lead-title", "客户明细" }
                    div { class: "tabs", style: "border-bottom:none; padding-bottom:0; gap:16px",
                        button {
                            class: if active_tab() == "all" { "tab active" } else { "tab" },
                            style: "padding-bottom:8px; margin-bottom:0",
                            onclick: move |_| active_tab.set("all".to_string()),
                            "全部客户"
                        }
                        button {
                            class: if active_tab() == "vip" { "tab active" } else { "tab" },
                            style: "padding-bottom:8px; margin-bottom:0",
                            onclick: move |_| active_tab.set("vip".to_string()),
                            "VIP客户"
                        }
                    }
                }

                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                } else if filtered.is_empty() {
                    EmptyState {
                        icon: rsx! { span { style: "font-size:40px", "👥" } },
                        title: "暂无客户".to_string(),
                        description: Some("邀请新用户开始使用".to_string()),
                        cta: None,
                    }
                } else {
                    table { class: "table", style: "margin-top:16px",
                        thead {
                            tr {
                                th { "ID" }
                                th { "用户名" }
                                th { "角色" }
                                th { "余额 (CNY)" }
                                th { "分组" }
                                th { "状态" }
                                th { style: "text-align:right", "操作" }
                            }
                        }
                        tbody {
                            for u in filtered {
                                tr {
                                    key: "{u.id}",
                                    td { span { class: "mono", style: "font-size:12px", "{u.id}" } }
                                    td { style: "font-weight:600", "{u.username}" }
                                    td { span { class: "pill neutral", "{u.role}" } }
                                    td { class: "mono", style: "color:var(--bc-text-primary); font-variant-numeric:tabular-nums",
                                        "{format_nano_to_cny(u.balance_cny)}"
                                    }
                                    td {
                                        if u.group == "VIP" {
                                            span { class: "pill", style: "background:var(--bc-primary-light); color:var(--bc-primary)", "VIP" }
                                        } else {
                                            span { class: "pill neutral", "{u.group}" }
                                        }
                                    }
                                    td {
                                        StatusPill {
                                            value: if u.status == 1 { "active".to_string() } else { "disabled".to_string() },
                                            label: if u.status == 1 { Some("Active".to_string()) } else { Some("Disabled".to_string()) },
                                        }
                                    }
                                    td { style: "text-align:right",
                                        button {
                                            class: "btn btn-ghost",
                                            style: "color:var(--bc-primary); font-weight:600",
                                            onclick: move |_| {
                                                show_topup.set(Some(u.id.clone()));
                                                topup_username.set(u.username.clone());
                                                topup_amount.set(0);
                                            },
                                            "充值"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Top-up modal
        if let Some(_uid) = show_topup() {
            div { class: "bc-modal-overlay", onclick: move |_| show_topup.set(None),
                div { class: "bc-modal", style: "width:440px", onclick: move |e| e.stop_propagation(),
                    div { class: "bc-modal-header",
                        span { class: "bc-modal-title", "账户充值" }
                        button { class: "btn-icon", onclick: move |_| show_topup.set(None), "✕" }
                    }
                    div { class: "bc-modal-body",
                        div { style: "display:flex; justify-content:space-between; align-items:center; padding:12px 16px; background:var(--bc-bg-hover); border-radius:8px",
                            span { style: "font-size:12px; color:var(--bc-text-secondary)", "目标账户" }
                            span { style: "font-weight:600", "{topup_username()}" }
                        }

                        div { style: "margin-top:16px",
                            label { class: "input-label", "充值金额 (CNY)" }
                            div { class: "input",
                                input {
                                    r#type: "number",
                                    placeholder: "0.00",
                                    value: if topup_amount() > 0 { format!("{}", topup_amount()) } else { String::new() },
                                    oninput: move |e| topup_amount.set(e.value().parse().unwrap_or(0)),
                                }
                            }
                        }

                        div { style: "display:grid; grid-template-columns:1fr 1fr 1fr; gap:8px; margin-top:12px",
                            button {
                                class: "btn btn-secondary",
                                onclick: move |_| topup_amount.set(100),
                                "¥100"
                            }
                            button {
                                class: "btn btn-secondary",
                                onclick: move |_| topup_amount.set(500),
                                "¥500"
                            }
                            button {
                                class: "btn btn-secondary",
                                onclick: move |_| topup_amount.set(1000),
                                "¥1000"
                            }
                        }
                    }
                    div { class: "bc-modal-footer",
                        button { class: "btn btn-ghost", onclick: move |_| show_topup.set(None), "取消" }
                        button { class: "btn btn-black",
                            disabled: topup_loading(),
                            onclick: move |_| {
                                let amount = topup_amount();
                                if amount <= 0 {
                                    toast.error("请输入有效金额");
                                    return;
                                }
                                let uid = show_topup().unwrap();
                                let amount_nano = amount * 1_000_000_000;
                                topup_loading.set(true);
                                spawn(async move {
                                    match UserService::topup(&uid, amount_nano, Some("CNY")).await {
                                        Ok(_) => {
                                            topup_loading.set(false);
                                            show_topup.set(None);
                                            users.restart();
                                            toast.success("充值成功");
                                        }
                                        Err(e) => {
                                            topup_loading.set(false);
                                            toast.error(&format!("充值失败: {}", e));
                                        }
                                    }
                                });
                            },
                            if topup_loading() { "处理中..." } else { "确认充值" }
                        }
                    }
                }
            }
        }

        // Invite new user modal
        BCModal {
            title: "邀请新用户".to_string(),
            open: show_invite(),
            onclose: move |_| show_invite.set(false),

            div { class: "flex flex-col gap-lg",
                div { style: "font-size:12px; color:var(--bc-text-secondary)", "创建新用户账户，用户可使用用户名和密码登录" }

                BCInput {
                    label: Some("用户名".to_string()),
                    r#type: "text".to_string(),
                    placeholder: "请输入用户名".to_string(),
                    value: invite_username(),
                    oninput: move |e| invite_username.set(e.value()),
                }

                BCInput {
                    label: Some("密码".to_string()),
                    r#type: "password".to_string(),
                    placeholder: "请输入密码（至少8位）".to_string(),
                    value: invite_password(),
                    oninput: move |e| invite_password.set(e.value()),
                }

                div { class: "flex justify-end gap-md mt-md",
                    BCButton {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| show_invite.set(false),
                        "取消"
                    }
                    BCButton {
                        variant: ButtonVariant::Black,
                        loading: invite_loading(),
                        disabled: invite_loading(),
                        onclick: move |_| {
                            let username = invite_username().trim().to_string();
                            let password = invite_password();
                            if username.is_empty() {
                                toast.error("请输入用户名");
                                return;
                            }
                            if password.len() < 8 {
                                toast.error("密码至少需要8位");
                                return;
                            }
                            invite_loading.set(true);
                            spawn(async move {
                                match UserService::create(&username, &password).await {
                                    Ok(()) => {
                                        toast.success("用户创建成功");
                                        show_invite.set(false);
                                        invite_username.set(String::new());
                                        invite_password.set(String::new());
                                        users.restart();
                                    }
                                    Err(e) => {
                                        toast.error(&format!("创建失败: {}", e));
                                    }
                                }
                                invite_loading.set(false);
                            });
                        },
                        "创建用户"
                    }
                }
            }
        }
    }
}