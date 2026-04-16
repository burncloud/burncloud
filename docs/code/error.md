# 错误处理规范

---

## 核心原则

1. **每层有自己的错误类型**，用 `thiserror` 定义
2. **错误转换用 `#[from]`**，不手写 `impl From` 样板
3. **`anyhow` 仅用于 `main.rs` / bin 入口**，lib 代码中禁用
4. **禁止 `.unwrap()` / `.expect()`**（测试代码和确定不会失败的场景除外）
5. **错误在最终处理点记录日志**，中间层只传播

---

## 三层错误链

```
DatabaseError (database crate)
      ↓  #[from]
XxxServiceError (service crate)
      ↓  显式 match
响应: err("User not found").into_response()
```

---

## Database 层错误

`burncloud_database::DatabaseError` 是所有 Database crate 的基础错误类型，不需要自定义。

Service 层通过 `#[from]` 自动转换：

```rust
#[derive(Debug, thiserror::Error)]
pub enum UserServiceError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] burncloud_database::DatabaseError),
    // ...
}
```

---

## Service 层错误（模式 A）

为有状态 Service 定义领域语义错误：

```rust
// crates/service/crates/service-user/src/lib.rs
#[derive(Debug, thiserror::Error)]
pub enum UserServiceError {
    // 来自 Database 的错误，自动转换
    #[error("Database error: {0}")]
    DatabaseError(#[from] burncloud_database::DatabaseError),

    // 业务语义错误
    #[error("User already exists")]
    UserAlreadyExists,

    #[error("User not found")]
    UserNotFound,

    #[error("Invalid credentials")]
    InvalidCredentials,

    // 外部库错误
    #[error("Password hashing error: {0}")]
    HashError(String),

    #[error("Token error: {0}")]
    TokenError(String),
}

// 方便调用方 ? 运算符
pub type Result<T> = std::result::Result<T, UserServiceError>;
```

---

## Service 层错误（模式 B）

无状态 Service 可直接复用 `DatabaseError`：

```rust
type Result<T> = std::result::Result<T, burncloud_database::DatabaseError>;
```

---

## Handler 层错误处理

Handler 通过 `match` 将 Service 错误转换为 HTTP 响应，HTTP 状态码映射集中在 Handler 中：

```rust
use crate::api::response::{ok, err};
use burncloud_service_user::UserServiceError;

async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDto>,
) -> impl IntoResponse {
    match state.user_service.register_user(&state.db, ...).await {
        Ok(user_id) => ok(user_id).into_response(),
        Err(UserServiceError::UserAlreadyExists) => {
            err("Username already exists").into_response()
        }
        Err(e) => {
            log::error!("create_user error: {}", e);
            err("Internal server error").into_response()
        }
    }
}
```

**规则：**
- 业务错误（NotFound、AlreadyExists 等）转换为有意义的错误消息
- 非预期错误（数据库故障等）用 `log::error!` 记录，返回通用消息（不暴露内部细节）
- 错误消息面向 API 消费者，不包含堆栈信息或内部路径

---

## 不要静默吞掉错误

```rust
// ✗ 完全静默
let _ = UserDatabase::assign_role(db, &user.id, "user").await;

// ✓ 降级处理但有日志
if let Err(e) = UserDatabase::assign_role(db, &user.id, "user").await {
    log::warn!("Failed to assign default role to user {}: {}", user.id, e);
    // 注意：此处选择继续而不是失败，因为注册本身已成功
}
```

---

## 反例

```rust
// ✗ 裸字符串错误
return Err("user not found".into());

// ✗ Box<dyn Error>（丢失类型信息）
fn do_thing() -> Result<(), Box<dyn std::error::Error>> { ... }

// ✗ anyhow 用于 lib 代码（anyhow 只用于 main/bin）
use anyhow::Result;
pub fn service_method() -> anyhow::Result<()> { ... }

// ✗ 不必要的 unwrap（请用 ? 或 match）
let user = get_user(db, id).await.unwrap();
```

---

## 测试中的 unwrap

测试代码中 `unwrap()` / `expect()` 是允许的，测试失败本来就应该 panic：

```rust
#[tokio::test]
async fn test_register() {
    let user_id = service.register_user(&db, "alice", "pass", None).await.unwrap();
    assert!(!user_id.is_empty());
}
```
