# 代码模板库

> 直接 copy-paste，替换 `Xxx` / `xxx` 为你的领域名。
> 所有模板均来自实际生产代码，可直接编译。

---

## 模板 1：Handler（无状态 Service）

**适用：** GroupService、TokenService 这类无状态 Service。

```rust
// crates/server/src/api/xxx.rs

use crate::api::response::{err, ok};
use crate::AppState;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use burncloud_service_xxx::{Xxx, XxxService};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateXxxRequest {
    pub name: String,
    // 添加字段...
}

#[derive(Serialize)]
struct XxxSummary {
    pub id: String,
    pub name: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/xxx", get(list_xxx).post(create_xxx))
        .route("/api/xxx/{id}", delete(delete_xxx))
}

async fn list_xxx(State(state): State<AppState>) -> impl IntoResponse {
    match XxxService::get_all(&state.db).await {
        Ok(items) => ok(items).into_response(),
        Err(e) => {
            log::error!("list_xxx error: {}", e);
            err("Failed to list xxx").into_response()
        }
    }
}

async fn create_xxx(
    State(state): State<AppState>,
    Json(payload): Json<CreateXxxRequest>,
) -> impl IntoResponse {
    let item = Xxx {
        id: uuid::Uuid::new_v4().to_string(),
        name: payload.name,
        // 填充字段...
    };
    match XxxService::create(&state.db, &item).await {
        Ok(_) => ok(item).into_response(),
        Err(e) => {
            log::error!("create_xxx error: {}", e);
            err("Failed to create xxx").into_response()
        }
    }
}

async fn delete_xxx(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match XxxService::delete(&state.db, &id).await {
        Ok(_) => ok("deleted").into_response(),
        Err(e) => {
            log::error!("delete_xxx error: {}", e);
            err("Failed to delete xxx").into_response()
        }
    }
}
```

---

## 模板 2：Handler（有状态 Service，挂在 AppState）

**适用：** UserService 这类需要配置字段（JWT secret 等）的 Service。

```rust
// crates/server/src/api/user.rs

use crate::api::response::{err, ok};
use crate::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
    routing::post,
    Router,
};
use burncloud_service_user::UserServiceError;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct RegisterDto {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

#[derive(Serialize)]
struct RegisterResponse {
    id: String,
    username: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/users/register", post(register))
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDto>,
) -> impl IntoResponse {
    match state
        .user_service
        .register_user(&state.db, &payload.username, &payload.password, payload.email)
        .await
    {
        Ok(user_id) => ok(RegisterResponse {
            id: user_id,
            username: payload.username,
        })
        .into_response(),
        Err(UserServiceError::UserAlreadyExists) => {
            err("Username already exists").into_response()
        }
        Err(e) => {
            log::error!("register error: {}", e);
            err("Registration failed").into_response()
        }
    }
}
```

---

## 模板 3：Service（模式 B，无状态）

```rust
// crates/service/crates/service-xxx/src/lib.rs

use burncloud_database::Database;
use burncloud_database_xxx::XxxModel;

// re-export 让 server 层不需要直接依赖 database-xxx
pub use burncloud_database_xxx::Xxx;

type Result<T> = std::result::Result<T, burncloud_database::DatabaseError>;

/// Xxx service (stateless)
pub struct XxxService;

impl XxxService {
    pub async fn get_all(db: &Database) -> Result<Vec<Xxx>> {
        XxxModel::get_all(db).await
    }

    pub async fn get(db: &Database, id: &str) -> Result<Option<Xxx>> {
        XxxModel::get(db, id).await
    }

    pub async fn create(db: &Database, item: &Xxx) -> Result<()> {
        XxxModel::create(db, item).await
    }

    pub async fn delete(db: &Database, id: &str) -> Result<()> {
        XxxModel::delete(db, id).await
    }
}
```

---

## 模板 4：Service（模式 A，有状态）

```rust
// crates/service/crates/service-xxx/src/lib.rs

use burncloud_database::Database;
use burncloud_database_xxx::XxxDatabase;
use thiserror::Error;

pub use burncloud_database_xxx::{Xxx, XxxRecord};

#[derive(Debug, Error)]
pub enum XxxServiceError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] burncloud_database::DatabaseError),

    #[error("Xxx not found")]
    NotFound,

    #[error("Xxx already exists")]
    AlreadyExists,
}

pub type Result<T> = std::result::Result<T, XxxServiceError>;

pub struct XxxService {
    // 放需要持有的配置字段
    api_key: String,
}

impl XxxService {
    pub fn new() -> Self {
        let api_key = std::env::var("XXX_API_KEY")
            .unwrap_or_else(|_| "default".to_string());
        Self { api_key }
    }

    pub async fn create(
        &self,
        db: &Database,
        name: &str,
    ) -> Result<Xxx> {
        // 检查是否已存在
        if let Ok(Some(_)) = XxxDatabase::get_by_name(db, name).await {
            return Err(XxxServiceError::AlreadyExists);
        }

        let record = XxxRecord {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
        };
        XxxDatabase::create(db, &record).await?;
        Ok(record.into())
    }
}

impl Default for XxxService {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## 模板 5：Database Model（静态方法集合）

```rust
// crates/database/crates/database-xxx/src/lib.rs

use burncloud_database::{Database, DatabaseError};
use burncloud_database::placeholder::{adapt_sql, ph, phs};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 数据库行类型（FromRow）
// 注：如果有 bool 字段，移除 FromRow derive，改用手写实现（见 model.md）
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Xxx {
    pub id: String,
    pub name: String,
    pub created_at: Option<i64>,
}

/// 操作类型（静态方法集合）
pub struct XxxModel;

impl XxxModel {
    pub async fn get_all(db: &Database) -> Result<Vec<Xxx>, DatabaseError> {
        let conn = db.get_connection()?;
        let sql = "SELECT id, name, created_at FROM xxx_table ORDER BY created_at DESC";
        let rows = sqlx::query_as::<_, Xxx>(sql)
            .fetch_all(conn.pool())
            .await?;
        Ok(rows)
    }

    pub async fn get(db: &Database, id: &str) -> Result<Option<Xxx>, DatabaseError> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = adapt_sql(
            is_postgres,
            "SELECT id, name, created_at FROM xxx_table WHERE id = ?",
        );
        let row = sqlx::query_as::<_, Xxx>(&sql)
            .bind(id)
            .fetch_optional(conn.pool())
            .await?;
        Ok(row)
    }

    pub async fn create(db: &Database, item: &Xxx) -> Result<(), DatabaseError> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = format!(
            "INSERT INTO xxx_table (id, name, created_at) VALUES ({}, {}, {})",
            ph(is_postgres, 1),
            ph(is_postgres, 2),
            ph(is_postgres, 3),
        );
        sqlx::query(&sql)
            .bind(&item.id)
            .bind(&item.name)
            .bind(item.created_at)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    pub async fn delete(db: &Database, id: &str) -> Result<(), DatabaseError> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";
        let sql = adapt_sql(
            is_postgres,
            "DELETE FROM xxx_table WHERE id = ?",
        );
        sqlx::query(&sql)
            .bind(id)
            .execute(conn.pool())
            .await?;
        Ok(())
    }
}
```

---

## 模板 6：错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum XxxServiceError {
    // 自动从 DatabaseError 转换
    #[error("Database error: {0}")]
    DatabaseError(#[from] burncloud_database::DatabaseError),

    // 业务语义错误
    #[error("Xxx not found")]
    NotFound,

    #[error("Xxx already exists")]
    AlreadyExists,

    // 外部操作失败（用 String 包裹原始错误消息）
    #[error("External operation failed: {0}")]
    ExternalError(String),
}

pub type Result<T> = std::result::Result<T, XxxServiceError>;
```

---

## 模板 7：集成测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use burncloud_database::create_default_database;
    use burncloud_database_xxx::XxxDatabase;

    #[tokio::test]
    async fn test_create_xxx() -> anyhow::Result<()> {
        let db = create_default_database().await?;
        XxxDatabase::init(&db).await?;

        let service = XxxService::new();
        let result = service.create(&db, "test-item").await?;

        assert!(!result.id.is_empty());
        assert_eq!(result.name, "test-item");

        Ok(())
    }
}
```

---

## 速查：import 清单

### Handler 文件顶部

```rust
use crate::api::response::{err, ok};
use crate::AppState;
use axum::{
    extract::{Json, Path, Query, State},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
```

### Service 文件顶部

```rust
use burncloud_database::Database;
use burncloud_database_xxx::XxxModel;
use thiserror::Error;
```

### Database Model 文件顶部

```rust
use burncloud_database::{Database, DatabaseError};
use burncloud_database::placeholder::{adapt_sql, ph, phs};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
```
