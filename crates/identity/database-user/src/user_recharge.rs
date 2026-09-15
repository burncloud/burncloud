use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Recharge record row type (user_recharges table).
/// Amount is stored as i64 nanodollars (9 decimal precision).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserRecharge {
    pub id: i32,
    pub user_id: String,
    /// Amount in nanodollars (9 decimal precision)
    pub amount: i64,
    /// Currency of the recharge (USD, CNY)
    #[sqlx(default)]
    pub currency: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<String>, // SQL datetime string
}
