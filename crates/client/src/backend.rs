use dioxus::prelude::*;
use reqwest::{Client, RequestBuilder, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurrentUser {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientState {
    pub last_username: Option<String>,
    pub auth_token: Option<String>,
    pub user_info: Option<String>,
}

impl ClientState {
    #[cfg(not(target_arch = "wasm32"))]
    fn path() -> std::path::PathBuf {
        let dir = if cfg!(target_os = "windows") {
            let profile = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
            std::path::PathBuf::from(profile)
                .join("AppData")
                .join("Local")
                .join("BurnCloud")
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".burncloud")
        };
        let _ = std::fs::create_dir_all(&dir);
        dir.join("client_state.json")
    }

    pub fn load() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            Self::default()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = Self::path();
            std::fs::read_to_string(path)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or_default()
        }
    }

    pub fn save(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = Self::path();
            if let Ok(text) = serde_json::to_string_pretty(self) {
                if std::fs::write(&path, text).is_ok() {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
                    }
                }
            }
        }
    }

    pub fn clear() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = std::fs::remove_file(Self::path());
        }
    }
}

#[derive(Clone, Copy)]
pub struct AuthContext {
    token: Signal<Option<String>>,
    user: Signal<Option<CurrentUser>>,
}

impl AuthContext {
    fn new() -> Self {
        let state = ClientState::load();
        let user = state
            .user_info
            .as_deref()
            .and_then(|json| serde_json::from_str::<CurrentUser>(json).ok());
        let token = if user.is_some() { state.auth_token } else { None };
        Self {
            token: Signal::new(token),
            user: Signal::new(user),
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.token.read().is_some() && self.user.read().is_some()
    }

    pub fn token(&self) -> Option<String> {
        self.token.read().clone()
    }

    pub fn user(&self) -> Option<CurrentUser> {
        self.user.read().clone()
    }

    pub fn set(mut self, token: String, user: CurrentUser, remember: bool) {
        *self.token.write() = Some(token.clone());
        *self.user.write() = Some(user.clone());
        if remember {
            ClientState {
                last_username: Some(user.username.clone()),
                auth_token: Some(token),
                user_info: serde_json::to_string(&user).ok(),
            }
            .save();
        } else {
            ClientState::clear();
        }
    }

    pub fn clear(mut self) {
        *self.token.write() = None;
        *self.user.write() = None;
        ClientState::clear();
    }
}

pub fn use_init_auth() -> AuthContext {
    use_context_provider(AuthContext::new)
}

pub fn use_auth() -> AuthContext {
    use_context::<AuthContext>()
}

fn port() -> String {
    std::env::var("PORT").unwrap_or_else(|_| burncloud_common::constants::DEFAULT_PORT.to_string())
}

pub fn server_root() -> String {
    std::env::var("BURNCLOUD_API_BASE")
        .unwrap_or_else(|_| format!("http://127.0.0.1:{}", port()))
        .trim_end_matches('/')
        .to_string()
}

fn url(path: &str) -> String {
    format!("{}{}", server_root(), path)
}

fn auth_token_from_disk() -> Option<String> {
    ClientState::load().auth_token
}

fn with_auth(request: RequestBuilder) -> RequestBuilder {
    if let Some(token) = auth_token_from_disk() {
        request.header("Authorization", format!("Bearer {token}"))
    } else {
        request
    }
}

fn with_token(request: RequestBuilder, token: &str) -> RequestBuilder {
    request.header("Authorization", format!("Bearer {token}"))
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

async fn decode_envelope<T: DeserializeOwned>(response: Response) -> Result<T, String> {
    let status = response.status();
    let text = response.text().await.map_err(|e| e.to_string())?;
    let envelope: ApiEnvelope<T> = serde_json::from_str(&text)
        .map_err(|e| format!("Invalid API response ({status}): {e}; body={text}"))?;
    if status.is_success() && envelope.success {
        envelope.data.ok_or_else(|| "API response did not include data".to_string())
    } else {
        Err(envelope
            .message
            .unwrap_or_else(|| format!("API request failed: {status}")))
    }
}

async fn decode_unit(response: Response) -> Result<(), String> {
    let status = response.status();
    let text = response.text().await.map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("Invalid API response ({status}): {e}; body={text}"))?;
    if status.is_success() && value.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
        Ok(())
    } else {
        Err(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("API request failed")
            .to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthData {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub roles: Vec<String>,
    pub token: String,
}

pub struct AuthService;

impl AuthService {
    pub async fn login(username: &str, password: &str) -> Result<AuthData, String> {
        let response = Client::new()
            .post(url("/api/auth/login"))
            .json(&serde_json::json!({ "username": username, "password": password }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_envelope(response).await
    }

    pub async fn register(username: &str, password: &str, email: Option<&str>) -> Result<AuthData, String> {
        let response = Client::new()
            .post(url("/api/auth/register"))
            .json(&serde_json::json!({ "username": username, "password": password, "email": email }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_envelope(response).await
    }

    pub async fn forgot_password(email: &str) -> Result<(), String> {
        let response = Client::new()
            .post(url("/api/auth/forgot-password"))
            .json(&serde_json::json!({ "email": email }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_unit(response).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct User {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub balance_usd: i64,
    #[serde(default)]
    pub balance_cny: i64,
    #[serde(default)]
    pub preferred_currency: Option<String>,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub status: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CurrentAccount {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub status: i32,
    #[serde(default)]
    pub balance_usd: i64,
    #[serde(default)]
    pub balance_cny: i64,
    #[serde(default)]
    pub preferred_currency: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UserRecharge {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub amount: i64,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

pub struct UserService;

impl UserService {
    pub async fn current_account(token: &str) -> Result<CurrentAccount, String> {
        let response = with_token(Client::new().get(url("/console/api/user/me")), token)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_envelope(response).await
    }

    pub async fn recharges(token: &str) -> Result<Vec<UserRecharge>, String> {
        let response = with_token(
            Client::new().get(url("/console/api/user/recharges")),
            token,
        )
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_envelope(response).await
    }

    pub async fn list() -> Result<Vec<User>, String> {
        let response = with_auth(Client::new().get(url("/console/api/list_users")))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_envelope(response).await
    }

    pub async fn create(username: &str, password: &str, email: Option<&str>) -> Result<AuthData, String> {
        let response = with_auth(Client::new().post(url("/console/api/user/register")))
            .json(&serde_json::json!({ "username": username, "password": password, "email": email }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_envelope(response).await
    }

    pub async fn topup(user_id: &str, amount_nano: i64, currency: &str) -> Result<i64, String> {
        #[derive(Deserialize)]
        struct TopupData { balance: i64 }
        let response = with_auth(Client::new().post(url("/console/api/user/topup")))
            .json(&serde_json::json!({ "user_id": user_id, "amount": amount_nano, "currency": currency }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_envelope::<TopupData>(response).await.map(|v| v.balance)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RouterLog {
    #[serde(default)] pub id: i64,
    #[serde(default)] pub request_id: String,
    #[serde(default)] pub user_id: Option<String>,
    #[serde(default)] pub path: String,
    #[serde(default)] pub upstream_id: Option<String>,
    #[serde(default)] pub status_code: i32,
    #[serde(default)] pub latency_ms: i64,
    #[serde(default)] pub prompt_tokens: i32,
    #[serde(default)] pub completion_tokens: i32,
    #[serde(default)] pub cost: i64,
    #[serde(default)] pub model: Option<String>,
    #[serde(default)] pub cache_read_tokens: i32,
    #[serde(default)] pub reasoning_tokens: i32,
    #[serde(default)] pub pricing_region: Option<String>,
    #[serde(default)] pub layer_decision: Option<String>,
    #[serde(default)] pub traffic_color: Option<String>,
    #[serde(default)] pub cost_status: Option<String>,
    #[serde(default)] pub error_type: Option<String>,
    #[serde(default)] pub created_at: Option<String>,
}

impl RouterLog {
    pub fn total_tokens(&self) -> i64 {
        self.prompt_tokens as i64 + self.completion_tokens as i64 + self.cache_read_tokens as i64 + self.reasoning_tokens as i64
    }

    pub fn cost_usd(&self) -> f64 {
        self.cost as f64 / 1_000_000_000.0
    }

    pub fn status_label(&self) -> &'static str {
        if self.status_code >= 500 || self.error_type.as_deref() == Some("timeout") {
            "Timeout"
        } else if self.layer_decision.as_deref().unwrap_or("").contains("failover") {
            "Fallback"
        } else if self.status_code >= 400 {
            "Error"
        } else {
            "Success"
        }
    }
}

#[derive(Debug, Deserialize)]
struct LogPage {
    #[serde(default)] data: Vec<RouterLog>,
}

pub struct LogService;
impl LogService {
    pub async fn list(limit: usize) -> Result<Vec<RouterLog>, String> {
        let page_size = limit.clamp(1, 500);
        let response = with_auth(Client::new().get(url(&format!("/console/api/logs?page=1&page_size={page_size}"))))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = response.status();
        let text = response.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("Logs API failed ({status}): {text}"));
        }
        serde_json::from_str::<LogPage>(&text)
            .map(|p| p.data)
            .map_err(|e| format!("Invalid logs response: {e}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Channel {
    #[serde(default)] pub id: i32,
    #[serde(rename = "type", default)] pub type_: i32,
    #[serde(default)] pub key: String,
    #[serde(default)] pub name: String,
    #[serde(default)] pub base_url: Option<String>,
    #[serde(default)] pub models: String,
    #[serde(default)] pub group: String,
    #[serde(default)] pub status: i32,
    #[serde(default)] pub weight: i32,
    #[serde(default)] pub priority: i64,
    #[serde(default)] pub param_override: Option<String>,
    #[serde(default)] pub header_override: Option<String>,
    #[serde(default)] pub api_version: Option<String>,
    #[serde(default)] pub model_mapping: Option<String>,
    #[serde(default)] pub rpm_cap: Option<i32>,
    #[serde(default)] pub tpm_cap: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ChannelListData {
    #[serde(default)] channels: Vec<Channel>,
}

#[derive(Debug, Serialize)]
struct ChannelPayload<'a> {
    id: Option<i32>,
    #[serde(rename = "type")]
    type_: i32,
    key: &'a str,
    name: &'a str,
    base_url: Option<&'a str>,
    models: &'a str,
    group: &'a str,
    weight: i32,
    priority: i64,
    param_override: Option<&'a str>,
    header_override: Option<&'a str>,
    api_version: Option<&'a str>,
    model_mapping: Option<&'a str>,
    rpm_cap: Option<i32>,
    tpm_cap: Option<i64>,
}

fn channel_payload(channel: &Channel) -> ChannelPayload<'_> {
    ChannelPayload {
        id: if channel.id > 0 { Some(channel.id) } else { None },
        type_: channel.type_,
        key: &channel.key,
        name: &channel.name,
        base_url: channel.base_url.as_deref(),
        models: &channel.models,
        group: &channel.group,
        weight: channel.weight,
        priority: channel.priority,
        param_override: channel.param_override.as_deref(),
        header_override: channel.header_override.as_deref(),
        api_version: channel.api_version.as_deref(),
        model_mapping: channel.model_mapping.as_deref(),
        rpm_cap: channel.rpm_cap,
        tpm_cap: channel.tpm_cap,
    }
}

pub struct ChannelService;
impl ChannelService {
    pub async fn list(limit: usize) -> Result<Vec<Channel>, String> {
        let limit = limit.clamp(1, 100);
        let response = with_auth(Client::new().get(url(&format!("/console/api/channel?limit={limit}&offset=0"))))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_envelope::<ChannelListData>(response).await.map(|d| d.channels)
    }

    pub async fn create(channel: &Channel) -> Result<(), String> {
        let response = with_auth(Client::new().post(url("/console/api/channel")))
            .json(&channel_payload(channel))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_unit(response).await
    }

    pub async fn update(channel: &Channel) -> Result<(), String> {
        let response = with_auth(Client::new().put(url("/console/api/channel")))
            .json(&channel_payload(channel))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_unit(response).await
    }

    pub async fn delete(id: i32) -> Result<(), String> {
        let response = with_auth(Client::new().delete(url(&format!("/console/api/channel/{id}"))))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_unit(response).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TokenDto {
    #[serde(default)] pub token: String,
    #[serde(default)] pub user_id: String,
    #[serde(default)] pub status: String,
    #[serde(default = "default_quota")] pub quota_limit: i64,
    #[serde(default)] pub used_quota: i64,
    #[serde(default)] pub key_version: i32,
    #[serde(default)] pub ip_whitelist: Option<String>,
    #[serde(default)] pub created_at: i64,
}

fn default_quota() -> i64 { -1 }

pub struct TokenService;
impl TokenService {
    pub async fn list() -> Result<Vec<TokenDto>, String> {
        let response = with_auth(Client::new().get(url("/console/api/tokens")))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_envelope(response).await
    }

    pub async fn list_with_token(token: &str) -> Result<Vec<TokenDto>, String> {
        let response = with_token(Client::new().get(url("/console/api/tokens")), token)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_envelope(response).await
    }

    pub async fn create(user_id: &str, quota_limit: Option<i64>) -> Result<String, String> {
        #[derive(Deserialize)] struct Created { token: String }
        let response = with_auth(Client::new().post(url("/console/api/tokens")))
            .json(&serde_json::json!({ "user_id": user_id, "quota_limit": quota_limit }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_envelope::<Created>(response).await.map(|d| d.token)
    }

    pub async fn set_status(token: &str, status: &str) -> Result<(), String> {
        let response = with_auth(Client::new().put(url(&format!("/console/api/tokens/{token}"))))
            .json(&serde_json::json!({ "status": status }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_unit(response).await
    }

    pub async fn delete(token: &str) -> Result<(), String> {
        let response = with_auth(Client::new().delete(url(&format!("/console/api/tokens/{token}"))))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_unit(response).await
    }

    pub async fn rotate(token: &str, hours: i32, revoke_old: bool) -> Result<serde_json::Value, String> {
        let response = with_auth(Client::new().post(url(&format!("/console/api/tokens/{token}/rotate"))))
            .json(&serde_json::json!({ "transition_period_hours": hours, "revoke_old": revoke_old }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_envelope(response).await
    }

    pub async fn set_ip_whitelist(token: &str, whitelist: &str) -> Result<(), String> {
        let response = with_auth(Client::new().post(url(&format!("/console/api/tokens/{token}/ip-whitelist"))))
            .json(&serde_json::json!({ "ip_whitelist": whitelist }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        decode_unit(response).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UsageStats {
    #[serde(default)] pub prompt_tokens: i64,
    #[serde(default)] pub completion_tokens: i64,
    #[serde(default)] pub total_tokens: i64,
}

pub async fn user_usage(user_id: &str, token: &str) -> Result<UsageStats, String> {
    let response = with_token(Client::new().get(url(&format!("/console/api/usage/{user_id}"))), token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    let text = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() { return Err(format!("Usage API failed ({status}): {text}")); }
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BillingModelSummary {
    #[serde(default)] pub model: String,
    #[serde(default)] pub requests: i64,
    #[serde(default)] pub prompt_tokens: i64,
    #[serde(default)] pub cache_read_tokens: i64,
    #[serde(default)] pub completion_tokens: i64,
    #[serde(default)] pub reasoning_tokens: i64,
    #[serde(default)] pub cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BillingSummary {
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    #[serde(default)] pub pre_migration_requests: i64,
    #[serde(default)] pub models: Vec<BillingModelSummary>,
    #[serde(default)] pub total_cost_usd: f64,
}

pub async fn billing_summary(token: &str) -> Result<BillingSummary, String> {
    let response = with_token(Client::new().get(url("/api/billing/summary")), token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    decode_envelope(response).await
}

pub async fn billing_summary_for_period(
    token: &str,
    start: &str,
    end: &str,
) -> Result<BillingSummary, String> {
    let response = with_token(
        Client::new()
            .get(url("/api/billing/summary"))
            .query(&[("start", start), ("end", end)]),
        token,
    )
    .send()
    .await
    .map_err(|e| e.to_string())?;
    decode_envelope(response).await
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryInfo {
    #[serde(default)] pub total: u64,
    #[serde(default)] pub used: u64,
    #[serde(default)] pub available: u64,
    #[serde(default)] pub usage_percent: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuInfo {
    #[serde(default)] pub usage_percent: f32,
    #[serde(default)] pub core_count: usize,
    #[serde(default)] pub frequency: u64,
    #[serde(default)] pub brand: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiskInfo {
    #[serde(default)] pub total: u64,
    #[serde(default)] pub used: u64,
    #[serde(default)] pub available: u64,
    #[serde(default)] pub usage_percent: f32,
    #[serde(default)] pub mount_point: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemMetrics {
    #[serde(default)] pub cpu: CpuInfo,
    #[serde(default)] pub memory: MemoryInfo,
    #[serde(default)] pub disks: Vec<DiskInfo>,
    #[serde(default)] pub timestamp: u64,
}

pub async fn system_metrics() -> Result<SystemMetrics, String> {
    let response = with_auth(Client::new().get(url("/console/api/monitor")))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    decode_envelope(response).await
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SecuritySummary {
    #[serde(default)] pub score: u8,
    #[serde(default)] pub blocked_count: u64,
    #[serde(default)] pub threat_source_count: u64,
    #[serde(default)] pub sparkline: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FilterConfig {
    #[serde(default)] pub content_filter_enabled: bool,
    #[serde(default)] pub blacklist_enabled: bool,
    #[serde(default)] pub custom_rules: Vec<String>,
}

pub async fn security_summary() -> Result<SecuritySummary, String> {
    let response = with_auth(Client::new().get(url("/console/api/monitor/security")))
        .send().await.map_err(|e| e.to_string())?;
    decode_envelope(response).await
}

pub async fn filter_config() -> Result<FilterConfig, String> {
    let response = with_auth(Client::new().get(url("/console/api/monitor/security/filters")))
        .send().await.map_err(|e| e.to_string())?;
    decode_envelope(response).await
}

pub async fn update_filter_config(config: &FilterConfig) -> Result<FilterConfig, String> {
    let response = with_auth(Client::new().put(url("/console/api/monitor/security/filters")))
        .json(config)
        .send().await.map_err(|e| e.to_string())?;
    decode_envelope(response).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatUsage {
    #[serde(default)] pub prompt_tokens: i64,
    #[serde(default)] pub completion_tokens: i64,
    #[serde(default)] pub total_tokens: i64,
}
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RouteTrace {
    pub channel_id: Option<String>,
    pub model_id: Option<String>,
}
#[derive(Debug, Deserialize)]
struct ChatChoiceMessage { content: Option<String> }
#[derive(Debug, Deserialize)]
struct ChatChoice { message: Option<ChatChoiceMessage> }
#[derive(Debug, Deserialize)]
struct ChatResponse {
    #[serde(default)] choices: Vec<ChatChoice>,
    #[serde(default)] usage: ChatUsage,
}

#[derive(Debug, Clone)]
pub struct ChatResult {
    pub content: String,
    pub usage: ChatUsage,
    pub trace: RouteTrace,
}

pub async fn chat_completion(
    messages: &[ChatMessage],
    model: &str,
    bearer_token: &str,
    temperature: f64,
    max_tokens: i64,
) -> Result<ChatResult, String> {
    let response = Client::new()
        .post(url("/v1/chat/completions"))
        .header("Authorization", format!("Bearer {bearer_token}"))
        .json(&serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
            "temperature": temperature,
            "max_tokens": max_tokens,
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    let trace = RouteTrace {
        channel_id: response.headers().get("X-Channel-Id").and_then(|v| v.to_str().ok()).map(str::to_string),
        model_id: response.headers().get("X-Model-Id").and_then(|v| v.to_str().ok()).map(str::to_string),
    };
    let text = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() { return Err(format!("Chat request failed ({status}): {text}")); }
    let parsed: ChatResponse = serde_json::from_str(&text).map_err(|e| format!("Invalid chat response: {e}"))?;
    let content = parsed.choices.first().and_then(|c| c.message.as_ref()).and_then(|m| m.content.clone()).unwrap_or_default();
    Ok(ChatResult { content, usage: parsed.usage, trace })
}

pub async fn first_active_api_token() -> Result<String, String> {
    TokenService::list()
        .await?
        .into_iter()
        .find(|token| token.status == "active")
        .map(|token| token.token)
        .ok_or_else(|| "No active API key is available. Create one in API Keys first.".to_string())
}
