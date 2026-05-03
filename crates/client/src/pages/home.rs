use crate::app::Route;
use burncloud_client_shared::i18n::{t, use_i18n};
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
        div { class: "h-screen w-screen bc-splash" }
    }
}

#[component]
pub fn HomePage() -> Element {
    let nav_padding = "landing-nav";
    let i18n = use_i18n();
    let lang = i18n.language;

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
                            a { href: "#features", {t(*lang.read(), "home.nav.features")} }
                            a { href: "#architecture", {t(*lang.read(), "home.nav.architecture")} }
                            a { href: "#roadmap", {t(*lang.read(), "home.nav.roadmap")} }
                            a { href: "#docs", {t(*lang.read(), "home.nav.docs")} }
                        }
                        div { class: "landing-nav-cta",
                            a { class: "landing-btn landing-btn-ghost", href: "#", "GitHub \u{2192}" }
                            Link { to: Route::RegisterPage {}, class: "landing-btn landing-btn-light", {t(*lang.read(), "home.cta.get_started")} }
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
                            {t(*lang.read(), "home.hero.title_1")}
                            br {}
                            span { class: "grad", {t(*lang.read(), "home.hero.title_2")} }
                        }
                        p { class: "landing-hero-sub",
                            {t(*lang.read(), "home.hero.sub")}
                            strong { class: "bc-hero-strong", {t(*lang.read(), "home.hero.sub_highlight")} }
                            {t(*lang.read(), "home.hero.sub_suffix")}
                        }
                        div { class: "landing-hero-ctas",
                            a { class: "landing-btn landing-btn-light", href: "#", "$ cargo install burncloud \u{2192}" }
                            a { class: "landing-btn landing-btn-ghost", href: "#", {t(*lang.read(), "home.cta.read_docs")} }
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
                            span { class: "landing-term-dot bc-term-dot-red" }
                            span { class: "landing-term-dot bc-term-dot-yellow" }
                            span { class: "landing-term-dot bc-term-dot-green" }
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
            section { class: "landing-strip no-pad",
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
                            h3 { class: "v-title", "Astonishing concurrency, " em { class: "bc-accent-orange", "mythical memory" } "." }
                            p { class: "v-desc", "A unique \"don't touch the body\" routing mode achieves byte-level zero-copy forwarding when no protocol conversion is required. Latency-conscious by default \u{2014} every request path is hot." }
                            div { class: "bc-kpi-row",
                                div {
                                    div { class: "bc-kpi-value", "~12 MB" }
                                    div { class: "bc-kpi-label", "Idle RSS" }
                                }
                                div {
                                    div { class: "bc-kpi-value", "< 1 ms" }
                                    div { class: "bc-kpi-label", "Pass-through overhead" }
                                }
                                div {
                                    div { class: "bc-kpi-value", "10K+" }
                                    div { class: "bc-kpi-label", "Concurrent streams" }
                                }
                            }
                        }

                        // Universal aggregation (span-5)
                        div { class: "landing-value-card span-5",
                            div { class: "v-icon v-icon-info",
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
                            div { class: "v-icon v-icon-warning",
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
                            div { class: "v-icon v-icon-success",
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
                            div { class: "v-icon v-icon-primary",
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
                        div { class: "landing-value-card span-12 bc-fluent-card",
                            div { class: "bc-fluent-grid",
                                div {
                                    div { class: "v-eyebrow bc-fluent-eyebrow", "Fluent experience" }
                                    h3 { class: "v-title bc-fluent-title", "A native client. Not a web wrapper." }
                                    p { class: "v-desc bc-fluent-desc", "Built with Dioxus and styled with Windows 11 Fluent Design. Real-time TPS, RPM and token consumption charts \u{2014} you'll never grep a log file again." }
                                }
                                // Mini dashboard mock
                                div { class: "bc-mock-dashboard",
                                    div { class: "bc-mock-titlebar",
                                        span { class: "bc-mock-dot bc-mock-dot-red" }
                                        span { class: "bc-mock-dot bc-mock-dot-yellow" }
                                        span { class: "bc-mock-dot bc-mock-dot-green" }
                                    }
                                    div { class: "bc-mock-body",
                                        // Requests/min card
                                        div { class: "bc-mock-stat",
                                            div { class: "bc-mock-stat-label", "Requests / min" }
                                            div { class: "bc-mock-stat-value", "14,820" }
                                            div { class: "bc-mock-spark",
                                                span { class: "bc-mock-bar bc-mock-bar-orange", style: "--bc-dynamic-height:40%" }
                                                span { class: "bc-mock-bar bc-mock-bar-orange", style: "--bc-dynamic-height:60%" }
                                                span { class: "bc-mock-bar bc-mock-bar-orange", style: "--bc-dynamic-height:55%" }
                                                span { class: "bc-mock-bar bc-mock-bar-orange", style: "--bc-dynamic-height:80%" }
                                                span { class: "bc-mock-bar bc-mock-bar-orange", style: "--bc-dynamic-height:70%" }
                                                span { class: "bc-mock-bar bc-mock-bar-orange", style: "--bc-dynamic-height:90%" }
                                                span { class: "bc-mock-bar bc-mock-bar-orange", style: "--bc-dynamic-height:100%" }
                                            }
                                        }
                                        // Tokens/hour card
                                        div { class: "bc-mock-stat",
                                            div { class: "bc-mock-stat-label", "Tokens / hour" }
                                            div { class: "bc-mock-stat-value", "2.4M" }
                                            div { class: "bc-mock-spark",
                                                span { class: "bc-mock-bar bc-mock-bar-blue", style: "--bc-dynamic-height:30%" }
                                                span { class: "bc-mock-bar bc-mock-bar-blue", style: "--bc-dynamic-height:55%" }
                                                span { class: "bc-mock-bar bc-mock-bar-blue", style: "--bc-dynamic-height:70%" }
                                                span { class: "bc-mock-bar bc-mock-bar-blue", style: "--bc-dynamic-height:60%" }
                                                span { class: "bc-mock-bar bc-mock-bar-blue", style: "--bc-dynamic-height:88%" }
                                                span { class: "bc-mock-bar bc-mock-bar-blue", style: "--bc-dynamic-height:75%" }
                                                span { class: "bc-mock-bar bc-mock-bar-blue", style: "--bc-dynamic-height:95%" }
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
                                span { class: "landing-arch-chip bc-chip-faded", "Redis (planned)" }
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
                        h2 { class: "landing-section-title bc-code-title", "One curl. Every model." }
                        p { class: "bc-code-desc", "Already using OpenAI? Change one URL. Already on Anthropic? Same. The router handles protocol, billing, and failover behind the scenes." }
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
                            span { class: "landing-term-dot bc-term-dot-red" }
                            span { class: "landing-term-dot bc-term-dot-yellow" }
                            span { class: "landing-term-dot bc-term-dot-green" }
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
                    h2 { class: "bc-cta-title", {t(*lang.read(), "home.cta.stop_runtime_tax")} br {} {t(*lang.read(), "home.cta.runtime_tax")} }
                    p { class: "bc-cta-desc", {t(*lang.read(), "home.cta.final_sub")} }
                    div { class: "bc-cta-actions",
                        Link { to: Route::RegisterPage {}, class: "landing-btn landing-btn-light", {t(*lang.read(), "home.cta.get_started")} " \u{2192}" }
                        a { class: "landing-btn landing-btn-ghost", href: "#", {t(*lang.read(), "home.cta.star_github")} }
                    }
                }
            }

            // ─── Footer ───
            footer { class: "landing-footer",
                div { class: "landing-wrap",
                    div { class: "landing-foot-grid",
                        div {
                            div { class: "landing-brand bc-foot-brand",
                                span { class: "landing-brand-mark", "B" }
                                span { "BurnCloud" }
                            }
                            div { class: "bc-foot-about", "A Rust-native LLM aggregation gateway. Apache 2.0 / MIT. Built with care by an open-source community." }
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
