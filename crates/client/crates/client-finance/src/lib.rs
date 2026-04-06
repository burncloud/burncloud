use burncloud_client_shared::services::usage_service::UsageService;
use dioxus::prelude::*;

#[component]
pub fn BillingPage() -> Element {
    let recharges =
        use_resource(move || async move { UsageService::list_recharges("demo-user").await });

    // Mock Data for "Left Brain" Finance View
    let total_spend = "¥ 12,450.00";
    let balance = "¥ 5,230.00";
    let projected = "¥ 18,000.00";

    rsx! {
        div { class: "flex flex-col h-full gap-xl",
            // Header
            div { class: "flex justify-between items-end",
                div {
                    h1 { class: "text-title font-semibold text-primary mb-xs tracking-tight", "财务中心" }
                    p { class: "text-caption text-secondary font-medium", "管理您的账户余额、充值记录与收支统计" }
                }
                button { class: "btn btn-primary btn-sm px-lg shadow-sm text-white", "充值余额" }
            }

            // Financial Overview Cards
            div { class: "grid grid-cols-3 gap-lg",
                // Spend
                div { class: "p-lg bc-card-solid flex flex-col gap-sm",
                    span { class: "text-xxs font-semibold uppercase tracking-wider text-tertiary", "本月支出" }
                    div { class: "flex items-baseline gap-sm",
                        span { class: "text-4xl font-bold text-primary tracking-tight", "{total_spend}" }
                        span { class: "text-xs font-medium px-sm py-0.5 rounded",
                            style: "color: var(--bc-danger); background: var(--bc-danger-light);",
                            "+15%"
                        }
                    }
                }
                // Balance
                div { class: "p-lg bc-card-solid flex flex-col gap-sm",
                    span { class: "text-xxs font-semibold uppercase tracking-wider text-tertiary", "账户余额" }
                    div { class: "flex items-baseline gap-sm",
                        span { class: "text-4xl font-bold text-primary tracking-tight", "{balance}" }
                    }
                }
                // Projected
                div { class: "p-lg bc-card-solid flex flex-col gap-sm",
                    span { class: "text-xxs font-semibold uppercase tracking-wider text-tertiary", "预估下月" }
                    div { class: "flex items-baseline gap-sm",
                        span { class: "text-4xl font-bold text-secondary tracking-tight", "{projected}" }
                    }
                }
            }

            // Transaction History
            div { class: "flex flex-col gap-md",
                h3 { class: "text-caption font-medium text-secondary border-b pb-sm", "充值记录" }

                div { class: "overflow-x-auto bc-card-solid",
                    table { class: "table w-full text-caption",
                        thead {
                            style: "background: var(--bc-bg-hover);",
                            tr {
                                th { class: "text-secondary font-medium", "交易 ID" }
                                th { class: "text-secondary font-medium", "时间" }
                                th { class: "text-secondary font-medium", "描述" }
                                th { class: "text-secondary font-medium text-right", "金额" }
                                th { class: "text-secondary font-medium text-right", "状态" }
                            }
                        }
                        tbody {
                            if let Some(Ok(list)) = recharges.read().as_ref() {
                                for item in list {
                                    tr {
                                        td { class: "font-mono text-xs text-primary", "#RECH-{item.id}" }
                                        td { class: "text-secondary", "{item.created_at.as_deref().unwrap_or(\"-\")}" }
                                        td { class: "text-secondary", "{item.description.as_deref().unwrap_or(\"账户充值\")}" }
                                        td { class: "text-right font-medium", style: "color: var(--bc-success);", "+ ¥ {item.amount:.2}" }
                                        td { class: "text-right text-xs font-bold", style: "color: var(--bc-success);", "成功" }
                                    }
                                }
                            } else {
                                tr {
                                    td { colspan: "5", class: "text-center py-xl text-tertiary",
                                        if recharges.read().is_none() { "加载中..." } else { "暂无充值记录" }
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
