/// Typed API response helpers — eliminates `serde_json::Value` from handler return types.
use axum::response::{IntoResponse, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct ApiSuccess<T: Serialize> {
    pub success: bool,
    pub data: T,
}

#[derive(Serialize)]
pub struct ApiFailure {
    pub success: bool,
    pub message: String,
}

pub fn ok<T: Serialize>(data: T) -> impl IntoResponse {
    Json(ApiSuccess {
        success: true,
        data,
    })
}

pub fn err(msg: impl ToString) -> impl IntoResponse {
    Json(ApiFailure {
        success: false,
        message: msg.to_string(),
    })
}
