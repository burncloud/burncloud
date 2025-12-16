//! # BurnCloud Service User
//!
//! User service layer providing register, login, and token management functionality.

use bcrypt::{hash, verify, DEFAULT_COST};
use burncloud_database::Database;
use burncloud_database_user::{DbUser, UserDatabase};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Default user status when created
const ACTIVE_STATUS: i32 = 1;

/// Default signup bonus for new users
const SIGNUP_BONUS: f64 = 10.0;

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
                eprintln!("WARNING: Using default JWT secret. Set JWT_SECRET environment variable in production!");
                "default-secret-key-change-in-production".to_string()
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
        let user = DbUser {
            id: Uuid::new_v4().to_string(),
            username: username.to_string(),
            email,
            password_hash: Some(password_hash),
            github_id: None,
            status: ACTIVE_STATUS,
            balance: SIGNUP_BONUS,
        };

        UserDatabase::create_user(db, &user).await?;

        // Assign default role - log warning if it fails but don't fail registration
        if let Err(e) = UserDatabase::assign_role(db, &user.id, "user").await {
            eprintln!(
                "Warning: Failed to assign default role to user {}: {}",
                user.id, e
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
        assert_eq!(user.unwrap().username, username);

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

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UserServiceError::UserAlreadyExists
        ));

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

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UserServiceError::InvalidCredentials
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_login_user_not_found() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();

        // Login non-existent user should fail
        let result = service.login_user(&db, "nonexistent", "password").await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UserServiceError::UserNotFound
        ));

        Ok(())
    }

    #[test]
    fn test_generate_token() {
        let service = UserService::with_secret("test-secret".to_string());

        let token = service.generate_token("user123", "testuser").unwrap();

        assert!(!token.token.is_empty());
        assert_eq!(token.user_id, "user123");
        assert_eq!(token.username, "testuser");
        assert!(token.expires_at > Utc::now().timestamp());
    }

    #[test]
    fn test_validate_token_success() {
        let service = UserService::with_secret("test-secret".to_string());

        let token = service.generate_token("user123", "testuser").unwrap();

        let (user_id, username) = service.validate_token(&token.token).unwrap();

        assert_eq!(user_id, "user123");
        assert_eq!(username, "testuser");
    }

    #[test]
    fn test_validate_token_invalid() {
        let service = UserService::with_secret("test-secret".to_string());

        let result = service.validate_token("invalid.token.here");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UserServiceError::TokenValidationError(_)
        ));
    }

    #[test]
    fn test_validate_token_wrong_secret() {
        let service1 = UserService::with_secret("secret1".to_string());
        let service2 = UserService::with_secret("secret2".to_string());

        let token = service1.generate_token("user123", "testuser").unwrap();

        // Validating with a different secret should fail
        let result = service2.validate_token(&token.token);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UserServiceError::TokenValidationError(_)
        ));
    }
}
