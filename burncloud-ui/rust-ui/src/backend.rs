use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Clone)]
pub struct BackendClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    pub status: Option<u16>,
    pub message: String,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ApiError {}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthData {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CurrentAccount {
    pub username: String,
    pub email: Option<String>,
    pub status: i32,
    pub balance_usd: i64,
    pub balance_cny: i64,
    pub preferred_currency: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

impl CurrentAccount {
    pub fn is_admin(&self) -> bool {
        self.roles.iter().any(|role| role == "admin")
    }

    pub fn balance(&self) -> f64 {
        let amount = if self.preferred_currency.as_deref() == Some("CNY") {
            self.balance_cny
        } else {
            self.balance_usd
        };
        amount as f64 / 1_000_000_000.0
    }

    pub fn currency_symbol(&self) -> &'static str {
        if self.preferred_currency.as_deref() == Some("CNY") {
            "¥"
        } else {
            "$"
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BillingModelSummary {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub requests: i64,
    #[serde(default)]
    pub prompt_tokens: i64,
    #[serde(default)]
    pub cache_read_tokens: i64,
    #[serde(default)]
    pub completion_tokens: i64,
    #[serde(default)]
    pub reasoning_tokens: i64,
    #[serde(default)]
    pub cost_usd: f64,
}

impl BillingModelSummary {
    pub fn total_tokens(&self) -> i64 {
        self.prompt_tokens + self.cache_read_tokens + self.completion_tokens + self.reasoning_tokens
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BillingSummary {
    #[serde(default)]
    pub pre_migration_requests: i64,
    #[serde(default)]
    pub models: Vec<BillingModelSummary>,
    #[serde(default)]
    pub total_cost_usd: f64,
}

impl BillingSummary {
    pub fn total_tokens(&self) -> i64 {
        self.models
            .iter()
            .map(BillingModelSummary::total_tokens)
            .sum()
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CatalogModel {
    pub id: String,
    #[serde(default)]
    pub providers: Vec<String>,
    #[serde(default)]
    pub available_channels: usize,
    pub p95_latency_ms: Option<i32>,
    pub input_price_per_million: Option<f64>,
    pub output_price_per_million: Option<f64>,
    pub context_window: Option<i64>,
    pub max_output_tokens: Option<i64>,
    #[serde(default)]
    pub supports_vision: bool,
    #[serde(default)]
    pub supports_function_calling: bool,
    pub model_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TokenSummary {
    pub token: String,
    #[serde(default)]
    pub token_hint: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Recharge {
    #[serde(default)]
    pub amount: i64,
    pub currency: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaygroundRequest {
    pub token_ref: String,
    pub model: String,
    pub messages: Vec<PlaygroundMessage>,
    pub temperature: f64,
    pub max_tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaygroundMessage {
    pub role: String,
    pub content: String,
}

pub struct ProxyResponse {
    pub status: StatusCode,
    pub content_type: String,
    pub channel_id: Option<String>,
    pub model_id: Option<String>,
    pub body: String,
}

impl BackendClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self, ApiError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|error| ApiError {
                status: None,
                message: format!("Failed to create backend client: {error}"),
            })?;
        Ok(Self { client, base_url })
    }

    pub fn from_environment() -> Result<Self, ApiError> {
        let base_url = std::env::var("BURNCLOUD_API_BASE")
            .unwrap_or_else(|_| "http://127.0.0.1:3002".to_string())
            .to_string();
        Self::new(base_url)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn decode<T: DeserializeOwned>(response: reqwest::Response) -> Result<T, ApiError> {
        let status = response.status();
        let body = response.text().await.map_err(|error| ApiError {
            status: Some(status.as_u16()),
            message: error.to_string(),
        })?;
        let envelope = serde_json::from_str::<Envelope<T>>(&body).map_err(|error| ApiError {
            status: Some(status.as_u16()),
            message: format!("Invalid BurnCloud API response: {error}"),
        })?;
        if status.is_success() && envelope.success {
            envelope.data.ok_or_else(|| ApiError {
                status: Some(status.as_u16()),
                message: "BurnCloud API response did not contain data".to_string(),
            })
        } else {
            Err(ApiError {
                status: Some(status.as_u16()),
                message: envelope
                    .message
                    .unwrap_or_else(|| format!("BurnCloud API request failed ({status})")),
            })
        }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str, token: &str) -> Result<T, ApiError> {
        let response = self
            .client
            .get(self.url(path))
            .bearer_auth(token)
            .send()
            .await
            .map_err(transport_error)?;
        Self::decode(response).await
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<AuthData, ApiError> {
        let response = self
            .client
            .post(self.url("/api/auth/login"))
            .json(&serde_json::json!({ "username": username, "password": password }))
            .send()
            .await
            .map_err(transport_error)?;
        Self::decode(response).await
    }

    pub async fn current_account(&self, token: &str) -> Result<CurrentAccount, ApiError> {
        self.get("/console/api/user/me", token).await
    }

    pub async fn billing_today(&self, token: &str) -> Result<BillingSummary, ApiError> {
        let today = chrono::Utc::now().date_naive();
        let response = self
            .client
            .get(self.url("/api/billing/summary"))
            .bearer_auth(token)
            .query(&[
                ("start", format!("{today}T00:00:00")),
                ("end", format!("{today}T23:59:59")),
            ])
            .send()
            .await
            .map_err(transport_error)?;
        Self::decode(response).await
    }

    pub async fn catalog(&self, token: &str) -> Result<Vec<CatalogModel>, ApiError> {
        self.get("/api/models/catalog", token).await
    }

    pub async fn tokens(&self, token: &str) -> Result<Vec<TokenSummary>, ApiError> {
        self.get("/console/api/tokens", token).await
    }

    pub async fn recharges(&self, token: &str) -> Result<Vec<Recharge>, ApiError> {
        self.get("/console/api/user/recharges", token).await
    }

    pub async fn playground(
        &self,
        token: &str,
        payload: &PlaygroundRequest,
    ) -> Result<ProxyResponse, ApiError> {
        let response = self
            .client
            .post(self.url("/console/api/playground/chat"))
            .bearer_auth(token)
            .json(payload)
            .send()
            .await
            .map_err(transport_error)?;
        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("application/json; charset=utf-8")
            .to_string();
        let channel_id = response
            .headers()
            .get("x-channel-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let model_id = response
            .headers()
            .get("x-model-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let body = response.text().await.map_err(|error| ApiError {
            status: Some(status.as_u16()),
            message: error.to_string(),
        })?;
        Ok(ProxyResponse {
            status,
            content_type,
            channel_id,
            model_id,
            body,
        })
    }
}

fn transport_error(error: reqwest::Error) -> ApiError {
    ApiError {
        status: error.status().map(|status| status.as_u16()),
        message: format!("BurnCloud backend is unavailable: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{BillingModelSummary, BillingSummary, CurrentAccount};

    #[test]
    fn converts_database_nanodollars_for_display() {
        let account = CurrentAccount {
            balance_usd: 128_500_000_000,
            ..CurrentAccount::default()
        };
        assert_eq!(account.balance(), 128.5);
        assert_eq!(account.currency_symbol(), "$".to_string());
    }

    #[test]
    fn totals_all_billable_token_classes() {
        let summary = BillingSummary {
            models: vec![BillingModelSummary {
                prompt_tokens: 10,
                completion_tokens: 20,
                cache_read_tokens: 3,
                reasoning_tokens: 7,
                ..BillingModelSummary::default()
            }],
            ..BillingSummary::default()
        };
        assert_eq!(summary.total_tokens(), 40);
    }
}
