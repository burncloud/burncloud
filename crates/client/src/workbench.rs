use dioxus::prelude::*;

use crate::{app::Route, components::Icon};

#[component]
pub fn WorkbenchLayout() -> Element {
    let route = use_route::<Route>();
    rsx! {
        div { class: "wb-app",
            aside { class: "wb-side",
                div { class: "wb-brand",
                    div { class: "wb-brand-mark", "B" }
                    div { class: "wb-brand-text",
                        div { class: "wb-brand-name", "BurnCloud" }
                        div { class: "wb-brand-role", "开发者 / 算力买方 · Pro" }
                    }
                }
                div { class: "wb-search",
                    Icon { name: "search" }
                    input {
                        r#type: "text",
                        placeholder: "搜索模型、API 接口、Token审计日志（如 DeepSeek, 延迟, key)...",
                        readonly: true,
                    }
                }
                div { class: "wb-nav-caption", "方舟全流程、模型市场、演练场测试、API Key 管理、用量与托管金" }
                nav { class: "wb-nav",
                    NavRow { route: Route::Overview {}, icon: "overview", label: "工作台概览", current: route.clone() }
                    NavRow { route: Route::Playground {}, icon: "terminal", label: "实时操练场", badge: Some("实时"), current: route.clone() }
                    NavRow { route: Route::Models {}, icon: "globe", label: "模型市场", current: route.clone() }
                    NavRow { route: Route::APIKeys {}, icon: "key", label: "API 密钥管理", current: route.clone() }
                    NavRow { route: Route::Evaluation {}, icon: "chart", label: "用量与消耗分析", current: route.clone() }
                    NavRow { route: Route::Billing {}, icon: "billing", label: "财务与充值托管", current: route.clone() }
                    NavRow { route: Route::Logs {}, icon: "routes", label: "请求调用日志", current: route.clone() }
                }
                div { class: "wb-side-foot",
                    div { class: "wb-balance-mini",
                        div { class: "wb-balance-mini-head",
                            span { "预付托管金余额" }
                            button { class: "wb-btn-topup", "+ Top Up" }
                        }
                        div { class: "wb-balance-mini-amount", "$128.50" }
                        div { class: "wb-balance-mini-foot",
                            div { class: "wb-portal-inline",
                                Icon { name: "globe" }
                                span { "门户主页" }
                            }
                            div { class: "wb-sla-inline",
                                span { class: "wb-dot" }
                                "99.999% SLA"
                            }
                        }
                    }
                }
            }
            div { class: "wb-main-col",
                header { class: "wb-top",
                    div { class: "wb-search-top",
                        Icon { name: "search" }
                        span { "搜索模型、API 接口、Token审计日志（如 DeepSeek, 延迟, key)..." }
                    }
                    div { class: "wb-top-right",
                        div { class: "wb-status-pill",
                            span { class: "wb-dot" }
                            span { "Autopilot 调度运行中" }
                        }
                        div { class: "wb-lang-pill",
                            span { class: "wb-lang-flag" }
                            span { "CN" }
                            span { "简体中文" }
                        }
                        div { class: "wb-user",
                            div { class: "wb-user-avatar", "B" }
                            div { class: "wb-user-text",
                                div { class: "wb-user-name", "burncloud.com" }
                                div { class: "wb-user-role", "开发者 / 算力买方 · Pro" }
                            }
                        }
                    }
                }
                main { class: "wb-content",
                    Workbench {}
                }
            }
        }
    }
}

#[component]
fn NavRow(route: Route, icon: &'static str, label: &'static str, badge: Option<&'static str>, current: Route) -> Element {
    let active = current == route;
    rsx! {
        Link {
            to: route,
            class: if active { "wb-nav-item active" } else { "wb-nav-item" },
            Icon { name: icon }
            span { class: "wb-nav-label", "{label}" }
            if let Some(b) = badge {
                span { class: "wb-nav-badge", "{b}" }
            }
        }
    }
}

#[component]
pub fn Workbench() -> Element {
    rsx! {
        div { class: "wb-page",
            div { class: "wb-page-head",
                div {
                    h1 { class: "wb-title", "开发者工作台概览" }
                    p { class: "wb-subtitle", "实时监控 Token 支出、预付托管金余额、在线模型路由及全球 P95 响应延迟。" }
                }
                div { class: "wb-head-actions",
                    button { class: "wb-btn-ghost", Icon { name: "terminal" }, "体验实时操练场" }
                    button { class: "wb-btn-primary", "探索模型市场" }
                }
            }

            div { class: "wb-banner",
                div { class: "wb-banner-left",
                    span { class: "wb-banner-icon", Icon { name: "check" } }
                    span { class: "wb-banner-text", "所有活跃模型路由均正常运行，已通过硬件密码学真实性校验。" }
                }
            }

            div { class: "wb-kpi-grid",
                div { class: "wb-card wb-kpi-card",
                    div { class: "wb-kpi-label", "今日 TOKEN 消耗" }
                    div { class: "wb-kpi-value", "$14.28" }
                    div { class: "wb-kpi-meta",
                        span { class: "wb-kpi-delta up", "+8.4% vs yesterday" }
                        span { class: "wb-kpi-note", "从预付托管金中扣除" }
                    }
                }
                div { class: "wb-card wb-kpi-card",
                    div { class: "wb-kpi-label-row",
                        span { "预付托管金余额" }
                        span { class: "wb-tag healthy", "HEALTHY" }
                    }
                    div { class: "wb-kpi-value", "$128.50" }
                    div { class: "wb-kpi-meta",
                        span { class: "wb-kpi-note", "预计可支撑 14 天用量" }
                    }
                }
                div { class: "wb-card wb-kpi-card",
                    div { class: "wb-kpi-label-row",
                        span { "API 服务可用率" }
                        span { class: "wb-tag healthy", "Healthy" }
                    }
                    div { class: "wb-kpi-value", "99.99%" }
                    div { class: "wb-kpi-meta",
                        span { class: "wb-kpi-note", "覆盖全量活跃路由" }
                    }
                }
                div { class: "wb-card wb-kpi-card",
                    div { class: "wb-kpi-label", "今日生成 TOKEN 数" }
                    div { class: "wb-kpi-value", "1.84M" }
                    div { class: "wb-kpi-meta",
                        span { class: "wb-kpi-unit", "tokens" }
                    }
                    div { class: "wb-kpi-extra",
                        div { class: "wb-extra-box",
                            span { class: "wb-extra-main", "85.4 req/min" }
                            span { class: "wb-extra-sub", "peak" }
                        }
                        div { class: "wb-extra-line", "620K 流入 +1.22M 输出" }
                    }
                }
            }

            div { class: "wb-card wb-table-card",
                div { class: "wb-table-head",
                    div {
                        div { class: "wb-table-title", "正在调用的模型路由" }
                        div { class: "wb-table-sub", "当前为您生产应用提供实时流量调度的主力模型。" }
                    }
                    a { class: "wb-link", "查看全部模型 →" }
                }
                table { class: "wb-table",
                    thead {
                        tr {
                            th { "模型名称" }
                            th { "优化等级" }
                            th { "今日 TOKEN 数" }
                            th { "P95 延迟" }
                            th { "今日费用" }
                            th { "服务状态" }
                            th { class: "wb-th-actions", "操作" }
                        }
                    }
                    tbody {
                        tr {
                            td {
                                div { class: "wb-model",
                                    div { class: "wb-model-avatar", "D" }
                                    div { class: "wb-model-text",
                                        div { class: "wb-model-name", "DeepSeek V3 (671B MoE)" }
                                        div { class: "wb-model-provider", "DeepSeek" }
                                    }
                                }
                            }
                            td { span { class: "wb-tier standard", "STANDARD" } }
                            td { "1,120,400" }
                            td { "388 ms" }
                            td { "$0.28" }
                            td { span { class: "wb-status healthy", Icon { name: "check" }, "Healthy" } }
                            td { a { class: "wb-action-link", "在操练场调试 →" } }
                        }
                        tr {
                            td {
                                div { class: "wb-model",
                                    div { class: "wb-model-avatar", "D" }
                                    div { class: "wb-model-text",
                                        div { class: "wb-model-name", "DeepSeek R1 Reasoning" }
                                        div { class: "wb-model-provider", "DeepSeek" }
                                    }
                                }
                            }
                            td { span { class: "wb-tier performance", "PERFORMANCE" } }
                            td { "410,200" }
                            td { "628 ms" }
                            td { "$0.89" }
                            td { span { class: "wb-status healthy", Icon { name: "check" }, "Healthy" } }
                            td { a { class: "wb-action-link", "在操练场调试 →" } }
                        }
                        tr {
                            td {
                                div { class: "wb-model",
                                    div { class: "wb-model-avatar", "Q" }
                                    div { class: "wb-model-text",
                                        div { class: "wb-model-name", "Qwen 2.5 72B Instruct" }
                                        div { class: "wb-model-provider", "Qwen" }
                                    }
                                }
                            }
                            td { span { class: "wb-tier standard", "STANDARD" } }
                            td { "310,800" }
                            td { "418 ms" }
                            td { "$0.18" }
                            td { span { class: "wb-status healthy", Icon { name: "check" }, "Healthy" } }
                            td { a { class: "wb-action-link", "在操练场调试 →" } }
                        }
                    }
                }

                div { class: "wb-card wb-activity-card",
                    div { class: "wb-table-head",
                        div {
                            div { class: "wb-table-title", "最近动态" }
                            div { class: "wb-table-sub", "账户核心事件、充值到账记录与自动容灾通知。" }
                        }
                        a { class: "wb-link", "查看完整调用日志 →" }
                    }
                    div { class: "wb-activity-list",
                        ActivityRow {
                            icon: "dollar",
                            icon_color: "green",
                            title: "Prepaid balance top-up completed ($100.00)",
                            desc: "Receipt #REC-8921 generated. Payment method: Visa ending in 4242.",
                            time: "12 mins ago",
                        }
                        ActivityRow {
                            icon: "zap",
                            icon_color: "purple",
                            title: "Sub-150ms smart fallback verified for DeepSeek V3",
                            desc: "BurnCloud automatically provisioned additional capacity in US-West cluster.",
                            time: "1 hour ago",
                        }
                        ActivityRow {
                            icon: "key",
                            icon_color: "blue",
                            title: r#"New API Key "Production Kubernetes Cluster" generated"#,
                            desc: "Associated with Standard & Performance tiers.",
                            time: "1 day ago",
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ActivityRow(icon: &'static str, icon_color: &'static str, title: &'static str, desc: &'static str, time: &'static str) -> Element {
    rsx! {
        div { class: "wb-activity-row",
            div { class: "wb-activity-icon {icon_color}", Icon { name: icon } }
            div { class: "wb-activity-body",
                div { class: "wb-activity-title", "{title}" }
                div { class: "wb-activity-desc", "{desc}" }
            }
            div { class: "wb-activity-time", "{time}" }
        }
    }
}
