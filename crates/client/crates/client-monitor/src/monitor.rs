use dioxus::prelude::*;

#[component]
pub fn ServiceMonitor() -> Element {
    // Mock Security Data
    let security_score = 94;
    let blocked_attacks = "1,204";
    let active_threats = 0;

    // Mock Threat Feed
    let threats = vec![
        ("10:42:15", "SQL Injection Attempt", "192.168.1.105", "High"),
        (
            "10:41:03",
            "Prompt Injection (Jailbreak)",
            "10.0.0.24",
            "Medium",
        ),
        ("10:35:22", "Rate Limit Exceeded", "172.16.0.4", "Low"),
        (
            "10:28:11",
            "NSFW Content Filtered",
            "192.168.1.200",
            "Medium",
        ),
        ("10:15:00", "Unknown User Agent", "45.33.22.11", "Low"),
    ];

    rsx! {
        div { class: "flex flex-col h-full gap-8",
            // Header
            div { class: "flex justify-between items-end",
                div {
                    h1 { class: "text-2xl font-semibold text-base-content mb-1 tracking-tight", "风控雷达" }
                    p { class: "text-sm text-base-content/60 font-medium", "实时威胁检测与内容安全防御" }
                }
                div { class: "flex gap-3",
                    button { class: "btn btn-ghost btn-sm text-base-content/70", "黑名单管理" }
                    button { class: "btn btn-error btn-sm px-6 text-white shadow-sm",
                        span { class: "loading loading-spinner loading-xs hidden" }
                        "紧急熔断"
                    }
                }
            }

            // Security HUD
            div { class: "grid grid-cols-4 gap-6",
                // Security Score
                div { class: "col-span-2 p-6 bg-base-100 rounded-xl border border-base-200 shadow-sm flex items-center justify-between relative overflow-hidden",
                    div { class: "flex flex-col gap-1 z-10",
                        span { class: "text-xs font-semibold text-base-content/40 uppercase tracking-wider", "当前安全评分" }
                        div { class: "flex items-baseline gap-3",
                            span { class: "text-5xl font-bold text-emerald-600 tracking-tighter", "{security_score}" }
                            span { class: "text-sm font-medium text-emerald-600/80", "安全状况良好" }
                        }
                    }
                    // Visual Decoration
                    div { class: "absolute right-0 top-0 h-full w-32 bg-gradient-to-l from-emerald-50 to-transparent opacity-50" }
                    div { class: "w-16 h-16 rounded-full border-4 border-emerald-100 flex items-center justify-center text-emerald-500",
                        svg { class: "w-8 h-8", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" }
                        }
                    }
                }

                // Blocked Attacks
                div { class: "p-5 bg-base-100 rounded-xl border border-base-200 shadow-sm flex flex-col gap-1",
                    span { class: "text-xs font-semibold text-base-content/40 uppercase tracking-wider", "已拦截攻击" }
                    div { class: "flex items-baseline gap-2",
                        span { class: "text-3xl font-bold text-base-content tracking-tight", "{blocked_attacks}" }
                        span { class: "text-xs font-medium text-red-500 bg-red-50 px-1.5 py-0.5 rounded", "+12 Today" }
                    }
                }

                // Active Threats
                div { class: "p-5 bg-base-100 rounded-xl border border-base-200 shadow-sm flex flex-col gap-1",
                    span { class: "text-xs font-semibold text-base-content/40 uppercase tracking-wider", "活跃威胁源" }
                    div { class: "flex items-baseline gap-2",
                        span { class: "text-3xl font-bold text-base-content tracking-tight", "{active_threats}" }
                        span { class: "text-xs font-medium text-base-content/40", "All Clear" }
                    }
                }
            }

            // Main Content Grid
            div { class: "grid grid-cols-3 gap-8",

                // Left: Live Threat Feed
                div { class: "col-span-2 flex flex-col gap-4",
                    h3 { class: "text-sm font-medium text-base-content/80 border-b border-base-content/10 pb-2", "实时威胁感知 (Live Threat Feed)" }

                    div { class: "flex flex-col gap-2",
                        for threat in threats {
                            div { class: "flex items-center justify-between p-3 bg-base-50/50 rounded-lg border border-base-200/50 hover:bg-white hover:shadow-sm transition-all group",
                                div { class: "flex items-center gap-4",
                                    span { class: "font-mono text-xs text-base-content/40", "{threat.0}" }
                                    div { class: "flex flex-col",
                                        span { class: "text-sm font-semibold text-base-content group-hover:text-red-600 transition-colors", "{threat.1}" }
                                        span { class: "text-xs text-base-content/50 font-mono", "Source: {threat.2}" }
                                    }
                                }
                                span {
                                    class: match threat.3 {
                                        "High" => "px-2 py-1 rounded text-xs font-bold bg-red-100 text-red-700 uppercase tracking-wide",
                                        "Medium" => "px-2 py-1 rounded text-xs font-bold bg-orange-100 text-orange-700 uppercase tracking-wide",
                                        _ => "px-2 py-1 rounded text-xs font-bold bg-base-200 text-base-content/60 uppercase tracking-wide",
                                    },
                                    "{threat.3}"
                                }
                            }
                        }
                    }
                }

                // Right: Content Safety Filters
                div { class: "col-span-1 flex flex-col gap-4",
                    h3 { class: "text-sm font-medium text-base-content/80 border-b border-base-content/10 pb-2", "内容风控策略" }

                    div { class: "flex flex-col gap-3",
                        // Filter 1
                        div { class: "p-4 border border-base-200 rounded-lg flex items-center justify-between",
                            div { class: "flex items-center gap-3",
                                div { class: "w-2 h-2 rounded-full bg-emerald-500" }
                                span { class: "text-sm font-medium", "敏感词过滤" }
                            }
                            input { type: "checkbox", class: "toggle toggle-success toggle-sm", checked: "true" }
                        }
                        // Filter 2
                        div { class: "p-4 border border-base-200 rounded-lg flex items-center justify-between",
                            div { class: "flex items-center gap-3",
                                div { class: "w-2 h-2 rounded-full bg-emerald-500" }
                                span { class: "text-sm font-medium", "政治敏感识别" }
                            }
                            input { type: "checkbox", class: "toggle toggle-success toggle-sm", checked: "true" }
                        }
                        // Filter 3
                        div { class: "p-4 border border-base-200 rounded-lg flex items-center justify-between",
                            div { class: "flex items-center gap-3",
                                div { class: "w-2 h-2 rounded-full bg-emerald-500" }
                                span { class: "text-sm font-medium", "PII 隐私保护" }
                            }
                            input { type: "checkbox", class: "toggle toggle-success toggle-sm", checked: "true" }
                        }
                        // Filter 4 (Disabled)
                        div { class: "p-4 border border-base-200 rounded-lg flex items-center justify-between opacity-60",
                            div { class: "flex items-center gap-3",
                                div { class: "w-2 h-2 rounded-full bg-base-300" }
                                span { class: "text-sm font-medium", "越狱攻击防护" }
                            }
                            input { type: "checkbox", class: "toggle toggle-sm" }
                        }
                    }

                    div { class: "mt-4 p-4 bg-blue-50 text-blue-800 rounded-lg text-xs leading-relaxed",
                        "💡 提示：开启隐私保护可能会略微增加请求延迟 (约 +50ms)。"
                    }
                }
            }
        }
    }
}
