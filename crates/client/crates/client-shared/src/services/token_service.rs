use crate::api_client::{TokenDto, API_CLIENT};
use anyhow::Result;

pub struct TokenService;

impl TokenService {
    pub async fn list() -> Result<Vec<TokenDto>> {
        API_CLIENT.list_tokens().await
    }

    pub async fn create(user_id: &str, quota_limit: Option<i64>) -> Result<String> {
        let quota = if let Some(q) = quota_limit {
            // Ensure we handle -1 or other "unlimited" logic if standard calls for it
            Some(q)
        } else {
            None
        };
        API_CLIENT.create_token(user_id, quota).await
    }

    pub async fn delete(token: &str) -> Result<()> {
        API_CLIENT.delete_token(token).await
    }
}
