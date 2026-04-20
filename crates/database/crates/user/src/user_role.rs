use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Role definition row type (user_roles table).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserRole {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}
