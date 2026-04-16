# BurnCloud Service User

User service layer providing registration, login, and JWT token management functionality for the BurnCloud platform.

## Features

- **User Registration**: Register new users with secure password hashing using bcrypt
- **User Login**: Authenticate users with password verification
- **JWT Token Management**: Generate and validate JWT tokens for authentication
- **Type-safe Error Handling**: Comprehensive error types for better error handling

## Usage

### Basic Example

```rust
use burncloud_database::create_default_database;
use burncloud_database_user::UserDatabase;
use burncloud_service_user::UserService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize database
    let db = create_default_database().await?;
    UserDatabase::init(&db).await?;

    // Create service
    let service = UserService::new();

    // Register a new user
    let user_id = service
        .register_user(&db, "username", "password", Some("email@example.com".to_string()))
        .await?;

    // Login
    let auth_token = service.login_user(&db, "username", "password").await?;

    // Validate token
    let (user_id, username) = service.validate_token(&auth_token.token)?;

    Ok(())
}
```

### Running Examples

```bash
cargo run -p burncloud-service-user --example usage
```

## API Reference

### UserService

#### `new() -> Self`
Creates a new UserService instance. The JWT secret is read from the `JWT_SECRET` environment variable, or uses a default value if not set.

#### `with_secret(jwt_secret: String) -> Self`
Creates a new UserService instance with a custom JWT secret.

#### `register_user(&self, db: &Database, username: &str, password: &str, email: Option<String>) -> Result<String>`
Registers a new user with the given credentials. Returns the user ID on success.

- Checks if username already exists
- Hashes the password using bcrypt
- Creates the user in the database
- Assigns default "user" role
- Returns `UserServiceError::UserAlreadyExists` if username is taken

#### `login_user(&self, db: &Database, username: &str, password: &str) -> Result<AuthToken>`
Authenticates a user and returns an authentication token.

- Fetches user from database
- Verifies password
- Generates JWT token valid for 24 hours
- Returns `UserServiceError::UserNotFound` if user doesn't exist
- Returns `UserServiceError::InvalidCredentials` if password is wrong

#### `generate_token(&self, user_id: &str, username: &str) -> Result<AuthToken>`
Generates a JWT token for the given user. Token is valid for 24 hours.

#### `validate_token(&self, token: &str) -> Result<(String, String)>`
Validates a JWT token and extracts user information. Returns a tuple of (user_id, username).

### AuthToken

```rust
pub struct AuthToken {
    pub token: String,
    pub user_id: String,
    pub username: String,
    pub expires_at: i64,  // Unix timestamp
}
```

### Error Types

```rust
pub enum UserServiceError {
    DatabaseError(DatabaseError),
    UserAlreadyExists,
    UserNotFound,
    InvalidCredentials,
    HashError(String),
    TokenError(String),
    TokenValidationError(String),
}
```

## Configuration

### Environment Variables

- `JWT_SECRET`: Secret key for JWT token signing (required in production)

## Dependencies

- `burncloud-database-user`: Database layer for user operations
- `burncloud-database`: Database connection and management
- `bcrypt`: Password hashing
- `jsonwebtoken`: JWT token generation and validation
- `uuid`: User ID generation
- `chrono`: Date/time handling for token expiration

## Security Considerations

1. **Password Hashing**: Uses bcrypt with default cost factor for secure password storage
2. **JWT Tokens**: Tokens expire after 24 hours
3. **Secret Management**: Always set `JWT_SECRET` environment variable in production
4. **Token Validation**: Validates token signature and expiration

## Testing

Run tests with:

```bash
cargo test -p burncloud-service-user
```

All tests use in-memory or temporary databases and don't affect production data.
