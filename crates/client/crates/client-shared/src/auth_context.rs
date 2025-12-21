use crate::utils::storage::ClientState;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: String,
    pub username: String,
    pub roles: Vec<String>,
}

#[derive(Clone, Copy)]
pub struct AuthContext {
    pub token: Signal<Option<String>>,
    pub current_user: Signal<Option<CurrentUser>>,
}

impl AuthContext {
    pub fn new() -> Self {
        // Try to load persisted state
        let state = ClientState::load();
        let (initial_token, initial_user) =
            if let (Some(token), Some(user_json)) = (state.auth_token, state.user_info) {
                match serde_json::from_str::<CurrentUser>(&user_json) {
                    Ok(user) => (Some(token), Some(user)),
                    Err(_) => (None, None),
                }
            } else {
                (None, None)
            };

        Self {
            token: Signal::new(initial_token),
            current_user: Signal::new(initial_user),
        }
    }

    pub fn set_auth(mut self, token: String, user: CurrentUser) {
        *self.token.write() = Some(token);
        *self.current_user.write() = Some(user);
    }

    pub fn clear_auth(mut self) {
        *self.token.write() = None;
        *self.current_user.write() = None;
    }

    pub fn is_authenticated(&self) -> bool {
        self.token.read().is_some()
    }

    pub fn get_token(&self) -> Option<String> {
        self.token.read().clone()
    }

    pub fn get_user(&self) -> Option<CurrentUser> {
        self.current_user.read().clone()
    }
}

impl Default for AuthContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn use_auth() -> AuthContext {
    use_context::<AuthContext>()
}

pub fn use_init_auth() -> AuthContext {
    use_context_provider(AuthContext::new)
}
