pub const DEFAULT_PORT: u16 = 3000;
pub const API_PREFIX: &str = "/console/api";
pub const INTERNAL_PREFIX: &str = "/console/internal";
pub const WS_PATH: &str = "/ws";

/// Dev-only fallback when `JWT_SECRET` is unset. Must match token signing (`UserService`).
pub const DEFAULT_JWT_SECRET: &str = "burncloud-default-secret-change-in-production";

/// Resolve JWT signing/verification secret from the environment.
pub fn jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| DEFAULT_JWT_SECRET.to_string())
}

// Helper to get base URL
pub fn get_base_url(port: u16) -> String {
    format!("http://127.0.0.1:{}", port)
}

// Helper to get API URL
pub fn get_api_url(port: u16, path: &str) -> String {
    let path = path.trim_start_matches('/');
    format!("{}{}/{}", get_base_url(port), API_PREFIX, path)
}
