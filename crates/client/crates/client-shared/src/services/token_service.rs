use crate::api_client::{TokenDto, API_CLIENT};
use anyhow::Result;

pub struct TokenService;

impl TokenService {
    pub async fn list() -> Result<Vec<TokenDto>> {
        API_CLIENT.list_tokens().await
    }

    pub async fn create(user_id: &str, quota_limit: Option<i64>) -> Result<String> {
        let quota = quota_limit;
        API_CLIENT.create_token(user_id, quota).await
    }

    pub async fn delete(token: &str) -> Result<()> {
        API_CLIENT.delete_token(token).await
    }

    pub async fn update_status(token: &str, status: &str) -> Result<()> {
        API_CLIENT.update_token_status(token, status).await
    }
}
