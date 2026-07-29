pub const DEFAULT_PORT: u16 = 3000;
pub const API_PREFIX: &str = "/console/api";
pub const INTERNAL_PREFIX: &str = "/console/internal";
pub const WS_PATH: &str = "/ws";

/// Dev-only fallback when `JWT_SECRET` is unset.
///
/// # ⚠️ SECURITY WARNING
/// This constant is intentionally a weak, predictable value.
/// It MUST NEVER be used in production environments.
/// Set the `JWT_SECRET` environment variable to a strong, random secret
/// (at least 32 bytes of entropy) before deploying to production.
pub const DEFAULT_JWT_SECRET: &str = "burncloud-default-secret-change-in-production";

/// Resolve JWT signing/verification secret from the environment.
///
/// # ⚠️ Security Warning
/// If `JWT_SECRET` is not set or is empty, this function falls back to
/// `DEFAULT_JWT_SECRET`, which is INSECURE for production use.
/// A warning will be logged when the fallback is used.
pub fn jwt_secret() -> String {
    match std::env::var("JWT_SECRET") {
        Ok(secret) if !secret.is_empty() => secret,
        _ => {
            tracing::warn!(
                "JWT_SECRET is not set or empty. Using insecure default secret. \
                 THIS IS INSECURE FOR PRODUCTION! Set JWT_SECRET to a strong random value."
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
