/// Typed API response helpers — eliminates `serde_json::Value` from handler return types.
use axum::http::StatusCode;
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

/// Return the standard failure envelope with an explicit HTTP status.
///
/// Prefer this helper for authorization, validation, and not-found branches so
/// callers, proxies, and SDKs can rely on the HTTP contract instead of parsing
/// `success: false` from a 200 response.
pub fn err_status(status: StatusCode, msg: impl ToString) -> impl IntoResponse {
    (
        status,
        Json(ApiFailure {
            success: false,
            message: msg.to_string(),
        }),
    )
}
