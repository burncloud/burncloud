// JSON Schema-driven UI — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::components::SchemaTable;
use burncloud_client_shared::schema::recharge_schema;
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

    let schema = recharge_schema();

    // Convert recharges to serde_json::Value for SchemaTable
    let table_data: Vec<serde_json::Value> = match recharges.read().as_ref() {
        Some(Ok(list)) => list
            .iter()
            .map(|item| {
                serde_json::json!({
                    "id": format!("RECH-{}", item.id),
                    "created_at": item.created_at.as_deref().unwrap_or("-"),
                    "description": item.description.as_deref().unwrap_or("账户充值"),
                    "amount": item.amount,
                    "status": "success"
                })
            })
            .collect(),
        _ => vec![],
    };

    let loading = recharges.read().is_none();

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
                div { class: "p-lg bc-card-solid flex flex-col gap-sm",
                    span { class: "text-xxs font-semibold uppercase tracking-wider text-tertiary", "账户余额" }
                    div { class: "flex items-baseline gap-sm",
                        span { class: "text-4xl font-bold text-primary tracking-tight", "{balance}" }
                    }
                }
                div { class: "p-lg bc-card-solid flex flex-col gap-sm",
                    span { class: "text-xxs font-semibold uppercase tracking-wider text-tertiary", "预估下月" }
                    div { class: "flex items-baseline gap-sm",
                        span { class: "text-4xl font-bold text-secondary tracking-tight", "{projected}" }
                    }
                }
            }

            // Transaction History via SchemaTable
            div { class: "flex flex-col gap-md",
                h3 { class: "text-caption font-medium text-secondary border-b pb-sm", "充值记录" }

                div { class: "overflow-x-auto bc-card-solid",
                    SchemaTable {
                        schema: schema.clone(),
                        data: table_data,
                        loading: loading,
                        on_row_click: move |_| {},
                    }
                }
            }
        }
    }
}
