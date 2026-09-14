use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserFundingRequest {
    pub id: i32,
    pub user_id: String,
    pub amount: i64,
    pub currency: String,
    pub note: Option<String>,
    pub status: String,
    pub created_at: Option<String>,
}
