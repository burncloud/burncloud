use dioxus::prelude::*;

use crate::{
    app::Route,
    components::{Badge, Icon, Logo},
};

#[component]
pub fn Home() -> Element {
    rsx! {
        div { class: "public-page",
            header { class: "public-header",
                div { class: "public-header-left",
                    Link { to: Route::Home {}, class: "brand-link",
                        Logo {}
                        span { class: "brand-name", "BurnCloud" }
                    }
                    nav { class: "public-nav",
                        a { href: "#workflow", "How it works" }
                        a { href: "#capabilities", "Capabilities" }
                        a { href: "#operations", "Operations" }
                    }
                }
                div { class: "public-header-right",
                    Link { to: Route::Login {}, class: "button button-ghost", "Sign In" }
                    Link { to: Route::Register {}, class: "button button-primary", "Create Account" }
                }
            }

            section { class: "hero",
                div { class: "hero-inner",
                    div { class: "trust-pill",
                        span { class: "green-dot" }
                        span { class: "mono strong", "Multi-provider AI gateway" }
                        span { class: "subtle", "•" }
                        span { class: "muted", "Real routing visibility" }
                    }
                    h1 {
                        "One API for your "
                        br {}
                        span { "model provider fleet" }
                    }
                    p {
                        "Connect upstream model providers, expose the models you actually have, route requests through one OpenAI-compatible endpoint, and see which upstream handled each request."
                    }
                    div { class: "hero-actions",
                        Link { to: Route::Register {}, class: "button button-primary button-lg", "Create BurnCloud Account" }
                        Link { to: Route::Login {}, class: "button button-secondary button-lg", "Open Console" }
                    }
                    div { class: "stat-strip",
                        PublicStat { value: "Providers", label: "Connect upstream supply" }
                        PublicStat { value: "Routes", label: "Control traffic preference" }
                        PublicStat { value: "Logs", label: "Inspect request outcomes" }
                        PublicStat { value: "Billing", label: "Track usage and cost" }
                    }
                }
            }

            section { id: "workflow", class: "features-section",
                div { class: "features-inner",
                    div { class: "features-head",
                        Badge { text: "OPERATOR WORKFLOW", tone: "brand" }
                        h2 { "From provider credential to verified request." }
                        p { "BurnCloud keeps setup and day-to-day operations in one workflow instead of hiding routing behind a black box." }
                    }
                    div { class: "feature-grid",
                        FeatureCard {
                            icon: "providers",
                            title: "1. Connect providers",
                            text: "Add the upstream credential, endpoint, model IDs, routing group, priority, weight, and optional capacity limits."
                        }
                        FeatureCard {
                            icon: "models",
                            title: "2. Review model coverage",
                            text: "See which model IDs are usable now and which important models still depend on a single active provider."
                        }
                        FeatureCard {
                            icon: "key",
                            title: "3. Create API access",
                            text: "Issue customer-owned API keys, control quota and network access, and rotate credentials without exposing stored secrets."
                        }
                        FeatureCard {
                            icon: "play",
                            title: "4. Run a real test",
                            text: "Use Playground with an explicitly selected account and API key so test usage follows the intended customer attribution."
                        }
                        FeatureCard {
                            icon: "logs",
                            title: "5. Inspect the route",
                            text: "Review request outcome, model, upstream, latency, tokens, cost, routing decision, and persisted operational metadata."
                        }
                        FeatureCard {
                            icon: "billing",
                            title: "6. Watch spend",
                            text: "See total spend and model-level cost drivers alongside request and token usage for the current billing period."
                        }
                    }
                }
            }

            section { id: "capabilities", class: "demo-section",
                div { class: "demo-inner",
                    div { class: "demo-head",
                        Badge { text: "WHAT THE PRODUCT DOES TODAY", tone: "success" }
                        h2 { "A console for operating real routed model traffic." }
                        p { "The public product description intentionally matches capabilities the current BurnCloud server and Dioxus console can actually read or change." }
                    }
                    div { class: "grid-3",
                        CapabilityCard {
                            icon: "routes",
                            title: "Provider-aware routing",
                            text: "Group providers and control preference with priority and weight while keeping provider health visible."
                        }
                        CapabilityCard {
                            icon: "users",
                            title: "Customer access",
                            text: "Manage customer accounts, wallet balances, API-key ownership, quotas, and IP allowlists."
                        }
                        CapabilityCard {
                            icon: "chart",
                            title: "Operational performance",
                            text: "Compare observed success rate, latency, errors, cost, and upstream diversity from real request logs."
                        }
                    }
                }
            }

            section { id: "operations", class: "features-section",
                div { class: "features-inner",
                    div { class: "features-head",
                        h2 { "Know what happened when traffic moves." }
                        p { "BurnCloud focuses on operational evidence already persisted by the router. It does not present ordinary routing metadata as cryptographic attestation."
                        }
                    }
                    div { class: "feature-grid",
                        FeatureCard {
                            icon: "activity",
                            title: "Request outcomes",
                            text: "Separate successful requests, fallbacks, HTTP errors, and true timeout events so operators can diagnose the right failure mode."
                        }
                        FeatureCard {
                            icon: "shield",
                            title: "Guardrails",
                            text: "Configure persisted traffic protections, review risk events, inspect circuit-breaker telemetry, and keep emergency shutdown isolated."
                        }
                        FeatureCard {
                            icon: "dollar",
                            title: "Cost visibility",
                            text: "Trace model-level spend and token composition without inventing savings claims or synthetic financial metrics."
                        }
                    }
                    div { class: "hero-actions", style: "justify-content:center;margin-top:28px",
                        Link { to: Route::Register {}, class: "button button-primary button-lg", "Create Account" }
                        Link { to: Route::Login {}, class: "button button-secondary button-lg", "Sign In" }
                    }
                }
            }

            footer { class: "public-footer",
                div { class: "row", style: "justify-content:center;gap:12px;flex-wrap:wrap",
                    span { "© 2026 BurnCloud" }
                    span { "•" }
                    span { "OpenAI-compatible multi-provider routing" }
                }
            }
        }
    }
}

#[component]
fn PublicStat(value: &'static str, label: &'static str) -> Element {
    rsx! {
        div {
            strong { "{value}" }
            span { "{label}" }
        }
    }
}

#[component]
fn FeatureCard(icon: &'static str, title: &'static str, text: &'static str) -> Element {
    rsx! {
        div { class: "card feature-card card-hover",
            div { class: "metric-icon tone-gray", Icon { name: icon } }
            h3 { "{title}" }
            p { "{text}" }
        }
    }
}

#[component]
fn CapabilityCard(icon: &'static str, title: &'static str, text: &'static str) -> Element {
    rsx! {
        div { class: "card card-pad-lg stack",
            div { class: "metric-icon tone-blue", Icon { name: icon } }
            h3 { style: "margin:0", "{title}" }
            p { class: "small muted", style: "line-height:1.65;margin:0", "{text}" }
        }
    }
}

#[component]
pub fn Landing() -> Element {
    rsx! { Home {} }
}
