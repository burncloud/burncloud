//! # BurnCloud Service User
//!
//! User service layer providing register, login, and token management functionality.

use bcrypt::{hash, verify, DEFAULT_COST};
use burncloud_database::Database;
use burncloud_database_user::PasswordResetDatabase;
use burncloud_database_user::UserDatabase;
use burncloud_traffic_contracts::TrafficColor;
use dashmap::DashMap;

// Re-export domain types so server can depend on service-user instead of database-user
pub use burncloud_database_user::{UserAccount, UserRecharge};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Default user status when created
const ACTIVE_STATUS: i32 = 1;

/// Default signup bonus for new users (in nanodollars: $10 = 10_000_000_000)
const SIGNUP_BONUS_NANO: i64 = 10_000_000_000;

/// Default token expiration time in hours
const DEFAULT_TOKEN_EXPIRATION_HOURS: i64 = 24;

/// Errors that can occur in the user service
#[derive(Debug, Error)]
pub enum UserServiceError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] burncloud_database::DatabaseError),

    #[error("User already exists")]
    UserAlreadyExists,

    #[error("User not found")]
    UserNotFound,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Password hashing error: {0}")]
    HashError(String),

    #[error("Token generation error: {0}")]
    TokenError(String),

    #[error("Token validation error: {0}")]
    TokenValidationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, UserServiceError>;

/// Validated JWT signing and verification secret owned by Identity.
#[derive(Clone)]
pub struct JwtSecret(Arc<str>);

impl JwtSecret {
    /// Validate an explicitly supplied JWT secret.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let value = value.trim();
        if value.is_empty() {
            return Err(UserServiceError::ConfigError(
                "JWT_SECRET is missing or empty".to_string(),
            ));
        }
        Ok(Self(Arc::from(value)))
    }

    /// Verify a JWT with this secret and return its claims.
    pub fn verify<T: DeserializeOwned>(&self, token: &str) -> jsonwebtoken::errors::Result<T> {
        decode::<T>(
            token,
            &DecodingKey::from_secret(self.0.as_bytes()),
            &Validation::default(),
        )
        .map(|token_data| token_data.claims)
    }

    fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Debug for JwtSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JwtSecret([REDACTED])")
    }
}

/// Authentication token structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub token: String,
    pub user_id: String,
    pub username: String,
    pub expires_at: i64,
}

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    username: String,
    exp: i64,
    iat: i64,
}

/// User service providing business logic for user operations
pub struct UserService {
    jwt_secret: JwtSecret,
    token_expiration_hours: i64,
}

impl UserService {
    /// Create a service with an explicitly validated JWT secret.
    pub fn new(jwt_secret: JwtSecret) -> Self {
        Self {
            jwt_secret,
            token_expiration_hours: DEFAULT_TOKEN_EXPIRATION_HOURS,
        }
    }

    /// Create a new UserService instance with a custom JWT secret
    pub fn with_secret(jwt_secret: String) -> Self {
        let jwt_secret = JwtSecret::new(jwt_secret)
            .unwrap_or_else(|_| panic!("JWT secret supplied to UserService must not be empty"));
        Self::new(jwt_secret)
    }

    /// Create a new UserService instance with custom JWT secret and token expiration
    pub fn with_config(jwt_secret: String, token_expiration_hours: i64) -> Self {
        let jwt_secret = JwtSecret::new(jwt_secret)
            .unwrap_or_else(|_| panic!("JWT secret supplied to UserService must not be empty"));
        Self {
            jwt_secret,
            token_expiration_hours,
        }
    }

    /// Resolve a user's DiffServ traffic color for the L1 Classifier in the router data plane.
    pub async fn resolve_traffic_class(db: &Database, user_id: &str) -> Result<TrafficColor> {
        use std::sync::OnceLock;
        static CACHE: OnceLock<DashMap<String, (TrafficColor, std::time::Instant)>> =
            OnceLock::new();
        let cache = CACHE.get_or_init(DashMap::new);
        const TTL: std::time::Duration = std::time::Duration::from_secs(300);

        if let Some(entry) = cache.get(user_id) {
            if entry.1.elapsed() < TTL {
                return Ok(entry.0);
            }
            drop(entry);
            cache.remove(user_id);
        }

        let color = match UserDatabase::get_user_roles(db, user_id).await {
            Ok(roles) => {
                if roles.iter().any(|r| r == "admin" || r == "enterprise") {
                    TrafficColor::Green
                } else {
                    TrafficColor::Yellow
                }
            }
            Err(error) => {
                tracing::warn!(
                    user_id,
                    error = %error,
                    "DB error resolving traffic class, defaulting to Yellow"
                );
                TrafficColor::Yellow
            }
        };

        cache.insert(user_id.to_string(), (color, std::time::Instant::now()));
        Ok(color)
    }

    async fn ensure_username_available(db: &Database, username: &str) -> Result<()> {
        if UserDatabase::get_user_by_username(db, username)
            .await?
            .is_some()
        {
            return Err(UserServiceError::UserAlreadyExists);
        }
        Ok(())
    }

    fn build_registered_user(
        username: &str,
        password: &str,
        email: Option<String>,
    ) -> Result<UserAccount> {
        let password_hash =
            hash(password, DEFAULT_COST).map_err(|e| UserServiceError::HashError(e.to_string()))?;
        Ok(UserAccount {
            id: Uuid::new_v4().to_string(),
            username: username.to_string(),
            email,
            password_hash: Some(password_hash),
            github_id: None,
            status: ACTIVE_STATUS,
            balance_usd: SIGNUP_BONUS_NANO,
            balance_cny: 0,
            preferred_currency: Some("USD".to_string()),
        })
    }

    async fn default_role_for_new_user(db: &Database) -> &'static str {
        match UserDatabase::count_users(db).await {
            Ok(count) => {
                tracing::info!("First-user-is-admin check: user count = {count}");
                if count == 0 {
                    "admin"
                } else {
                    "user"
                }
            }
            Err(error) => {
                tracing::warn!(%error, "First-user-is-admin check failed");
                "user"
            }
        }
    }

    /// Register a new user.
    pub async fn register_user(
        &self,
        db: &Database,
        username: &str,
        password: &str,
        email: Option<String>,
    ) -> Result<String> {
        Self::ensure_username_available(db, username).await?;
        let user = Self::build_registered_user(username, password, email)?;
        let default_role = Self::default_role_for_new_user(db).await;

        UserDatabase::create_user(db, &user).await?;
        if let Err(error) = UserDatabase::assign_role(db, &user.id, default_role).await {
            tracing::warn!(
                role = default_role,
                user_id = user.id,
                %error,
                "failed to assign default role to newly registered user"
            );
        }
        Ok(user.id)
    }

    /// Login user and return authentication token
    pub async fn login_user(
        &self,
        db: &Database,
        username: &str,
        password: &str,
    ) -> Result<AuthToken> {
        let user = UserDatabase::get_user_by_username(db, username)
            .await?
            .ok_or(UserServiceError::UserNotFound)?;
        let password_hash = user
            .password_hash
            .ok_or(UserServiceError::InvalidCredentials)?;
        let valid = verify(password, &password_hash)
            .map_err(|e| UserServiceError::HashError(e.to_string()))?;
        if !valid {
            return Err(UserServiceError::InvalidCredentials);
        }
        self.generate_token(&user.id, &user.username)
    }

    /// Generate JWT token for a user
    pub fn generate_token(&self, user_id: &str, username: &str) -> Result<AuthToken> {
        let now = Utc::now();
        let expiration = now + Duration::hours(self.token_expiration_hours);
        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            exp: expiration.timestamp(),
            iat: now.timestamp(),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| UserServiceError::TokenError(e.to_string()))?;
        Ok(AuthToken {
            token,
            user_id: user_id.to_string(),
            username: username.to_string(),
            expires_at: expiration.timestamp(),
        })
    }

    /// List all users (no pagination — use sparingly in production)
    pub async fn list_users(&self, db: &Database) -> Result<Vec<UserAccount>> {
        UserDatabase::list_users(db).await.map_err(Into::into)
    }

    /// Check whether a username is already taken. Returns `true` if available.
    pub async fn is_username_available(&self, db: &Database, username: &str) -> Result<bool> {
        let existing = UserDatabase::get_user_by_username(db, username).await?;
        Ok(existing.is_none())
    }

    /// Get roles for a user
    pub async fn get_user_roles(&self, db: &Database, user_id: &str) -> Result<Vec<String>> {
        UserDatabase::get_user_roles(db, user_id)
            .await
            .map_err(Into::into)
    }

    /// Topup a user's balance and return the new balance in nanodollars.
    pub async fn topup(
        &self,
        db: &Database,
        user_id: &str,
        amount: i64,
        currency: &str,
    ) -> Result<i64> {
        let recharge = UserRecharge {
            id: 0,
            user_id: user_id.to_string(),
            amount,
            currency: Some(currency.to_string()),
            description: Some("账户充值".to_string()),
            created_at: None,
        };
        UserDatabase::create_recharge(db, &recharge)
            .await
            .map_err(UserServiceError::DatabaseError)?;
        let balance = UserDatabase::update_balance(db, user_id, 0, Some(currency))
            .await
            .unwrap_or(0);
        Ok(balance)
    }

    /// List recharge history for a user
    pub async fn list_recharges(&self, db: &Database, user_id: &str) -> Result<Vec<UserRecharge>> {
        UserDatabase::list_recharges(db, user_id)
            .await
            .map_err(Into::into)
    }

    /// Validate JWT token and extract user information
    pub fn validate_token(&self, token: &str) -> Result<(String, String)> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| UserServiceError::TokenValidationError(e.to_string()))?;
        Ok((token_data.claims.sub, token_data.claims.username))
    }

    pub async fn request_password_reset(&self, db: &Database, email: &str) -> Result<String> {
        let user = UserDatabase::get_user_by_email(db, email)
            .await?
            .ok_or(UserServiceError::UserNotFound)?;
        let token = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + Duration::hours(1);
        PasswordResetDatabase::create_token(db, &token, &user.id, &expires_at.to_rfc3339()).await?;
        Ok(token)
    }

    pub async fn reset_password(
        &self,
        db: &Database,
        token: &str,
        new_password: &str,
    ) -> Result<()> {
        let reset_token = PasswordResetDatabase::get_token(db, token)
            .await?
            .ok_or(UserServiceError::InvalidCredentials)?;
        if reset_token.used_at.is_some() {
            return Err(UserServiceError::InvalidCredentials);
        }
        let expires_at = chrono::DateTime::parse_from_rfc3339(&reset_token.expires_at)
            .map_err(|e| UserServiceError::ConfigError(e.to_string()))?;
        if Utc::now() > expires_at {
            return Err(UserServiceError::InvalidCredentials);
        }
        let password_hash = hash(new_password, DEFAULT_COST)
            .map_err(|e| UserServiceError::HashError(e.to_string()))?;
        UserDatabase::update_password_hash(db, &reset_token.user_id, &password_hash).await?;
        PasswordResetDatabase::mark_used(db, token).await?;
        Ok(())
    }

    pub fn oauth_url(provider: &str) -> Result<String> {
        match provider {
            "google" => {
                let client_id = std::env::var("GOOGLE_CLIENT_ID")
                    .map_err(|e| UserServiceError::ConfigError(e.to_string()))?;
                let redirect_uri = std::env::var("GOOGLE_REDIRECT_URI").unwrap_or_else(|_| {
                    "http://localhost:8080/console/api/auth/google/callback".to_string()
                });
                Ok(format!(
                    "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile",
                    client_id, redirect_uri
                ))
            }
            "github" => {
                let client_id = std::env::var("GITHUB_CLIENT_ID")
                    .map_err(|e| UserServiceError::ConfigError(e.to_string()))?;
                let redirect_uri = std::env::var("GITHUB_REDIRECT_URI").unwrap_or_else(|_| {
                    "http://localhost:8080/console/api/auth/github/callback".to_string()
                });
                Ok(format!(
                    "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=user:email",
                    client_id, redirect_uri
                ))
            }
            _ => Err(UserServiceError::ConfigError(format!(
                "Unknown OAuth provider: {provider}"
            ))),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use burncloud_database::create_database_with_url;

    struct TempDb {
        db: Database,
        path: std::path::PathBuf,
    }

    impl TempDb {
        async fn new(tag: &str) -> anyhow::Result<Self> {
            let path = std::env::temp_dir().join(format!(
                "bc_service_user_{}_{}_{}.db",
                tag,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ));
            let _ = std::fs::remove_file(&path);
            let normalized = path.to_string_lossy().replace('\\', "/");
            let url = format!("sqlite:///{}?mode=rwc", normalized);
            let db = create_database_with_url(&url).await?;
            Ok(Self { db, path })
        }

        async fn cleanup(self) {
            let path = self.path.clone();
            self.db.close().await.ok();
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            std::fs::remove_file(&path).unwrap_or_else(|e| {
                panic!("test database {} was not removed ({e})", path.display())
            });
        }
    }

    async fn test_db(tag: &str) -> anyhow::Result<TempDb> {
        let temp = TempDb::new(tag).await?;
        UserDatabase::init(&temp.db).await?;
        Ok(temp)
    }

    fn test_service() -> UserService {
        UserService::new(
            JwtSecret::new("burncloud-service-user-test-secret")
                .unwrap_or_else(|e| panic!("test JWT secret must be valid: {e}")),
        )
    }

    #[test]
    fn jwt_secret_rejects_empty_values() {
        for value in ["", "   ", "\n\t"] {
            let result = JwtSecret::new(value);
            assert!(matches!(result, Err(UserServiceError::ConfigError(_))));
        }
    }

    #[test]
    fn jwt_secret_debug_is_redacted() {
        let plaintext = "must-never-appear";
        let secret = JwtSecret::new(plaintext)
            .unwrap_or_else(|e| panic!("test JWT secret must be valid: {e}"));
        let rendered = format!("{secret:?}");
        assert!(!rendered.contains(plaintext));
        assert_eq!(rendered, "JwtSecret([REDACTED])");
    }

    #[tokio::test]
    async fn test_register_user() -> anyhow::Result<()> {
        let temp = test_db("register_user").await?;
        let db = &temp.db;
        let service = test_service();
        let username = format!("testuser_{}", Uuid::new_v4());
        let user_id = service
            .register_user(
                db,
                &username,
                "password123",
                Some("test@example.com".to_string()),
            )
            .await?;
        assert!(!user_id.is_empty());
        let user = UserDatabase::get_user_by_username(db, &username).await?;
        assert!(user.is_some());
        assert_eq!(
            user.unwrap_or_else(|| panic!("user should exist for {username}"))
                .username,
            username
        );
        temp.cleanup().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_register_duplicate_user() -> anyhow::Result<()> {
        let temp = test_db("register_duplicate").await?;
        let db = &temp.db;
        let service = test_service();
        let username = format!("testuser_{}", Uuid::new_v4());
        service
            .register_user(db, &username, "password123", None)
            .await?;
        let result = service
            .register_user(db, &username, "password123", None)
            .await;
        let Err(error) = result else {
            panic!("duplicate registration should fail");
        };
        assert!(matches!(error, UserServiceError::UserAlreadyExists));
        temp.cleanup().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_login_user_success() -> anyhow::Result<()> {
        let temp = test_db("login_success").await?;
        let db = &temp.db;
        let service = test_service();
        let username = format!("testuser_{}", Uuid::new_v4());
        let password = "password123";
        service.register_user(db, &username, password, None).await?;
        let token = service.login_user(db, &username, password).await?;
        assert!(!token.token.is_empty());
        assert_eq!(token.username, username);
        assert!(token.expires_at > Utc::now().timestamp());
        temp.cleanup().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_login_user_wrong_password() -> anyhow::Result<()> {
        let temp = test_db("login_wrong_password").await?;
        let db = &temp.db;
        let service = test_service();
        let username = format!("testuser_{}", Uuid::new_v4());
        service
            .register_user(db, &username, "password123", None)
            .await?;
        let result = service.login_user(db, &username, "wrongpassword").await;
        let Err(error) = result else {
            panic!("wrong password login should fail");
        };
        assert!(matches!(error, UserServiceError::InvalidCredentials));
        temp.cleanup().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_login_user_not_found() -> anyhow::Result<()> {
        let temp = test_db("login_not_found").await?;
        let db = &temp.db;
        let service = test_service();
        let result = service.login_user(db, "nonexistent", "password").await;
        let Err(error) = result else {
            panic!("nonexistent user login should fail");
        };
        assert!(matches!(error, UserServiceError::UserNotFound));
        temp.cleanup().await;
        Ok(())
    }

    #[test]
    fn test_generate_token() {
        let service = UserService::with_secret("test-secret".to_string());
        let token = service
            .generate_token("user123", "testuser")
            .unwrap_or_else(|e| panic!("token generation should succeed: {e}"));
        assert!(!token.token.is_empty());
        assert_eq!(token.user_id, "user123");
        assert_eq!(token.username, "testuser");
        assert!(token.expires_at > Utc::now().timestamp());
    }

    #[test]
    fn test_validate_token_success() {
        let service = UserService::with_secret("test-secret".to_string());
        let token = service
            .generate_token("user123", "testuser")
            .unwrap_or_else(|e| panic!("token generation should succeed: {e}"));
        let (user_id, username) = service
            .validate_token(&token.token)
            .unwrap_or_else(|e| panic!("token validation should succeed: {e}"));
        assert_eq!(user_id, "user123");
        assert_eq!(username, "testuser");
    }

    #[test]
    fn test_validate_token_invalid() {
        let service = UserService::with_secret("test-secret".to_string());
        let result = service.validate_token("invalid.token.here");
        let Err(error) = result else {
            panic!("invalid token validation should fail");
        };
        assert!(matches!(error, UserServiceError::TokenValidationError(_)));
    }

    #[test]
    fn test_validate_token_wrong_secret() {
        let service1 = UserService::with_secret("secret1".to_string());
        let service2 = UserService::with_secret("secret2".to_string());
        let token = service1
            .generate_token("user123", "testuser")
            .unwrap_or_else(|e| panic!("token generation should succeed: {e}"));
        let result = service2.validate_token(&token.token);
        let Err(error) = result else {
            panic!("wrong-secret validation should fail");
        };
        assert!(matches!(error, UserServiceError::TokenValidationError(_)));
    }
}
