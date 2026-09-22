use super::model::{MarketplaceModel, ModelCategory, ModelStatus, RoutingTier};
use super::state::MarketplaceState;
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Badge, Icon, IconName},
    },
};
use dioxus::prelude::*;

fn category_label(category: ModelCategory, copy: &crate::i18n::LocaleStrings) -> &'static str {
    match category {
        ModelCategory::All => copy.marketplace_category_all,
        ModelCategory::General => copy.marketplace_category_general,
        ModelCategory::Reasoning => copy.marketplace_category_reasoning,
        ModelCategory::Coding => copy.marketplace_category_coding,
        ModelCategory::LowLatency => copy.marketplace_category_low_latency,
    }
}

fn tier_tone(tier: RoutingTier) -> &'static str {
    match tier {
        RoutingTier::Economy => "brand",
        RoutingTier::Standard => "neutral",
        RoutingTier::Performance => "marketplace-accent",
    }
}

fn tier_label(tier: RoutingTier, copy: &crate::i18n::LocaleStrings) -> &'static str {
    match tier {
        RoutingTier::Economy => copy.marketplace_tier_economy,
        RoutingTier::Standard => copy.marketplace_tier_standard,
        RoutingTier::Performance => copy.marketplace_tier_performance,
    }
}

fn status_class(status: ModelStatus) -> &'static str {
    match status {
        ModelStatus::Healthy => "status",
        ModelStatus::Degraded => "status status-warning",
    }
}

fn format_price(price: f64) -> String {
    let price = format!("{price:.2}");
    price
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn model_status(model: &MarketplaceModel) -> Element {
    rsx! {
        span { class: status_class(model.status),
            span { class: "status-dot" }
            span { class: "status-label", {format!("{}% SLA", model.availability)} }
        }
    }
}

fn close_drawer(mut state: Signal<MarketplaceState>) {
    state.with_mut(MarketplaceState::close_drawer);
}

#[component]
pub fn BuyerMarketplace() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let mut state = use_signal(MarketplaceState::default);
    let snapshot = state.read().clone();
    let models = snapshot.filtered_models();
    let selected_model = snapshot.selected_model();

    rsx! {
        BuyerShell {
            div { class: "marketplace-stack",
                section { class: "page-header marketplace-page-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.marketplace_title} }
                        p { {copy.marketplace_subtitle} }
                    }
                }
                div { class: "conclusion conclusion-healthy marketplace-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {copy.marketplace_conclusion} }
                }
                div { class: "marketplace-toolbar",
                    div { class: "marketplace-categories", role: "group", aria_label: copy.marketplace_category_all,
                        for category in ModelCategory::ALL {
                            button {
                                r#type: "button",
                                class: if snapshot.category == category { "marketplace-category active" } else { "marketplace-category" },
                                aria_pressed: if snapshot.category == category { "true" } else { "false" },
                                onclick: move |_| state.with_mut(|value| value.set_category(category)),
                                {category_label(category, copy)}
                            }
                        }
                    }
                    label { class: "marketplace-search",
                        Icon { name: IconName::Search, size: 14 }
                        input {
                            r#type: "search",
                            value: snapshot.search.clone(),
                            placeholder: copy.marketplace_search_placeholder,
                            aria_label: copy.marketplace_search_placeholder,
                            oninput: move |event| state.with_mut(|value| value.set_search(event.value()))
                        }
                    }
                }
                if models.is_empty() {
                    div { class: "marketplace-empty", role: "status",
                        Icon { name: IconName::Search, size: 20 }
                        strong { {copy.marketplace_no_results} }
                    }
                } else {
                    div { class: "marketplace-grid",
                        for model in models {
                            article { class: "panel marketplace-card",
                                div { class: "marketplace-card-main",
                                    div { class: "marketplace-card-meta",
                                        span { class: "marketplace-family", {model.family} }
                                        {model_status(model)}
                                    }
                                    div { class: "marketplace-card-heading",
                                        h2 { {model.name} }
                                        p { {model.tagline} }
                                    }
                                    div { class: "marketplace-price-box",
                                        div { class: "marketplace-price-copy",
                                            span { class: "marketplace-eyebrow", {format!("{} / {}", copy.marketplace_input_price, copy.marketplace_output_price)} }
                                            div { class: "marketplace-price-values",
                                                strong { {format!("${}", format_price(model.input_price_per_million))} }
                                                span { "/" }
                                                strong { {format!("${}", format_price(model.output_price_per_million))} }
                                            }
                                            small { {copy.marketplace_per_million_tokens} }
                                        }
                                        div { class: "marketplace-context-copy",
                                            span { class: "marketplace-eyebrow", {copy.marketplace_context} }
                                            strong { {model.context_window} }
                                            small {
                                                span { class: "marketplace-latency-value", {format!("{}{}ms", copy.marketplace_sub_latency, model.p95_latency_ms)} }
                                                " TTFT"
                                            }
                                        }
                                    }
                                    div { class: "marketplace-tier-row",
                                        span { class: "marketplace-eyebrow", {copy.marketplace_tiers} }
                                        for tier in model.supported_tiers {
                                            Badge { label: tier_label(*tier, copy).to_string(), tone: tier_tone(*tier).to_string() }
                                        }
                                    }
                                }
                                div { class: "marketplace-card-actions",
                                    button {
                                        r#type: "button",
                                        class: "marketplace-details-button",
                                        onclick: move |_| state.with_mut(|value| value.open_details(model.id)),
                                        {copy.marketplace_view_details}
                                    }
                                    Link {
                                        role: "button",
                                        class: "button button-primary marketplace-playground-link",
                                        to: "/buyer/playground",
                                        span { {copy.marketplace_test_in_playground} }
                                        Icon { name: IconName::ArrowRight, size: 14 }
                                    }
                                }
                            }
                        }
                    }
                }
            }
                if snapshot.drawer_open {
                    if let Some(model) = selected_model {
                        div { class: "marketplace-drawer-backdrop", role: "presentation", onclick: move |_| close_drawer(state) }
                        aside { class: "marketplace-drawer", role: "dialog", aria_modal: "true", aria_label: model.name,
                            div { class: "marketplace-drawer-header",
                                div {
                                    h2 { {model.name} }
                                    p { {format!("{} · {}", model.family, category_label(model.category, copy))} }
                                }
                                button {
                                    r#type: "button",
                                    class: "icon-button marketplace-drawer-close",
                                    title: copy.marketplace_drawer_close,
                                    aria_label: copy.marketplace_drawer_close,
                                    onclick: move |_| close_drawer(state),
                                    Icon { name: IconName::X, size: 16 }
                                }
                            }
                            div { class: "marketplace-drawer-body",
                                section { class: "marketplace-drawer-section",
                                    h3 { {copy.marketplace_drawer_specs} }
                                    p { {model.description} }
                                }
                                section { class: "marketplace-direct-rate",
                                    div { class: "marketplace-direct-rate-header",
                                        strong { {copy.marketplace_direct_rate} }
                                        Badge { label: copy.marketplace_zero_markup.to_string(), tone: "success".to_string() }
                                    }
                                    div { class: "marketplace-direct-rate-grid",
                                        div { span { {copy.marketplace_input_price} } strong { {format!("${}", format_price(model.input_price_per_million))} } small { {copy.marketplace_per_million_tokens} } }
                                        div { span { {copy.marketplace_output_price} } strong { {format!("${}", format_price(model.output_price_per_million))} } small { {copy.marketplace_per_million_tokens} } }
                                    }
                                }
                                section { class: "marketplace-drawer-section",
                                    h3 { {copy.marketplace_benchmarks} }
                                    div { class: "marketplace-benchmarks",
                                        for benchmark in model.benchmarks {
                                            div { class: "marketplace-benchmark",
                                                span { {benchmark.name} }
                                                strong { {benchmark.score} }
                                            }
                                        }
                                    }
                                }
                                section { class: "marketplace-drawer-section",
                                    h3 { {copy.marketplace_recommended_for} }
                                    p { class: "marketplace-recommendation", {model.recommended_for} }
                                }
                                section { class: "marketplace-slo-section",
                                    button {
                                        r#type: "button",
                                        class: "marketplace-slo-toggle",
                                        aria_expanded: snapshot.slo_expanded,
                                        onclick: move |_| state.with_mut(MarketplaceState::toggle_slo),
                                        span { class: "marketplace-slo-label", Icon { name: IconName::Gauge, size: 14 } span { {copy.marketplace_slo_data} } }
                                        Icon { name: IconName::ChevronDown, size: 16 }
                                    }
                                    if snapshot.slo_expanded {
                                        div { class: "marketplace-slo-data",
                                            div { span { {format!("{}:", copy.marketplace_drawer_latency)} } strong { {format!("{} ms", model.p95_latency_ms)} } }
                                            div { span { {format!("{}:", copy.marketplace_drawer_context)} } strong { {model.context_window} } }
                                            div { span { {format!("{}:", copy.marketplace_drawer_status)} } strong { {format!("{}% SLA", model.availability)} } }
                                            div { span { {format!("{}:", copy.marketplace_drawer_attestation)} } strong { {copy.marketplace_attestation_value} } }
                                        }
                                    }
                                }
                                div { class: "marketplace-drawer-footer",
                                    Link {
                                        role: "button",
                                        class: "button button-primary marketplace-drawer-cta",
                                        to: "/buyer/playground",
                                        onclick: move |_| close_drawer(state),
                                        Icon { name: IconName::Terminal, size: 15 }
                                        span { {copy.marketplace_test_in_playground} }
                                    }
                                }
                            }
                        }
                    }
                }
        }
    }
}
