use thiserror::Error;

/// Billing errors — price missing or DB failure
#[derive(Debug, Error)]
pub enum BillingError {
    #[error("Price not configured for model: {0}")]
    PriceNotFound(String),

    #[error("Database error: {0}")]
    Database(#[from] burncloud_database::DatabaseError),

    #[error("i64 overflow calculating cost (request_id={0}); truncated to i64::MAX")]
    Overflow(String),
}

/// Usage parsing errors — JSON decode failure or missing field
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Missing required field: {0}")]
    MissingField(String),
}
