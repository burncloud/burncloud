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
        Self {
            token: Signal::new(None),
            current_user: Signal::new(None),
        }
    }

    pub fn set_auth(mut self, token: String, user: CurrentUser) {
        self.token.set(Some(token));
        self.current_user.set(Some(user));
    }

    pub fn clear_auth(mut self) {
        self.token.set(None);
        self.current_user.set(None);
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
