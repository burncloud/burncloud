# 编程规范 — 核心规则

> 每次写代码前过一遍这 9 条。
> 详细规范见本目录其他文件。

---

## 硬性禁止

### 1. 禁止跨层直接调用

- Handler 不得直接调用 Database crate 的方法
- Service 不得引入 `axum`、`Router`、`Json` 等 HTTP 类型
- 依赖方向：`Server → Service → Database → Common`

### 2. 禁止持有 Database 连接

Service 方法通过参数接收 `db: &Database`，不得在结构体字段中持有 `Database` 实例。

```rust
// ✓ 正确
pub async fn get_all(db: &Database) -> Result<Vec<T>> { ... }

// ✗ 错误
pub struct XxxService { db: Database }
```

### 3. 禁止硬编码 SQL 占位符

双数据库（SQLite / PostgreSQL）环境下 `sqlx::query!` 宏不可用，必须用统一工具函数。

```rust
// ✓ 正确
use burncloud_database::placeholder::{ph, phs, adapt_sql};
let is_postgres = db.kind() == "postgres";
let sql = format!("WHERE id = {}", ph(is_postgres, 1));

// ✗ 错误
let sql = "WHERE id = $1";  // PostgreSQL 专用
let sql = "WHERE id = ?";   // SQLite 专用
```

### 4. 禁止裸字符串错误

```rust
// ✓ 正确 — 用 thiserror
#[derive(Debug, thiserror::Error)]
pub enum UserServiceError {
    #[error("User not found")]
    UserNotFound,
}

// ✗ 错误
return Err("user not found".into());
return Err(Box::new(std::io::Error::new(...)));
```

### 5. 禁止 `unwrap()` / `expect()`（非测试代码）

```rust
// ✓ 正确 — 用 ? 或 match
let user = get_user(db, id).await?;

// ✗ 错误
let user = get_user(db, id).await.unwrap();
let user = get_user(db, id).await.expect("should exist");
```

测试代码中允许 `unwrap()`/`expect()`，测试失败本来就应该 panic。

---

### 6. 新代码禁止 `println!` / `eprintln!`

新代码统一使用 `log::` 宏。存量代码（如 `auth.rs`）按优先级迁移，不强制立即重写。

```rust
// ✓ 正确（新代码）
log::warn!("JWT generation failed: {}", e);
log::error!("Registration error: {}", e);

// ✗ 新代码中禁止
eprintln!("error: {}", e);
```

---

## 强制要求

### 7. Handler 响应必须用 `ok` / `err` helper

`ok()` 和 `err()` 定义在 `crate::api::response`，统一响应结构 `{ success, data/message }`。

```rust
use crate::api::response::{ok, err};

// 成功
ok(data).into_response()

// 失败
err("Username already exists").into_response()
```

### 8. Handler 状态用 `State<AppState>`（不加 Arc）

`AppState` 本身 `#[derive(Clone)]`，内部字段已持 Arc。不需要外层再套 Arc。

```rust
// ✓ 正确
async fn handle(State(state): State<AppState>, ...) -> impl IntoResponse

// ✗ 常见误写（来自其他 axum 项目的习惯）
async fn handle(State(state): State<Arc<AppState>>, ...)
```

### 9. re-export 业务类型

Service crate 应 re-export 自己依赖的 Database crate 类型，让 server 只需依赖 service。

```rust
// service-user/src/lib.rs
pub use burncloud_database_user::{UserAccount, UserRecharge};
```

---

## 自检清单（提交前）

- [ ] 是否有跨层调用？
- [ ] Service 是否持有 Database 字段？
- [ ] SQL 是否用了硬编码 `$1` 或 `?`？
- [ ] 错误是否用了 `thiserror`？
- [ ] 是否有 `unwrap()` / `expect()`（非测试代码）？
- [ ] 新代码是否有 `eprintln!` / `println!`？
- [ ] Handler 是否用了 `ok()/err()`？

---

## 进一步阅读

| 主题 | 文件 |
|------|------|
| Handler 层详细规范 | `server.md` |
| Service 层详细规范 | `service.md` |
| Repository / SQL 规范 | `repository.md` |
| 数据模型规范 | `model.md` |
| 错误处理规范 | `error.md` |
| 数据库命名 & SQL | `DATABASE.md` |
| 完整 copy-paste 模板 | `templates.md` |
