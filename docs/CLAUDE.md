# BurnCloud — AI 助手快速参考

> 此文件在每次 Claude Code 会话启动时自动加载。
> 阅读时间 < 2 分钟。

---

## 架构速览

```
Client (Dioxus) → Server (axum) → Router (data plane)
                         ↓
                  Service (business logic)
                         ↓
                  Database crates (SQLx)
                         ↓
                  Common (shared utilities)
```

依赖方向严格单向：Server 依赖 Service，Service 依赖 Database，Database 依赖 Common。

**禁止反向依赖。禁止 Service 引入 axum 类型。**

---

## 关键约定（违反会导致编译错误或运行时 bug）

| 约定 | 正确 | 错误 |
|------|------|------|
| Handler 状态注入 | `State(state): State<AppState>` | `State<Arc<AppState>>` |
| Handler 返回类型 | `impl IntoResponse` | `Result<Json<T>, AppError>` |
| 响应构造 | `ok(data).into_response()` | 手写 `Json(...)` |
| 响应错误 | `err("message").into_response()` | 手写 `StatusCode::BAD_REQUEST` |
| SQL 占位符 | `ph(db.is_postgres(), 1)` / `phs(...)` | `$1` 或 `?` 硬编码 |
| Service 传参 | `db: &Database` 按引用传入 | 持有 `db: Database` 字段 |
| 日志 | `log::info!` / `log::warn!` / `log::error!` | `eprintln!` / `println!` |
| 错误类型 | `thiserror::Error` derive | 裸 `String` / `Box<dyn Error>` |

---

## Service 两种模式

**模式 A（有状态，有配置字段）** — 例如 `UserService`
```rust
pub struct UserService { jwt_secret: String }
impl UserService {
    pub fn new() -> Self { ... }
    pub async fn register(&self, db: &Database, ...) -> Result<...> { ... }
}
```

**模式 B（无状态，纯操作封装）** — 例如 `GroupService`, `TokenService`
```rust
pub struct GroupService;
impl GroupService {
    pub async fn get_all(db: &Database) -> Result<Vec<RouterGroup>> { ... }
}
```

---

## 完整规范文档

| 主题 | 文件 |
|------|------|
| 核心规则（必读） | `docs/code/README.md` |
| Handler / Router | `docs/code/SERVER.md` |
| Service 层 | `docs/code/SERVICE.md` |
| 数据库命名 & SQL | `docs/code/DATABASE.md` |
| 数据模型规范 | `docs/code/MODEL.md` |
| 错误处理 | `docs/code/ERROR.md` |
| 完整 copy-paste 模板 | `docs/code/TEMPLATES.md` |

---

## Workspace Crate 结构

```
crates/
  server/        ← axum HTTP 层，Handler 和路由
  service/       ← 业务逻辑（service-user, service-group 等）
  database/      ← 核心 Database + schema + placeholder utils
  database-*/    ← 各业务域 DB crate（database-user, database-token 等）
  common/        ← CrudRepository trait，跨 crate 纯工具
  router/        ← LLM 数据面路由（独立，不在 Service 依赖链内）
  client/        ← Dioxus 前端
  cli/           ← 命令行工具
```
