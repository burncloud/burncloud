use super::{
    actions::{download_csv, to_csv},
    model::AdminRevenueModel,
};
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName, MetricCard},
    },
};
use dioxus::prelude::*;

#[component]
pub fn AdminRevenue() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let model = AdminRevenueModel::reference();
    let csv = to_csv(model);

    rsx! {
        BuyerShell {
            div { class: "admin-revenue-stack",
                section { class: "page-header admin-revenue-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.admin_revenue_title} }
                        p { {copy.admin_revenue_subtitle} }
                    }
                    button {
                        r#type: "button",
                        class: "admin-revenue-export",
                        title: copy.admin_revenue_export,
                        aria_label: copy.admin_revenue_export,
                        onclick: move |_| download_csv(&csv),
                        Icon { name: IconName::Download, size: 15 }
                    }
                }
                div { class: "conclusion conclusion-healthy admin-revenue-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {copy.admin_revenue_conclusion} }
                }
                section { class: "metric-grid admin-revenue-metrics", aria_label: copy.admin_revenue_title,
                    MetricCard {
                        label: String::new(),
                        value: model.gmv.to_string(),
                        detail: String::new(),
                        trend: Some(copy.admin_revenue_gmv_trend.to_string()),
                        trend_positive: true,
                    }
                    MetricCard {
                        label: String::new(),
                        value: model.supplier_payouts.to_string(),
                        detail: copy.admin_revenue_payouts_detail.to_string(),
                    }
                    MetricCard {
                        label: String::new(),
                        value: model.net_revenue.to_string(),
                        detail: String::new(),
                        trend: Some(copy.admin_revenue_margin_detail.to_string()),
                        trend_positive: true,
                    }
                    MetricCard {
                        label: String::new(),
                        value: model.prepaid_credits.to_string(),
                        detail: copy.admin_revenue_prepaid_detail.to_string(),
                    }
                }
                section { class: "panel admin-revenue-panel", aria_label: copy.admin_revenue_title,
                    h2 { class: "admin-visually-hidden", {copy.admin_revenue_title} }
                    div { class: "table-scroll admin-revenue-table-scroll",
                        table { class: "admin-revenue-table", aria_label: copy.admin_revenue_title,
                            thead { class: "admin-revenue-table-head",
                                tr {
                                    for column in copy.admin_revenue_columns {
                                        th { scope: "col", span { class: "admin-visually-hidden", {column} } }
                                    }
                                }
                            }
                            tbody {
                                for row in model.rows {
                                    tr {
                                        td { class: "admin-revenue-model", strong { {row.model} } }
                                        td { class: "admin-revenue-tokens", {row.tokens} }
                                        td { class: "admin-revenue-gmv", strong { {row.gmv} } }
                                        td { class: "admin-revenue-payout", {row.supplier_payout} }
                                        td { class: "admin-revenue-net", strong { {row.net_revenue} } }
                                        td { class: "admin-revenue-margin", strong { {row.net_margin} } }
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
