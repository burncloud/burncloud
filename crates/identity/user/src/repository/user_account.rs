use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// User account row type (user_accounts table).
///
/// Balance fields use i64 nanodollars (9 decimal precision) for PostgreSQL BIGINT compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserAccount {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: Option<String>, // Nullable for OIDC users
    pub github_id: Option<String>,
    #[sqlx(default)]
    pub status: i32, // 1: Active, 0: Disabled
    /// USD balance in nanodollars (9 decimal precision)
    #[sqlx(default)]
    pub balance_usd: i64,
    /// CNY balance in nanodollars (9 decimal precision)
    #[sqlx(default)]
    pub balance_cny: i64,
    /// User's preferred currency for display
    #[sqlx(default)]
    pub preferred_currency: Option<String>,
    // created_at handled by DB
}

/// Input for creating a new user account.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserAccountInput {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub github_id: Option<String>,
    pub status: i32,
    pub balance_usd: i64,
    pub balance_cny: i64,
    pub preferred_currency: Option<String>,
}

impl From<UserAccount> for UserAccountInput {
    fn from(u: UserAccount) -> Self {
        Self {
            id: u.id,
            username: u.username,
            email: u.email,
            password_hash: u.password_hash,
            github_id: u.github_id,
            status: u.status,
            balance_usd: u.balance_usd,
            balance_cny: u.balance_cny,
            preferred_currency: u.preferred_currency,
        }
    }
}
