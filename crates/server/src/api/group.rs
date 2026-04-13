use crate::AppState;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use burncloud_service_group::{DbGroup, DbGroupMember, GroupMemberService, GroupService};
use serde::{Deserialize, Serialize};

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

#[derive(Serialize)]
struct GroupOpResult {
    status: &'static str,
    id: String,
}

#[derive(Serialize)]
struct SetMembersResult {
    status: &'static str,
}

#[derive(Serialize)]
struct ApiError {
    error: String,
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

fn group_err(e: impl ToString) -> impl IntoResponse {
    Json(ApiError {
        error: e.to_string(),
    })
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/groups", post(create_group).get(list_groups))
        .route("/groups/{id}", get(get_group).delete(delete_group))
        .route("/groups/{id}/members", get(get_members).put(set_members))
}

async fn list_groups(State(state): State<AppState>) -> impl IntoResponse {
    match GroupService::get_all(&state.db).await {
        Ok(groups) => Json(groups).into_response(),
        Err(e) => group_err(e).into_response(),
    }
}

async fn create_group(
    State(state): State<AppState>,
    Json(payload): Json<GroupDto>,
) -> impl IntoResponse {
    let group: DbGroup = payload.into();
    match GroupService::create(&state.db, &group).await {
        Ok(_) => Json(GroupOpResult {
            status: "created",
            id: group.id,
        })
        .into_response(),
        Err(e) => group_err(e).into_response(),
    }
}

async fn get_group(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match GroupService::get(&state.db, &id).await {
        Ok(Some(g)) => Json(g).into_response(),
        Ok(None) => group_err("Not Found").into_response(),
        Err(e) => group_err(e).into_response(),
    }
}

async fn delete_group(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match GroupService::delete(&state.db, &id).await {
        Ok(_) => Json(GroupOpResult {
            status: "deleted",
            id,
        })
        .into_response(),
        Err(e) => group_err(e).into_response(),
    }
}

async fn get_members(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match GroupMemberService::get_by_group(&state.db, &id).await {
        Ok(members) => Json(members).into_response(),
        Err(e) => group_err(e).into_response(),
    }
}

async fn set_members(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Vec<GroupMemberDto>>,
) -> impl IntoResponse {
    let members: Vec<DbGroupMember> = payload
        .into_iter()
        .map(|m| DbGroupMember {
            group_id: id.clone(),
            upstream_id: m.upstream_id,
            weight: m.weight,
        })
        .collect();

    match GroupMemberService::set_for_group(&state.db, &id, members).await {
        Ok(_) => Json(SetMembersResult { status: "updated" }).into_response(),
        Err(e) => group_err(e).into_response(),
    }
}
