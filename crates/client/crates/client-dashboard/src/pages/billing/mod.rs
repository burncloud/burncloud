use dioxus::prelude::*;

#[component]
pub fn BillingPage() -> Element {
    // Mock Data for "Left Brain" Finance View
    let total_spend = "¥ 12,450.00";
    let balance = "¥ 5,230.00";
    let projected = "¥ 18,000.00";

    rsx! {
        div { class: "flex flex-col h-full gap-8",
            // Header
            div { class: "flex justify-between items-end",
                div {
                    h1 { class: "text-2xl font-semibold text-base-content mb-1 tracking-tight", "账单与订阅" }
                    p { class: "text-sm text-base-content/60 font-medium", "管理您的账户余额、充值记录与发票" }
                }
                button { class: "btn btn-primary btn-sm px-6 shadow-sm text-white", "充值余额" }
            }

            // Financial Overview Cards (Moved from Channel Page)
            div { class: "grid grid-cols-3 gap-6",
                // Spend
                div { class: "p-6 bg-base-100 rounded-xl border border-base-200 shadow-sm flex flex-col gap-2",
                    span { class: "text-xs font-semibold text-base-content/40 uppercase tracking-wider", "本月支出" }
                    div { class: "flex items-baseline gap-2",
                        span { class: "text-4xl font-bold text-base-content tracking-tight", "{total_spend}" }
                        span { class: "text-xs font-medium text-red-600 bg-red-50 px-1.5 py-0.5 rounded", "+15%" }
                    }
                }
                // Balance
                div { class: "p-6 bg-base-100 rounded-xl border border-base-200 shadow-sm flex flex-col gap-2",
                    span { class: "text-xs font-semibold text-base-content/40 uppercase tracking-wider", "账户余额" }
                    div { class: "flex items-baseline gap-2",
                        span { class: "text-4xl font-bold text-base-content tracking-tight", "{balance}" }
                    }
                }
                // Projected
                div { class: "p-6 bg-base-100 rounded-xl border border-base-200 shadow-sm flex flex-col gap-2",
                    span { class: "text-xs font-semibold text-base-content/40 uppercase tracking-wider", "预估下月" }
                    div { class: "flex items-baseline gap-2",
                        span { class: "text-4xl font-bold text-base-content/60 tracking-tight", "{projected}" }
                    }
                }
            }

            // Transaction History Placeholder
            div { class: "flex flex-col gap-4",
                h3 { class: "text-sm font-medium text-base-content/80 border-b border-base-content/10 pb-2", "近期交易" }
                
                div { class: "overflow-x-auto border border-base-200 rounded-lg",
                    table { class: "table w-full text-sm",
                        thead { class: "bg-base-50 text-base-content/60",
                            tr {
                                th { "交易 ID" }
                                th { "时间" }
                                th { "描述" }
                                th { class: "text-right", "金额" }
                                th { class: "text-right", "状态" }
                            }
                        }
                        tbody {
                            tr {
                                td { class: "font-mono text-xs", "#TRX-8821" }
                                td { "2025-12-09 14:30" }
                                td { "API 调用费用 (OpenAI)" }
                                td { class: "text-right font-medium", "- ¥ 120.50" }
                                td { class: "text-right text-success text-xs font-bold", "成功" }
                            }
                            tr {
                                td { class: "font-mono text-xs", "#TRX-8820" }
                                td { "2025-12-08 09:15" }
                                td { "账户充值" }
                                td { class: "text-right font-medium text-success", "+ ¥ 1,000.00" }
                                td { class: "text-right text-success text-xs font-bold", "成功" }
                            }
                        }
                    }
                }
            }
        }
    }
}
