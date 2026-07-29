pub const DEFAULT_PORT: u16 = 3000;
pub const API_PREFIX: &str = "/console/api";
pub const INTERNAL_PREFIX: &str = "/console/internal";
pub const WS_PATH: &str = "/ws";

/// Dev-only fallback when `JWT_SECRET` is unset. Must match token signing (`UserService`).
pub const DEFAULT_JWT_SECRET: &str = "burncloud-default-secret-change-in-production";

/// Resolve JWT signing/verification secret from the environment.
///
/// # Warning
/// If `JWT_SECRET` environment variable is not set, the default dev-only secret will be used.
/// This is insecure for production environments. A warning log will be emitted.
pub fn jwt_secret() -> String {
    match std::env::var("JWT_SECRET") {
        Ok(secret) if !secret.is_empty() => secret,
        _ => {
            tracing::warn!(
                "JWT_SECRET environment variable is not set or empty. Using insecure default secret. \
                 THIS IS INSECURE FOR PRODUCTION! Please set a strong JWT_SECRET value."
            );
            DEFAULT_JWT_SECRET.to_string()
        }
    }
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
