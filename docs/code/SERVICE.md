# Service 层规范

> 仅在编写 `crates/service/crates/service-*/src/` 下的代码时需要详细阅读。

---

## 核心约束

1. **不持有 Database 连接**。方法签名通过 `db: &Database` 传入，不存字段。
2. **不引入 HTTP 类型**。`axum`、`Json`、`StatusCode` 不出现在 Service crate。
3. **不直接操作 SQL**。调用 Database crate 的 Model/Repository 方法。
4. **re-export 业务类型**，让 Server 只依赖 Service 而不依赖 Database crate。

---

## 两种 Service 模式

### 模式 A：有状态 Service（有配置字段）

**适用场景：** Service 需要持有配置值（JWT secret、超时时长、外部 URL 等）。

实例挂在 `AppState` 上（`Arc<XxxService>`），在服务启动时创建一次。

```rust
// crates/service/crates/service-user/src/lib.rs
pub struct UserService {
    jwt_secret: String,
    token_expiration_hours: i64,
}

impl UserService {
    pub fn new() -> Self {
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "default-secret".to_string());
        Self { jwt_secret, token_expiration_hours: 24 }
    }

    pub async fn register_user(
        &self,
        db: &Database,
        username: &str,
        password: &str,
        email: Option<String>,
    ) -> Result<String, UserServiceError> {
        // 业务逻辑...
    }
}
```

**在 Handler 中调用：**
```rust
state.user_service.register_user(&state.db, ...).await
```

---

### 模式 B：无状态 Service（纯操作封装）

**适用场景：** Service 只是封装对 Database 的调用，没有需要持有的状态。

不需要放进 `AppState`，直接在 Handler 中调用静态方法。

```rust
// crates/service/crates/service-group/src/lib.rs
pub struct GroupService;

impl GroupService {
    pub async fn get_all(db: &Database) -> Result<Vec<RouterGroup>, DatabaseError> {
        RouterGroupModel::get_all(db).await
    }

    pub async fn create(db: &Database, group: &RouterGroup) -> Result<(), DatabaseError> {
        RouterGroupModel::create(db, group).await
    }
}
```

**在 Handler 中调用：**
```rust
GroupService::get_all(&state.db).await
```

---

## 选择哪种模式？

| 问题 | 答案 | 选择 |
|------|------|------|
| Service 需要持有配置（Secret、URL、超时）吗？ | 是 | 模式 A |
| Service 只是薄薄包装 Database 调用？ | 是 | 模式 B |
| 需要可测试的依赖注入？ | 是 | 模式 A（传入 mock） |

---

## 错误类型

**模式 A**：定义自己的 `XxxServiceError`：

```rust
#[derive(Debug, thiserror::Error)]
pub enum UserServiceError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] burncloud_database::DatabaseError),

    #[error("User already exists")]
    UserAlreadyExists,

    #[error("Invalid credentials")]
    InvalidCredentials,
}

pub type Result<T> = std::result::Result<T, UserServiceError>;
```

**模式 B**：可直接复用 `DatabaseError`（无需包装层）：

```rust
type Result<T> = std::result::Result<T, burncloud_database::DatabaseError>;
```

---

## Re-export 规则

Service crate 应 re-export 上游 Database crate 的业务类型，让 Server 只声明对 Service 的依赖：

```rust
// service-user/src/lib.rs
pub use burncloud_database_user::{UserAccount, UserRecharge};

// service-group/src/lib.rs
pub use burncloud_database_group::{RouterGroup, RouterGroupMember};
```

这样 `crates/server/Cargo.toml` 只需要：
```toml
burncloud_service_user = { path = "..." }
# 不需要 burncloud_database_user
```

---

## 并发操作

可并行的异步操作使用 `tokio::try_join!`：

```rust
// ✓ 并行执行
let (users, groups) = tokio::try_join!(
    UserService::list(&state.db),
    GroupService::get_all(&state.db)
)?;

// ✗ 可以并行但串行执行（浪费）
let users = UserService::list(&state.db).await?;
let groups = GroupService::get_all(&state.db).await?;
```

---

## 反例

```rust
// ✗ 禁止：持有 Database 字段
pub struct XxxService {
    db: Database,
}

// ✗ 禁止：引入 HTTP 类型
use axum::http::StatusCode;
use axum::response::Json;

// ✗ 禁止：Service 直接写 SQL（应通过 Model 层）
let rows = sqlx::query!("SELECT * FROM users").fetch_all(&pool).await?;
```
