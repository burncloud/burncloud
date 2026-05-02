use burncloud_client_shared::components::{
    PageHeader,
    SkeletonCard, SkeletonVariant,
};
use dioxus::prelude::*;

fn sev_pill(level: &str) -> Element {
    let (cls, label) = match level {
        "High" => ("danger", "HIGH"),
        "Medium" => ("warning", "MEDIUM"),
        "Low" => ("neutral", "LOW"),
        _ => ("neutral", level),
    };
    rsx! {
        span { class: "pill {cls}", style: "font-family:var(--bc-font-mono); font-size:11px; padding:2px 8px; letter-spacing:0.04em",
            "{label}"
        }
    }
}

#[component]
pub fn ServiceMonitor() -> Element {
    let loading = false;

    // Threat feed matching design spec
    let threats = vec![
        ("10:42:15", "SQL Injection Attempt",        "192.168.1.105", "High"),
        ("10:41:03", "Prompt Injection (Jailbreak)", "10.0.0.24",     "Medium"),
        ("10:35:22", "Rate Limit Exceeded",          "172.16.0.4",    "Low"),
        ("10:28:11", "NSFW Content Filtered",        "192.168.1.200", "Medium"),
        ("10:15:00", "Unknown User Agent",           "45.33.22.11",   "Low"),
    ];

    // Content filter switches
    let mut filter_sensitive = use_signal(|| true);
    let mut filter_political = use_signal(|| true);
    let mut filter_pii = use_signal(|| true);
    let mut filter_jailbreak = use_signal(|| false);

    let spark_security = [78.0, 82.0, 80.0, 86.0, 88.0, 90.0, 94.0];

    rsx! {
        PageHeader {
            title: "风控雷达",
            subtitle: Some("实时威胁检测与内容安全防御".to_string()),
            actions: rsx! {
                button { class: "btn btn-secondary", "黑名单管理" }
                button { class: "btn btn-danger", style: "padding-left:24px; padding-right:24px", "紧急熔断" }
            },
        }

        div { class: "page-content", style: "display:flex; flex-direction:column; gap:24px",

            // Security HUD: 4-col grid, security score spans 2
            div { class: "stats-grid cols-4",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    // Security score card (span 2)
                    div { class: "stat-card", style: "grid-column:span 2; flex-direction:row; align-items:center; justify-content:space-between; padding:24px; position:relative; overflow:hidden",
                        // Gradient glow
                        div { style: "position:absolute; right:0; top:0; bottom:0; width:160px; background:linear-gradient(to left, var(--bc-success-light), transparent); opacity:0.45; pointer-events:none" }
                        div { style: "display:flex; flex-direction:column; gap:6px; z-index:1",
                            span { class: "stat-eyebrow", "当前安全评分" }
                            div { style: "display:flex; align-items:baseline; gap:16px",
                                span { style: "font-size:56px; font-weight:700; letter-spacing:-0.03em; line-height:1; color:var(--bc-success)", "94" }
                                span { style: "font-size:13px; font-weight:500; color:var(--bc-success)", "安全状况良好" }
                            }
                            div { style: "display:flex; align-items:center; gap:8px; margin-top:6px",
                                span { style: "font-size:11px; color:var(--bc-text-tertiary); text-transform:uppercase; letter-spacing:0.16em", "7d" }
                                div { class: "spark sm", style: "width:120px; height:24px",
                                    for (i, v) in spark_security.iter().enumerate() {
                                        {
                                            let idx = i;
                                            let opacity = 0.4 + idx as f64 * 0.1;
                                            let height_pct = *v;
                                            rsx! {
                                                div {
                                                    key: "{idx}",
                                                    class: "bar success",
                                                    style: "height:{height_pct}%; opacity:{opacity}",
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // Shield icon circle
                        div { style: "width:64px; height:64px; border-radius:99px; border:4px solid var(--bc-success-light); color:var(--bc-success); display:flex; align-items:center; justify-content:center; z-index:1; font-size:28px",
                            "🛡"
                        }
                    }

                    // Intercepted attacks
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "已拦截攻击" }
                        div { class: "stat-value",
                            "1,204 "
                            span { class: "stat-pill danger", "+12 Today" }
                        }
                    }

                    // Active threat sources
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "活跃威胁源" }
                        div { class: "stat-value",
                            "0 "
                            span { class: "stat-pill muted", "All Clear" }
                        }
                    }
                }
            }

            // Two-column: threat feed | filters
            div { style: "display:grid; grid-template-columns:2fr 1fr; gap:24px",
                // Threat feed
                div {
                    div { class: "section-h",
                        span { class: "lead-title", "实时威胁感知 (Live Threat Feed)" }
                    }
                    div { style: "display:flex; flex-direction:column; gap:8px",
                        for (time, kind, src, sev) in &threats {
                            div { class: "row-card outlined", style: "padding:16px",
                                div { style: "display:flex; align-items:center; gap:16px",
                                    span { class: "mono", style: "font-size:11px; color:var(--bc-text-tertiary)", "{time}" }
                                    div { style: "display:flex; flex-direction:column; gap:2px",
                                        span { style: "font-size:13px; font-weight:600", "{kind}" }
                                        span { class: "mono", style: "font-size:11px; color:var(--bc-text-tertiary)", "Source: {src}" }
                                    }
                                }
                                {sev_pill(sev)}
                            }
                        }
                    }
                }

                // Content filter switches
                div {
                    div { class: "section-h",
                        span { class: "lead-title", "内容风控策略" }
                    }
                    div { style: "display:flex; flex-direction:column; gap:12px",
                        // Filter row: 敏感词过滤
                        div { class: "row-card outlined", style: if !filter_sensitive() { "opacity:0.6" } else { "" },
                            div { style: "display:flex; align-items:center; gap:12px",
                                span { style: "width:8px; height:8px; border-radius:99px; background:if filter_sensitive() {{ \"var(--bc-success)\" }} else {{ \"var(--bc-border-hover)\" }}" }
                                span { style: "font-size:13px; font-weight:500", "敏感词过滤" }
                            }
                            label { class: "switch",
                                input { r#type: "checkbox", checked: filter_sensitive(), onchange: move |_| filter_sensitive.set(!filter_sensitive()) }
                                span { class: "switch-track" }
                            }
                        }
                        // Filter row: 政治敏感识别
                        div { class: "row-card outlined", style: if !filter_political() { "opacity:0.6" } else { "" },
                            div { style: "display:flex; align-items:center; gap:12px",
                                span { style: "width:8px; height:8px; border-radius:99px; background:if filter_political() {{ \"var(--bc-success)\" }} else {{ \"var(--bc-border-hover)\" }}" }
                                span { style: "font-size:13px; font-weight:500", "政治敏感识别" }
                            }
                            label { class: "switch",
                                input { r#type: "checkbox", checked: filter_political(), onchange: move |_| filter_political.set(!filter_political()) }
                                span { class: "switch-track" }
                            }
                        }
                        // Filter row: PII 隐私保护
                        div { class: "row-card outlined", style: if !filter_pii() { "opacity:0.6" } else { "" },
                            div { style: "display:flex; align-items:center; gap:12px",
                                span { style: "width:8px; height:8px; border-radius:99px; background:if filter_pii() {{ \"var(--bc-success)\" }} else {{ \"var(--bc-border-hover)\" }}" }
                                span { style: "font-size:13px; font-weight:500", "PII 隐私保护" }
                            }
                            label { class: "switch",
                                input { r#type: "checkbox", checked: filter_pii(), onchange: move |_| filter_pii.set(!filter_pii()) }
                                span { class: "switch-track" }
                            }
                        }
                        // Filter row: 越狱攻击防护
                        div { class: "row-card outlined", style: if !filter_jailbreak() { "opacity:0.6" } else { "" },
                            div { style: "display:flex; align-items:center; gap:12px",
                                span { style: "width:8px; height:8px; border-radius:99px; background:if filter_jailbreak() {{ \"var(--bc-success)\" }} else {{ \"var(--bc-border-hover)\" }}" }
                                span { style: "font-size:13px; font-weight:500", "越狱攻击防护" }
                            }
                            label { class: "switch",
                                input { r#type: "checkbox", checked: filter_jailbreak(), onchange: move |_| filter_jailbreak.set(!filter_jailbreak()) }
                                span { class: "switch-track" }
                            }
                        }
                    }
                    // Info tip
                    div { style: "margin-top:16px; padding:16px; font-size:12px; line-height:1.6; background:var(--bc-info-light); color:var(--bc-info); border-radius:12px",
                        "💡 提示：开启隐私保护可能会略微增加请求延迟 (约 +50ms)。"
                    }
                }
            }
        }
    }
}
