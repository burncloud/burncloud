use burncloud_client_shared::components::{
    PageHeader, StatKpi, StatusPill, ColumnDef, PageTable,
    SkeletonCard, SkeletonVariant,
};
use burncloud_client_shared::services::user_service::UserService;
use dioxus::prelude::*;

fn format_balance(nano: i64) -> String {
    format!("{:.2}", nano as f64 / 1_000_000_000.0)
}

fn status_label(status: i32) -> String {
    if status == 1 { "active".to_string() } else { "disabled".to_string() }
}

#[component]
pub fn BillingPage() -> Element {
    let users = use_resource(move || async move {
        UserService::list().await.unwrap_or_default()
    });

    let user_list = users.read().clone().unwrap_or_default();
    let loading = users.read().is_none();
    let total_balance: f64 = user_list.iter()
        .map(|u| u.balance_cny as f64 / 1_000_000_000.0)
        .sum();
    let _total_recharge: f64 = 0.0; // Not available from UserService
    let _total_consumption: f64 = 0.0; // Not available from UserService

    let columns = vec![
        ColumnDef { key: "id".to_string(), label: "ID".to_string(), width: Some("80px".to_string()) },
        ColumnDef { key: "name".to_string(), label: "User".to_string(), width: None },
        ColumnDef { key: "balance_cny".to_string(), label: "Balance (CNY)".to_string(), width: Some("140px".to_string()) },
        ColumnDef { key: "balance_usd".to_string(), label: "Balance (USD)".to_string(), width: Some("140px".to_string()) },
        ColumnDef { key: "status".to_string(), label: "Status".to_string(), width: Some("120px".to_string()) },
    ];

    rsx! {
        PageHeader {
            title: "财务中心",
            subtitle: Some("管理您的账户余额、充值记录与收支统计".to_string()),
        }

        div { class: "page-content",
            // Stats
            div { class: "stats-grid cols-4",
                StatKpi {
                    label: "总余额 (CNY)",
                    value: "¥{total_balance:.2}",
                    large: Some(true),
                }
                StatKpi {
                    label: "总余额 (USD)",
                    value: "${total_balance:.2}",
                }
                StatKpi {
                    label: "活跃用户",
                    value: "{user_list.iter().filter(|u| u.status == 1).count()}",
                }
                StatKpi {
                    label: "总用户",
                    value: "{user_list.len()}",
                }
            }

            // Table
            if loading {
                SkeletonCard { variant: Some(SkeletonVariant::Row) }
                SkeletonCard { variant: Some(SkeletonVariant::Row) }
                SkeletonCard { variant: Some(SkeletonVariant::Row) }
            } else {
                PageTable {
                columns: columns,
                for user in &user_list {
                    tr {
                        key: "{user.id}",
                        td { class: "mono", "{user.id}" }
                        td { "{user.username}" }
                        td { class: "mono", style: "font-size:13px; color:var(--bc-success); font-weight:600",
                            "¥{format_balance(user.balance_cny)}"
                        }
                        td { class: "mono", style: "font-size:13px",
                            "${format_balance(user.balance_usd)}"
                        }
                        td {
                            StatusPill {
                                value: status_label(user.status)
                            }
                        }
                    }
                }
            }
            }
        }
    }
}
