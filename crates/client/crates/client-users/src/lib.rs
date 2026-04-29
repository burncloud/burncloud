use burncloud_client_shared::components::{
    BCButton, ButtonVariant, PageHeader, StatKpi, StatusPill, ColumnDef, PageTable,
    EmptyState, SkeletonCard, SkeletonVariant,
};
use burncloud_client_shared::services::user_service::UserService;
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

fn format_cents(cents: i64) -> String {
    let yuan = cents as f64 / 100.0;
    format!("¥ {yuan:.2}")
}

#[component]
pub fn UsersPage() -> Element {
    let mut active_tab = use_signal(|| "all".to_string());
    let mut show_topup = use_signal(|| None::<String>);
    let mut topup_amount = use_signal(|| 0i64);
    let toast = use_toast();

    let users = use_resource(move || async move {
        UserService::list().await.unwrap_or_default()
    });

    let user_list = users.read().clone().unwrap_or_default();
    let loading = users.read().is_none();

    let total = user_list.len();
    let active_count = user_list.iter().filter(|u| u.status == 1).count();

    let filtered: Vec<_> = user_list.iter().cloned().filter(|u| {
        match active_tab().as_str() {
            "vip" => u.group == "VIP",
            _ => true,
        }
    }).collect();

    let columns = vec![
        ColumnDef { key: "id".to_string(), label: "ID".to_string(), width: Some("80px".to_string()) },
        ColumnDef { key: "name".to_string(), label: "用户名".to_string(), width: None },
        ColumnDef { key: "role".to_string(), label: "角色".to_string(), width: Some("100px".to_string()) },
        ColumnDef { key: "balance".to_string(), label: "余额 (CNY)".to_string(), width: Some("120px".to_string()) },
        ColumnDef { key: "group".to_string(), label: "分组".to_string(), width: Some("100px".to_string()) },
        ColumnDef { key: "status".to_string(), label: "状态".to_string(), width: Some("100px".to_string()) },
        ColumnDef { key: "action".to_string(), label: "操作".to_string(), width: Some("80px".to_string()) },
    ];

    rsx! {
        PageHeader {
            title: "客户列表",
            subtitle: Some("用户增长与留存管理".to_string()),
            actions: rsx! {
                BCButton {
                    class: "btn-black",
                    onclick: move |_| {},
                    "邀请新用户"
                }
            },
        }

        div { class: "page-content", style: "display:flex; flex-direction:column; gap:24px",
            // KPI strip
            div { class: "stats-grid cols-3",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    StatKpi {
                        label: "总用户数".to_string(),
                        value: format!("{total}"),
                        delta: rsx! { span { class: "stat-foot up", "▲ +24 This Week" } },
                    }
                    StatKpi {
                        label: "今日活跃".to_string(),
                        value: format!("{active_count}"),
                        delta: rsx! { span { class: "stat-pill muted", "67% 活跃率" } },
                    }
                    StatKpi {
                        label: "用户资金池".to_string(),
                        value: "¥ 452,000.00".to_string(),
                    }
                }
            }

            // Tabs + table
            div {
                div { class: "section-h",
                    span { class: "lead-title", "客户明细" }
                    div { class: "tabs",
                        span {
                            class: if active_tab() == "all" { "tab active" } else { "tab" },
                            onclick: move |_| active_tab.set("all".to_string()),
                            "全部客户"
                        }
                        span {
                            class: if active_tab() == "vip" { "tab active" } else { "tab" },
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
                    PageTable {
                        columns: columns,
                        for u in filtered {
                            tr {
                                key: "{u.id}",
                                td { class: "mono", style: "font-size:12px; color:var(--bc-text-secondary)", "{u.id}" }
                                td { style: "font-weight:600", "{u.username}" }
                                td {
                                    span { class: "pill neutral", "{u.role}" }
                                }
                                td { class: "mono", style: "color:var(--bc-text-primary)",
                                    "{format_cents(u.balance_cny)}"
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

        // Top-up modal
        if let Some(uid) = show_topup() {
            div { class: "bc-modal-overlay", onclick: move |_| show_topup.set(None),
                div { class: "bc-modal", style: "width:440px", onclick: move |e| e.stop_propagation(),
                    div { class: "bc-modal-header",
                        h3 { "账户充值" }
                    }
                    div { class: "bc-modal-body",
                        div { style: "display:flex; justify-content:space-between; align-items:center; padding:12px 16px; background:var(--bc-bg-hover); border-radius:8px",
                            span { style: "font-size:12px; color:var(--bc-text-secondary)", "目标账户" }
                            span { style: "font-weight:600", "ID: {uid}" }
                        }

                        div { style: "margin-top:16px",
                            label { class: "input-label", "充值金额 (CNY)" }
                            div { class: "bc-input",
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
                        BCButton {
                            variant: ButtonVariant::Secondary,
                            onclick: move |_| show_topup.set(None),
                            "取消"
                        }
                        BCButton {
                            class: "btn-black",
                            onclick: move |_| {
                                show_topup.set(None);
                                toast.success("充值成功");
                            },
                            "确认充值"
                        }
                    }
                }
            }
        }
    }
}
