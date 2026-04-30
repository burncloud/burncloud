use crate::app::Route;
use burncloud_client_shared::DesktopMode;
use dioxus::prelude::*;

#[component]
pub fn Root() -> Element {
    let navigator = use_navigator();

    use_effect(move || {
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            navigator.replace(Route::HomePage {});
        });
    });

    rsx! {
        div { class: "h-screen w-screen", style: "background-color: var(--bc-bg-canvas);" }
    }
}

#[component]
pub fn HomePage() -> Element {
    let is_desktop = try_use_context::<DesktopMode>().is_some();
    let nav_padding = if is_desktop {
        "landing-nav"
    } else {
        "landing-nav"
    };

    rsx! {
        div { class: "landing",

            // ─── Hero ───
            header { class: "landing-hero",
                nav { class: "{nav_padding}",
                    div { class: "landing-wrap landing-nav-inner",
                        div { class: "landing-brand",
                            span { class: "landing-brand-mark", "B" }
                            span { "BurnCloud" }
                        }
                        div { class: "landing-nav-links",
                            a { href: "#features", "Features" }
                            a { href: "#architecture", "Architecture" }
                            a { href: "#roadmap", "Roadmap" }
                            a { href: "#docs", "Docs" }
                        }
                        div { class: "landing-nav-cta",
                            a { class: "landing-btn landing-btn-ghost", href: "#", "GitHub \u{2192}" }
                            Link { to: Route::RegisterPage {}, class: "landing-btn landing-btn-light", "Get Started" }
                        }
                    }
                }

                div { class: "landing-wrap landing-hero-grid",
                    div {
                        div { class: "landing-eyebrow",
                            span { class: "pulse-dot" }
                            " v0.3 \u{00b7} E2E test suite shipped"
                        }
                        h1 { class: "landing-hero-title",
                            "The next-gen AI gateway,"
                            br {}
                            span { class: "grad", "built for Rust speed." }
                        }
                        p { class: "landing-hero-sub",
                            "A high-performance LLM aggregation gateway that unifies Anthropic, Gemini, Azure & Qwen behind one OpenAI-compatible interface. "
                            strong { style: "color:#fff", "MB-level memory" }
                            ", zero-overhead routing, single binary."
                        }
                        div { class: "landing-hero-ctas",
                            a { class: "landing-btn landing-btn-light", href: "#", "$ cargo install burncloud \u{2192}" }
                            a { class: "landing-btn landing-btn-ghost", href: "#", "Read the docs" }
                        }
                        div { class: "landing-hero-meta",
                            div { class: "item",
                                svg { view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "3", width: "14", height: "14",
                                    polyline { points: "20 6 9 17 4 12" }
                                }
                                " Apache 2.0 / MIT"
                            }
                            div { class: "item",
                                svg { view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "3", width: "14", height: "14",
                                    polyline { points: "20 6 9 17 4 12" }
                                }
                                " Windows \u{00b7} Linux \u{00b7} macOS"
                            }
                            div { class: "item",
                                svg { view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "3", width: "14", height: "14",
                                    polyline { points: "20 6 9 17 4 12" }
                                }
                                " No runtime deps"
                            }
                        }
                    }

                    // Terminal mock
                    div { class: "landing-terminal",
                        div { class: "landing-term-bar",
                            span { class: "landing-term-dot", style: "background:#FF5F57" }
                            span { class: "landing-term-dot", style: "background:#FEBC2E" }
                            span { class: "landing-term-dot", style: "background:#28C840" }
                            span { class: "landing-term-title", "curl \u{00b7} burncloud router" }
                        }
                        div { class: "landing-term-body",
                            span { class: "tk-prompt", "$" } " curl http://localhost:3000/v1/chat/completions \\\n"
                            "   " span { class: "tk-flag", "-H" } " "
                            span { class: "tk-string", "\"Authorization: Bearer sk-burn...\"" } " \\\n"
                            "   " span { class: "tk-flag", "-d" } " "
                            span { class: "tk-string", "'{{ \"model\": \"claude-3-sonnet\" }}'" } "\n\n"
                            span { class: "tk-comment", "# router selects upstream, translates protocol" } "\n"
                            span { class: "tk-comment", "# streams response back as OpenAI format" }
                        }
                    }
                }
            }

            // ─── Trust strip ───
            section { class: "landing-strip", style: "padding:0",
                div { class: "landing-wrap",
                    div { class: "landing-strip-inner",
                        div { class: "landing-strip-item",
                            div { class: "landing-strip-num", "~12" span { class: "unit", "MB" } }
                            div { class: "landing-strip-label", "RSS at idle \u{2014} vs GBs of GC-paused competitors" }
                        }
                        div { class: "landing-strip-item",
                            div { class: "landing-strip-num", "0" span { class: "unit", "copy" } }
                            div { class: "landing-strip-label", "Body passthrough \u{2014} byte-level zero-overhead routing" }
                        }
                        div { class: "landing-strip-item",
                            div { class: "landing-strip-num", "1" span { class: "unit", "binary" } }
                            div { class: "landing-strip-label", "No Python, Node or JVM. cargo build & ship." }
                        }
                        div { class: "landing-strip-item",
                            div { class: "landing-strip-num", "5" span { class: "unit", "protocols" } }
                            div { class: "landing-strip-label", "OpenAI \u{00b7} Anthropic \u{00b7} Gemini \u{00b7} Azure \u{00b7} Qwen" }
                        }
                    }
                }
            }

            // ─── Why BurnCloud ───
            section { id: "features", class: "landing-section",
                div { class: "landing-wrap",
                    div { class: "landing-section-eyebrow", "Why BurnCloud" }
                    h2 { class: "landing-section-title", "All the gateway you need." br {} "None of the runtime tax." }
                    p { class: "landing-section-sub", "Built on Axum and Tokio. Designed to benchmark against \u{2014} and surpass \u{2014} the existing One API ecosystem on every axis that matters." }

                    div { class: "landing-values",
                        // Performance first (dark, span-7)
                        div { class: "landing-value-card span-7 dark",
                            div { class: "v-icon",
                                svg { width: "24", height: "24", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.6",
                                    polygon { points: "13 2 3 14 12 14 11 22 21 10 12 10 13 2" }
                                }
                            }
                            div { class: "v-eyebrow", "Performance first" }
                            h3 { class: "v-title", "Astonishing concurrency, " em { style: "font-style:normal; color:#FF6B3D", "mythical memory" } "." }
                            p { class: "v-desc", "A unique \"don't touch the body\" routing mode achieves byte-level zero-copy forwarding when no protocol conversion is required. Latency-conscious by default \u{2014} every request path is hot." }
                            div { style: "display:flex; gap:32px; margin-top:32px; padding-top:24px; border-top:1px solid rgba(255,255,255,0.08);",
                                div {
                                    div { style: "font-size:24px;font-weight:700;color:#fff", "~12 MB" }
                                    div { style: "font-size:11px;color:rgba(255,255,255,0.5);text-transform:uppercase;letter-spacing:0.12em;margin-top:4px", "Idle RSS" }
                                }
                                div {
                                    div { style: "font-size:24px;font-weight:700;color:#fff", "< 1 ms" }
                                    div { style: "font-size:11px;color:rgba(255,255,255,0.5);text-transform:uppercase;letter-spacing:0.12em;margin-top:4px", "Pass-through overhead" }
                                }
                                div {
                                    div { style: "font-size:24px;font-weight:700;color:#fff", "10K+" }
                                    div { style: "font-size:11px;color:rgba(255,255,255,0.5);text-transform:uppercase;letter-spacing:0.12em;margin-top:4px", "Concurrent streams" }
                                }
                            }
                        }

                        // Universal aggregation (span-5)
                        div { class: "landing-value-card span-5",
                            div { class: "v-icon", style: "background:var(--bc-info-light);color:var(--bc-info)",
                                svg { width: "24", height: "24", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.6",
                                    path { d: "M5 12h14M5 12l4-4M5 12l4 4M19 12l-4-4M19 12l-4 4" }
                                }
                            }
                            div { class: "v-eyebrow", "Universal aggregation" }
                            h3 { class: "v-title", "All to OpenAI format." }
                            p { class: "v-desc", "Anthropic, Gemini, Azure, Qwen \u{2014} pipe them all through one stable Base URL. Your LangChain or AutoGPT app gets a free model upgrade without a single code change." }
                        }

                        // Smart load balancing (span-4)
                        div { class: "landing-value-card span-4",
                            div { class: "v-icon", style: "background:var(--bc-warning-light);color:var(--bc-warning)",
                                svg { width: "24", height: "24", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.6",
                                    path { d: "M3 6h18M3 12h18M3 18h18" }
                                    circle { cx: "6", cy: "6", r: "2", fill: "currentColor" }
                                    circle { cx: "14", cy: "12", r: "2", fill: "currentColor" }
                                    circle { cx: "9", cy: "18", r: "2", fill: "currentColor" }
                                }
                            }
                            div { class: "v-eyebrow", "Smart load balancing" }
                            h3 { class: "v-title", "Round-robin, weighted, failover." }
                            p { class: "v-desc", "When one gpt-4 goes down, thousands stand up. Per-channel speed tests detect degradation in real time." }
                        }

                        // Precise billing (span-4)
                        div { class: "landing-value-card span-4",
                            div { class: "v-icon", style: "background:var(--bc-success-light);color:var(--bc-success)",
                                svg { width: "24", height: "24", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.6",
                                    path { d: "M12 2v8M12 22v-4M2 12h8M22 12h-4" }
                                    circle { cx: "12", cy: "12", r: "4", stroke_width: "1.6", fill: "none" }
                                }
                            }
                            div { class: "v-eyebrow", "Precise billing" }
                            h3 { class: "v-title", "Per-token, per-model, per-tenant." }
                            p { class: "v-desc", "Custom model ratios, group multipliers, atomic quota deduction, OpenAI-compatible 402 errors when limits hit." }
                        }

                        // Real-world tested (span-4)
                        div { class: "landing-value-card span-4",
                            div { class: "v-icon", style: "background:var(--bc-primary-light);color:var(--bc-primary)",
                                svg { width: "24", height: "24", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.6",
                                    path { d: "M12 2L4 6v6c0 5 3.5 9 8 10 4.5-1 8-5 8-10V6l-8-4z" }
                                    polyline { points: "9 12 11 14 15 10" }
                                }
                            }
                            div { class: "v-eyebrow", "Real-world tested" }
                            h3 { class: "v-title", "No fake mocks. No regressions." }
                            p { class: "v-desc", "CI validates end-to-end against real OpenAI & Gemini APIs. Headless Chrome verifies every Dioxus render path." }
                        }

                        // Fluent experience (span-12, warm gradient)
                        div { class: "landing-value-card span-12", style: "background: linear-gradient(135deg, #FFF5F0 0%, #FFFBF7 100%); border-color:#FFD9C2",
                            div { style: "display:grid; grid-template-columns:1fr 1fr; gap:48px; align-items:center;",
                                div {
                                    div { class: "v-eyebrow", style: "color:#C2410C", "Fluent experience" }
                                    h3 { class: "v-title", style: "font-size:32px;", "A native client. Not a web wrapper." }
                                    p { class: "v-desc", style: "font-size:15px;", "Built with Dioxus and styled with Windows 11 Fluent Design. Real-time TPS, RPM and token consumption charts \u{2014} you'll never grep a log file again." }
                                }
                                // Mini dashboard mock
                                div { style: "background:#fff; border-radius:12px; box-shadow: 0 8px 32px rgba(194,65,12,0.12); overflow:hidden; border:1px solid var(--bc-border);",
                                    div { style: "height: 28px; background:#fff; border-bottom:1px solid var(--bc-border); display:flex; align-items:center; padding:0 12px; gap:6px;",
                                        span { style: "width:10px;height:10px;border-radius:50%;background:#FF5F57" }
                                        span { style: "width:10px;height:10px;border-radius:50%;background:#FEBC2E" }
                                        span { style: "width:10px;height:10px;border-radius:50%;background:#28C840" }
                                    }
                                    div { style: "padding:20px; display:grid; grid-template-columns:1fr 1fr; gap:12px;",
                                        // Requests/min card
                                        div { style: "padding:12px; background:var(--bc-bg-canvas); border-radius:8px;",
                                            div { style: "font-size:10px; color:var(--bc-text-tertiary); text-transform:uppercase; letter-spacing:0.16em", "Requests / min" }
                                            div { style: "font-size:22px; font-weight:700; margin-top:4px", "14,820" }
                                            div { style: "display:flex; align-items:flex-end; gap:2px; height:24px; margin-top:6px",
                                                span { style: "flex:1;background:#FF6B3D;border-radius:1px;height:40%" }
                                                span { style: "flex:1;background:#FF6B3D;border-radius:1px;height:60%" }
                                                span { style: "flex:1;background:#FF6B3D;border-radius:1px;height:55%" }
                                                span { style: "flex:1;background:#FF6B3D;border-radius:1px;height:80%" }
                                                span { style: "flex:1;background:#FF6B3D;border-radius:1px;height:70%" }
                                                span { style: "flex:1;background:#FF6B3D;border-radius:1px;height:90%" }
                                                span { style: "flex:1;background:#FF6B3D;border-radius:1px;height:100%" }
                                            }
                                        }
                                        // Tokens/hour card
                                        div { style: "padding:12px; background:var(--bc-bg-canvas); border-radius:8px;",
                                            div { style: "font-size:10px; color:var(--bc-text-tertiary); text-transform:uppercase; letter-spacing:0.16em", "Tokens / hour" }
                                            div { style: "font-size:22px; font-weight:700; margin-top:4px", "2.4M" }
                                            div { style: "display:flex; align-items:flex-end; gap:2px; height:24px; margin-top:6px",
                                                span { style: "flex:1;background:#007AFF;border-radius:1px;height:30%" }
                                                span { style: "flex:1;background:#007AFF;border-radius:1px;height:55%" }
                                                span { style: "flex:1;background:#007AFF;border-radius:1px;height:70%" }
                                                span { style: "flex:1;background:#007AFF;border-radius:1px;height:60%" }
                                                span { style: "flex:1;background:#007AFF;border-radius:1px;height:88%" }
                                                span { style: "flex:1;background:#007AFF;border-radius:1px;height:75%" }
                                                span { style: "flex:1;background:#007AFF;border-radius:1px;height:95%" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ─── Aggregation diagram ───
            section { class: "landing-section landing-agg",
                div { class: "landing-wrap",
                    div { class: "landing-section-eyebrow", "Universal protocol layer" }
                    h2 { class: "landing-section-title", "Five SDKs in. One Base URL out." }
                    p { class: "landing-section-sub", "Migrate from any provider in under a minute. Switch underlying models without touching application code." }

                    div { class: "landing-agg-stage",
                        div { class: "landing-agg-col",
                            div { class: "landing-agg-node", strong { "Anthropic" } span { class: "meta", "claude-3" } }
                            div { class: "landing-agg-node", strong { "Google" } span { class: "meta", "gemini-1.5" } }
                            div { class: "landing-agg-node", strong { "Azure" } span { class: "meta", "openai-svc" } }
                            div { class: "landing-agg-node", strong { "Alibaba" } span { class: "meta", "qwen-max" } }
                        }
                        div { class: "landing-agg-arrow", "\u{2192}" }
                        div { class: "landing-agg-col center",
                            div { class: "landing-agg-core",
                                div { class: "label", "BurnCloud Router" }
                                div { class: "name", "Don't touch the body" }
                                div { class: "ver", "axum + tokio \u{00b7} zero-copy" }
                            }
                        }
                        div { class: "landing-agg-arrow", "\u{2192}" }
                        div { class: "landing-agg-col",
                            div { class: "landing-agg-node landing-agg-out", strong { "POST /v1/chat/completions" } span { class: "meta", "openai-format" } }
                            div { class: "landing-agg-node landing-agg-out", strong { "POST /v1/embeddings" } span { class: "meta", "openai-format" } }
                            div { class: "landing-agg-node landing-agg-out", strong { "SSE stream" } span { class: "meta", "delta tokens" } }
                            div { class: "landing-agg-node landing-agg-out", strong { "usage stats" } span { class: "meta", "live billing" } }
                        }
                    }
                }
            }

            // ─── Architecture ───
            section { id: "architecture", class: "landing-section landing-arch",
                div { class: "landing-wrap",
                    div { class: "landing-section-eyebrow", "Architecture" }
                    h2 { class: "landing-section-title", "Four layers. Strict boundaries." }
                    p { class: "landing-section-sub", "High cohesion, low coupling. Each crate owns one concern \u{2014} no implicit dependencies, no architecture rot." }

                    div { class: "landing-arch-diagram",
                        div { class: "landing-arch-layer",
                            div {
                                div { class: "landing-arch-tag", "crates/router" }
                                div { class: "landing-arch-name", "Gateway Layer" }
                                div { class: "landing-arch-desc", "Data plane. High-concurrency traffic, auth, rate limiting, protocol conversion." }
                            }
                            div { class: "landing-arch-flow",
                                span { class: "landing-arch-chip accent", "Axum" }
                                span { class: "landing-arch-chip accent", "Tokio" }
                                span { class: "landing-arch-chip", "SigV4 signing" }
                                span { class: "landing-arch-chip", "Streaming pipe" }
                                span { class: "landing-arch-chip", "Token meter" }
                            }
                        }
                        div { class: "landing-arch-layer",
                            div {
                                div { class: "landing-arch-tag", "crates/server" }
                                div { class: "landing-arch-name", "Control Layer" }
                                div { class: "landing-arch-desc", "Control plane. RESTful APIs for UI calls, configuration and state management." }
                            }
                            div { class: "landing-arch-flow",
                                span { class: "landing-arch-chip", "REST API" }
                                span { class: "landing-arch-chip", "Channel admin" }
                                span { class: "landing-arch-chip", "Token issuance" }
                                span { class: "landing-arch-chip", "Group routing" }
                            }
                        }
                        div { class: "landing-arch-layer",
                            div {
                                div { class: "landing-arch-tag", "crates/service" }
                                div { class: "landing-arch-name", "Service Layer" }
                                div { class: "landing-arch-desc", "Business logic. Billing, monitoring, channel speed-testing, quota enforcement." }
                            }
                            div { class: "landing-arch-flow",
                                span { class: "landing-arch-chip", "Async billing" }
                                span { class: "landing-arch-chip", "Speed-test scheduler" }
                                span { class: "landing-arch-chip", "Quota engine" }
                                span { class: "landing-arch-chip", "Pricing rules" }
                            }
                        }
                        div { class: "landing-arch-layer",
                            div {
                                div { class: "landing-arch-tag", "crates/database" }
                                div { class: "landing-arch-name", "Data Layer" }
                                div { class: "landing-arch-desc", "Persistence. SQLx + SQLite/PostgreSQL. Redis cache integration on the v1.0 roadmap." }
                            }
                            div { class: "landing-arch-flow",
                                span { class: "landing-arch-chip", "SQLx" }
                                span { class: "landing-arch-chip", "SQLite" }
                                span { class: "landing-arch-chip", "PostgreSQL" }
                                span { class: "landing-arch-chip", style: "opacity:0.5", "Redis (planned)" }
                            }
                        }
                    }
                }
            }

            // ─── Code section ───
            section { class: "landing-section landing-code-section",
                div { class: "landing-wrap landing-code-grid",
                    div {
                        div { class: "landing-section-eyebrow", "Drop-in compatible" }
                        h2 { class: "landing-section-title", style: "font-size:42px;", "One curl. Every model." }
                        p { style: "font-size:16px;color:var(--bc-text-secondary);line-height:1.55;margin:0", "Already using OpenAI? Change one URL. Already on Anthropic? Same. The router handles protocol, billing, and failover behind the scenes." }
                        div { class: "landing-code-feat",
                            div { class: "pt",
                                span { class: "pt-mark", "1" }
                                div { strong { "Bring your own keys." } "Each upstream channel keeps its own credentials, encrypted at rest." }
                            }
                            div { class: "pt",
                                span { class: "pt-mark", "2" }
                                div { strong { "Stream-aware billing." } "Token counts parsed from stream_options.include_usage, deducted atomically." }
                            }
                            div { class: "pt",
                                span { class: "pt-mark", "3" }
                                div { strong { "OpenAI-shaped errors." } "402 for quota, 401 for expired tokens \u{2014} your existing retry logic just works." }
                            }
                        }
                    }

                    div { class: "landing-terminal",
                        div { class: "landing-term-bar",
                            span { class: "landing-term-dot", style: "background:#FF5F57" }
                            span { class: "landing-term-dot", style: "background:#FEBC2E" }
                            span { class: "landing-term-dot", style: "background:#28C840" }
                            span { class: "landing-term-title", "app.py \u{00b7} using BurnCloud as base" }
                        }
                        div { class: "landing-term-body",
                            span { class: "tk-comment", "# Before \u{2014} locked to one vendor" } "\n"
                            span { class: "tk-key", "from" } " openai " span { class: "tk-key", "import" } " OpenAI\n"
                            "client = OpenAI(\n"
                            "  api_key=\"sk-original...\",\n"
                            "  base_url=\"https://api.openai.com/v1\"\n"
                            ")\n\n"
                            span { class: "tk-comment", "# After \u{2014} every model, one URL" } "\n"
                            span { class: "tk-key", "from" } " openai " span { class: "tk-key", "import" } " OpenAI\n"
                            "client = OpenAI(\n"
                            "  api_key=\"sk-burn-...\",\n"
                            "  base_url=\"https://gw.burncloud.io/v1\"\n"
                            ")"
                        }
                    }
                }
            }

            // ─── Roadmap ───
            section { id: "roadmap", class: "landing-section",
                div { class: "landing-wrap",
                    div { class: "landing-section-eyebrow", "Roadmap" }
                    h2 { class: "landing-section-title", "Shipped fast. Planned in public." }

                    div { class: "landing-roadmap-track",
                        div { class: "landing-rm-step done",
                            div { class: "landing-rm-dot", "\u{2713}" }
                            div { class: "landing-rm-ver", "v0.1" }
                            div { class: "landing-rm-title", "Routing core" }
                            div { class: "landing-rm-desc", "Basic routing & AWS SigV4 signing." }
                            span { class: "landing-rm-pill done", "Shipped" }
                        }
                        div { class: "landing-rm-step done",
                            div { class: "landing-rm-dot", "\u{2713}" }
                            div { class: "landing-rm-ver", "v0.2" }
                            div { class: "landing-rm-title", "DB & auth" }
                            div { class: "landing-rm-desc", "New API core replication, channel mgmt, async billing." }
                            span { class: "landing-rm-pill done", "Shipped" }
                        }
                        div { class: "landing-rm-step done",
                            div { class: "landing-rm-dot", "\u{2713}" }
                            div { class: "landing-rm-ver", "v0.3" }
                            div { class: "landing-rm-title", "Protocol unify" }
                            div { class: "landing-rm-desc", "OpenAI / Gemini / Claude adaptors + E2E suite." }
                            span { class: "landing-rm-pill done", "Shipped" }
                        }
                        div { class: "landing-rm-step active",
                            div { class: "landing-rm-dot", "\u{2192}" }
                            div { class: "landing-rm-ver", "v0.4" }
                            div { class: "landing-rm-title", "Smart LB & failover" }
                            div { class: "landing-rm-desc", "Health-aware weighting, automatic channel demotion." }
                            span { class: "landing-rm-pill in-prog", "In progress" }
                        }
                        div { class: "landing-rm-step",
                            div { class: "landing-rm-dot", "5" }
                            div { class: "landing-rm-ver", "v0.5" }
                            div { class: "landing-rm-title", "Web console polish" }
                            div { class: "landing-rm-desc", "Multi-tenant admin UI, redemption codes UX." }
                            span { class: "landing-rm-pill next", "Q3 2026" }
                        }
                        div { class: "landing-rm-step",
                            div { class: "landing-rm-dot", "6" }
                            div { class: "landing-rm-ver", "v1.0" }
                            div { class: "landing-rm-title", "GA" }
                            div { class: "landing-rm-desc", "Redis cache, official release, LTS commitment." }
                            span { class: "landing-rm-pill next", "2026" }
                        }
                    }
                }
            }

            // ─── Final CTA ───
            section { class: "landing-final-cta",
                div { class: "landing-wrap landing-final-cta-inner",
                    h2 { style: "font-size:64px;font-weight:700;letter-spacing:-0.03em;line-height:1.05;margin:0 0 20px;color:#fff", "Stop paying the" br {} "runtime tax." }
                    p { style: "font-size:19px;color:rgba(255,255,255,0.7);margin:0 auto 36px;max-width:540px;line-height:1.5", "One binary. Five protocols. Zero config drama. Try BurnCloud locally in under a minute." }
                    div { style: "display:inline-flex;gap:12px;",
                        Link { to: Route::RegisterPage {}, class: "landing-btn landing-btn-light", "Get Started \u{2192}" }
                        a { class: "landing-btn landing-btn-ghost", href: "#", "Star on GitHub" }
                    }
                }
            }

            // ─── Footer ───
            footer { class: "landing-footer",
                div { class: "landing-wrap",
                    div { class: "landing-foot-grid",
                        div {
                            div { class: "landing-brand", style: "color:#fff; margin-bottom:16px",
                                span { class: "landing-brand-mark", "B" }
                                span { "BurnCloud" }
                            }
                            div { style: "max-width:320px;line-height:1.6;", "A Rust-native LLM aggregation gateway. Apache 2.0 / MIT. Built with care by an open-source community." }
                        }
                        div {
                            div { class: "landing-foot-h", "Product" }
                            ul {
                                li { a { href: "#", "Features" } }
                                li { a { href: "#", "Architecture" } }
                                li { a { href: "#", "Roadmap" } }
                                li { a { href: "#", "Pricing" } }
                            }
                        }
                        div {
                            div { class: "landing-foot-h", "Developers" }
                            ul {
                                li { a { href: "#", "Documentation" } }
                                li { a { href: "#", "API Reference" } }
                                li { a { href: "#", "CLI" } }
                                li { a { href: "#", "GitHub" } }
                            }
                        }
                        div {
                            div { class: "landing-foot-h", "Community" }
                            ul {
                                li { a { href: "#", "Discord" } }
                                li { a { href: "#", "Twitter" } }
                                li { a { href: "#", "Contributors" } }
                                li { a { href: "#", "Constitution" } }
                            }
                        }
                    }
                    div { class: "landing-foot-bottom",
                        div { "\u{00a9} 2026 BurnCloud Team \u{00b7} MIT Licensed" }
                        div { "v0.3 \u{00b7} 89caad0" }
                    }
                }
            }
        }
    }
}
