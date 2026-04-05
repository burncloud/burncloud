use burncloud_client_shared::components::{BCBadge, BCButton, BCCard, BadgeVariant, ButtonVariant};
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
        div { class: "flex flex-col h-full gap-xl",
            // Header
            div { class: "flex justify-between items-center",
                div {
                    h1 { class: "text-large-title font-semibold text-primary mb-xs tracking-tight", "风控雷达" }
                    p { class: "text-caption text-secondary font-medium", "实时威胁检测与内容安全防御" }
                }
                div { class: "flex gap-sm",
                    BCButton {
                        variant: ButtonVariant::Ghost,
                        size: burncloud_client_shared::components::ButtonSize::Small,
                        "黑名单管理"
                    }
                    BCButton {
                        variant: ButtonVariant::Danger,
                        size: burncloud_client_shared::components::ButtonSize::Small,
                        class: "px-xl",
                        span { class: "loading loading-spinner loading-xs hidden" }
                        "紧急熔断"
                    }
                }
            }

            // Security HUD
            div { class: "grid grid-cols-4 gap-md",
                // Security Score
                BCCard {
                    class: "col-span-2 p-lg flex items-center justify-between relative overflow-hidden",
                    div { class: "flex flex-col gap-xs z-10",
                        span { class: "text-xxs font-semibold text-tertiary uppercase tracking-wider", "当前安全评分" }
                        div { class: "flex items-baseline gap-md",
                            span { class: "text-display font-bold tracking-tighter",
                                style: "color: var(--bc-success)",
                                "{security_score}"
                            }
                            span { class: "text-caption font-medium",
                                style: "color: var(--bc-success)",
                                "安全状况良好"
                            }
                        }
                    }
                    // Visual Decoration
                    div {
                        class: "absolute right-0 top-0 h-full",
                        style: "width: 128px; background: linear-gradient(to left, var(--bc-success-light), transparent); opacity: 0.5;",
                    }
                    div {
                        class: "flex items-center justify-center",
                        style: "width: 64px; height: 64px; border-radius: 9999px; border: 4px solid var(--bc-success-light); color: var(--bc-success);",
                        svg { class: "", style: "width: 32px; height: 32px;", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" }
                        }
                    }
                }

                // Blocked Attacks
                BCCard {
                    class: "flex flex-col gap-xs",
                    span { class: "text-xxs font-semibold text-tertiary uppercase tracking-wider", "已拦截攻击" }
                    div { class: "flex items-baseline gap-sm",
                        span { class: "text-title font-bold text-primary tracking-tight", "{blocked_attacks}" }
                        span {
                            class: "text-xxs font-medium px-xs rounded",
                            style: "color: var(--bc-danger); background: var(--bc-danger-light); padding-top: 2px; padding-bottom: 2px;",
                            "+12 Today"
                        }
                    }
                }

                // Active Threats
                BCCard {
                    class: "flex flex-col gap-xs",
                    span { class: "text-xxs font-semibold text-tertiary uppercase tracking-wider", "活跃威胁源" }
                    div { class: "flex items-baseline gap-sm",
                        span { class: "text-title font-bold text-primary tracking-tight", "{active_threats}" }
                        span { class: "text-xxs font-medium text-tertiary", "All Clear" }
                    }
                }
            }

            // Main Content Grid
            div { class: "grid grid-cols-3 gap-xl",

                // Left: Live Threat Feed
                div { class: "col-span-2 flex flex-col gap-md",
                    h3 {
                        class: "text-caption font-medium text-secondary pb-sm border-b",
                        "实时威胁感知 (Live Threat Feed)"
                    }

                    div { class: "flex flex-col gap-sm",
                        for threat in threats {
                            BCCard {
                                variant: burncloud_client_shared::components::CardVariant::Outlined,
                                class: "flex items-center justify-between p-md hover:shadow-sm transition-all group",
                                div { class: "flex items-center gap-md",
                                    span { class: "text-xxs text-tertiary", style: "font-family: monospace;", "{threat.0}" }
                                    div { class: "flex flex-col",
                                        span { class: "text-caption font-semibold text-primary group-hover:text-primary transition-colors",
                                            "{threat.1}"
                                        }
                                        span { class: "text-xxs text-tertiary", style: "font-family: monospace;", "Source: {threat.2}" }
                                    }
                                }
                                {
                                    let badge_variant = match threat.3 {
                                        "High" => BadgeVariant::Danger,
                                        "Medium" => BadgeVariant::Warning,
                                        _ => BadgeVariant::Neutral,
                                    };
                                    rsx! {
                                        BCBadge {
                                            variant: badge_variant,
                                            class: "uppercase tracking-wide",
                                            "{threat.3}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Right: Content Safety Filters
                div { class: "col-span-1 flex flex-col gap-md",
                    h3 {
                        class: "text-caption font-medium text-secondary pb-sm border-b",
                        "内容风控策略"
                    }

                    div { class: "flex flex-col gap-md",
                        // Filter 1
                        BCCard {
                            variant: burncloud_client_shared::components::CardVariant::Outlined,
                            class: "flex items-center justify-between",
                            div { class: "flex items-center gap-md",
                                div { class: "", style: "width: 8px; height: 8px; border-radius: 9999px; background: var(--bc-success);" }
                                span { class: "text-caption font-medium", "敏感词过滤" }
                            }
                            input { type: "checkbox", class: "toggle toggle-success toggle-sm", checked: "true" }
                        }
                        // Filter 2
                        BCCard {
                            variant: burncloud_client_shared::components::CardVariant::Outlined,
                            class: "flex items-center justify-between",
                            div { class: "flex items-center gap-md",
                                div { class: "", style: "width: 8px; height: 8px; border-radius: 9999px; background: var(--bc-success);" }
                                span { class: "text-caption font-medium", "政治敏感识别" }
                            }
                            input { type: "checkbox", class: "toggle toggle-success toggle-sm", checked: "true" }
                        }
                        // Filter 3
                        BCCard {
                            variant: burncloud_client_shared::components::CardVariant::Outlined,
                            class: "flex items-center justify-between",
                            div { class: "flex items-center gap-md",
                                div { class: "", style: "width: 8px; height: 8px; border-radius: 9999px; background: var(--bc-success);" }
                                span { class: "text-caption font-medium", "PII 隐私保护" }
                            }
                            input { type: "checkbox", class: "toggle toggle-success toggle-sm", checked: "true" }
                        }
                        // Filter 4 (Disabled)
                        BCCard {
                            variant: burncloud_client_shared::components::CardVariant::Outlined,
                            class: "flex items-center justify-between opacity-60",
                            div { class: "flex items-center gap-md",
                                div { class: "", style: "width: 8px; height: 8px; border-radius: 9999px; background: var(--bc-border-hover);" }
                                span { class: "text-caption font-medium", "越狱攻击防护" }
                            }
                            input { type: "checkbox", class: "toggle toggle-sm" }
                        }
                    }

                    div {
                        class: "mt-md p-lg text-caption leading-relaxed",
                        style: "background: var(--bc-info-light); color: var(--bc-info); border-radius: var(--bc-radius-md);",
                        "💡 提示：开启隐私保护可能会略微增加请求延迟 (约 +50ms)。"
                    }
                }
            }
        }
    }
}
