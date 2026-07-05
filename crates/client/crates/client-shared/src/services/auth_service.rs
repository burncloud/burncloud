// HTTP service — API response parsing — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

use crate::api_client::{LoginResponse, API_CLIENT};
use crate::auth_context::CurrentUser;
use crate::utils::storage::ClientState;

pub struct AuthService;

impl AuthService {
    pub async fn login(username: &str, password: &str) -> Result<LoginResponse, String> {
        let result = API_CLIENT.login(username, password).await;

        if let Ok(ref response) = result {
            let mut state = ClientState::load();
            state.last_username = Some(username.to_string());
            state.auth_token = Some(response.token.clone());
            state.save();
        }

        result
    }

    pub async fn register(
        username: &str,
        password: &str,
        email: Option<&str>,
    ) -> Result<LoginResponse, String> {
        let result = API_CLIENT.register(username, password, email).await;

        if let Ok(ref response) = result {
            let mut state = ClientState::load();
            state.auth_token = Some(response.token.clone());
            state.user_info = Some(
                serde_json::to_string(&CurrentUser {
                    id: response.id.clone(),
                    username: response.username.clone(),
                    roles: response.roles.clone(),
                })
                .unwrap_or_default(),
            );
            state.save();
        }

        result
    }

    pub async fn check_username_availability(username: &str) -> Result<bool, String> {
        API_CLIENT.check_username_availability(username).await
    }

    pub async fn forgot_password(email: &str) -> Result<(), String> {
        API_CLIENT.forgot_password(email).await
    }

    pub async fn reset_password(token: &str, new_password: &str) -> Result<(), String> {
        API_CLIENT.reset_password(token, new_password).await
    }

    pub async fn get_oauth_url(provider: &str) -> Result<String, String> {
        API_CLIENT.oauth_url(provider).await
    }
}
