use super::{
    actions::progress_width,
    model::{UsageBarTone, UsageMetricKind, UsageModel},
    state::{UsagePeriod, UsageState},
};
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName, MetricCard},
    },
};
use dioxus::prelude::*;

fn period_label(period: UsagePeriod, copy: &crate::i18n::LocaleStrings) -> &'static str {
    match period {
        UsagePeriod::SevenDays => copy.usage_period_7d,
        UsagePeriod::ThirtyDays => copy.usage_period_30d,
        UsagePeriod::NinetyDays => copy.usage_period_90d,
    }
}

fn tone_class(tone: UsageBarTone) -> &'static str {
    match tone {
        UsageBarTone::Dark => "usage-fill-dark",
        UsageBarTone::Indigo => "usage-fill-indigo",
        UsageBarTone::Green => "usage-fill-green",
        UsageBarTone::Amber => "usage-fill-amber",
    }
}

fn metric_label(kind: UsageMetricKind) -> &'static str {
    match kind {
        UsageMetricKind::Tokens => "",
        UsageMetricKind::Cost => "",
        UsageMetricKind::Requests => "",
        UsageMetricKind::Latency => "",
    }
}

fn metric_detail(kind: UsageMetricKind, copy: &crate::i18n::LocaleStrings) -> String {
    match kind {
        UsageMetricKind::Tokens => String::new(),
        UsageMetricKind::Cost => String::new(),
        UsageMetricKind::Requests => copy.usage_success_rate.to_string(),
        UsageMetricKind::Latency => String::new(),
    }
}

fn metric_trend(kind: UsageMetricKind, copy: &crate::i18n::LocaleStrings) -> Option<String> {
    match kind {
        UsageMetricKind::Tokens => Some(copy.usage_daily_tokens_growth.to_string()),
        UsageMetricKind::Latency => Some(copy.usage_latency_reduction.to_string()),
        UsageMetricKind::Cost | UsageMetricKind::Requests => None,
    }
}

#[component]
pub fn BuyerUsage() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let mut state = use_signal(UsageState::default);
    let snapshot = *state.read();
    let usage = UsageModel::reference();

    rsx! {
        BuyerShell {
            div { class: "usage-stack",
                section { class: "usage-page-header",
                    div { class: "usage-header",
                        div { class: "page-heading-copy",
                            h1 { {copy.usage_title} }
                            p { {copy.usage_subtitle} }
                        }
                        div { class: "usage-toolbar",
                            div { class: "usage-periods", role: "group", aria_label: copy.usage_period_label,
                                for period in UsagePeriod::ALL {
                                    button {
                                        r#type: "button",
                                        class: if snapshot.period == period { "usage-period active" } else { "usage-period" },
                                        aria_pressed: if snapshot.period == period { "true" } else { "false" },
                                        onclick: move |_| state.with_mut(|value| value.set_period(period)),
                                        {period_label(period, copy)}
                                    }
                                }
                            }
                            button { r#type: "button", class: "usage-export", aria_label: copy.usage_export,
                                Icon { name: IconName::Download, size: 14 }
                                span { {copy.usage_export} }
                            }
                        }
                    }
                    div { class: "usage-conclusion", role: "status",
                        Icon { name: IconName::CheckCircle, size: 16 }
                        span { {copy.usage_conclusion} }
                    }
                }
                section { class: "usage-metrics", aria_label: copy.usage_title,
                    for metric in usage.metrics {
                        MetricCard {
                            label: metric_label(metric.kind).to_string(),
                            value: metric.value.to_string(),
                            unit: metric.unit.map(str::to_string),
                            detail: metric_detail(metric.kind, copy),
                            trend: metric_trend(metric.kind, copy),
                            trend_positive: true,
                        }
                    }
                }
                section { class: "usage-panel",
                    div { class: "usage-panel-header",
                        div {
                            h3 {}
                            p {}
                        }
                    }
                    div { class: "usage-distribution",
                        for item in usage.breakdown {
                            div { class: "usage-distribution-row",
                                div { class: "usage-distribution-meta",
                                    strong { {item.name} }
                                    span { {format!("{} tokens ({}%) • ", item.tokens, item.percent)} strong { {item.cost} } }
                                }
                                div { class: "usage-track", aria_hidden: "true",
                                    span { class: format!("usage-fill {}", tone_class(item.tone)), style: progress_width(item.percent) }
                                }
                            }
                        }
                    }
                    div { class: "usage-table-scroll",
                        table { class: "usage-table",
                            thead { tr {
                                th { aria_label: "Model" }
                                th { aria_label: "Input tokens" }
                                th { aria_label: "Output tokens" }
                                th { aria_label: "Requests" }
                                th { aria_label: "Latency" }
                                th { aria_label: "Cost" }
                            } }
                            tbody {
                                for row in usage.rows {
                                    tr {
                                        td { class: "usage-model", {row.model} }
                                        td { class: "mono", {row.input_tokens} }
                                        td { class: "mono", {row.output_tokens} }
                                        td { class: "mono", {row.requests} }
                                        td { class: "mono", {row.latency} }
                                        td { class: "mono usage-cost", {row.cost} }
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
