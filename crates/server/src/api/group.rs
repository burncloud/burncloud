//! Group API routes for managing channel groups

use crate::api::auth::Claims;
use crate::api::response::{err, ok};
use crate::AppState;
use axum::{
    extract::{Path, State},
    middleware,
    response::IntoResponse,
    routing::{delete, get},
    Router,
};
use burncloud_database_group::{GroupMemberInput, RouterGroup};
use burncloud_service_group::GroupService;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupDto {
    pub id: String,
    pub name: String,
    pub strategy: String,
    pub match_path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupMemberDto {
    pub upstream_id: i32,
    pub weight: i32,
}

impl GroupDto {
    fn from_router_group(group: RouterGroup) -> Self {
        Self {
            id: group.id,
            name: group.name,
            strategy: group.strategy,
            match_path: group.match_path,
        }
    }

    fn into_router_group(self) -> RouterGroup {
        RouterGroup {
            id: self.id,
            name: self.name,
            strategy: self.strategy,
            match_path: self.match_path,
            created_at: None,
            updated_at: None,
        }
    }
}

impl GroupMemberDto {
    fn from_group_member(member: burncloud_database_group::GroupMember) -> Self {
        Self {
            upstream_id: member.upstream_id,
            weight: member.weight,
        }
    }

    fn into_group_member_input(self) -> GroupMemberInput {
        GroupMemberInput {
            upstream_id: self.upstream_id,
            weight: self.weight,
        }
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/groups", get(list_groups).post(create_group))
        .route("/groups/{id}", delete(delete_group))
        .route("/groups/{id}/members", get(get_group_members).put(update_group_members))
        .layer(middleware::from_fn(crate::auth_middleware))
}

async fn list_groups(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
) -> impl IntoResponse {
    // Check admin role
    let roles = burncloud_database_user::UserDatabase::get_user_roles(&state.db, &claims.sub).await;
    match roles {
        Ok(roles) if roles.iter().any(|r| r == "admin") => {}
        Ok(_) => return err("Admin access required").into_response(),
        Err(e) => {
            tracing::error!("Failed to get user roles: {}", e);
            return err("Database error").into_response();
        }
    }

    match GroupService::list(&state.db).await {
        Ok(groups) => {
            let dtos: Vec<GroupDto> = groups.into_iter().map(GroupDto::from_router_group).collect();
            ok(dtos).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list groups: {}", e);
            err("Failed to list groups").into_response()
        }
    }
}

async fn create_group(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
    axum::Json(dto): axum::Json<GroupDto>,
) -> impl IntoResponse {
    // Check admin role
    let roles = burncloud_database_user::UserDatabase::get_user_roles(&state.db, &claims.sub).await;
    match roles {
        Ok(roles) if roles.iter().any(|r| r == "admin") => {}
        Ok(_) => return err("Admin access required").into_response(),
        Err(e) => {
            tracing::error!("Failed to get user roles: {}", e);
            return err("Database error").into_response();
        }
    }

    let group = dto.into_router_group();
    match GroupService::create(&state.db, &group).await {
        Ok(_) => ok("Group created").into_response(),
        Err(e) => {
            tracing::error!("Failed to create group: {}", e);
            err("Failed to create group").into_response()
        }
    }
}

async fn delete_group(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Check admin role
    let roles = burncloud_database_user::UserDatabase::get_user_roles(&state.db, &claims.sub).await;
    match roles {
        Ok(roles) if roles.iter().any(|r| r == "admin") => {}
        Ok(_) => return err("Admin access required").into_response(),
        Err(e) => {
            tracing::error!("Failed to get user roles: {}", e);
            return err("Database error").into_response();
        }
    }

    match GroupService::delete(&state.db, &id).await {
        Ok(_) => ok("Group deleted").into_response(),
        Err(e) => {
            tracing::error!("Failed to delete group: {}", e);
            err("Failed to delete group").into_response()
        }
    }
}

async fn get_group_members(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Check admin role
    let roles = burncloud_database_user::UserDatabase::get_user_roles(&state.db, &claims.sub).await;
    match roles {
        Ok(roles) if roles.iter().any(|r| r == "admin") => {}
        Ok(_) => return err("Admin access required").into_response(),
        Err(e) => {
            tracing::error!("Failed to get user roles: {}", e);
            return err("Database error").into_response();
        }
    }

    match GroupService::get_members(&state.db, &id).await {
        Ok(members) => {
            let dtos: Vec<GroupMemberDto> = members.into_iter().map(GroupMemberDto::from_group_member).collect();
            ok(dtos).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get group members: {}", e);
            err("Failed to get group members").into_response()
        }
    }
}

async fn update_group_members(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
    Path(id): Path<String>,
    axum::Json(dtos): axum::Json<Vec<GroupMemberDto>>,
) -> impl IntoResponse {
    // Check admin role
    let roles = burncloud_database_user::UserDatabase::get_user_roles(&state.db, &claims.sub).await;
    match roles {
        Ok(roles) if roles.iter().any(|r| r == "admin") => {}
        Ok(_) => return err("Admin access required").into_response(),
        Err(e) => {
            tracing::error!("Failed to get user roles: {}", e);
            return err("Database error").into_response();
        }
    }

    let inputs: Vec<GroupMemberInput> = dtos.into_iter().map(GroupMemberDto::into_group_member_input).collect();
    match GroupService::update_members(&state.db, &id, &inputs).await {
        Ok(_) => ok("Group members updated").into_response(),
        Err(e) => {
            tracing::error!("Failed to update group members: {}", e);
            err("Failed to update group members").into_response()
        }
    }
}
