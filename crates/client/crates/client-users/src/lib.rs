use burncloud_client_shared::components::{
    BCButton, PageHeader, StatusPill,
    EmptyState, SkeletonCard, SkeletonVariant,
};
use burncloud_client_shared::i18n::t;
use burncloud_client_shared::services::user_service::UserService;
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

fn format_cents(cents: i64) -> String {
    let yuan = cents as f64 / 100.0;
    format!("¥ {yuan:.2}")
}

#[component]
pub fn UsersPage() -> Element {
    let i18n = burncloud_client_shared::i18n::use_i18n();
    let lang = i18n.language;
    let mut active_tab = use_signal(|| "all".to_string());
    let mut show_topup = use_signal(|| None::<String>);
    let mut topup_amount = use_signal(|| 0i64);
    let mut topup_username = use_signal(String::new);
    let toast = use_toast();

    let users = use_resource(move || async move {
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
            title: t(*lang.read(), "users.title"),
            subtitle: Some(t(*lang.read(), "users.subtitle").to_string()),
            actions: rsx! {
                BCButton {
                    class: "btn-black",
                    onclick: move |_| {},
                    {t(*lang.read(), "users.invite")}
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
                        span { class: "stat-eyebrow", {t(*lang.read(), "users.kpi.total_users")} }
                        div { class: "stat-value",
                            "{total}"
                        }
                    }
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", {t(*lang.read(), "users.kpi.active_today")} }
                        div { class: "stat-value",
                            "{active_count}"
                        }
                    }
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", {t(*lang.read(), "users.kpi.fund_pool")} }
                        div { class: "stat-value", "{format_cents(user_list.iter().map(|u| u.balance_cny).sum::<i64>())}" }
                    }
                }
            }

            // Tabs + table
            div {
                div { class: "section-h row", style: "margin-bottom:0",
                    span { class: "lead-title", {t(*lang.read(), "users.detail_title")} }
                    div { class: "tabs", style: "border-bottom:none; padding-bottom:0; gap:16px",
                        button {
                            class: if active_tab() == "all" { "tab active" } else { "tab" },
                            style: "padding-bottom:8px; margin-bottom:0",
                            onclick: move |_| active_tab.set("all".to_string()),
                            {t(*lang.read(), "users.tab.all")}
                        }
                        button {
                            class: if active_tab() == "vip" { "tab active" } else { "tab" },
                            style: "padding-bottom:8px; margin-bottom:0",
                            onclick: move |_| active_tab.set("vip".to_string()),
                            {t(*lang.read(), "users.tab.vip")}
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
                        title: t(*lang.read(), "users.empty_title").to_string(),
                        description: Some(t(*lang.read(), "users.empty_desc").to_string()),
                        cta: None,
                    }
                } else {
                    table { class: "table", style: "margin-top:16px",
                        thead {
                            tr {
                                th { "ID" }
                                th { {t(*lang.read(), "users.col.username")} }
                                th { {t(*lang.read(), "users.col.role")} }
                                th { {t(*lang.read(), "users.col.balance")} }
                                th { {t(*lang.read(), "users.col.group")} }
                                th { {t(*lang.read(), "users.col.status")} }
                                th { style: "text-align:right", {t(*lang.read(), "users.col.actions")} }
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
                                                topup_username.set(u.username.clone());
                                                topup_amount.set(0);
                                            },
                                            {t(*lang.read(), "users.topup")}
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
                        span { class: "bc-modal-title", {t(*lang.read(), "users.topup_modal.title")} }
                        button { class: "btn-icon", onclick: move |_| show_topup.set(None), "✕" }
                    }
                    div { class: "bc-modal-body",
                        div { style: "display:flex; justify-content:space-between; align-items:center; padding:12px 16px; background:var(--bc-bg-hover); border-radius:8px",
                            span { style: "font-size:12px; color:var(--bc-text-secondary)", {t(*lang.read(), "users.topup_modal.target_account")} }
                            span { style: "font-weight:600", "{topup_username()}" }
                        }

                        div { style: "margin-top:16px",
                            label { class: "input-label", {t(*lang.read(), "users.topup_modal.amount")} }
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
                        button { class: "btn btn-ghost", onclick: move |_| show_topup.set(None), {t(*lang.read(), "common.cancel")} }
                        button { class: "btn btn-black", onclick: move |_| {
                            show_topup.set(None);
                            toast.success(t(*lang.read(), "users.topup_modal.success"));
                        }, {t(*lang.read(), "users.topup_modal.confirm")} }
                    }
                }
            }
        }
    }
}
