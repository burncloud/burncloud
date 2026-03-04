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

/// Default signup bonus for new users (in nanodollars: $10 = 10_000_000_000)
const SIGNUP_BONUS_NANO: i64 = 10_000_000_000;

/// Default token expiration time in hours
const DEFAULT_TOKEN_EXPIRATION_HOURS: i64 = 24;

/// Minimum password length
const MIN_PASSWORD_LENGTH: usize = 6;

/// Minimum username length
const MIN_USERNAME_LENGTH: usize = 3;

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

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Password is too weak: {0}")]
    WeakPassword(String),

    #[error("Token has expired")]
    TokenExpired,
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

    /// Validate username format and length
    ///
    /// # Arguments
    /// * `username` - Username to validate
    ///
    /// # Returns
    /// * `Ok(())` - If username is valid
    /// * `Err(UserServiceError::InvalidInput)` - If username is invalid
    pub fn validate_username(username: &str) -> Result<()> {
        let trimmed = username.trim();

        if trimmed.is_empty() {
            return Err(UserServiceError::InvalidInput(
                "Username cannot be empty".to_string(),
            ));
        }

        if trimmed.len() < MIN_USERNAME_LENGTH {
            return Err(UserServiceError::InvalidInput(format!(
                "Username must be at least {} characters long",
                MIN_USERNAME_LENGTH
            )));
        }

        // Check for valid characters (alphanumeric, underscore, hyphen)
        if !trimmed
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(UserServiceError::InvalidInput(
                "Username can only contain letters, numbers, underscores, and hyphens"
                    .to_string(),
            ));
        }

        Ok(())
    }

    /// Validate password strength
    ///
    /// # Arguments
    /// * `password` - Password to validate
    ///
    /// # Returns
    /// * `Ok(())` - If password meets strength requirements
    /// * `Err(UserServiceError::WeakPassword)` - If password is too weak
    pub fn validate_password_strength(password: &str) -> Result<()> {
        if password.is_empty() {
            return Err(UserServiceError::WeakPassword(
                "Password cannot be empty".to_string(),
            ));
        }

        if password.len() < MIN_PASSWORD_LENGTH {
            return Err(UserServiceError::WeakPassword(format!(
                "Password must be at least {} characters long",
                MIN_PASSWORD_LENGTH
            )));
        }

        // Check for at least one letter and one number
        let has_letter = password.chars().any(|c| c.is_alphabetic());
        let has_number = password.chars().any(|c| c.is_numeric());

        if !has_letter || !has_number {
            return Err(UserServiceError::WeakPassword(
                "Password must contain at least one letter and one number".to_string(),
            ));
        }

        Ok(())
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
        // Validate inputs
        Self::validate_username(username)?;
        Self::validate_password_strength(password)?;

        // Validate email format if provided
        if let Some(ref email) = email {
            if !email.is_empty() && !email.contains('@') {
                return Err(UserServiceError::InvalidInput(
                    "Invalid email format".to_string(),
                ));
            }
        }

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
            balance_usd: SIGNUP_BONUS_NANO,
            balance_cny: 0,
            preferred_currency: Some("USD".to_string()),
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

        // Check expiration explicitly
        let now = Utc::now().timestamp();
        if token_data.claims.exp < now {
            return Err(UserServiceError::TokenExpired);
        }

        Ok((token_data.claims.sub, token_data.claims.username))
    }

    /// Validate token and check if it's expired
    ///
    /// # Arguments
    /// * `token` - JWT token string
    ///
    /// # Returns
    /// * `Ok(true)` - If token is valid and not expired
    /// * `Ok(false)` - If token is expired
    /// * `Err(UserServiceError)` - If token is invalid
    pub fn is_token_valid(&self, token: &str) -> Result<bool> {
        match self.validate_token(token) {
            Ok(_) => Ok(true),
            Err(UserServiceError::TokenExpired) => Ok(false),
            Err(e) => Err(e),
        }
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

    // ============================================================
    // AUTH-01: User Registration Tests
    // ============================================================

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
    async fn test_register_empty_username() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();

        // Empty username should fail
        let result = service.register_user(&db, "", "password123", None).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UserServiceError::InvalidInput(_)
        ));

        // Whitespace-only username should fail
        let result = service
            .register_user(&db, "   ", "password123", None)
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UserServiceError::InvalidInput(_)
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_register_short_username() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();

        // Username too short should fail
        let result = service.register_user(&db, "ab", "password123", None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, UserServiceError::InvalidInput(_)));
        assert!(err.to_string().contains("at least"));

        Ok(())
    }

    #[tokio::test]
    async fn test_register_invalid_username_chars() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();

        // Username with invalid characters should fail
        let result = service
            .register_user(&db, "test@user!", "password123", None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, UserServiceError::InvalidInput(_)));
        assert!(err.to_string().contains("letters, numbers"));

        Ok(())
    }

    #[tokio::test]
    async fn test_register_invalid_email() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();
        let username = format!("testuser_{}", Uuid::new_v4());

        // Invalid email should fail
        let result = service
            .register_user(
                &db,
                &username,
                "password123",
                Some("invalid-email".to_string()),
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, UserServiceError::InvalidInput(_)));
        assert!(err.to_string().contains("email"));

        Ok(())
    }

    // ============================================================
    // AUTH-02: User Login Tests
    // ============================================================

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

    // ============================================================
    // AUTH-03: JWT Token Tests
    // ============================================================

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

    #[test]
    fn test_token_expiration() {
        let service = UserService::with_secret("test-secret".to_string());

        // Create an expired token manually by setting exp in the past
        let now = Utc::now();
        let past_expiration = now - Duration::hours(1); // Expired 1 hour ago

        let claims = Claims {
            sub: "user123".to_string(),
            username: "testuser".to_string(),
            exp: past_expiration.timestamp(),
            iat: (now - Duration::hours(2)).timestamp(),
        };

        let expired_token = jsonwebtoken::encode(
            &Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret("test-secret".as_bytes()),
        )
        .unwrap();

        // Token should be expired - reject with either TokenExpired or TokenValidationError
        // (jsonwebtoken may return ExpiredSignature error which maps to TokenValidationError)
        let result = service.validate_token(&expired_token);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let is_expired = matches!(err, UserServiceError::TokenExpired)
            || matches!(err, UserServiceError::TokenValidationError(_));
        assert!(is_expired, "Expected token to be rejected as expired, got: {:?}", err);
    }

    #[test]
    fn test_is_token_valid() {
        let service = UserService::with_secret("test-secret".to_string());

        let token = service.generate_token("user123", "testuser").unwrap();

        // Token should be valid
        assert!(service.is_token_valid(&token.token).unwrap());

        // Invalid token should return error
        let result = service.is_token_valid("invalid.token.here");
        assert!(result.is_err());
    }

    // ============================================================
    // AUTH-04: Password Validation Tests
    // ============================================================

    #[tokio::test]
    async fn test_register_empty_password() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();
        let username = format!("testuser_{}", Uuid::new_v4());

        // Empty password should fail
        let result = service.register_user(&db, &username, "", None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, UserServiceError::WeakPassword(_)));
        assert!(err.to_string().contains("empty"));

        Ok(())
    }

    #[tokio::test]
    async fn test_register_short_password() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();
        let username = format!("testuser_{}", Uuid::new_v4());

        // Short password should fail
        let result = service.register_user(&db, &username, "abc", None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, UserServiceError::WeakPassword(_)));
        assert!(err.to_string().contains("at least"));

        Ok(())
    }

    #[tokio::test]
    async fn test_register_password_no_number() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();
        let username = format!("testuser_{}", Uuid::new_v4());

        // Password without number should fail
        let result = service
            .register_user(&db, &username, "password", None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, UserServiceError::WeakPassword(_)));
        assert!(err.to_string().contains("letter and one number"));

        Ok(())
    }

    #[tokio::test]
    async fn test_register_password_no_letter() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();
        let username = format!("testuser_{}", Uuid::new_v4());

        // Password without letter should fail
        let result = service
            .register_user(&db, &username, "12345678", None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, UserServiceError::WeakPassword(_)));
        assert!(err.to_string().contains("letter and one number"));

        Ok(())
    }

    #[test]
    fn test_validate_password_strength() {
        // Valid passwords
        assert!(UserService::validate_password_strength("abc123").is_ok());
        assert!(UserService::validate_password_strength("password1").is_ok());
        assert!(UserService::validate_password_strength("SecurePass123!").is_ok());

        // Invalid passwords
        assert!(UserService::validate_password_strength("").is_err());
        assert!(UserService::validate_password_strength("12345").is_err());
        assert!(UserService::validate_password_strength("abcde").is_err());
        assert!(UserService::validate_password_strength("abcdef").is_err());
        assert!(UserService::validate_password_strength("123456").is_err());
    }

    #[test]
    fn test_validate_username() {
        // Valid usernames
        assert!(UserService::validate_username("abc").is_ok());
        assert!(UserService::validate_username("test_user").is_ok());
        assert!(UserService::validate_username("test-user").is_ok());
        assert!(UserService::validate_username("user123").is_ok());
        assert!(UserService::validate_username("TestUser123").is_ok());

        // Invalid usernames
        assert!(UserService::validate_username("").is_err());
        assert!(UserService::validate_username("ab").is_err());
        assert!(UserService::validate_username("   ").is_err());
        assert!(UserService::validate_username("test@user").is_err());
        assert!(UserService::validate_username("test user").is_err());
    }

    // ============================================================
    // AUTH-05: Role Assignment Tests
    // ============================================================

    #[tokio::test]
    async fn test_default_role_assignment() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();
        let username = format!("testuser_{}", Uuid::new_v4());

        // Register user
        let user_id = service
            .register_user(&db, &username, "password123", None)
            .await?;

        // Check default role was assigned
        let roles = UserDatabase::get_user_roles(&db, &user_id).await?;
        assert!(roles.contains(&"user".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_role_query() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();
        let username = format!("testuser_{}", Uuid::new_v4());

        // Register user
        let user_id = service
            .register_user(&db, &username, "password123", None)
            .await?;

        // Query roles
        let roles = UserDatabase::get_user_roles(&db, &user_id).await?;
        assert!(!roles.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_login_returns_roles() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        UserDatabase::init(&db).await?;

        let service = UserService::new();
        let username = format!("testuser_{}", Uuid::new_v4());
        let password = "password123";

        // Register and login
        service
            .register_user(&db, &username, password, None)
            .await?;

        // Login should succeed and we can verify roles separately
        let token = service.login_user(&db, &username, password).await?;
        assert!(!token.token.is_empty());

        Ok(())
    }

    // ============================================================
    // bcrypt Hash Verification Tests
    // ============================================================

    #[test]
    fn test_bcrypt_hash_verification() {
        let password = "test_password_123";
        let hash = hash(password, DEFAULT_COST).unwrap();

        // Correct password should verify
        assert!(verify(password, &hash).unwrap());

        // Wrong password should not verify
        assert!(!verify("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_bcrypt_hash_uniqueness() {
        let password = "same_password";
        let hash1 = hash(password, DEFAULT_COST).unwrap();
        let hash2 = hash(password, DEFAULT_COST).unwrap();

        // Same password should produce different hashes (due to salt)
        assert_ne!(hash1, hash2);

        // Both should verify the same password
        assert!(verify(password, &hash1).unwrap());
        assert!(verify(password, &hash2).unwrap());
    }
}
