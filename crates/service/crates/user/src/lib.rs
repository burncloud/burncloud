//! # BurnCloud Service User
//!
//! User service layer providing register, login, and token management functionality.

use bcrypt::{hash, verify, DEFAULT_COST};
use burncloud_common::TrafficColor;
use burncloud_database::Database;
use burncloud_database_user::UserDatabase;
use dashmap::DashMap;

// Re-export domain types so server can depend on service-user instead of database-user
pub use burncloud_database_user::{UserAccount, UserRecharge};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
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
    sub: String,      // Subject (user ID)
    username: String, // Username
    exp: i64,         // Expiration time
    iat: i64,         // Issued at
}

/// User service providing business logic for user operations
pub struct UserService {
    jwt_secret: String,
    token_expiration_hours: i64,
}

impl UserService {
    /// Create a new UserService instance
    ///
    /// # Panics
    /// Panics if JWT_SECRET environment variable is not set in production builds
    pub fn new() -> Self {
        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            #[cfg(not(debug_assertions))]
            panic!("JWT_SECRET environment variable must be set in production");

            #[cfg(debug_assertions)]
            {
                tracing::warn!("WARNING: Using default JWT secret. Set JWT_SECRET environment variable in production!");
                "burncloud-default-secret-change-in-production".to_string()
            }
        });

        Self {
            jwt_secret,
            token_expiration_hours: DEFAULT_TOKEN_EXPIRATION_HOURS,
        }
    }

    /// Create a new UserService instance with a custom JWT secret
    pub fn with_secret(jwt_secret: String) -> Self {
        Self {
            jwt_secret,
            token_expiration_hours: DEFAULT_TOKEN_EXPIRATION_HOURS,
        }
    }

    /// Create a new UserService instance with custom JWT secret and token expiration
    pub fn with_config(jwt_secret: String, token_expiration_hours: i64) -> Self {
        Self {
            jwt_secret,
            token_expiration_hours,
        }
    }

    /// Resolve a user's DiffServ traffic color for the L1 Classifier in the
    /// router data plane.
    ///
    /// Router's `proxy_logic` hot path calls this and injects the result into
    /// `SchedulingRequest` — the router crate stays color-agnostic (audit
    /// decision E-D3).
    ///
    /// Uses a TTL cache (5 min, audit decision D13) to avoid per-request DB
    /// queries. Falls back to Yellow on cache miss + DB failure.
    pub async fn resolve_traffic_class(
        db: &Database,
        user_id: &str,
    ) -> Result<TrafficColor> {
        use std::sync::OnceLock;
        static CACHE: OnceLock<DashMap<String, (TrafficColor, std::time::Instant)>> = OnceLock::new();
        let cache = CACHE.get_or_init(DashMap::new);
        const TTL: std::time::Duration = std::time::Duration::from_secs(300);

        // Check cache
        if let Some(entry) = cache.get(user_id) {
            if entry.1.elapsed() < TTL {
                return Ok(entry.0);
            }
            drop(entry);
            cache.remove(user_id);
        }

        // Cache miss — query DB for user roles
        let color = match UserDatabase::get_user_roles(db, user_id).await {
            Ok(roles) => {
                if roles.iter().any(|r| r == "admin" || r == "enterprise") {
                    TrafficColor::Green
                } else {
                    TrafficColor::Yellow
                }
            }
            Err(e) => {
                tracing::warn!(
                    user_id, error = %e,
                    "DB error resolving traffic class, defaulting to Yellow"
                );
                TrafficColor::Yellow
            }
        };

        cache.insert(user_id.to_string(), (color, std::time::Instant::now()));
        Ok(color)
    }

    /// Register a new user
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `username` - Username for the new user
    /// * `password` - Plain text password (will be hashed)
    /// * `email` - Optional email address
    ///
    /// # Returns
    /// * `Ok(String)` - The ID of the newly created user
    /// * `Err(UserServiceError)` - If registration fails
    pub async fn register_user(
        &self,
        db: &Database,
        username: &str,
        password: &str,
        email: Option<String>,
    ) -> Result<String> {
        // Check if user already exists
        if let Ok(Some(_)) = UserDatabase::get_user_by_username(db, username).await {
            return Err(UserServiceError::UserAlreadyExists);
        }

        // Hash the password
        let password_hash =
            hash(password, DEFAULT_COST).map_err(|e| UserServiceError::HashError(e.to_string()))?;

        // Create user
        let user = UserAccount {
            id: Uuid::new_v4().to_string(),
            username: username.to_string(),
            email,
            password_hash: Some(password_hash),
            github_id: None,
            status: ACTIVE_STATUS,
            balance_usd: SIGNUP_BONUS_NANO,
            balance_cny: 0,
            preferred_currency: Some("USD".to_string()),
        };

        UserDatabase::create_user(db, &user).await?;

        // First-user-is-admin: if no users existed before this one, assign
        // "admin" instead of the default "user" role (audit decision D3).
        // Uses count_users (SELECT COUNT) instead of list_users to avoid
        // loading all user rows just for the count.
        let is_first_user = match UserDatabase::count_users(db).await {
            Ok(count) => {
                tracing::info!("First-user-is-admin check: {count} users in DB");
                count == 1
            }
            Err(e) => {
                tracing::warn!("First-user-is-admin check failed: {}", e);
                false
            }
        };
        let default_role = if is_first_user { "admin" } else { "user" };

        if let Err(e) = UserDatabase::assign_role(db, &user.id, default_role).await {
            tracing::warn!(
                "Warning: Failed to assign {} role to user {}: {}",
                default_role,
                user.id,
                e
            );
        }

        Ok(user.id)
    }

    /// Login user and return authentication token
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `username` - Username
    /// * `password` - Plain text password
    ///
    /// # Returns
    /// * `Ok(AuthToken)` - Authentication token with user info
    /// * `Err(UserServiceError)` - If login fails
    pub async fn login_user(
        &self,
        db: &Database,
        username: &str,
        password: &str,
    ) -> Result<AuthToken> {
        // Fetch user
        let user = UserDatabase::get_user_by_username(db, username)
            .await?
            .ok_or(UserServiceError::UserNotFound)?;

        // Verify password
        let password_hash = user
            .password_hash
            .ok_or(UserServiceError::InvalidCredentials)?;

        let valid = verify(password, &password_hash)
            .map_err(|e| UserServiceError::HashError(e.to_string()))?;

        if !valid {
            return Err(UserServiceError::InvalidCredentials);
        }

        // Generate token
        self.generate_token(&user.id, &user.username)
    }

    /// Generate JWT token for a user
    ///
    /// # Arguments
    /// * `user_id` - User ID
    /// * `username` - Username
    ///
    /// # Returns
    /// * `Ok(AuthToken)` - Generated authentication token
    /// * `Err(UserServiceError)` - If token generation fails
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

        // create_recharge already updates the balance; read it back
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
    ///
    /// # Arguments
    /// * `token` - JWT token string
    ///
    /// # Returns
    /// * `Ok((user_id, username))` - Tuple of user ID and username
    /// * `Err(UserServiceError)` - If token is invalid or expired
    pub fn validate_token(&self, token: &str) -> Result<(String, String)> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| UserServiceError::TokenValidationError(e.to_string()))?;

        Ok((token_data.claims.sub, token_data.claims.username))
    }
}

impl Default for UserService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use burncloud_database::create_default_database;

    #[tokio::test]
    async fn test_register_user() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();
        let username = format!("testuser_{}", Uuid::new_v4());

        let user_id = service
            .register_user(
                &db,
                &username,
                "password123",
                Some("test@example.com".to_string()),
            )
            .await?;

        assert!(!user_id.is_empty());

        // Verify user exists
        let user = UserDatabase::get_user_by_username(&db, &username).await?;
        assert!(user.is_some());
        assert_eq!(
            user.unwrap_or_else(|| panic!("user should exist for {username}"))
                .username,
            username
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_register_duplicate_user() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();
        let username = format!("testuser_{}", Uuid::new_v4());

        // First registration should succeed
        service
            .register_user(&db, &username, "password123", None)
            .await?;

        // Second registration should fail
        let result = service
            .register_user(&db, &username, "password123", None)
            .await;

        let Err(e) = result else {
            panic!("duplicate registration should fail");
        };
        assert!(matches!(e, UserServiceError::UserAlreadyExists));

        Ok(())
    }

    #[tokio::test]
    async fn test_login_user_success() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();
        let username = format!("testuser_{}", Uuid::new_v4());
        let password = "password123";

        // Register user
        service
            .register_user(&db, &username, password, None)
            .await?;

        // Login should succeed
        let token = service.login_user(&db, &username, password).await?;

        assert!(!token.token.is_empty());
        assert_eq!(token.username, username);
        assert!(token.expires_at > Utc::now().timestamp());

        Ok(())
    }

    #[tokio::test]
    async fn test_login_user_wrong_password() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();
        let username = format!("testuser_{}", Uuid::new_v4());

        // Register user
        service
            .register_user(&db, &username, "password123", None)
            .await?;

        // Login with wrong password should fail
        let result = service.login_user(&db, &username, "wrongpassword").await;

        let Err(e) = result else {
            panic!("wrong password login should fail");
        };
        assert!(matches!(e, UserServiceError::InvalidCredentials));

        Ok(())
    }

    #[tokio::test]
    async fn test_login_user_not_found() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();

        // Login non-existent user should fail
        let result = service.login_user(&db, "nonexistent", "password").await;

        let Err(e) = result else {
            panic!("nonexistent user login should fail");
        };
        assert!(matches!(e, UserServiceError::UserNotFound));

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

        let Err(e) = result else {
            panic!("invalid token validation should fail");
        };
        assert!(matches!(e, UserServiceError::TokenValidationError(_)));
    }

    #[test]
    fn test_validate_token_wrong_secret() {
        let service1 = UserService::with_secret("secret1".to_string());
        let service2 = UserService::with_secret("secret2".to_string());

        let token = service1
            .generate_token("user123", "testuser")
            .unwrap_or_else(|e| panic!("token generation should succeed: {e}"));

        // Validating with a different secret should fail
        let result = service2.validate_token(&token.token);

        let Err(e) = result else {
            panic!("wrong-secret validation should fail");
        };
        assert!(matches!(e, UserServiceError::TokenValidationError(_)));
    }
}
