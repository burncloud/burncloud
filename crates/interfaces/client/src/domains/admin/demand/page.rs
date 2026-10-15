use super::model::AdminDemandModel;
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName, MetricCard},
    },
};
use dioxus::prelude::*;

#[component]
pub fn AdminDemand() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let model = AdminDemandModel::mock();

    rsx! {
        BuyerShell {
            div { class: "admin-demand-stack",
                section { class: "page-header admin-demand-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.admin_demand_title} }
                        p { {copy.admin_demand_subtitle} }
                    }
                }
                div { class: "conclusion conclusion-healthy admin-demand-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {copy.admin_demand_conclusion} }
                }
                section { class: "metric-grid admin-demand-metrics", aria_label: copy.admin_demand_title,
                    MetricCard {
                        label: copy.admin_demand_throughput_label.to_string(),
                        value: model.throughput.to_string(),
                        unit: Some(copy.admin_capacity_tokens_unit.to_string()),
                        detail: String::new(),
                        trend: Some(copy.admin_demand_throughput_trend.to_string()),
                        trend_positive: true,
                    }
                    MetricCard {
                        label: copy.admin_demand_top_accounts_label.to_string(),
                        value: model.top_accounts.to_string(),
                        detail: copy.admin_demand_top_accounts_detail.to_string(),
                    }
                    MetricCard {
                        label: copy.admin_demand_low_latency_label.to_string(),
                        value: model.low_latency_share.to_string(),
                        detail: copy.admin_demand_low_latency_detail.to_string(),
                    }
                    MetricCard {
                        label: copy.admin_demand_prompt_completion_label.to_string(),
                        value: model.prompt_completion_tokens.to_string(),
                        unit: Some(copy.admin_demand_tokens_unit.to_string()),
                        detail: copy.admin_demand_prompt_completion_detail.to_string(),
                    }
                }
                section { class: "panel admin-demand-panel", aria_label: copy.admin_demand_accounts_title,
                    div { class: "section-header admin-demand-table-heading",
                        h2 { {copy.admin_demand_accounts_title} }
                    }
                    div { class: "table-scroll admin-demand-table-scroll",
                        table { class: "admin-demand-table", aria_label: copy.admin_demand_accounts_title,
                            thead { class: "admin-demand-table-head",
                                tr {
                                    th { scope: "col", {copy.admin_demand_col_tenant} }
                                    th { scope: "col", {copy.admin_demand_col_tier} }
                                    th { scope: "col", {copy.admin_demand_col_tokens} }
                                    th { scope: "col", {copy.admin_demand_col_spend} }
                                    th { scope: "col", {copy.admin_demand_col_model} }
                                    th { scope: "col", {copy.admin_demand_col_model_spend} }
                                }
                            }
                            tbody {
                                for account in model.accounts.iter().copied() {
                                    tr {
                                        td { class: "admin-demand-tenant", strong { {account.tenant} } }
                                        td { span { class: "tier", {account.tier} } }
                                        td { class: "admin-demand-tokens", {account.tokens} }
                                        td { class: "admin-demand-spend", {account.spend} }
                                        td { class: "admin-demand-model", {account.model} }
                                        td { class: "admin-demand-model-spend", strong { {account.model_spend} } }
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
