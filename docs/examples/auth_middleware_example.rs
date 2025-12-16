// Example: Using the auth middleware to protect routes
// This file demonstrates how to use the JWT authentication middleware

use axum::{
    extract::Extension,
    middleware,
    response::Json,
    routing::{get, post},
    Router,
};
use burncloud_server::{auth_middleware, AppState, Claims};
use serde_json::{json, Value};

// Example protected handler that requires authentication
async fn get_profile(Extension(claims): Extension<Claims>) -> Json<Value> {
    Json(json!({
        "success": true,
        "data": {
            "user_id": claims.sub,
            "username": claims.username,
            "message": "This is a protected endpoint"
        }
    }))
}

// Example admin-only handler
async fn admin_action(Extension(claims): Extension<Claims>) -> Json<Value> {
    // Note: You would typically check if the user has admin role here
    // This is just a placeholder showing how to access claims
    Json(json!({
        "success": true,
        "data": {
            "performed_by": claims.username,
            "action": "admin_action_completed"
        }
    }))
}

// Example of how to create a router with protected routes
pub fn create_protected_routes() -> Router<AppState> {
    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(|| async { "OK" }));

    // Protected routes (require JWT auth)
    let protected_routes = Router::new()
        .route("/profile", get(get_profile))
        .route("/admin/action", post(admin_action))
        // Apply the auth middleware to all routes in this router
        .layer(middleware::from_fn(auth_middleware));

    // Combine routers
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
}

// Example usage in main application:
/*
use burncloud_server::create_app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create the app
    let db = Arc::new(create_default_database().await?);
    let mut app = create_app(db, true).await?;
    
    // Add protected routes
    app = app.merge(create_protected_routes());
    
    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
*/
