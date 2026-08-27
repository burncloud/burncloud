mod auth;
mod backend;
mod marketplace;
mod overview;
mod playground;

use std::{env, net::SocketAddr};

use auth::{expired_session_cookie, render_login, safe_next, session_cookie, token_from_headers};
use axum::{
    Form, Json, Router,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, Uri, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use backend::{BackendClient, BillingSummary, CurrentAccount, PlaygroundRequest};
use marketplace::render_marketplace;
use overview::{OverviewData, ShellContext, render_overview, render_role_placeholder};
use playground::render_playground;
use serde::Deserialize;

const STYLES: &str = include_str!("../assets/styles.css");
const I18N_SCRIPT: &str = include_str!("../assets/i18n.js");
const SCRIPT: &str = include_str!("../assets/app.js");
const PLAYGROUND_SCRIPT: &str = include_str!("../assets/playground.js");
const MARKETPLACE_SCRIPT: &str = include_str!("../assets/marketplace.js");

#[derive(Clone)]
struct AppState {
    backend: BackendClient,
}

#[derive(Debug, Deserialize, Default)]
struct LoginQuery {
    next: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginForm {
    username: String,
    password: String,
    next: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ModelQuery {
    model: Option<String>,
}

#[tokio::main]
async fn main() {
    let port = env::var("BURNCLOUD_UI_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3001);
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let backend = BackendClient::from_environment()
        .unwrap_or_else(|error| panic!("failed to configure BurnCloud backend client: {error}"));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .unwrap_or_else(|error| panic!("failed to bind BurnCloud UI at {address}: {error}"));

    println!("BurnCloud Rust UI: http://{address}/buyer/overview");
    axum::serve(listener, app(AppState { backend }))
        .await
        .expect("BurnCloud UI server stopped unexpectedly");
}

fn app(state: AppState) -> Router {
    Router::new()
        .route(
            "/",
            get(|| async { Redirect::temporary("/buyer/overview") }),
        )
        .route("/login", get(login_page))
        .route("/session/login", post(login_session))
        .route("/session/logout", post(logout_session))
        .route(
            "/buyer",
            get(|| async { Redirect::temporary("/buyer/overview") }),
        )
        .route("/buyer/overview", get(overview_page))
        .route("/buyer/playground", get(playground_page))
        .route("/playground", get(playground_page))
        .route("/buyer/marketplace", get(marketplace_page))
        .route("/marketplace", get(marketplace_page))
        .route("/models", get(marketplace_page))
        .route("/supplier", get(supplier_page))
        .route("/supplier/overview", get(supplier_page))
        .route("/admin", get(admin_page))
        .route("/admin/overview", get(admin_page))
        .route("/api/playground/chat", post(playground_api))
        .route("/favicon.ico", get(favicon))
        .route("/assets/styles.css", get(styles))
        .route("/assets/i18n.js", get(i18n_script))
        .route("/assets/app.js", get(script))
        .route("/assets/playground.js", get(playground_script))
        .route("/assets/marketplace.js", get(marketplace_script))
        .fallback(not_found)
        .with_state(state)
}

async fn login_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<LoginQuery>,
) -> Response {
    if let Some(token) = token_from_headers(&headers) {
        if state.backend.current_account(&token).await.is_ok() {
            return Redirect::to(safe_next(query.next.as_deref())).into_response();
        }
    }
    no_store(Html(render_login(
        safe_next(query.next.as_deref()),
        query.error.as_deref() == Some("credentials"),
        query.error.as_deref() == Some("backend"),
    )))
}

async fn login_session(State(state): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    let next = safe_next(form.next.as_deref()).to_string();
    if form.username.trim().is_empty() || form.password.is_empty() {
        return Redirect::to(&format!(
            "/login?error=credentials&next={}",
            query_component(&next)
        ))
        .into_response();
    }
    match state
        .backend
        .login(form.username.trim(), &form.password)
        .await
    {
        Ok(auth) => {
            let mut response = Redirect::to(&next).into_response();
            if let Ok(value) = HeaderValue::from_str(&session_cookie(&auth.token)) {
                response.headers_mut().insert(header::SET_COOKIE, value);
            }
            response
        }
        Err(error) => {
            let kind = if error.status.is_none() {
                "backend"
            } else {
                "credentials"
            };
            Redirect::to(&format!(
                "/login?error={kind}&next={}",
                query_component(&next)
            ))
            .into_response()
        }
    }
}

async fn logout_session() -> Response {
    let mut response = Redirect::to("/login").into_response();
    if let Ok(value) = HeaderValue::from_str(&expired_session_cookie()) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

async fn overview_page(State(state): State<AppState>, headers: HeaderMap, uri: Uri) -> Response {
    let (token, account) = match require_session(&state, &headers, uri.path()).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    let (billing, catalog, tokens, recharges) = tokio::join!(
        state.backend.billing_today(&token),
        state.backend.catalog(&token),
        state.backend.tokens(&token),
        state.backend.recharges(&token),
    );
    let mut warnings = Vec::new();
    let billing = billing.unwrap_or_else(|error| {
        warnings.push(format!("今日账单：{error}"));
        BillingSummary::default()
    });
    let catalog = catalog.unwrap_or_else(|error| {
        warnings.push(format!("模型目录：{error}"));
        Vec::new()
    });
    let tokens = tokens.unwrap_or_else(|error| {
        warnings.push(format!("API 密钥：{error}"));
        Vec::new()
    });
    let recharges = recharges.unwrap_or_else(|error| {
        warnings.push(format!("充值记录：{error}"));
        Vec::new()
    });
    let shell = shell_context(account, !warnings.is_empty());
    no_store(Html(render_overview(&OverviewData {
        shell,
        billing,
        catalog,
        tokens,
        recharges,
        warnings,
    })))
}

async fn playground_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    uri: Uri,
    Query(query): Query<ModelQuery>,
) -> Response {
    let (token, account) = match require_session(&state, &headers, uri.path()).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    let (catalog, tokens) =
        tokio::join!(state.backend.catalog(&token), state.backend.tokens(&token));
    let mut warnings = Vec::new();
    let catalog = catalog.unwrap_or_else(|error| {
        warnings.push(format!("模型目录：{error}"));
        Vec::new()
    });
    let tokens = tokens.unwrap_or_else(|error| {
        warnings.push(format!("API 密钥：{error}"));
        Vec::new()
    });
    let shell = shell_context(account, !warnings.is_empty());
    no_store(Html(render_playground(
        &shell,
        &catalog,
        &tokens,
        query.model.as_deref(),
        &warnings,
    )))
}

async fn marketplace_page(State(state): State<AppState>, headers: HeaderMap, uri: Uri) -> Response {
    let (token, account) = match require_session(&state, &headers, uri.path()).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    let (catalog, warnings) = match state.backend.catalog(&token).await {
        Ok(catalog) => (catalog, Vec::new()),
        Err(error) => (Vec::new(), vec![format!("模型目录：{error}")]),
    };
    let shell = shell_context(account, !warnings.is_empty());
    no_store(Html(render_marketplace(&shell, &catalog, &warnings)))
}

async fn supplier_page(State(state): State<AppState>, headers: HeaderMap, uri: Uri) -> Response {
    let (_, account) = match require_session(&state, &headers, uri.path()).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    let shell = shell_context(account, false);
    no_store(Html(render_role_placeholder(&shell, "supplier", true)))
}

async fn admin_page(State(state): State<AppState>, headers: HeaderMap, uri: Uri) -> Response {
    let (_, account) = match require_session(&state, &headers, uri.path()).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    let allowed = account.is_admin();
    let shell = shell_context(account, !allowed);
    let status = if allowed {
        StatusCode::OK
    } else {
        StatusCode::FORBIDDEN
    };
    no_store((
        status,
        Html(render_role_placeholder(&shell, "admin", allowed)),
    ))
}

async fn playground_api(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PlaygroundRequest>,
) -> Response {
    let (token, account) = match require_session(&state, &headers, "/buyer/playground").await {
        Ok(session) => session,
        Err(response) => return response,
    };
    if account.status != 1 {
        return json_error(StatusCode::FORBIDDEN, "当前账户不可用");
    }
    if payload.token_ref.trim().is_empty()
        || payload.model.trim().is_empty()
        || payload.messages.is_empty()
        || payload.messages.len() > 32
        || payload
            .messages
            .iter()
            .any(|message| message.content.len() > 32_000)
        || !(0.0..=2.0).contains(&payload.temperature)
        || !(1..=32_768).contains(&payload.max_tokens)
    {
        return json_error(StatusCode::BAD_REQUEST, "推理参数无效或超出限制");
    }
    match state.backend.playground(&token, &payload).await {
        Ok(proxy) => {
            let mut response = (proxy.status, proxy.body).into_response();
            if let Ok(value) = HeaderValue::from_str(&proxy.content_type) {
                response.headers_mut().insert(header::CONTENT_TYPE, value);
            }
            if let Some(channel_id) = proxy
                .channel_id
                .and_then(|value| HeaderValue::from_str(&value).ok())
            {
                response.headers_mut().insert("x-channel-id", channel_id);
            }
            if let Some(model_id) = proxy
                .model_id
                .and_then(|value| HeaderValue::from_str(&value).ok())
            {
                response.headers_mut().insert("x-model-id", model_id);
            }
            no_store(response)
        }
        Err(error) => json_error(
            error
                .status
                .and_then(|value| StatusCode::from_u16(value).ok())
                .unwrap_or(StatusCode::BAD_GATEWAY),
            &error.message,
        ),
    }
}

async fn require_session(
    state: &AppState,
    headers: &HeaderMap,
    next: &str,
) -> Result<(String, CurrentAccount), Response> {
    let Some(token) = token_from_headers(headers) else {
        return Err(
            Redirect::to(&format!("/login?next={}", query_component(next))).into_response(),
        );
    };
    match state.backend.current_account(&token).await {
        Ok(account) => Ok((token, account)),
        Err(error) if matches!(error.status, Some(401 | 403 | 404)) => {
            let mut response =
                Redirect::to(&format!("/login?next={}", query_component(next))).into_response();
            if let Ok(value) = HeaderValue::from_str(&expired_session_cookie()) {
                response.headers_mut().insert(header::SET_COOKIE, value);
            }
            Err(response)
        }
        Err(error) => Err(no_store((
            StatusCode::SERVICE_UNAVAILABLE,
            Html(render_service_error(&error.message)),
        ))),
    }
}

fn shell_context(account: CurrentAccount, has_warning: bool) -> ShellContext {
    let balance_label = format!("{}{:.2}", account.currency_symbol(), account.balance());
    let attention = has_warning || account.status != 1 || account.balance() < 20.0;
    ShellContext {
        account,
        balance_label,
        attention,
    }
}

fn render_service_error(message: &str) -> String {
    format!(
        r#"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>服务不可用 - BurnCloud</title><link rel="stylesheet" href="/assets/styles.css"><script src="/assets/i18n.js" defer></script></head><body><main class="standalone-message"><div><span class="eyebrow">BACKEND CONNECTION</span><h1>BurnCloud 后端暂不可用</h1><p>{}</p><a class="button primary" href="/buyer/overview">重新连接</a></div></main></body></html>"#,
        auth::escape_html(message)
    )
}

fn json_error(status: StatusCode, message: &str) -> Response {
    no_store((
        status,
        Json(serde_json::json!({ "error": { "message": message } })),
    ))
}

fn no_store<T: IntoResponse>(value: T) -> Response {
    let mut response = value.into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
        .headers_mut()
        .insert("referrer-policy", HeaderValue::from_static("same-origin"));
    response
}

fn query_component(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

async fn favicon() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn styles() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], STYLES)
}
async fn i18n_script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        I18N_SCRIPT,
    )
}
async fn script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        SCRIPT,
    )
}
async fn playground_script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        PLAYGROUND_SCRIPT,
    )
}
async fn marketplace_script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        MARKETPLACE_SCRIPT,
    )
}

async fn not_found(headers: HeaderMap, State(state): State<AppState>, uri: Uri) -> Response {
    if uri.path().starts_with("/buyer/")
        || uri.path().starts_with("/supplier/")
        || uri.path().starts_with("/admin/")
    {
        if let Err(response) = require_session(&state, &headers, uri.path()).await {
            return response;
        }
    }
    (StatusCode::NOT_FOUND, Html(r#"<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>页面不存在 - BurnCloud</title><link rel="stylesheet" href="/assets/styles.css"><script src="/assets/i18n.js" defer></script><body><main class="standalone-message"><div><span class="eyebrow">404</span><h1>页面不存在</h1><p>请求的路由尚未迁移或不存在。</p><a class="button primary" href="/buyer/overview">返回概览</a></div></main></body></html>"#)).into_response()
}

#[cfg(test)]
mod tests {
    use super::{AppState, app};
    use crate::backend::BackendClient;

    #[test]
    fn router_builds_with_protected_buyer_routes() {
        let backend = BackendClient::new("http://127.0.0.1:9").expect("backend client");
        let _router = app(AppState { backend });
    }
}
