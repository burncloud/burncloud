# Authentication API Documentation

## Overview

The authentication system provides JWT-based authentication for the BurnCloud platform. It includes user registration, login, and middleware for protecting routes.

## Endpoints

### POST /api/auth/register

Register a new user and receive a JWT token.

**Request Body:**
```json
{
  "username": "string",
  "password": "string",
  "email": "string (optional)"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "user-uuid",
    "username": "string",
    "token": "jwt-token"
  }
}
```

### POST /api/auth/login

Login with existing credentials and receive a JWT token.

**Request Body:**
```json
{
  "username": "string",
  "password": "string"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "user-uuid",
    "username": "string",
    "roles": ["role1", "role2"],
    "token": "jwt-token"
  }
}
```

## Using the Auth Middleware

To protect routes with JWT authentication, use the `auth_middleware` layer:

```rust
use burncloud_server::auth_middleware;
use axum::{Router, routing::get, middleware};

let protected_routes = Router::new()
    .route("/protected/resource", get(protected_handler))
    .layer(middleware::from_fn(auth_middleware));
```

The middleware will:
1. Extract the JWT token from the `Authorization: Bearer <token>` header
2. Validate the token
3. Add the `Claims` to the request extensions for use in handlers
4. Return 401 Unauthorized if the token is invalid or missing

### Accessing Claims in Handlers

```rust
use axum::Extension;
use burncloud_server::api::auth::Claims;

async fn protected_handler(Extension(claims): Extension<Claims>) -> String {
    format!("Hello, {}! Your user ID is: {}", claims.username, claims.sub)
}
```

## Configuration

Set the JWT secret using the `JWT_SECRET` environment variable:

```bash
export JWT_SECRET="your-secret-key-here"
```

If not set, a default secret will be used (not recommended for production).

## JWT Token Structure

The JWT token contains the following claims:

- `sub`: User ID (UUID)
- `username`: Username
- `exp`: Expiration time (7 days from issue)
- `iat`: Issued at timestamp

## Security Notes

1. Always use HTTPS in production to protect tokens in transit
2. Set a strong `JWT_SECRET` in production
3. Tokens expire after 7 days - users will need to re-authenticate
4. Passwords are hashed using bcrypt with default cost factor
