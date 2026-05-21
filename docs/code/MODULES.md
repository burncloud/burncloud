# 模块职责规范

> **版本**: v1.0
> **最后更新**: 2026-05-19

---

## 一、Workspace Crate 结构

```
crates/
├── server/        ← axum HTTP 层，Handler 和路由
├── service/       ← 业务逻辑（service-user, service-group 等）
├── database/      ← 核心 Database + schema + placeholder utils
│                 ← 子 crate 在 database/crates/ 下按业务域组织
├── common/        ← CrudRepository trait，跨 crate 纯工具
├── router/        ← LLM 数据面路由（独立，不在 Service 依赖链内）
├── client/        ← Dioxus 前端
└── cli/           ← 命令行工具
```

---

## 二、Server (crates/server)

### 职责

- HTTP 路由注册
- Axum Handler
- AppState 管理
- HTTP 错误响应

### 文件结构

```
crates/server/
├── src/
│   ├── main.rs           # 入口
│   ├── lib.rs            # 库入口
│   ├── routes/           # HTTP 路由注册
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── token.rs
│   │   ├── channel.rs
│   │   └── ...
│   ├── handlers/         # Axum Handler
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── token.rs
│   │   ├── channel.rs
│   │   └── ...
│   ├── state.rs          # AppState 管理
│   └── error.rs          # HTTP 错误响应
└── Cargo.toml
```

### Handler 规范

```rust
// ✓ 正确
async fn get_users(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let users = UserService::get_all(&state.db).await?;
    ok(users).into_response()
}

// ✗ 错误 — 不要直接调用 Database
async fn get_users(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let users = UserRepository::find_all(&state.db).await?; // 跨层调用！
    ok(users).into_response()
}
```

---

## 三、Service (crates/service)

### 职责

- 业务逻辑
- 数据验证
- 事务协调

### 子 crate 结构

```
crates/service/
├── service-user/         # 用户注册、登录、JWT
├── service-token/        # Token 管理、验证
├── service-channel/      # Channel CRUD、测速
├── service-billing/      # 计费、价格缓存、成本计算
├── service-router-log/   # 请求日志
├── service-monitor/      # 监控统计
├── service-group/        # 用户组管理
├── service-models/       # 模型管理
└── service-setting/      # 系统设置
```

### Service 规范

**模式 A（有状态，有配置字段）**：
```rust
pub struct UserService { jwt_secret: String }
impl UserService {
    pub fn new(jwt_secret: String) -> Self { ... }
    pub async fn register(&self, db: &Database, ...) -> Result<...> { ... }
}
```

**模式 B（无状态，纯操作封装）**：
```rust
pub struct GroupService;
impl GroupService {
    pub async fn get_all(db: &Database) -> Result<Vec<RouterGroup>> { ... }
}
```

### Re-export 规范

Service crate 应 re-export 自己依赖的 Database crate 类型：

```rust
// service-user/src/lib.rs
pub use burncloud_database_user::{UserAccount, UserRecharge};
```

---

## 四、Database (crates/database)

### 职责

- SQL 执行
- Schema 定义
- Placeholder 工具
- Repository trait 实现

### 子 crate 结构

```
crates/database/
├── src/
│   ├── lib.rs            # 核心 Database 类型
│   ├── schema.rs         # Schema 定义
│   └── placeholder.rs    # ph()/phs() 工具
├── crates/
│   ├── user/             # User 表操作
│   ├── token/            # Token 表操作
│   ├── channel/          # Channel 表操作
│   ├── router/           # Router 配置表
│   ├── billing/          # Billing 表操作
│   ├── models/           # Model 价格表
│   └── setting/          # Setting 表操作
└── Cargo.toml
```

### Placeholder 工具

```rust
use burncloud_database::placeholder::{ph, phs};

let is_postgres = db.kind() == "postgres";

// 单个占位符
let sql = format!("WHERE id = {}", ph(is_postgres, 1));

// 多个占位符
let sql = format!("WHERE id IN ({})", phs(is_postgres, 3));
```

---

## 五、Common (crates/common)

### 职责

- CrudRepository trait
- 跨 crate 纯工具

### 文件结构

```
crates/common/
├── src/
│   ├── lib.rs
│   ├── repository.rs     # CrudRepository trait
│   └── utils.rs          # 跨 crate 工具
└── Cargo.toml
```

### CrudRepository Trait

```rust
pub trait CrudRepository<T> {
    async fn find_by_id(db: &Database, id: i64) -> Result<Option<T>>;
    async fn find_all(db: &Database) -> Result<Vec<T>>;
    async fn save(db: &Database, entity: &T) -> Result<()>;
    async fn delete(db: &Database, id: i64) -> Result<()>;
}
```

---

## 六、Router (crates/router)

### 职责

- 高并发流量处理
- 认证、限流
- 协议转换
- 零拷贝转发

### 文件结构

```
crates/router/
├── src/
│   ├── lib.rs              # 主路由逻辑
│   ├── channel_state.rs    # Channel 状态管理
│   ├── model_router.rs     # 模型路由决策
│   ├── passthrough.rs      # 零拷贝转发核心
│   ├── response_parser.rs  # 响应解析
│   ├── stream_parser.rs    # SSE 流解析
│   ├── aimd_limiter.rs     # AIMD 限流器
│   ├── circuit_breaker.rs  # 熔断器
│   ├── balancer/           # 负载均衡策略
│   ├── adaptor/            # 协议适配器
│   └── scheduler/          # 请求调度
└── Cargo.toml
```

### 宪法例外

Router 可以依赖：
- `service-billing`：PriceCache, CostCalculator
- `service-user`：UserService::resolve_traffic_class

---

## 七、Client (crates/client)

### 职责

- Dioxus 前端
- LiveView 交互

### 子 crate 结构

```
crates/client/
├── client-dashboard/   # 主仪表盘
├── client-monitor/     # 监控面板
├── client-finance/     # 财务面板
├── client-access/      # 访问控制面板
├── client-channel/     # Channel 管理
├── client-users/       # 用户管理
└── client-settings/    # 设置面板
```

### 关键原则

- Dioxus LiveView
- 不直接调用 Service
- 通过 Server API 交互

---

## 八、模块间依赖图

```
┌─────────────┐
│   client    │
└──────┬──────┘
       │ HTTP API
       ↓
┌─────────────┐     ┌─────────────┐
│   server    │────→│   router    │
└──────┬──────┘     └──────┬──────┘
       │                   │ (宪法例外)
       ↓                   ↓
┌─────────────┐     ┌─────────────┐
│   service   │←────│ service-*   │
└──────┬──────┘     └─────────────┘
       │
       ↓
┌─────────────┐
│  database   │
└──────┬──────┘
       │
       ↓
┌─────────────┐
│   common    │
└─────────────┘
```
