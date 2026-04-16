# Server 层规范（Handler / Router）

> 仅在编写 `crates/server/src/api/` 下的代码时需要详细阅读。

---

## Handler 职责边界

Handler 只做三件事：**解析请求 → 调用 Service → 构造响应**。

业务逻辑（条件判断、计算、规则）一律下沉到 Service。

---

## 标准 Handler 签名

```rust
use crate::api::response::{ok, err};
use crate::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use burncloud_service_xxx::XxxService;

async fn handle_xxx(
    State(state): State<AppState>,
    Json(payload): Json<XxxRequest>,
) -> impl IntoResponse {
    match XxxService::do_something(&state.db, &payload).await {
        Ok(result) => ok(result).into_response(),
        Err(e) => {
            log::error!("handle_xxx error: {}", e);
            err(e.to_string()).into_response()
        }
    }
}
```

**关键点：**

- 返回类型是 `impl IntoResponse`，不是 `Result<Json<T>, AppError>`
- State 是 `State<AppState>`，不是 `State<Arc<AppState>>`（AppState 已实现 Clone）
- `ok()` / `err()` 来自 `crate::api::response`，不是全局函数

---

## 响应 helper：`ok` 和 `err`

源码位置：`crates/server/src/api/response.rs`

```rust
// 成功响应：{ "success": true, "data": <T> }
ok(data).into_response()

// 失败响应：{ "success": false, "message": "<msg>" }
err("Username already exists").into_response()
```

`err()` 接受任何实现 `ToString` 的类型，包括 `XxxServiceError` 的 Display 输出。

**注意：** `token.rs` 目前使用自定义 `ApiError { error }` 结构，这是存量代码的不一致。
新代码统一使用 `ok()/err()`。

---

## AppState 结构

```rust
// crates/server/src/lib.rs
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub monitor: Arc<SystemMonitorService>,
    pub user_service: Arc<UserService>,
    pub force_sync_tx: mpsc::Sender<oneshot::Sender<SyncResult>>,
}
```

- `db` 是 `Arc<Database>`，Service 方法接收 `&state.db` 作为 `&Database`
- 有状态的 Service（如 UserService）挂在 AppState 上作为 `Arc<XxxService>`
- 无状态的 Service（如 GroupService、TokenService）不需要放进 AppState

---

## 路由注册

每个模块暴露 `pub fn routes() -> Router<AppState>`，在 `api/mod.rs` 的 `routes()` 中 merge：

```rust
// api/mod.rs
pub fn routes(state: AppState) -> Router {
    Router::new()
        .merge(auth::routes())
        .merge(xxx::routes())
        .with_state(state)
}
```

路由函数内不加 `.with_state()`，让顶层统一注入。

---

## DTO（请求/响应结构）

DTO 定义在各自的 handler 文件内，不需要单独的 dto 模块：

```rust
#[derive(Deserialize)]
pub struct CreateXxxRequest {
    pub field_a: String,
    pub field_b: Option<i64>,
}

#[derive(Serialize)]
struct XxxResponse {
    pub id: String,
    pub field_a: String,
}
```

- 请求 DTO 用 `Deserialize`，响应 DTO 用 `Serialize`
- 敏感字段（`password_hash`）不出现在响应 DTO 中
- 可用 `#[serde(skip_serializing_if = "Option::is_none")]` 过滤空字段

---

## 鉴权中间件

JWT 鉴权通过 `auth_middleware` 实现，在需要保护的路由上单独 layer，不要在每个 Handler 内重复解析 token。

```rust
use crate::api::auth::{auth_middleware, Claims};
use axum::middleware;
use axum::Extension;

Router::new()
    .route("/api/protected", get(handler))
    .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
```

---

## 反例

```rust
// ✗ 禁止：Handler 直接访问数据库
async fn handle_user(State(state): State<AppState>) -> impl IntoResponse {
    let user = sqlx::query!("SELECT ...").fetch_one(&state.db.pool).await?;
}

// ✗ 禁止：手写错误响应格式
return (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid"}))).into_response();

// ✗ 禁止：State<Arc<AppState>>（AppState 已 Clone，无需二次包装）
async fn handle(State(state): State<Arc<AppState>>) -> impl IntoResponse { ... }
```
