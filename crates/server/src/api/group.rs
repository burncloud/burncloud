use crate::AppState;
use axum::{
    extract::{Path, State},
    response::Json,
    routing::{get, post},
    Router,
};
use burncloud_service_group::{DbGroup, DbGroupMember, GroupMemberService, GroupService};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize, Serialize)]
pub struct GroupDto {
    pub id: String,
    pub name: String,
    pub strategy: String,
    pub match_path: String,
}

#[derive(Deserialize, Serialize)]
pub struct GroupMemberDto {
    pub upstream_id: String,
    pub weight: i32,
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
        .route("/groups/{id}/members", get(get_members).put(set_members))
}

async fn list_groups(State(state): State<AppState>) -> Json<Value> {
    match GroupService::get_all(&state.db).await {
        Ok(groups) => Json(json!(groups)),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn create_group(State(state): State<AppState>, Json(payload): Json<GroupDto>) -> Json<Value> {
    let group: DbGroup = payload.into();
    match GroupService::create(&state.db, &group).await {
        Ok(_) => Json(json!({ "status": "created", "id": group.id })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn get_group(State(state): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    match GroupService::get(&state.db, &id).await {
        Ok(Some(g)) => Json(json!(g)),
        Ok(None) => Json(json!({ "error": "Not Found" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn delete_group(State(state): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    match GroupService::delete(&state.db, &id).await {
        Ok(_) => Json(json!({ "status": "deleted", "id": id })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn get_members(State(state): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    match GroupMemberService::get_by_group(&state.db, &id).await {
        Ok(members) => Json(json!(members)),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn set_members(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Vec<GroupMemberDto>>,
) -> Json<Value> {
    let members: Vec<DbGroupMember> = payload
        .into_iter()
        .map(|m| DbGroupMember {
            group_id: id.clone(),
            upstream_id: m.upstream_id,
            weight: m.weight,
        })
        .collect();

    match GroupMemberService::set_for_group(&state.db, &id, members).await {
        Ok(_) => Json(json!({ "status": "updated" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}
