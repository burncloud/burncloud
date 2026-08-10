use dioxus::prelude::*;

use crate::{
    app::Route,
    components::{Badge, Drawer, Icon, Logo, MetricCard},
    data::{Customer, LogRow, CUSTOMERS, LOGS, ROUTES},
};

fn page_header(title: &'static str, subtitle: &'static str, actions: Element) -> Element {
    rsx! {
        div { class: "page-header",
            div {
                h2 { class: "page-title", "{title}" }
                p { class: "page-subtitle", "{subtitle}" }
            }
            div { class: "header-actions", {actions} }
        }
    }
}

fn status_tone(status: &'static str) -> &'static str {
    match status {
        "Active" | "Connected" | "Success" | "Enabled" => "success",
        "Testing" | "Limited" | "Degraded" | "Fallback" | "Pending Invite" => "warning",
        "Timeout" | "Error" | "Suspended" => "error",
        _ => "neutral",
    }
}

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
                        pre { class: "terminal", style: "white-space:pre-wrap;line-height:1.65",
                            "{\n  \"request_id\": \"req_8f1a2c9d4e3f7a10\",\n  \"timestamp\": \"2026-07-17T00:51:38.125Z\",\n  \"model_requested\": \"claude-fable-5\",\n  \"provider_target\": \"aws-bedrock-us-east-1\",\n  \"tpm_signature\": \"0x8e1f5b3a...09d\",\n  \"hardware_signature\": \"SIG_TPM_NITRO_91f8\",\n  \"authenticity_score\": \"100%\",\n  \"audit_status\": \"PASSED\"\n}"
                        }
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

#[component]
pub fn Logs() -> Element {
    let mut selected = use_signal(|| None::<LogRow>);
    let mut query = use_signal(String::new);
    let query_value = query().to_lowercase();
    let visible_logs: Vec<LogRow> = LOGS
        .iter()
        .copied()
        .filter(|log| {
            query_value.is_empty()
                || log.request_id.to_lowercase().contains(&query_value)
                || log.customer.to_lowercase().contains(&query_value)
                || log.route.to_lowercase().contains(&query_value)
                || log.model.to_lowercase().contains(&query_value)
        })
        .collect();

    rsx! {
        div { class: "page",
            {page_header(
                "Logs",
                "Detailed observability into every routed request.",
                rsx! {
                    div { class: "search-field", style: "width:288px",
                        Icon { name: "search" }
                        input {
                            class: "input",
                            placeholder: "Search by request ID, customer, route...",
                            value: query(),
                            oninput: move |evt| query.set(evt.value()),
                        }
                    }
                    button { class: "button button-secondary", Icon { name: "settings" } "Filter" }
                },
            )}

            div { class: "card table-card",
                div { class: "table-wrap",
                    table { class: "data-table",
                        thead { tr {
                            th { "Timestamp" }
                            th { "Request ID" }
                            th { "Customer" }
                            th { "Route / Model" }
                            th { "Status" }
                            th { class: "right", "Latency" }
                            th { class: "right", "Tokens" }
                            th { class: "right", "Cost" }
                        } }
                        tbody {
                            for log in visible_logs {
                                tr {
                                    onclick: move |_| selected.set(Some(log)),
                                    style: "cursor:pointer",
                                    td { class: "mono muted", "{log.timestamp}" }
                                    td { class: "mono table-primary", "{log.request_id}" }
                                    td { "{log.customer}" }
                                    td { div { class: "two-line", span { class: "table-primary", "{log.route}" } small { "{log.model} • {log.provider}" } } }
                                    td { Badge { text: log.status, tone: status_tone(log.status) } }
                                    td { class: "right muted tabular", "{log.latency}ms" }
                                    td { class: "right muted tabular", "{log.tokens}" }
                                    td { class: "right strong tabular", "${log.cost:.3}" }
                                }
                            }
                        }
                    }
                }
            }

            Drawer {
                title: "Request Detail",
                open: selected().is_some(),
                on_close: move |_| selected.set(None),
                if let Some(log) = selected() {
                    div { class: "stack-lg",
                        div { class: "card card-pad", style: "display:grid;grid-template-columns:repeat(3,1fr);gap:16px",
                            DetailStat { label: "Request ID", value: log.request_id, mono: true }
                            DetailStat { label: "Customer", value: log.customer }
                            DetailStat { label: "Total Cost", value: Box::leak(format!("${:.3}", log.cost).into_boxed_str()) }
                        }
                        div { class: "stack",
                            h3 { class: "section-label", "Routing Timeline" }
                            TimelineStep { tone: "neutral", text: "Request received" }
                            TimelineStep { tone: "blue", text: Box::leak(format!("Matched route: {}", log.route).into_boxed_str()) }
                            TimelineStep { tone: "blue", text: Box::leak(format!("Selected primary model: {}", if log.status == "Success" { log.model } else { "claude-fable-5" }).into_boxed_str()) }
                            if log.status == "Timeout" {
                                TimelineStep { tone: "red", text: "Timeout after 10s" }
                                TimelineStep { tone: "amber", text: "Triggered fallback condition: Timeout > 8s" }
                                TimelineStep { tone: "blue", text: "Retried with fallback provider" }
                            }
                            if log.status == "Fallback" {
                                TimelineStep { tone: "amber", text: "Provider error rate exceeded threshold" }
                                TimelineStep { tone: "blue", text: "Falling back to alternate model" }
                            }
                            TimelineStep { tone: if log.status == "Timeout" { "red" } else { "green" }, text: Box::leak(format!("Response completed in {}ms", log.latency).into_boxed_str()) }
                        }
                        div { class: "stack",
                            h3 { class: "section-label", "Prompt Snippet" }
                            pre { class: "terminal", style: "white-space:pre-wrap", "\"system\": \"You are a senior legal...\"\n\n\"user\": \"Summarize the following contract and...\"" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DetailStat(label: &'static str, value: &'static str, #[props(default)] mono: bool) -> Element {
    rsx! { div { span { class: "tiny subtle", "{label}" } div { class: if mono { "small strong mono" } else { "small strong" }, "{value}" } } }
}

#[component]
fn TimelineStep(tone: &'static str, text: &'static str) -> Element {
    let dot_color = match tone { "red" => "#ef4444", "amber" => "#f59e0b", "green" => "#22c55e", "blue" => "#60a5fa", _ => "#d1d5db" };
    rsx! {
        div { class: "row gap-2", style: "align-items:flex-start",
            span { style: "width:10px;height:10px;border-radius:50%;background:{dot_color};margin-top:4px;flex:0 0 auto" }
            span { class: "small muted", "{text}" }
        }
    }
}

#[derive(Clone, PartialEq)]
struct Tenant {
    id: String,
    name: String,
    environment: String,
    spend: u32,
    budget: u32,
    rps: u32,
    route: String,
    status: String,
    keys: u32,
    requests: u32,
}

impl Tenant {
    fn from_customer(index: usize, customer: Customer) -> Self {
        Self {
            id: format!("c{}", index + 1),
            name: customer.name.to_string(),
            environment: customer.environment.to_string(),
            spend: customer.spend,
            budget: customer.budget,
            rps: customer.rps,
            route: customer.route.to_string(),
            status: "Active".to_string(),
            keys: customer.keys,
            requests: customer.requests,
        }
    }
}

#[component]
pub fn Customers() -> Element {
    let mut tenants = use_signal(|| CUSTOMERS.iter().copied().enumerate().map(|(i, c)| Tenant::from_customer(i, c)).collect::<Vec<_>>());
    let mut query = use_signal(String::new);
    let mut drawer_open = use_signal(|| false);
    let mut edit_id = use_signal(|| None::<String>);
    let mut form_name = use_signal(String::new);
    let mut form_env = use_signal(|| "Production".to_string());
    let mut form_budget = use_signal(|| 5000u32);
    let mut form_rps = use_signal(|| 50u32);
    let mut form_route = use_signal(|| "production-chat-default".to_string());
    let mut save_error = use_signal(String::new);

    let tenant_snapshot = tenants();
    let query_value = query().to_lowercase();
    let visible: Vec<Tenant> = tenant_snapshot
        .iter()
        .filter(|tenant| query_value.is_empty() || tenant.name.to_lowercase().contains(&query_value) || tenant.route.to_lowercase().contains(&query_value))
        .cloned()
        .collect();
    let total_spend: u32 = tenant_snapshot.iter().map(|t| t.spend).sum();
    let total_requests: u32 = tenant_snapshot.iter().map(|t| t.requests).sum();
    let critical = tenant_snapshot.iter().filter(|t| t.budget > 0 && (t.spend as f64 / t.budget as f64) >= 0.9).count();
    let selected = edit_id().and_then(|id| tenant_snapshot.iter().find(|t| t.id == id).cloned());

    rsx! {
        div { class: "page",
            {page_header(
                "Customers",
                "Manage tenant metadata, dynamic rate limits, model access budgets, and route policy mapping.",
                rsx! {
                    button {
                        class: "button button-primary",
                        onclick: move |_| {
                            edit_id.set(None);
                            form_name.set(String::new());
                            form_env.set("Production".to_string());
                            form_budget.set(5000);
                            form_rps.set(50);
                            form_route.set("production-chat-default".to_string());
                            save_error.set(String::new());
                            drawer_open.set(true);
                        },
                        Icon { name: "plus" }
                        "Add Tenant Customer"
                    }
                },
            )}

            div { class: "metrics",
                MetricCard { label: "Total Tenants", value: Box::leak(tenant_snapshot.len().to_string().into_boxed_str()), note: "", icon: "users", tone: "tone-gray" }
                MetricCard { label: "Active Month Spend", value: Box::leak(format!("${:.2}K", total_spend as f64 / 1000.0).into_boxed_str()), note: "", icon: "dollar", tone: "tone-green" }
                MetricCard { label: "Total Demands", value: Box::leak(format!("{:.2}M Req", total_requests as f64 / 1_000_000.0).into_boxed_str()), note: "", icon: "activity", tone: "tone-blue" }
                MetricCard { label: "Budget Alerts", value: Box::leak(format!("{} Critical", critical).into_boxed_str()), note: "", icon: "shield", tone: "tone-amber" }
            }

            div { class: "card table-card",
                div { style: "padding:20px;border-bottom:1px solid #f3f4f6;display:flex;align-items:center;gap:16px",
                    div { class: "search-field", style: "max-width:420px;flex:1",
                        Icon { name: "search" }
                        input {
                            class: "input",
                            placeholder: "Search tenants or active policies...",
                            value: query(),
                            oninput: move |evt| query.set(evt.value()),
                        }
                    }
                    span { class: "small muted", "Showing {visible.len()} of {tenant_snapshot.len()} tenants" }
                }
                div { class: "table-wrap",
                    table { class: "data-table",
                        thead { tr {
                            th { "Tenant Name" }
                            th { "Environment" }
                            th { "Monthly Cost Track" }
                            th { class: "right", "Quota Rate Limit" }
                            th { "Assigned Default Route" }
                            th { class: "center", "API Keys" }
                            th { class: "center", "Status" }
                            th { "" }
                        } }
                        tbody {
                            for tenant in visible {
                                {
                                    let id_for_edit = tenant.id.clone();
                                    let name_for_edit = tenant.name.clone();
                                    let env_for_edit = tenant.environment.clone();
                                    let route_for_edit = tenant.route.clone();
                                    let budget_for_edit = tenant.budget;
                                    let rps_for_edit = tenant.rps;
                                    let id_for_toggle = tenant.id.clone();
                                    let ratio = if tenant.budget == 0 { 0.0 } else { tenant.spend as f64 / tenant.budget as f64 };
                                    let width = format!("width:{:.0}%", (ratio * 100.0).min(100.0));
                                    let progress_tone = if ratio >= 0.9 { "background:#ef4444" } else if ratio >= 0.6 { "background:#f59e0b" } else { "background:#10b981" };
                                    rsx! {
                                        tr {
                                            td { class: "table-primary", "{tenant.name}" }
                                            td {
                                                span { class: if tenant.environment == "Production" { "badge badge-brand" } else if tenant.environment == "Staging" { "badge badge-warning" } else { "badge badge-neutral" }, "{tenant.environment}" }
                                            }
                                            td {
                                                div { class: "two-line", style: "min-width:150px",
                                                    span { class: if ratio >= 0.9 { "strong danger" } else { "strong" }, "${tenant.spend} / ${tenant.budget}" }
                                                    div { class: "progress tenant-progress", span { style: "{width};{progress_tone}" } }
                                                }
                                            }
                                            td { class: "right tabular", "{tenant.rps} RPS" }
                                            td { class: "mono muted", "{tenant.route}" }
                                            td { class: "center", span { class: "badge badge-neutral mono", "🔑 {tenant.keys}" } }
                                            td { class: "center",
                                                button {
                                                    class: if tenant.status == "Active" { "badge badge-success" } else { "badge badge-neutral" },
                                                    onclick: move |_| {
                                                        let mut next = tenants();
                                                        if let Some(item) = next.iter_mut().find(|item| item.id == id_for_toggle) {
                                                            item.status = if item.status == "Active" { "Suspended".to_string() } else { "Active".to_string() };
                                                        }
                                                        tenants.set(next);
                                                    },
                                                    "{tenant.status}"
                                                }
                                            }
                                            td {
                                                button {
                                                    class: "button button-ghost button-sm",
                                                    onclick: move |_| {
                                                        edit_id.set(Some(id_for_edit.clone()));
                                                        form_name.set(name_for_edit.clone());
                                                        form_env.set(env_for_edit.clone());
                                                        form_budget.set(budget_for_edit);
                                                        form_rps.set(rps_for_edit);
                                                        form_route.set(route_for_edit.clone());
                                                        save_error.set(String::new());
                                                        drawer_open.set(true);
                                                    },
                                                    "Edit"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Drawer {
                title: if edit_id().is_some() { "Tenant Profile" } else { "Create Tenant Customer" },
                open: drawer_open(),
                on_close: move |_| drawer_open.set(false),
                div { class: "stack-lg",
                    if let Some(ref tenant) = selected {
                        div { class: "card card-pad stack",
                            div { class: "row between", span { class: "section-label", "Historical Demands" } span { class: "tiny mono subtle", "ID: {tenant.id}" } }
                            div { class: "grid-2",
                                DetailStatOwned { label: "Total API Demands", value: format!("{} requests", tenant.requests) }
                                DetailStatOwned { label: "Spend Velocity", value: format!("${:.2} / day", tenant.spend as f64 / 30.0) }
                            }
                        }
                    }
                    div { class: "field",
                        label { "Tenant Customer Name" }
                        input { class: "input", value: form_name(), placeholder: "e.g. AeroTech Corp", oninput: move |evt| form_name.set(evt.value()) }
                    }
                    div { class: "grid-2",
                        div { class: "field",
                            label { "Environment" }
                            select { class: "select", value: form_env(), oninput: move |evt| form_env.set(evt.value()),
                                option { value: "Production", "Production" }
                                option { value: "Staging", "Staging" }
                                option { value: "Development", "Development" }
                            }
                        }
                        div { class: "field",
                            label { "Default Route Policy" }
                            select { class: "select", value: form_route(), oninput: move |evt| form_route.set(evt.value()),
                                for route in ROUTES { option { value: route.name, "{route.name}" } }
                            }
                        }
                    }
                    div { class: "field",
                        div { class: "row between", label { "Rate Limit Quota" } strong { class: "mono small", "{form_rps()} RPS" } }
                        input { r#type: "range", min: "5", max: "500", step: "5", value: form_rps().to_string(), oninput: move |evt| if let Ok(value) = evt.value().parse::<u32>() { form_rps.set(value); } }
                        span { class: "tiny subtle", "Throttle traffic automatically when this tenant exceeds the requests-per-second quota." }
                    }
                    div { class: "field",
                        div { class: "row between", label { "Monthly Budget Threshold" } strong { class: "mono small", "${form_budget()}" } }
                        input { r#type: "range", min: "500", max: "50000", step: "500", value: form_budget().to_string(), oninput: move |evt| if let Ok(value) = evt.value().parse::<u32>() { form_budget.set(value); } }
                        span { class: "tiny subtle", "Notifications or model failovers trigger once cumulative tenant spend crosses this threshold." }
                    }
                    if !save_error().is_empty() { div { class: "badge badge-error", style: "padding:10px", "{save_error}" } }
                    div { class: "row", style: "justify-content:flex-end",
                        button { class: "button button-secondary", onclick: move |_| drawer_open.set(false), "Cancel" }
                        button {
                            class: "button button-primary",
                            onclick: move |_| {
                                let name = form_name().trim().to_string();
                                if name.is_empty() {
                                    save_error.set("Tenant name is required.".to_string());
                                    return;
                                }
                                let mut next = tenants();
                                if let Some(id) = edit_id() {
                                    if let Some(item) = next.iter_mut().find(|item| item.id == id) {
                                        item.name = name;
                                        item.environment = form_env();
                                        item.budget = form_budget();
                                        item.rps = form_rps();
                                        item.route = form_route();
                                    }
                                } else {
                                    next.insert(0, Tenant {
                                        id: format!("c{}", next.len() + 1),
                                        name,
                                        environment: form_env(),
                                        spend: 0,
                                        budget: form_budget(),
                                        rps: form_rps(),
                                        route: form_route(),
                                        status: "Active".to_string(),
                                        keys: 1,
                                        requests: 0,
                                    });
                                }
                                tenants.set(next);
                                drawer_open.set(false);
                            },
                            if edit_id().is_some() { "Update Controls" } else { "Create Tenant" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Users() -> Element {
    rsx! { Customers {} }
}

#[component]
fn DetailStatOwned(label: &'static str, value: String) -> Element {
    rsx! { div { span { class: "tiny subtle", "{label}" } div { class: "small strong mono", "{value}" } } }
}

#[component]
pub fn Login() -> Element {
    let navigator = use_navigator();
    let password_nav = navigator.clone();
    let passkey_nav = navigator.clone();
    let demo_nav = navigator.clone();
    let mut passkey = use_signal(|| false);
    let mut email = use_signal(|| "wei@burncloud.io".to_string());
    let mut password = use_signal(|| "••••••••••••".to_string());
    let mut remember = use_signal(|| true);
    let mut status = use_signal(String::new);

    rsx! {
        div { class: "auth-page",
            header { class: "auth-header",
                Link { to: Route::Home {}, class: "brand-link", Logo {} span { class: "brand-name", "BurnCloud" } }
                div { class: "auth-header-note", span { "Don't have an account?" } Link { to: Route::Register {}, class: "strong", "Create account" } }
            }
            main { class: "auth-main",
                div { class: "auth-wrap",
                    div { class: "auth-intro",
                        Badge { text: "TPM Hardware Attested Portal", tone: "brand" }
                        h1 { "Sign in to BurnCloud" }
                        p { "Access your silicon route Gateway console and cryptographic receipts." }
                    }
                    div { class: "card auth-card",
                        div { class: "auth-tabs",
                            button { r#type: "button", class: if !passkey() { "auth-tab active" } else { "auth-tab" }, onclick: move |_| { passkey.set(false); status.set(String::new()); }, "Password & 2FA" }
                            button { r#type: "button", class: if passkey() { "auth-tab active" } else { "auth-tab" }, onclick: move |_| { passkey.set(true); status.set(String::new()); }, "◉ Passkey / Enclave" }
                        }
                        if !passkey() {
                            div { class: "auth-form",
                                div { class: "field", label { "Work Email Address" } input { class: "input", r#type: "email", required: true, value: email(), placeholder: "name@company.com", oninput: move |evt| email.set(evt.value()) } }
                                div { class: "field",
                                    div { class: "row between", label { "Password" } button { r#type: "button", class: "button button-ghost button-sm", onclick: move |_| status.set("A password reset token has been issued to your registered hardware key.".to_string()), "Forgot?" } }
                                    input { class: "input", r#type: "password", required: true, value: password(), placeholder: "••••••••••••", oninput: move |evt| password.set(evt.value()) }
                                }
                                div { class: "check-row",
                                    label { class: "row gap-2", input { r#type: "checkbox", checked: remember(), onclick: move |_| remember.set(!remember()) } "Remember this session" }
                                    span { class: "mono tiny subtle", "256-bit TLS" }
                                }
                                if !status().is_empty() { div { class: "terminal", style: "padding:10px", "{status}" } }
                                button {
                                    r#type: "button",
                                    class: "button button-primary",
                                    style: "width:100%",
                                    onclick: move |_| {
                                        if email().trim().is_empty() || password().trim().is_empty() {
                                            status.set("Email and password are required.".to_string());
                                        } else {
                                            status.set("Authentication accepted by BurnCloud Hardware Enclave.".to_string());
                                            password_nav.push(Route::Overview {});
                                        }
                                    },
                                    "Sign in to Console →"
                                }
                            }
                        } else {
                            div { class: "auth-form",
                                div { style: "text-align:center",
                                    div { class: "metric-icon tone-purple", style: "margin:0 auto;width:64px;height:64px", Icon { name: "lock" } }
                                    h3 { "Touch ID / YubiKey Authentication" }
                                    p { class: "small muted", "Authenticate using your physical hardware key or biometric enclave bound to your account." }
                                }
                                if !status().is_empty() { div { class: "terminal", style: "padding:10px", "{status}" } }
                                button { r#type: "button", class: "button button-primary", style: "width:100%", onclick: move |_| { status.set("Hardware security key accepted via WebAuthn TPM.".to_string()); passkey_nav.push(Route::Overview {}); }, "◉ Prompt Passkey Challenge" }
                            }
                        }
                        div { style: "border-top:1px solid #f3f4f6;padding-top:16px;text-align:center",
                            span { class: "section-label", "Or Quick Test Drive" }
                            button { r#type: "button", class: "button button-secondary", style: "width:100%;margin-top:12px", onclick: move |_| demo_nav.push(Route::Overview {}), "✨ One-Click Instant Demo Login" }
                        }
                    }
                    p { class: "tiny subtle mono", style: "text-align:center", "BurnCloud Security Enclave • Hardware Proof Protocol v2.4" }
                }
            }
            footer { class: "public-footer",
                div { class: "row", style: "justify-content:center",
                    Link { to: Route::Home {}, "Home" }
                    span { "•" }
                    Link { to: Route::Register {}, "Register" }
                    span { "•" }
                    a { href: "#privacy", "Privacy Policy" }
                    span { "•" }
                    a { href: "#terms", "Terms of Service" }
                }
            }
        }
    }
}

#[component]
pub fn Register() -> Element {
    let navigator = use_navigator();
    let submit_nav = navigator.clone();
    let demo_nav = navigator.clone();
    let mut tier = use_signal(|| 1usize);
    let mut full_name = use_signal(|| "Wei Huang".to_string());
    let mut company = use_signal(|| "BurnCloud AI Labs".to_string());
    let mut email = use_signal(|| "wei@burncloud.io".to_string());
    let mut password = use_signal(|| "••••••••••••".to_string());
    let mut terms = use_signal(|| true);
    let mut status = use_signal(String::new);

    rsx! {
        div { class: "auth-page",
            header { class: "auth-header",
                Link { to: Route::Home {}, class: "brand-link", Logo {} span { class: "brand-name", "BurnCloud" } }
                div { class: "auth-header-note", span { "Already have an account?" } Link { to: Route::Login {}, class: "strong", "Sign in" } }
            }
            main { class: "auth-main",
                div { class: "auth-wrap register",
                    div { class: "auth-intro",
                        Badge { text: "Includes $5 Free Token Credits • Pay-As-You-Go", tone: "success" }
                        h1 { "Deploy Your Silicon Gateway" }
                        p { "Get instant access to hardware-bound LLM routing, smart fallbacks, and verifiable cryptographic receipts." }
                    }
                    div { class: "card auth-card",
                        div { class: "auth-form",
                            div { class: "field",
                                label { "Select Initial Account Type" }
                                div { class: "tier-grid",
                                    TierButton { index: 0, current: tier(), name: "Free Sandbox", price: "$0 Free", detail: "2M Test Tokens", on_select: move |index| tier.set(index) }
                                    TierButton { index: 1, current: tier(), name: "Pay-As-You-Go", price: "Pay Per Token", detail: "$5 Free Credits", popular: true, on_select: move |index| tier.set(index) }
                                    TierButton { index: 2, current: tier(), name: "Enterprise", price: "Volume Rate", detail: "Post-paid Invoice", on_select: move |index| tier.set(index) }
                                }
                            }
                            div { class: "auth-form-grid",
                                div { class: "field", label { "Full Name" } input { class: "input", r#type: "text", required: true, value: full_name(), placeholder: "Jane Doe", oninput: move |evt| full_name.set(evt.value()) } }
                                div { class: "field", label { "Company / Team Name" } input { class: "input", r#type: "text", required: true, value: company(), placeholder: "Acme Corp", oninput: move |evt| company.set(evt.value()) } }
                            }
                            div { class: "field", label { "Work Email Address" } input { class: "input", r#type: "email", required: true, value: email(), placeholder: "name@company.com", oninput: move |evt| email.set(evt.value()) } }
                            div { class: "field", label { "Password" } input { class: "input", r#type: "password", required: true, value: password(), placeholder: "At least 8 characters", oninput: move |evt| password.set(evt.value()) } }
                            label { class: "small row gap-2", style: "align-items:flex-start",
                                input { r#type: "checkbox", checked: terms(), onclick: move |_| terms.set(!terms()) }
                                span { "I agree to BurnCloud's Terms of Service and Privacy Policy, including hardware attestation logging." }
                            }
                            if !status().is_empty() { div { class: "terminal", style: "padding:10px", "{status}" } }
                            button {
                                r#type: "button",
                                class: "button button-primary button-lg",
                                style: "width:100%",
                                onclick: move |_| {
                                    if !terms() {
                                        status.set("Please accept the Terms of Service to proceed.".to_string());
                                    } else if full_name().trim().is_empty() || company().trim().is_empty() || email().trim().is_empty() || password().trim().is_empty() {
                                        status.set("Complete all required account fields.".to_string());
                                    } else {
                                        status.set("Cryptographic workspace and TPM keys provisioned.".to_string());
                                        submit_nav.push(Route::Overview {});
                                    }
                                },
                                "Create Account & Open Console →"
                            }
                        }
                        div { style: "border-top:1px solid #f3f4f6;padding-top:16px;text-align:center",
                            button { r#type: "button", class: "button button-secondary", style: "width:100%", onclick: move |_| demo_nav.push(Route::Overview {}), "⚡ Instant Demo Registration" }
                        }
                    }
                    p { class: "tiny subtle mono", style: "text-align:center", "BurnCloud Gateway • Silicon Attested Multi-Cloud Infrastructure" }
                }
            }
            footer { class: "public-footer",
                div { class: "row", style: "justify-content:center",
                    Link { to: Route::Home {}, "Home" }
                    span { "•" }
                    Link { to: Route::Login {}, "Sign In" }
                    span { "•" }
                    a { href: "#privacy", "Privacy Policy" }
                    span { "•" }
                    a { href: "#terms", "Terms of Service" }
                }
            }
        }
    }
}

#[component]
fn TierButton(
    index: usize,
    current: usize,
    name: &'static str,
    price: &'static str,
    detail: &'static str,
    #[props(default)] popular: bool,
    on_select: EventHandler<usize>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: if current == index { "tier active" } else { "tier" },
            onclick: move |_| on_select.call(index),
            if popular { span { class: "popular", "POPULAR" } }
            strong { "{name}" }
            span { class: "price", "{price}" }
            small { "{detail}" }
        }
    }
}
