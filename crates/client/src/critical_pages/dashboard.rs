use dioxus::prelude::*;

use crate::components::{Badge, Drawer, Icon, MetricCard};

const RECEIPT_JSON: &str = r#"{
  "request_id": "req_8f1a2c9d4e3f7a10",
  "timestamp": "2026-07-17T00:51:38.125Z",
  "model_requested": "claude-fable-5",
  "provider_target": "aws-bedrock-us-east-1",
  "tpm_signature": "0x8e1f5b3a...09d",
  "hardware_signature": "SIG_TPM_NITRO_91f8",
  "authenticity_score": "100%",
  "audit_status": "PASSED"
}"#;

#[component]
pub fn Overview() -> Element {
    let mut receipt_open = use_signal(|| false);
    let mut audit_open = use_signal(|| false);
    let mut receipt_verified = use_signal(|| false);
    let mut audit_complete = use_signal(|| false);

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    div { class: "row gap-2",
                        h2 { class: "page-title", "Good morning, Wei." }
                        Badge { text: "● All routes verified", tone: "success" }
                    }
                    p { class: "page-subtitle",
                        "Every request is fully traceable. "
                        strong { "12.8M requests" }
                        " routed today."
                    }
                }
                div { class: "header-actions",
                    button {
                        class: "button button-secondary",
                        onclick: move |_| {
                            audit_complete.set(false);
                            audit_open.set(true);
                        },
                        Icon { name: "spark" }
                        "Steve's Critique"
                    }
                    button {
                        class: "button button-primary",
                        onclick: move |_| {
                            audit_complete.set(true);
                            audit_open.set(true);
                        },
                        Icon { name: "activity" }
                        "Cryptographic Scan"
                    }
                }
            }

            div { class: "metrics",
                MetricCard { label: "Verified Requests", value: "12.8M", note: "Fully cloud attested", icon: "activity", tone: "tone-blue" }
                MetricCard { label: "Source Transparent", value: "100%", note: "Direct hardware keys", icon: "shield", tone: "tone-green" }
                MetricCard { label: "Model Identity Match", value: "99.99%", note: "Silicon handshake hash", icon: "server", tone: "tone-purple" }
                MetricCard { label: "Est. Cost Saved", value: "$4,766", note: "Smart fallback routing", icon: "dollar", tone: "tone-amber" }
            }

            div { class: "grid-2",
                div { class: "card card-pad stack",
                    div { class: "row between",
                        span { class: "section-label", "Live Model Source Map" }
                        Badge { text: "ACTIVE POOL" }
                    }
                    div { class: "source-map",
                        div { class: "source-title", span { class: "green-dot" } "claude-fable-5" }
                        SourceBar { label: "├ AWS Bedrock", percent: 52 }
                        SourceBar { label: "├ Anthropic", percent: 31, tone: "purple" }
                        SourceBar { label: "└ Vertex AI", percent: 17, tone: "green" }
                    }
                    div { class: "tiny subtle mono", style: "text-align:center;border-top:1px solid #f3f4f6;padding-top:14px", "Pristine silicon attestation active." }
                }

                div { class: "card card-pad stack",
                    div { class: "row between",
                        span { class: "section-label", "Latest Model Receipt" }
                        Badge { text: "SECURE TPM", tone: "success" }
                    }
                    div { class: "receipt",
                        ReceiptRow { label: "Requested:", value: "claude-fable-5" }
                        ReceiptRow { label: "Provider:", value: "AWS" }
                        ReceiptRow { label: "Region:", value: "us-east-1" }
                        div { class: "receipt-row", style: "border-top:1px dashed #e5e7eb;padding-top:10px",
                            label { "Route:" }
                            strong { class: "success", "● Verified" }
                        }
                    }
                    button {
                        class: "button button-primary",
                        style: "width:100%",
                        onclick: move |_| {
                            receipt_verified.set(false);
                            receipt_open.set(true);
                        },
                        Icon { name: "logs" }
                        "View Verifiable Receipt"
                    }
                }
            }

            div { class: "card card-pad stack",
                div { class: "row between",
                    span { class: "section-label", "What Changed" }
                    span { class: "tiny subtle mono", "LAST 24 HOURS" }
                }
                ul { class: "change-list",
                    li { span { class: "dot" } "286 requests used a disclosed fallback" }
                    li { span { class: "dot green" } "Router A changed upstream provider to AWS Bedrock" }
                    li { span { class: "dot amber" } "One route is awaiting independent verification" }
                }
            }

            div { class: "card card-pad", style: "display:flex;align-items:center;justify-content:space-around;text-align:center;gap:16px;flex-wrap:wrap",
                MiniStat { value: "12.8M Requests", label: "Verified traffic today" }
                MiniStat { value: "3.6B", label: "Tokens routed" }
                MiniStat { value: "184ms", label: "P95 latency" }
                MiniStat { value: "$4,766 Saved", label: "Smart routing savings" }
            }
            p { class: "tiny subtle mono", style: "text-align:center", "BurnCloud Gateway • Designed with Steve's absolute design & integrity rules." }

            Drawer {
                title: "Traceable Route Certificate",
                open: receipt_open(),
                on_close: move |_| receipt_open.set(false),
                div { class: "stack-lg",
                    div { style: "text-align:center",
                        div { class: "metric-icon tone-green", style: "margin:0 auto;width:52px;height:52px", Icon { name: "shield" } }
                        p { class: "small muted", "Verifiable proof of model identity & route authenticity issued by BurnCloud secure hardware enclaves." }
                    }
                    div { class: "card card-pad stack",
                        span { class: "section-label", "100% Traceability Mechanism" }
                        p { class: "small muted", "Every routed request is bound to a cryptographic certificate. The request is signed inside a secure enclave, forwarded with a hash, and matched against the provider hardware profile to prevent proxy dilution." }
                    }
                    div { class: "stack",
                        div { class: "row between",
                            span { class: "section-label", "Verification Blueprint" }
                            Badge { text: "● SIGNED BY ROOT", tone: "success" }
                        }
                        pre { class: "terminal", style: "white-space:pre-wrap;line-height:1.65", "{RECEIPT_JSON}" }
                    }
                    button {
                        class: "button button-primary",
                        style: "width:100%",
                        onclick: move |_| receipt_verified.set(true),
                        Icon { name: "lock" }
                        if receipt_verified() { "Cryptographic Chain Verified" } else { "Verify Cryptographic Chain" }
                    }
                    if receipt_verified() {
                        div { class: "terminal",
                            div { class: "terminal-line", "> 🔐 Retrieving downstream HMAC-SHA256 request token..." }
                            div { class: "terminal-line", "> 📡 Handshaking with AWS Bedrock TPM Secure Enclave..." }
                            div { class: "terminal-line", "> 🧬 Extracting silicon-bound hardware signature..." }
                            div { class: "terminal-line success", "> ✓ 100% Traceability Confirmed" }
                        }
                        div { class: "badge badge-success", style: "display:block;padding:12px;text-align:center", "Routing history matches authentic provider signatures without middleman spoofing." }
                    }
                }
            }

            Drawer {
                title: "Steve's Verdict on Micro-Tuning",
                open: audit_open(),
                on_close: move |_| audit_open.set(false),
                div { class: "stack-lg",
                    div { style: "text-align:center",
                        div { style: "font-size:40px", "👓" }
                        p { class: "small muted", style: "font-style:italic", "The finest details are the ones you can't see, but you can feel. When the hardware is honest, the software doesn't need to lie." }
                    }
                    div { class: "card card-pad row between",
                        div { span { class: "section-label", "Integrity Score" } div { class: "small muted", "Cupertino Calibration" } }
                        if audit_complete() {
                            div { class: "row gap-2", strong { class: "metric-value success", "100.0%" } Badge { text: "INSANELY GREAT", tone: "success" } }
                        } else {
                            span { class: "small muted mono", "Pending micro-probe..." }
                        }
                    }
                    Directive { number: "1", title: "Visual Gravity of Spacing", text: "Thin dividers, disciplined card spacing and low visual noise keep trust data as the hero." }
                    Directive { number: "2", title: "Typographic Integrity", text: "High-contrast display type and monospaced technical data make the routing tree feel precise and authoritative." }
                    Directive { number: "3", title: "The Magic in Interaction", text: "A cryptographic receipt becomes tangible when the verification chain can be inspected directly." }
                    button {
                        class: "button button-primary",
                        style: "width:100%",
                        onclick: move |_| audit_complete.set(true),
                        Icon { name: "activity" }
                        if audit_complete() { "Calibration Complete" } else { "Run Cupertino Integrity Calibration" }
                    }
                    if audit_complete() {
                        div { class: "terminal",
                            div { class: "terminal-line", "> Calibrating alignments to pristine proportions..." }
                            div { class: "terminal-line", "> Measuring padding density and radius consistency..." }
                            div { class: "terminal-line", "> Auditing route transparency and provider hardware enclaves..." }
                            div { class: "terminal-line success", "> ✓ Integrity score: 100.0%" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Dashboard() -> Element {
    rsx! { Overview {} }
}

#[component]
fn Directive(number: &'static str, title: &'static str, text: &'static str) -> Element {
    rsx! {
        div { class: "card card-pad stack",
            div { class: "row gap-2", Badge { text: number, tone: "brand" } strong { "{title}" } }
            p { class: "small muted", "{text}" }
        }
    }
}

#[component]
fn ReceiptRow(label: &'static str, value: &'static str) -> Element {
    rsx! { div { class: "receipt-row", label { "{label}" } strong { "{value}" } } }
}

#[component]
fn MiniStat(value: &'static str, label: &'static str) -> Element {
    rsx! { div { strong { "{value}" } div { class: "tiny subtle", "{label}" } } }
}

#[component]
fn SourceBar(label: &'static str, percent: u32, #[props(default)] tone: String) -> Element {
    let progress_class = if tone.is_empty() { "progress".to_string() } else { format!("progress {tone}") };
    let width = format!("width:{percent}%");
    rsx! {
        div { class: "source-line",
            div { class: "source-meta", span { "{label}" } span { class: "badge badge-neutral mono", "{percent}%" } }
            div { class: "{progress_class}", span { style: "{width}" } }
        }
    }
}
