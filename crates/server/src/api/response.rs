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

/// 生产环境隐藏详细错误信息，开发环境显示详细信息
pub fn safe_err(e: impl std::fmt::Display) -> impl IntoResponse {
    let is_production = std::env::var("ENVIRONMENT").unwrap_or_default() == "production";

    if is_production {
        err("Internal server error".to_string()) // 统一为 String 类型
    } else {
        err(e.to_string())
    }
}

/// 数据库错误专用响应（始终隐藏详细信息）
pub fn db_err() -> impl IntoResponse {
    err("Database operation failed")
}
