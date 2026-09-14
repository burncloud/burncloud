use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{
        buyer_logs, buyer_models, buyer_overview_snapshot, buyer_playground_chat,
        create_funding_request, funding_requests, BuyerOverviewSnapshot, FundingRequest, TokenDto,
        TokenService,
    },
    components::Icon,
};

fn compact(value: i64) -> String {
    if value.abs() >= 1_000_000 {
        format!("{:.2}M", value as f64 / 1_000_000.0)
    } else if value.abs() >= 1_000 {
        format!("{:.2}K", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

fn format_balance(snapshot: &BuyerOverviewSnapshot) -> String {
    let amount = snapshot.balance_nano as f64 / 1_000_000_000.0;
    if snapshot.balance_currency.eq_ignore_ascii_case("CNY") {
        format!("CNY {amount:.2}")
    } else {
        format!("${amount:.2}")
    }
}

#[component]
pub fn BuyerAPIKeys() -> Element {
    let user_id = crate::backend::use_auth()
        .user()
        .map(|user| user.id)
        .unwrap_or_default();
    let create_user_id = user_id.clone();
    let mut resource = use_resource(move || async move { TokenService::list().await });
    let mut busy = use_signal(|| false);
    let mut error = use_signal(String::new);
    let mut secret = use_signal(|| None::<String>);
    let snapshot = resource.read().clone();
    let loading = snapshot.is_none();
    let load_error = snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let tokens: Vec<TokenDto> = snapshot.and_then(Result::ok).unwrap_or_default();

    rsx! {
        div { class: "page buyer-workspace-page",
            div { class: "page-header",
                div { h2 { class: "page-title", "API Keys" } p { class: "page-subtitle", "Credentials owned by your Buyer account." } }
                button {
                    class: "button button-primary button-sm",
                    disabled: loading || busy() || create_user_id.is_empty(),
                    onclick: move |_| {
                        let owner = create_user_id.clone();
                        busy.set(true);
                        error.set(String::new());
                        spawn(async move {
                            match TokenService::create(&owner, None).await {
                                Ok(value) => { secret.set(Some(value)); resource.restart(); }
                                Err(message) => error.set(message),
                            }
                            busy.set(false);
                        });
                    },
                    Icon { name: "key" }
                    if busy() { "Creating..." } else { "Create API key" }
                }
            }
            if let Some(value) = secret() {
                section { class: "card card-pad stack", role: "status",
                    strong { "API key created" }
                    p { class: "small muted", "This bearer secret is shown once. Store it securely before dismissing it." }
                    code { class: "terminal", "{value}" }
                    button { class: "button button-primary button-sm", onclick: move |_| secret.set(None), "I stored this key" }
                }
            }
            if !error().is_empty() { div { class: "terminal auth-status auth-status-error", role: "alert", "{error}" } }
            if loading {
                div { class: "card product-empty", div { class: "product-empty-inner", h3 { "Loading API keys" } p { "Reading owner-scoped credential records." } } }
            } else if let Some(message) = load_error {
                div { class: "card card-pad stack", role: "alert", strong { "API keys unavailable" } code { class: "terminal", "{message}" } button { class: "button button-secondary button-sm", onclick: move |_| resource.restart(), "Retry" } }
            } else if tokens.is_empty() {
                div { class: "card product-empty", div { class: "product-empty-inner", h3 { "No API keys" } p { "Create a key to authenticate Buyer API requests." } } }
            } else {
                div { class: "card table-card", div { class: "table-wrap",
                    table { class: "data-table",
                        thead { tr { th { "Credential" } th { "Status" } th { "Used" } th { "Limit" } } }
                        tbody { for token in tokens.iter() { tr { key: "{token.token}", td { class: "mono", "{token.token}" } td { "{token.status}" } td { "{token.used_quota}" } td { if token.quota_limit < 0 { "Unlimited" } else { "{token.quota_limit}" } } } } }
                    }
                } }
            }
        }
    }
}

#[component]
pub fn BuyerUsage() -> Element {
    let auth = crate::backend::use_auth();
    let token = auth.token().unwrap_or_default();
    let snapshot_token = token.clone();
    let overview = use_resource(move || {
        let token = snapshot_token.clone();
        async move { buyer_overview_snapshot(&token).await }
    });
    let mut requests = use_resource(move || async move { funding_requests().await });
    let mut amount = use_signal(String::new);
    let mut note = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(String::new);
    let snapshot = overview.read().clone().and_then(Result::ok);
    let funding: Vec<FundingRequest> = requests
        .read()
        .clone()
        .and_then(Result::ok)
        .unwrap_or_default();

    rsx! {
        div { class: "page buyer-workspace-page",
            div { class: "page-header", div { h2 { class: "page-title", "Usage & Funding" } p { class: "page-subtitle", "UTC-today usage, account balance, and funding requests." } } }
            if let Some(snapshot) = snapshot.as_ref() {
                div { class: "metrics",
                    div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Today Spend" } span { class: "metric-value", {snapshot.today_spend_usd.map(|value| format!("${value:.4}")).unwrap_or_else(|| "Unavailable".to_string())} } span { class: "metric-note", "UTC today" } } }
                    div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Tokens Today" } span { class: "metric-value", {snapshot.tokens_today.map(compact).unwrap_or_else(|| "Unavailable".to_string())} } span { class: "metric-note", "UTC today" } } }
                    div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Balance" } span { class: "metric-value", {format_balance(snapshot)} } span { class: "metric-note", "{snapshot.balance_state}" } } }
                }
            } else {
                div { class: "card product-empty", div { class: "product-empty-inner", h3 { "Loading usage" } p { "Reading Buyer usage and balance." } } }
            }
            section { class: "card card-pad stack",
                div { class: "product-section-head", div { h3 { "Request funding" } p { "Requests are submitted for review and do not change balance automatically." } } }
                div { class: "form-grid",
                    label { class: "field", span { "Amount (USD)" } input { value: "{amount}", inputmode: "decimal", oninput: move |event| amount.set(event.value()) } }
                    label { class: "field", span { "Note" } input { value: "{note}", maxlength: "500", oninput: move |event| note.set(event.value()) } }
                }
                if !error().is_empty() { div { class: "terminal auth-status auth-status-error", role: "alert", "{error}" } }
                button {
                    class: "button button-primary button-sm",
                    disabled: busy(),
                    onclick: move |_| {
                        let parsed = amount().trim().parse::<f64>();
                        let note_value = note();
                        match parsed {
                            Ok(value) if value > 0.0 => {
                                busy.set(true);
                                error.set(String::new());
                                spawn(async move {
                                    match create_funding_request((value * 1_000_000_000.0).round() as i64, "USD", Some(&note_value)).await {
                                        Ok(_) => { amount.set(String::new()); note.set(String::new()); requests.restart(); }
                                        Err(message) => error.set(message),
                                    }
                                    busy.set(false);
                                });
                            }
                            _ => error.set("Enter a positive funding amount.".to_string()),
                        }
                    },
                    if busy() { "Submitting..." } else { "Submit funding request" }
                }
            }
            section { class: "card card-pad stack",
                div { class: "product-section-head", div { h3 { "Funding requests" } p { "Pending requests require account review." } } }
                if funding.is_empty() { p { class: "small muted", "No funding requests submitted." } }
                for request in funding.iter() {
                    div { class: "buyer-overview-model-row", key: "{request.id}",
                        div { strong { {format!("{} {:.2}", request.currency, request.amount as f64 / 1_000_000_000.0)} } span { {request.note.as_deref().unwrap_or("No note")} } }
                        div { class: "buyer-overview-model-usage", strong { "{request.status}" } span { "{request.created_at.as_deref().unwrap_or(\"Time unavailable\")}" } }
                    }
                }
            }
        }
    }
}

#[component]
pub fn BuyerBilling() -> Element {
    rsx! { BuyerUsage {} }
}

#[component]
pub fn BuyerMarketplace() -> Element {
    let token = crate::backend::use_auth().token().unwrap_or_default();
    let mut resource = use_resource(move || {
        let token = token.clone();
        async move { buyer_models(&token).await }
    });
    let snapshot = resource.read().clone();
    let models = snapshot.clone().and_then(Result::ok).unwrap_or_default();
    let error = snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    rsx! {
        div { class: "page buyer-workspace-page",
            div { class: "page-header", div { h2 { class: "page-title", "Marketplace" } p { class: "page-subtitle", "Buyer-safe models available for API requests." } } button { class: "button button-secondary button-sm", onclick: move |_| resource.restart(), "Refresh" } }
            if let Some(message) = error { div { class: "terminal auth-status auth-status-error", role: "alert", "{message}" } }
            else if snapshot.is_none() { div { class: "card product-empty", div { class: "product-empty-inner", h3 { "Loading models" } } } }
            else if models.is_empty() { div { class: "card product-empty", div { class: "product-empty-inner", h3 { "No models available" } p { "No Buyer model is currently enabled." } } } }
            else { div { class: "buyer-overview-model-list",
                for model in models.iter() { Link { class: "card buyer-overview-model-row", to: Route::BuyerPlayground {}, key: "{model.name}", div { strong { "{model.name}" } span { "Tier {model.tier}" } } div { class: "buyer-overview-model-usage", strong { "{model.status}" } span { "Open in Playground" } } } }
            } }
        }
    }
}

#[component]
pub fn BuyerLogs() -> Element {
    let token = crate::backend::use_auth().token().unwrap_or_default();
    let mut resource = use_resource(move || {
        let token = token.clone();
        async move { buyer_logs(&token).await }
    });
    let snapshot = resource.read().clone();
    let logs = snapshot.clone().and_then(Result::ok).unwrap_or_default();
    let error = snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    rsx! {
        div { class: "page buyer-workspace-page",
            div { class: "page-header", div { h2 { class: "page-title", "Logs" } p { class: "page-subtitle", "Recent requests owned by your Buyer account." } } button { class: "button button-secondary button-sm", onclick: move |_| resource.restart(), "Refresh" } }
            if let Some(message) = error { div { class: "terminal auth-status auth-status-error", role: "alert", "{message}" } }
            else if snapshot.is_none() { div { class: "card product-empty", div { class: "product-empty-inner", h3 { "Loading logs" } } } }
            else if logs.is_empty() { div { class: "card product-empty", div { class: "product-empty-inner", h3 { "No requests recorded" } } } }
            else { div { class: "card table-card", div { class: "table-wrap", table { class: "data-table",
                thead { tr { th { "Time" } th { "Model" } th { "Status" } th { "Tokens" } th { "Latency" } th { "Cost" } } }
                tbody { for log in logs.iter() { tr { key: "{log.request_id}", td { {log.created_at_utc.as_deref().unwrap_or("Unknown")} } td { {log.model.as_deref().unwrap_or("Unknown")} } td { "{log.status_code}" } td { "{log.tokens}" } td { "{log.latency_ms} ms" } td { {format!("${:.4}", log.cost_usd)} } } } }
            } } } }
        }
    }
}

#[component]
pub fn BuyerPlayground() -> Element {
    let token = crate::backend::use_auth().token().unwrap_or_default();
    let model_token = token.clone();
    let models_resource = use_resource(move || {
        let token = model_token.clone();
        async move { buyer_models(&token).await }
    });
    let tokens_resource = use_resource(move || async move { TokenService::list().await });
    let models = models_resource
        .read()
        .clone()
        .and_then(Result::ok)
        .unwrap_or_default();
    let tokens = tokens_resource
        .read()
        .clone()
        .and_then(Result::ok)
        .unwrap_or_default();
    let mut model = use_signal(String::new);
    let mut token_ref = use_signal(String::new);
    let mut prompt = use_signal(String::new);
    let mut response = use_signal(String::new);
    let mut error = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let selected_model = if model().is_empty() {
        models
            .first()
            .map(|item| item.name.clone())
            .unwrap_or_default()
    } else {
        model()
    };
    let selected_token = if token_ref().is_empty() {
        tokens
            .iter()
            .find(|item| item.status.eq_ignore_ascii_case("active"))
            .map(|item| item.token.clone())
            .unwrap_or_default()
    } else {
        token_ref()
    };
    rsx! {
        div { class: "page buyer-workspace-page",
            div { class: "page-header", div { h2 { class: "page-title", "Playground" } p { class: "page-subtitle", "Send a smoke-test request with an owned API key." } } }
            section { class: "card card-pad stack",
                label { class: "field", span { "Model" } select { value: "{selected_model}", onchange: move |event| model.set(event.value()), for item in models.iter() { option { value: "{item.name}", "{item.name}" } } } }
                label { class: "field", span { "API key" } select { value: "{selected_token}", onchange: move |event| token_ref.set(event.value()), for item in tokens.iter().filter(|item| item.status.eq_ignore_ascii_case("active")) { option { value: "{item.token}", "{item.token}" } } } }
                label { class: "field", span { "Prompt" } textarea { value: "{prompt}", rows: "6", oninput: move |event| prompt.set(event.value()) } }
                if !error().is_empty() { div { class: "terminal auth-status auth-status-error", role: "alert", "{error}" } }
                button {
                    class: "button button-primary",
                    disabled: busy() || selected_model.is_empty() || selected_token.is_empty() || prompt().trim().is_empty(),
                    onclick: move |_| {
                        let model_value = selected_model.clone();
                        let token_value = selected_token.clone();
                        let prompt_value = prompt();
                        busy.set(true); error.set(String::new()); response.set(String::new());
                        spawn(async move {
                            match buyer_playground_chat(&token_value, &model_value, &prompt_value).await { Ok(value) => response.set(value), Err(message) => error.set(message) }
                            busy.set(false);
                        });
                    },
                    if busy() { "Sending..." } else { "Send request" }
                }
            }
            if !response().is_empty() { section { class: "card card-pad stack", h3 { "Response" } pre { class: "terminal", "{response}" } } }
            if models.is_empty() { div { class: "product-note", "No Buyer model is currently available." } }
            if tokens.iter().all(|item| !item.status.eq_ignore_ascii_case("active")) { div { class: "product-note", Link { to: Route::BuyerAPIKeys {}, "Create or activate an API key before sending requests." } } }
        }
    }
}

#[component]
pub fn SupplierWorkspace() -> Element {
    rsx! { div { class: "page", div { class: "card product-empty", div { class: "product-empty-inner", h3 { "Supplier workspace unavailable" } p { "This account has the Supplier role, but no Supplier workspace capability is configured in this build." } } } } }
}
