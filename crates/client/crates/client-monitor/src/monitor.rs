use burncloud_client_shared::components::{
    BCBadge, BadgeVariant, EmptyState, ErrorBanner, PageHeader, Sparkline, StatusPill,
    SkeletonCard, SkeletonVariant,
};
use burncloud_client_shared::i18n::{t, t_fmt, Language};
use burncloud_client_shared::monitor_service::{
    FilterConfig, MonitorService, RiskEvent,
};
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

fn score_color(score: u8) -> &'static str {
    if score >= 80 {
        "var(--bc-success)"
    } else if score >= 50 {
        "var(--bc-warning)"
    } else {
        "var(--bc-danger)"
    }
}

fn filter_dot_bg(enabled: bool) -> &'static str {
    if enabled {
        "var(--bc-success)"
    } else {
        "var(--bc-border-hover)"
    }
}

fn score_label(score: u8, lang: Language) -> &'static str {
    if score >= 80 {
        t(lang, "monitor.score.good")
    } else if score >= 50 {
        t(lang, "monitor.score.attention")
    } else {
        t(lang, "monitor.score.high_risk")
    }
}

fn severity_pill(sev: &str) -> Element {
    let value = match sev {
        "critical" => "danger",
        "warning" => "warning",
        _ => "neutral",
    };
    rsx! { StatusPill { value: value.to_string(), label: Some(sev.to_string()) } }
}

#[component]
pub fn ServiceMonitor() -> Element {
    let i18n = burncloud_client_shared::i18n::use_i18n();
    let lang = i18n.language;
    let toast = use_toast();

    let mut summary_res = use_resource(move || async move {
        MonitorService::get_security_summary().await
    });
    let mut events_res = use_resource(move || async move {
        MonitorService::list_risk_events(1, 20).await
    });
    let mut filter_res = use_resource(move || async move {
        MonitorService::get_filter_config().await
    });

    let mut show_emergency_modal = use_signal(|| false);
    let mut emergency_reason = use_signal(String::new);
    let mut emergency_loading = use_signal(|| false);
    let mut emergency_error = use_signal(|| None::<String>);

    let summary_loading = summary_res.read().is_none();
    let events_loading = events_res.read().is_none();
    let filter_loading = filter_res.read().is_none();

    let summary = summary_res.read().as_ref().and_then(|r| r.as_ref().ok()).cloned();
    let summary_error = summary_res.read().as_ref().and_then(|r| r.as_ref().err()).cloned();
    let events_page = events_res.read().as_ref().and_then(|r| r.as_ref().ok()).cloned();
    let events_error = events_res.read().as_ref().and_then(|r| r.as_ref().err()).cloned();
    let filter_config = filter_res.read().as_ref().and_then(|r| r.as_ref().ok()).cloned();
    let filter_error = filter_res.read().as_ref().and_then(|r| r.as_ref().err()).cloned();

    let spark_data: Vec<f64> = summary
        .as_ref()
        .map(|s| s.sparkline.iter().map(|&v| v as f64).collect())
        .unwrap_or_default();

    let score = summary.as_ref().map(|s| s.score).unwrap_or(0);
    let blocked_count = summary.as_ref().map(|s| s.blocked_count).unwrap_or(0);
    let threat_count = summary.as_ref().map(|s| s.threat_source_count).unwrap_or(0);
    let events: Vec<RiskEvent> = events_page.map(|p| p.data).unwrap_or_default();

    let mut content_filter_enabled = use_signal(|| filter_config.as_ref().map(|c| c.content_filter_enabled).unwrap_or(true));
    let mut blacklist_enabled = use_signal(|| filter_config.as_ref().map(|c| c.blacklist_enabled).unwrap_or(true));

    // Sync signals when filter config loads
    if let Some(cfg) = &filter_config {
        if content_filter_enabled() != cfg.content_filter_enabled {
            content_filter_enabled.set(cfg.content_filter_enabled);
        }
        if blacklist_enabled() != cfg.blacklist_enabled {
            blacklist_enabled.set(cfg.blacklist_enabled);
        }
    }

    let custom_rules = use_memo(move || {
        filter_config.as_ref().map(|c| c.custom_rules.clone()).unwrap_or_default()
    });

    let update_filter = move |new_cf: bool, new_bl: bool| {
        let prev_cf = content_filter_enabled();
        let prev_bl = blacklist_enabled();
        let cfg = FilterConfig {
            content_filter_enabled: new_cf,
            blacklist_enabled: new_bl,
            custom_rules: custom_rules(),
        };
        spawn(async move {
            if MonitorService::update_filter_config(&cfg).await.is_ok() {
                filter_res.restart();
            } else {
                // Rollback on failure
                content_filter_enabled.set(prev_cf);
                blacklist_enabled.set(prev_bl);
                toast.error(t(*lang.read(), "monitor.filter.save_failed_rollback"));
            }
        });
    };

    let handle_emergency = move |_| {
        let reason = emergency_reason().trim().to_string();
        if reason.is_empty() {
            emergency_error.set(Some(t(*lang.read(), "monitor.emergency.reason_required").to_string()));
            return;
        }
        emergency_loading.set(true);
        emergency_error.set(None);
        spawn(async move {
            match MonitorService::emergency_circuit_break(reason).await {
                Ok(_) => {
                    emergency_loading.set(false);
                    show_emergency_modal.set(false);
                    emergency_reason.set(String::new());
                    summary_res.restart();
                    events_res.restart();
                    toast.success(t(*lang.read(), "monitor.emergency.executed"));
                }
                Err(e) => {
                    emergency_loading.set(false);
                    emergency_error.set(Some(e.clone()));
                    toast.error(&t_fmt(*lang.read(), "monitor.emergency.execute_failed", &[("error", &e.to_string())]));
                }
            }
        });
    };

    rsx! {
        PageHeader {
            title: t(*lang.read(), "monitor.title"),
            subtitle: Some(t(*lang.read(), "monitor.subtitle").to_string()),
            actions: rsx! {
                button { class: "btn btn-secondary", disabled: true, title: t(*lang.read(), "monitor.blacklist_mgmt_tooltip"), {t(*lang.read(), "monitor.blacklist_mgmt")} }
                button {
                    class: "btn btn-danger bc-btn-emergency",
                    onclick: move |_| show_emergency_modal.set(true),
                    {t(*lang.read(), "monitor.emergency.button")}
                }
            },
        }

        // Emergency circuit break modal
        {rsx! {
            div {
                class: "fixed inset-0 z-50 flex items-center justify-center",
                style: if show_emergency_modal() { "--bc-dynamic-display:flex" } else { "--bc-dynamic-display:none" },

                div {
                    class: "absolute inset-0 bg-[rgba(0,0,0,0.4)] backdrop-blur-sm",
                    onclick: move |_| show_emergency_modal.set(false),
                }

                div {
                    class: "bc-card-solid relative z-10 w-full max-w-lg mx-md animate-scale-in",
                    onclick: |e| e.stop_propagation(),

                    div { class: "flex items-center justify-between p-lg border-b border-[var(--bc-border)]",
                        h3 { class: "text-subtitle font-bold text-primary m-0", {t(*lang.read(), "monitor.emergency.confirm_title")} }
                        button {
                            class: "btn-subtle w-8 h-8 flex items-center justify-center rounded-full text-lg",
                            onclick: move |_| show_emergency_modal.set(false),
                            "✕"
                        }
                    }

                    div { class: "p-lg",
                        div { class: "bc-modal-warning",
                            {t(*lang.read(), "monitor.emergency.warning")}
                        }

                        div { class: "bc-modal-form-row",
                            label { class: "bc-modal-form-label", {t(*lang.read(), "monitor.emergency.reason_label")} }
                            input {
                                class: "bc-input w-full",
                                r#type: "text",
                                value: "{emergency_reason}",
                                oninput: move |e| emergency_reason.set(e.value()),
                                placeholder: t(*lang.read(), "monitor.emergency.reason_placeholder"),
                                disabled: emergency_loading(),
                            }
                        }

                        if let Some(err) = emergency_error() {
                            div { class: "bc-error-text mb-md", "{err}" }
                        }

                        div { class: "bc-flex-row-end",
                            button {
                                class: "btn btn-secondary",
                                onclick: move |_| show_emergency_modal.set(false),
                                disabled: emergency_loading(),
                                {t(*lang.read(), "common.cancel")}
                            }
                            button {
                                class: "btn btn-danger",
                                onclick: handle_emergency,
                                disabled: emergency_loading(),
                                if emergency_loading() { {t(*lang.read(), "monitor.emergency.executing")} } else { {t(*lang.read(), "monitor.emergency.confirm")} }
                            }
                        }
                    }
                }
            }
        }}

        div { class: "page-content flex flex-col bc-gap-6",

            // Security HUD: 4-col grid, security score spans 2
            div { class: "stats-grid cols-4",
                if summary_loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else if let Some(err) = &summary_error {
                    div { class: "stat-card bc-col-span-4",
                        ErrorBanner {
                            message: t_fmt(*lang.read(), "monitor.summary.load_failed", &[("error", &err.to_string())]),
                            on_retry: None,
                        }
                    }
                } else {
                    // Security score card (span 2)
                    div { class: "stat-card bc-score-card",
                        style: "--bc-dynamic-color:{score_color(score)}; --bc-dynamic-bg:linear-gradient(to left, {score_color(score)}22, transparent); --bc-dynamic-border-color:{score_color(score)}33",
                        div { class: "bc-score-glow bc-dynamic-bg bc-dynamic-opacity", }
                        div { class: "bc-score-body",
                            span { class: "stat-eyebrow", {t(*lang.read(), "monitor.score.current")} }
                            div { class: "flex items-baseline gap-lg",
                                span { class: "bc-score-value bc-dynamic-color", "{score}" }
                                span { class: "bc-score-label bc-dynamic-color", "{score_label(score, *lang.read())}" }
                            }
                            div { class: "flex items-center gap-sm bc-mt-6",
                                span { class: "bc-eyebrow", "7d" }
                                Sparkline { data: spark_data, tone: Some("success".to_string()), sm: Some(true) }
                            }
                        }
                        div { class: "bc-score-shield bc-dynamic-color",
                            style: "--bc-dynamic-border-color:{score_color(score)}33",
                            "🛡"
                        }
                    }

                    // Intercepted attacks
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", {t(*lang.read(), "monitor.stats.blocked_attacks")} }
                        div { class: "stat-value",
                            "{blocked_count}"
                        }
                    }

                    // Active threat sources
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", {t(*lang.read(), "monitor.stats.active_threats")} }
                        div { class: "stat-value",
                            "{threat_count}"
                            if threat_count == 0 {
                                BCBadge { variant: BadgeVariant::Success, "All Clear" }
                            }
                        }
                    }
                }
            }

            // Two-column: threat feed | filters
            div { class: "bc-grid-2-1 bc-gap-6",
                // Threat feed
                div {
                    div { class: "section-h",
                        span { class: "lead-title", {t(*lang.read(), "monitor.threat_feed.title")} }
                    }
                    if events_loading {
                        div { class: "flex flex-col gap-sm",
                            SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                            SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                            SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                        }
                    } else if let Some(err) = &events_error {
                        ErrorBanner {
                            message: t_fmt(*lang.read(), "monitor.events.load_failed", &[("error", &err.to_string())]),
                            on_retry: None,
                        }
                    } else if events.is_empty() {
                        EmptyState {
                            icon: rsx! { span { class: "bc-font-emoji-sm", "🛡" } },
                            title: t(*lang.read(), "monitor.events.empty_title").to_string(),
                            description: Some(t(*lang.read(), "monitor.events.empty_desc").to_string()),
                        }
                    } else {
                        div { class: "flex flex-col gap-sm",
                            for event in events.iter() {
                                div { class: "row-card outlined p-md",
                                    key: "{event.id}",
                                    div { class: "flex items-center gap-lg",
                                        span { class: "mono bc-font-11 text-tertiary", "{event.time}" }
                                        div { class: "flex flex-col bc-gap-xs",
                                            span { class: "bc-font-13 font-semibold", "{event.event_type}" }
                                            span { class: "mono bc-font-11 text-tertiary", "Source: {event.source} → {event.target} ({event.detail})" }
                                        }
                                    }
                                    {severity_pill(&event.severity)}
                                }
                            }
                        }
                    }
                }

                // Content filter switches
                div {
                    div { class: "section-h",
                        span { class: "lead-title", {t(*lang.read(), "monitor.filter.title")} }
                    }
                    if filter_loading {
                        SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    } else if let Some(err) = &filter_error {
                        ErrorBanner {
                            message: t_fmt(*lang.read(), "monitor.filter.load_failed", &[("error", &err.to_string())]),
                            on_retry: None,
                        }
                    } else {
                        div { class: "flex flex-col gap-md",
                            // Content filter
                            div { class: "row-card outlined",
                                style: if !content_filter_enabled() { "--bc-dynamic-opacity:0.6" } else { "" },
                                div { class: "flex items-center gap-md",
                                    span { class: "bc-status-dot",
                                        style: "--bc-dynamic-bg:{filter_dot_bg(content_filter_enabled())}",
                                    }
                                    span { class: "bc-font-13 font-medium", {t(*lang.read(), "monitor.filter.content_filter")} }
                                }
                                label { class: "switch",
                                    input {
                                        r#type: "checkbox",
                                        checked: content_filter_enabled(),
                                        onchange: move |_| {
                                            let new_val = !content_filter_enabled();
                                            content_filter_enabled.set(new_val);
                                            update_filter(new_val, blacklist_enabled());
                                        }
                                    }
                                    span { class: "switch-track" }
                                }
                            }
                            // Blacklist
                            div { class: "row-card outlined",
                                style: if !blacklist_enabled() { "--bc-dynamic-opacity:0.6" } else { "" },
                                div { class: "flex items-center gap-md",
                                    span { class: "bc-status-dot",
                                        style: "--bc-dynamic-bg:{filter_dot_bg(blacklist_enabled())}",
                                    }
                                    span { class: "bc-font-13 font-medium", {t(*lang.read(), "monitor.filter.blacklist")} }
                                }
                                label { class: "switch",
                                    input {
                                        r#type: "checkbox",
                                        checked: blacklist_enabled(),
                                        onchange: move |_| {
                                            let new_val = !blacklist_enabled();
                                            blacklist_enabled.set(new_val);
                                            update_filter(content_filter_enabled(), new_val);
                                        }
                                    }
                                    span { class: "switch-track" }
                                }
                            }
                        }
                        // Info tip
                        div { class: "bc-info-tip",
                            {t(*lang.read(), "monitor.filter.latency_tip")}
                        }
                    }
                }
            }
        }
    }
}