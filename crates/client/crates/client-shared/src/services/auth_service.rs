// HTTP service — API response parsing — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

use crate::api_client::{LoginResponse, API_CLIENT};
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
        API_CLIENT.register(username, password, email).await
    }

    pub async fn check_username_availability(username: &str) -> Result<bool, String> {
        API_CLIENT.check_username_availability(username).await
    }
}
