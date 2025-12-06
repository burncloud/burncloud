use axum::{
    extract::{Path, State},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::AppState;
use burncloud_database_router::{RouterDatabase, DbToken};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub struct CreateTokenRequest {
    pub user_id: String,
    // Optional alias or name could go here later
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/tokens", post(create_token).get(list_tokens))
        .route("/tokens/{token}", get(delete_token).delete(delete_token)) // Support GET for delete link? Standard is DELETE.
}

async fn list_tokens(State(state): State<AppState>) -> Json<Value> {
    match RouterDatabase::list_tokens(&state.db).await {
        Ok(tokens) => Json(json!(tokens)),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn create_token(State(state): State<AppState>, Json(payload): Json<CreateTokenRequest>) -> Json<Value> {
    // Generate a random sk- key
    let token = format!("sk-burncloud-{}", Uuid::new_v4());
    
    let db_token = DbToken {
        token: token.clone(),
        user_id: payload.user_id,
        status: "active".to_string(),
    };

    match RouterDatabase::create_token(&state.db, &db_token).await {
        Ok(_) => Json(json!({ "status": "created", "token": token })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn delete_token(State(state): State<AppState>, Path(token): Path<String>) -> Json<Value> {
    match RouterDatabase::delete_token(&state.db, &token).await {
        Ok(_) => Json(json!({ "status": "deleted", "token": token })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}
