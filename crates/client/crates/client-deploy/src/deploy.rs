use dioxus::prelude::*;

#[component]
pub fn DeployConfig() -> Element {
    // Mock Data for Quota Management
    let total_requests = "1,248,932";
    let token_usage = "842.5M";
    let budget_used = 75; // percentage

    rsx! {
        div { class: "flex flex-col h-full",
            // Header Section
            div { class: "flex justify-between items-end mb-10",
                div {
                    h1 { class: "text-2xl font-semibold text-base-content mb-1 tracking-tight",
                        "配额管理"
                    }
                    p { class: "text-sm text-base-content/60 font-medium",
                        "监控资源消耗与全剧限制策略"
                    }
                }
                div { class: "flex gap-3",
                     button { class: "btn btn-ghost btn-sm font-normal text-base-content/70 hover:bg-base-content/5",
                        "导出报表"
                    }
                    button { class: "btn btn-neutral btn-sm px-6 text-white shadow-sm",
                        "调整策略"
                    }
                }
            }

            // Main Content
            div { class: "w-full max-w-4xl flex flex-col gap-10",
                
                // Section 1: KPI Overview
                div { class: "grid grid-cols-3 gap-6",
                    // Card 1: Requests
                    div { class: "p-5 bg-base-100 rounded-xl border border-base-200 shadow-sm flex flex-col gap-1",
                        span { class: "text-xs font-semibold text-base-content/40 uppercase tracking-wider", "本月总调用量" }
                        div { class: "flex items-baseline gap-2",
                            span { class: "text-3xl font-bold text-base-content tracking-tight", "{total_requests}" }
                            span { class: "text-xs font-medium text-emerald-600 bg-emerald-50 px-1.5 py-0.5 rounded", "+12%" }
                        }
                    }

                    // Card 2: Tokens
                    div { class: "p-5 bg-base-100 rounded-xl border border-base-200 shadow-sm flex flex-col gap-1",
                        span { class: "text-xs font-semibold text-base-content/40 uppercase tracking-wider", "Token 消耗" }
                        div { class: "flex items-baseline gap-2",
                            span { class: "text-3xl font-bold text-base-content tracking-tight", "{token_usage}" }
                        }
                    }

                    // Card 3: Budget
                    div { class: "p-5 bg-base-100 rounded-xl border border-base-200 shadow-sm flex flex-col gap-3",
                        div { class: "flex justify-between items-center",
                            span { class: "text-xs font-semibold text-base-content/40 uppercase tracking-wider", "预算使用率" }
                            span { class: "text-xs font-bold text-base-content/80", "{budget_used}%" }
                        }
                        div { class: "w-full bg-base-200 rounded-full h-2 overflow-hidden",
                            div { class: "bg-base-content h-2 rounded-full", style: "width: {budget_used}%" }
                        }
                    }
                }

                // Section 2: Quota Policies
                div { class: "flex flex-col gap-4",
                     h3 { class: "text-sm font-medium text-base-content/80 border-b border-base-content/10 pb-2 mb-2", "生效中的限制策略" }
                     
                     div { class: "flex flex-col gap-3",
                        // Policy Item 1
                        div { class: "flex items-center justify-between p-4 bg-base-50/50 rounded-lg border border-base-200/50 hover:bg-base-50 transition-colors",
                            div { class: "flex items-center gap-4",
                                div { class: "w-10 h-10 rounded-full bg-blue-50 flex items-center justify-center text-blue-600",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" } }
                                }
                                div { class: "flex flex-col",
                                    span { class: "text-sm font-semibold text-base-content", "每日请求上限 (Daily Cap)" }
                                    span { class: "text-xs text-base-content/50", "达到 50,000 次请求后自动熔断" }
                                }
                            }
                            div { class: "flex items-center gap-4",
                                span { class: "text-xs font-mono bg-base-200 px-2 py-1 rounded text-base-content/70", "50k / Day" }
                                input { type: "checkbox", class: "toggle toggle-sm toggle-success", checked: "true" }
                            }
                        }

                        // Policy Item 2
                        div { class: "flex items-center justify-between p-4 bg-base-50/50 rounded-lg border border-base-200/50 hover:bg-base-50 transition-colors",
                            div { class: "flex items-center gap-4",
                                div { class: "w-10 h-10 rounded-full bg-orange-50 flex items-center justify-center text-orange-600",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M13 10V3L4 14h7v7l9-11h-7z" } }
                                }
                                div { class: "flex flex-col",
                                    span { class: "text-sm font-semibold text-base-content", "速率限制 (Rate Limiting)" }
                                    span { class: "text-xs text-base-content/50", "单 IP 并发请求限制" }
                                }
                            }
                            div { class: "flex items-center gap-4",
                                span { class: "text-xs font-mono bg-base-200 px-2 py-1 rounded text-base-content/70", "60 req / min" }
                                input { type: "checkbox", class: "toggle toggle-sm toggle-success", checked: "true" }
                            }
                        }
                     }
                }
            }
        }
    }
}
