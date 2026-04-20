use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// User-role binding row type (user_role_bindings table).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserRoleBinding {
    pub user_id: String,
    pub role_id: String,
}
