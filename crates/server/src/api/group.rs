use axum::{
    extract::{Path, State},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::AppState;
use burncloud_database_router::{RouterDatabase, DbGroup};

#[derive(Deserialize, Serialize)]
pub struct GroupDto {
    pub id: String,
    pub name: String,
    pub strategy: String,
    pub match_path: String,
}

impl From<GroupDto> for DbGroup {
    fn from(dto: GroupDto) -> Self {
        DbGroup {
            id: dto.id,
            name: dto.name,
            strategy: dto.strategy,
            match_path: dto.match_path,
        }
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/groups", post(create_group).get(list_groups))
        .route("/groups/{id}", get(get_group).delete(delete_group))
}

async fn list_groups(State(state): State<AppState>) -> Json<Value> {
    match RouterDatabase::get_all_groups(&state.db).await {
        Ok(groups) => Json(json!(groups)),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn create_group(State(state): State<AppState>, Json(payload): Json<GroupDto>) -> Json<Value> {
    let group: DbGroup = payload.into();
    match RouterDatabase::create_group(&state.db, &group).await {
        Ok(_) => Json(json!({ "status": "created", "id": group.id })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn get_group(Path(id): Path<String>) -> Json<Value> {
    // TODO: Implement get_group in DB
    Json(json!({ "id": id, "name": "Placeholder Group" }))
}

async fn delete_group(State(state): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    match RouterDatabase::delete_group(&state.db, &id).await {
        Ok(_) => Json(json!({ "status": "deleted", "id": id })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}
