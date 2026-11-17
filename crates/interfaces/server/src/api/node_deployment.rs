// 从 crate 根模块引入 AppState（应用全局共享状态，通常包含数据库连接、配置、服务实例等）
use crate::AppState;

// 引入 axum 框架的相关功能
use axum::{
    // extract 模块：用于从请求中提取数据（如路径参数、状态、JSON body）
    extract::{Path, State},
    // http::StatusCode：HTTP 状态码类型（如 200、404 等）
    http::StatusCode,
    // response::{IntoResponse, Response}：
    //   IntoResponse trait 允许类型转换为 HTTP 响应
    //   Response 是 axum 的响应类型别名
    response::{IntoResponse, Response},
    // routing::{get, post}：路由辅助函数，用于定义 GET/POST 路由
    routing::{get, post},
    // Json：用于 JSON 请求体提取和 JSON 响应序列化
    // Router：路由器类型，用于组合路由
    Json,
    Router,
};

// 引入 serde 的序列化/反序列化 trait
use serde::{Deserialize, Serialize};

// 引入 PathBuf，用于文件系统路径操作
use std::path::PathBuf;

// 定义"启动部署"请求体结构
// Debug：便于调试打印
// Deserialize：允许从 JSON 反序列化得到此结构
#[derive(Debug, Deserialize)]
struct StartDeploymentRequest {
    // 客户端传入的模型文件路径（字符串形式）
    model_path: String,
}

// 定义 API 错误响应结构
// Debug：便于调试
// Serialize：允许序列化为 JSON 返回给客户端
#[derive(Debug, Serialize)]
struct ApiError {
    // 错误信息文本
    error: String,
}

// 定义"停止部署"响应结构
// Debug：便于调试
// Serialize：可序列化为 JSON 响应
#[derive(Debug, Serialize)]
struct StopDeploymentResponse {
    // 被停止部署的模型 ID
    model_id: String,
    // 是否实际停止了部署（true 表示成功停止）
    stopped: bool,
}

// 定义路由构造函数，返回一个绑定到 AppState 的 Router
// pub(crate)：仅在当前 crate 内可见
pub(crate) fn routes() -> Router<AppState> {
    // 创建新路由器
    Router::new()
        // 注册 POST /console/api/node/deployments 路由，处理函数为 start_deployment
        .route("/console/api/node/deployments", post(start_deployment))
        // 注册 GET /console/api/node/deployments/{model_id} 路由
        // {model_id} 是路径参数占位符，会由 axum 提取为 Path<String>
        .route(
            "/console/api/node/deployments/{model_id}",
            get(get_deployment),
        )
        // 注册 POST /console/api/node/deployments/{model_id}/stop 路由
        .route(
            "/console/api/node/deployments/{model_id}/stop",
            post(stop_deployment),
        )
}

// 处理"启动部署"请求的异步函数
// State(state): 通过 axum 提取器注入应用状态
// Json(request): 自动解析请求体 JSON 为 StartDeploymentRequest
// 返回值：Response（axum 的通用响应类型）
async fn start_deployment(
    State(state): State<AppState>,
    Json(request): Json<StartDeploymentRequest>,
) -> Response {
    // 去除路径首尾空白字符，得到模型路径
    let model_path = request.model_path.trim();

    // 校验：路径为空 或 路径不指向一个已存在的文件 => 返回 400 错误
    if model_path.is_empty() || !PathBuf::from(model_path).is_file() {
        return api_error(
            StatusCode::BAD_REQUEST,                           // 400 Bad Request
            "model_path must point to an existing model file", // 提示信息
        );
    }

    // 调用 NodeDeployment 服务的 start_local 方法启动本地模型部署
    // 传入 PathBuf 形式的模型路径，await 异步等待结果
    match state
        .node_deployment
        .start_local(PathBuf::from(model_path))
        .await
    {
        // 成功：返回 200 和部署快照 JSON
        Ok(snapshot) => (StatusCode::OK, Json(snapshot)).into_response(),
        // 失败：返回 503 Service Unavailable 和错误信息
        Err(error) => api_error(StatusCode::SERVICE_UNAVAILABLE, &error),
    }
}

// 处理"查询部署状态"请求
// Path(model_id): 从 URL 路径中提取 model_id 参数
async fn get_deployment(State(state): State<AppState>, Path(model_id): Path<String>) -> Response {
    // 查询指定 model_id 的部署状态
    match state.node_deployment.status(&model_id).await {
        // 找到：返回该部署的快照 JSON
        Some(snapshot) => Json(snapshot).into_response(),
        // 未找到：返回 404 Not Found 和错误信息
        None => api_error(StatusCode::NOT_FOUND, "deployment was not found"),
    }
}

// 处理"停止部署"请求
async fn stop_deployment(State(state): State<AppState>, Path(model_id): Path<String>) -> Response {
    // 调用 stop 方法尝试停止指定部署
    match state.node_deployment.stop(&model_id).await {
        // 成功：返回 200，包含 model_id 和 stopped 状态
        Ok(stopped) => Json(StopDeploymentResponse { model_id, stopped }).into_response(),
        // 失败（例如部署正在使用、状态冲突等）：返回 409 Conflict 和错误字符串
        Err(error) => api_error(StatusCode::CONFLICT, &error.to_string()),
    }
}

// 辅助函数：构造统一格式的 API 错误响应
// 参数 status: HTTP 状态码
// 参数 message: 错误信息（会被复制为 String）
fn api_error(status: StatusCode, message: &str) -> Response {
    // 将状态码与 JSON 错误体组合
    (
        status, // HTTP 状态码
        Json(ApiError {
            error: message.to_string(), // 将 &str 转为 String 以序列化
        }),
    )
        // 通过 IntoResponse 将元组转换为 axum 的 Response
        // 元组 (StatusCode, Json<T>) 会自动序列化为对应状态码 + JSON body
        .into_response()
}
