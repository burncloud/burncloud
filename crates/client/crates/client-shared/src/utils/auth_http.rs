use crate::utils::storage::ClientState;

/// Load the persisted JWT from client storage.
pub fn bearer_token() -> Option<String> {
    ClientState::load()
        .auth_token
        .filter(|token| !token.is_empty())
}

/// Attach `Authorization: Bearer …` when a token is available.
pub fn with_auth(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    match bearer_token() {
        Some(token) => request.header("Authorization", format!("Bearer {token}")),
        None => request,
    }
}
