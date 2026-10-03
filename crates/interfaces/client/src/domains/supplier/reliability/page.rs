use super::model::{
    ReliabilityAuditKind, ReliabilityTier, ReliabilityTierKind, ReliabilityTierStatus,
    SupplierReliabilityModel,
};
use crate::{
    i18n::{strings, Locale, LocaleStrings},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName, MetricCard},
    },
};
use dioxus::prelude::*;

fn tier_name(kind: ReliabilityTierKind, copy: &LocaleStrings) -> &'static str {
    match kind {
        ReliabilityTierKind::Community => copy.supplier_reliability_tier_community,
        ReliabilityTierKind::Verified => copy.supplier_reliability_tier_verified,
        ReliabilityTierKind::Professional => copy.supplier_reliability_tier_professional,
        ReliabilityTierKind::Strategic => copy.supplier_reliability_tier_strategic,
    }
}

fn tier_requirement(kind: ReliabilityTierKind, copy: &LocaleStrings) -> &'static str {
    match kind {
        ReliabilityTierKind::Community => copy.supplier_reliability_requirement_community,
        ReliabilityTierKind::Verified => copy.supplier_reliability_requirement_verified,
        ReliabilityTierKind::Professional => copy.supplier_reliability_requirement_professional,
        ReliabilityTierKind::Strategic => copy.supplier_reliability_requirement_strategic,
    }
}

fn tier_status(status: ReliabilityTierStatus, copy: &LocaleStrings) -> &'static str {
    match status {
        ReliabilityTierStatus::Completed => copy.supplier_reliability_status_completed,
        ReliabilityTierStatus::Current => copy.supplier_reliability_status_current,
        ReliabilityTierStatus::Eligible => copy.supplier_reliability_status_eligible,
    }
}

fn tier_class(status: ReliabilityTierStatus) -> &'static str {
    match status {
        ReliabilityTierStatus::Completed => "supplier-reliability-tier",
        ReliabilityTierStatus::Current => {
            "supplier-reliability-tier supplier-reliability-tier-current"
        }
        ReliabilityTierStatus::Eligible => {
            "supplier-reliability-tier supplier-reliability-tier-eligible"
        }
    }
}

fn audit_copy(
    kind: ReliabilityAuditKind,
    copy: &LocaleStrings,
) -> (&'static str, &'static str, &'static str, IconName) {
    match kind {
        ReliabilityAuditKind::Thermal => (
            copy.supplier_reliability_audit_thermal_title,
            copy.supplier_reliability_audit_thermal_detail,
            copy.supplier_reliability_status_mitigated,
            IconName::AlertTriangle,
        ),
        ReliabilityAuditKind::Driver => (
            copy.supplier_reliability_audit_driver_title,
            copy.supplier_reliability_audit_driver_detail,
            copy.supplier_reliability_status_planned,
            IconName::RefreshCw,
        ),
        ReliabilityAuditKind::Interconnect => (
            copy.supplier_reliability_audit_interconnect_title,
            copy.supplier_reliability_audit_interconnect_detail,
            copy.supplier_reliability_status_verified,
            IconName::Gauge,
        ),
    }
}

fn tier_card(tier: ReliabilityTier, copy: LocaleStrings) -> Element {
    rsx! {
        article { class: tier_class(tier.status),
            header { class: "supplier-reliability-tier-header",
                strong { {format!("{} {}", tier.level, tier_name(tier.kind, &copy))} }
                span { class: "supplier-reliability-tier-status", {tier_status(tier.status, &copy)} }
            }
            div { class: "supplier-reliability-share",
                strong { {tier.revenue_share} }
                span { {copy.supplier_reliability_rev_share} }
            }
            p { {tier_requirement(tier.kind, &copy)} }
        }
    }
}

#[component]
pub fn SupplierReliability() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let model = SupplierReliabilityModel::reference();

    rsx! {
        BuyerShell {
            div { class: "supplier-reliability-stack",
                section { class: "page-header supplier-reliability-page-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.supplier_reliability_title} }
                        p { {copy.supplier_reliability_subtitle} }
                    }
                    div { class: "conclusion conclusion-healthy supplier-reliability-conclusion", role: "status",
                        Icon { name: IconName::CheckCircle, size: 16 }
                        span { class: "conclusion-text", {copy.supplier_reliability_conclusion} }
                    }
                }
                section { class: "metric-grid supplier-reliability-metrics", aria_label: copy.supplier_reliability_title,
                    MetricCard {
                        label: String::new(),
                        value: model.score.to_string(),
                        unit: Some("/100".to_string()),
                        detail: copy.supplier_reliability_score_detail.to_string(),
                        badge: Some(copy.supplier_reliability_score_badge.to_string()),
                    }
                    MetricCard {
                        label: String::new(),
                        value: model.availability.to_string(),
                        detail: String::new(),
                        trend: Some(copy.supplier_reliability_availability_badge.to_string()),
                        trend_positive: true,
                    }
                    MetricCard {
                        label: String::new(),
                        value: model.current_tier.to_string(),
                        detail: copy.supplier_reliability_tier_detail.to_string(),
                    }
                    MetricCard {
                        label: String::new(),
                        value: model.revenue_share.to_string(),
                        detail: copy.supplier_reliability_share_detail.to_string(),
                    }
                }
                section { class: "panel supplier-reliability-tiers-panel", aria_label: copy.supplier_reliability_tiers_title,
                    h2 { class: "supplier-reliability-visually-hidden", {copy.supplier_reliability_tiers_title} }
                    div { class: "supplier-reliability-tier-grid",
                        for tier in model.tiers.iter().copied() {
                            {tier_card(tier, *copy)}
                        }
                    }
                }
                section { class: "panel supplier-reliability-audit-panel", aria_label: copy.supplier_reliability_audit_title,
                    h2 { class: "supplier-reliability-visually-hidden", {copy.supplier_reliability_audit_title} }
                    div { class: "supplier-reliability-audit-list",
                        for audit in model.audits.iter().copied() {
                            {
                                let (title, detail, status, icon) = audit_copy(audit.kind, copy);
                                rsx! {
                                    article { class: "supplier-reliability-audit-item",
                                        div { class: "supplier-reliability-audit-main",
                                            div { class: "supplier-reliability-audit-icon",
                                                Icon { name: icon, size: 16 }
                                            }
                                            div { class: "supplier-reliability-audit-copy",
                                                h3 { {format!("{} {}", title, audit.node)} }
                                                p { {detail} }
                                            }
                                        }
                                        span { class: "supplier-reliability-audit-status", {status} }
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
