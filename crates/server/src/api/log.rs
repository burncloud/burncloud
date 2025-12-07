use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::get,
    Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use crate::AppState;
use burncloud_database_router::RouterDatabase;

#[derive(Deserialize)]
pub struct Pagination {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/console/api/logs", get(list_logs))
        .route("/console/api/usage/{user_id}", get(get_user_usage))
}

async fn list_logs(
    State(state): State<AppState>, 
    Query(params): Query<Pagination>
) -> Json<Value> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(50);
    let offset = (page - 1) * page_size;

    match RouterDatabase::get_logs(&state.db, page_size, offset).await {
        Ok(logs) => Json(json!({
            "data": logs,
            "page": page,
            "page_size": page_size
        })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn get_user_usage(
    State(state): State<AppState>, 
    Path(user_id): Path<String>
) -> Json<Value> {
    match RouterDatabase::get_usage_by_user(&state.db, &user_id).await {
        Ok((prompt, completion)) => Json(json!({
            "user_id": user_id,
            "prompt_tokens": prompt,
            "completion_tokens": completion,
            "total_tokens": prompt + completion
        })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}
