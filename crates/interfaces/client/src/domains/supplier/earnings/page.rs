use super::{
    actions::{download_csv, to_csv},
    model::SupplierEarningsModel,
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
pub fn SupplierEarnings() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let model = SupplierEarningsModel::reference();
    let csv = to_csv(model);

    rsx! {
        BuyerShell {
            div { class: "supplier-earnings-stack",
                section { class: "page-header supplier-earnings-page-header",
                    div { class: "supplier-earnings-heading-row",
                        div { class: "page-heading-copy",
                            h1 { {copy.supplier_earnings_page_title} }
                            p { {copy.supplier_earnings_page_subtitle} }
                        }
                        button {
                            r#type: "button",
                            class: "supplier-earnings-export",
                            title: copy.supplier_earnings_export,
                            aria_label: copy.supplier_earnings_export,
                            onclick: move |_| download_csv(&csv),
                            Icon { name: IconName::Download, size: 15 }
                        }
                    }
                    div { class: "conclusion conclusion-healthy supplier-earnings-conclusion", role: "status",
                        Icon { name: IconName::CheckCircle, size: 16 }
                        span { class: "conclusion-text", {copy.supplier_earnings_conclusion} }
                    }
                }
                section { class: "metric-grid supplier-earnings-metrics", aria_label: copy.supplier_earnings_page_title,
                    MetricCard {
                        label: String::new(),
                        value: model.today_earnings.to_string(),
                        detail: String::new(),
                        trend: Some(model.today_trend.to_string()),
                        trend_positive: true,
                    }
                    MetricCard {
                        label: String::new(),
                        value: model.monthly_earnings.to_string(),
                        detail: model.monthly_period.to_string(),
                    }
                    MetricCard {
                        label: String::new(),
                        value: model.total_tokens.to_string(),
                        unit: Some("tokens".to_string()),
                        detail: model.valid_tokens.to_string(),
                    }
                    MetricCard {
                        label: String::new(),
                        value: model.effective_rate.to_string(),
                        detail: model.effective_rate_detail.to_string(),
                    }
                }
                section { class: "panel supplier-earnings-panel",
                    div { class: "table-scroll supplier-earnings-table-scroll",
                        table { class: "supplier-earnings-table",
                            thead {
                                tr {
                                    th { {copy.supplier_node} }
                                    th { class: "supplier-earnings-hardware-heading", {copy.supplier_earnings_gpu_hardware} }
                                    th { {copy.supplier_earnings_model} }
                                    th { {copy.supplier_earnings_tokens} }
                                    th { {copy.supplier_earnings_revenue_share} }
                                    th { {copy.supplier_earnings} }
                                }
                            }
                            tbody {
                                for row in model.rows {
                                    tr {
                                        td { class: "supplier-earnings-node", {row.node} }
                                        td { class: "supplier-earnings-hardware", {row.hardware} }
                                        td { class: "supplier-earnings-model", {row.model} }
                                        td { class: "supplier-earnings-number", {row.tokens} }
                                        td { class: "supplier-earnings-number supplier-earnings-share", {row.revenue_share} }
                                        td { class: "supplier-earnings-value", {row.earnings} }
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
