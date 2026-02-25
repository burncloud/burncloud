# BurnCloud 专家级 Rust 开发规范 (v2.1)

本文档整合了 BurnCloud 项目的架构规范、代码模式和最佳实践，为 AI 助手和开发者提供完整的开发指南。

> **v2.1 更新**: 新增 Crate 颗粒度指标 (1.4)、边界划分原则 (1.5)、Database/Service 对齐矩阵 (1.6)

---

## 1. 核心架构原则 (The Architectural Core)

### 1.1 四层架构

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 0: Client (GUI/LiveView)                             │
│  crates/client + crates/client/crates/*                     │
│  - Dioxus-based GUI (Desktop + Web)                         │
│  - Feature modules: dashboard, monitor, users, etc.         │
├─────────────────────────────────────────────────────────────┤
│  Layer 1: Server (Control Plane)                            │
│  crates/server                                              │
│  - RESTful APIs, LiveView hosting                           │
│  - Entry point: burncloud_server::start_server()            │
├─────────────────────────────────────────────────────────────┤
│  Layer 2: Router (Data Plane/Gateway)                       │
│  crates/router + crates/router/crates/*                     │
│  - High-concurrency traffic handling                        │
│  - Auth, rate limiting, protocol conversion                 │
│  - Core principle: "Don't Touch the Body"                   │
├─────────────────────────────────────────────────────────────┤
│  Layer 3: Service (Business Logic)                          │
│  crates/service + crates/service/crates/*                   │
│  - Pure business logic, no UI dependencies                  │
│  - Sub-crates: inference, monitor, user, models, etc.       │
├─────────────────────────────────────────────────────────────┤
│  Layer 4: Database (Persistence)                            │
│  crates/database + crates/database/crates/*                 │
│  - SQLx-based (SQLite primary, PostgreSQL optional)         │
│  - Sub-crates: user, models, router, setting, download      │
├─────────────────────────────────────────────────────────────┤
│  Foundation: Common (Shared Types)                          │
│  crates/common                                              │
│  - Core types, error definitions, utilities                 │
│  - No external crate dependencies beyond basics             │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 树状依赖法则 (The Tree Law)

项目严格遵循单向分层依赖，禁止跨层调用和循环依赖。

```
Client  →  Server  →  Router  →  Service  →  Database  →  Common
           ↓           ↓           ↓            ↓
           └───────────┴───────────┴────────────┘
                          ↓
                      Common (共享基础类型)
```

**层级职责**:

| 层级 | Crate | 职责 | 禁止事项 |
|------|-------|------|----------|
| Foundation | crates/common | 仅包含类型定义、Trait 定义、工具函数 | 无任何业务逻辑 |
| Data | crates/database/* | 仅处理 SQLx 操作 | 禁止包含 HTTP 逻辑或复杂业务校验 |
| Service | crates/service/* | 纯业务逻辑，事务编排 | 不得将 SQLx 类型直接暴露给上层 |
| Interface | crates/server, crates/client | 处理输入输出 | 禁止直接调用 Database 层 |

> 💡 **核心原则**: 依赖只能向下流动，Common 层不依赖任何内部 crate。同层模块可相互依赖但需谨慎。

### 1.3 细粒度 Crate 策略 (Atomic Crates)

- **One Thing, One Crate**: 任何独立的业务域（如 billing, user, audit）必须是独立的 Crate
- **禁止巨型 Crate**: 如果一个 Crate 的 `src/lib.rs` 超过 500 行或 mod 超过 5 个，必须拆分

### 1.4 Crate 颗粒度指标 (Granularity Metrics)

| 指标 | 警戒值 | 强制拆分值 | 说明 |
|------|--------|------------|------|
| `lib.rs` 行数 | 300 行 | 500 行 | 超过必须拆分到子模块 |
| 同级 mod 数量 | 5 个 | 8 个 | 超过考虑按领域拆分子 crate |
| 单文件行数 | 200 行 | 400 行 | 超过必须拆分 |
| 公开函数数量 | 15 个 | 25 个 | 超过考虑职责拆分 |

**拆分信号** (出现以下情况必须拆分):

- 文件名出现 `and` 或 `_or_`（如 `user_and_billing.rs`）
- 测试文件需要 mock 超过 3 个外部依赖
- `struct` 数量超过 10 个
- 存在明显独立的子领域（如 `Price` 独立于 `Model`）

### 1.5 边界划分原则 (Boundary Rules)

**单个 Crate 的职责边界**:

1. **单一领域**: 只处理一个业务实体或概念（如 User、Price、Channel）
2. **独立可测**: 可以在不依赖其他子 crate 的情况下进行单元测试
3. **独立演进**: 版本更新不需要同步修改其他 crate

**禁止的反模式**:

```rust
// 🛑 禁止: 巨型 lib.rs（所有代码堆在一个文件）
// crates/database/crates/database-router/src/lib.rs (938行)

// ✅ 正确: 按实体拆分
// crates/database/crates/database-router/src/
// ├── lib.rs          (导出，<100行)
// ├── channel.rs      (ChannelModel)
// ├── api_key.rs      (ApiKeyModel)
// ├── price.rs        (PriceModel)
// └── error.rs        (错误定义)
```

### 1.6 Database ↔ Service 对齐矩阵 (Alignment Matrix)

Database 和 Service 子 crate **必须一一对应**，形成垂直切分：

| 领域 | Database Crate | Service Crate | 职责 |
|------|----------------|---------------|------|
| User | database-user | service-user | 用户认证、权限、配置 |
| Model | database-models | service-models | 模型元数据、能力 |
| Price | database-price | service-price | 定价、计费规则 |
| Channel | database-channel | service-channel | 渠道配置、密钥管理 |
| Billing | database-billing | service-billing | 账单、消费记录 |
| Setting | database-setting | service-setting | 系统配置 |
| Inference | database-inference | service-inference | 推理请求、日志 |

**对齐规则**:

- 命名强制对齐: `database-{domain}` ↔ `service-{domain}`
- 不允许单边存在（除非明确标记为 "待实现" 并记录在技术债务中）
- 新增领域时，同时创建 database 和 service 子 crate

**例外情况** (无需对应):

- 纯外部服务封装（如 `service-redis`、`service-ip`）
- 纯计算/无状态服务（如 `service-monitor`）

---

## 2. 代码安全性与健壮性

### 2.1 错误处理 (Error Handling)

**库 (Library) 层级 (common, database, service)**:

- ✅ 必须使用 `thiserror` 定义结构化错误
- ✅ 必须向上传递错误上下文，而不是简单地 `unwrap`
- 🛑 禁止使用 `anyhow`（库代码不应强制决定错误报告格式）
- 🛑 禁止在库代码中 `panic!` (除了 test 和 const 上下文)

**应用 (Application) 层级 (server, cli)**:

- ✅ 推荐使用 `anyhow` 统一处理错误
- ✅ 必须在最顶层（如 HTTP Handler）捕获错误并转换为适当的 HTTP 状态码

**库级别错误模式** (`error.rs`):
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    Connection(#[from] sqlx::Error),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Record not found: {0}")]
    NotFound(String),
}

/// 类型别名简化返回类型
pub type Result<T> = std::result::Result<T, DatabaseError>;
```

**应用级别错误模式**:
```rust
use anyhow::{Context, Result};

async fn handle_request() -> Result<()> {
    let data = fetch_data()
        .await
        .context("Failed to fetch data")?;
    Ok(())
}
```

### 2.2 预防 Panic (Panic Prevention)

- 🛑 生产代码严禁使用 `.unwrap()` 或 `.expect()`
- ✅ 必须使用模式匹配、`?` 操作符或 `unwrap_or_else`
- ⚡ **例外**: 初始化阶段的全局配置加载（如果配置错了，程序本就该挂掉）或 `mutex.lock()`（仅当确信无污染时）

### 2.3 数值精度 (Numeric Precision)

- 🛑 **金融红线**: 涉及金额、价格、余额计算，严禁使用 `f32` / `f64`
- 🛑 **禁用**: `rust_decimal::Decimal`（项目统一使用 i64 纳美元）
- ✅ **强制**: 使用 `i64` 纳美元（nanodollar）存储所有金额
- ✅ **强制**: 数据库中使用 `BIGINT` 类型

**为什么必须使用 i64 纳美元**:
1. **精度**: 9位小数精度，避免浮点误差
2. **兼容性**: PostgreSQL BIGINT 是有符号的
3. **显示**: `$0.002` = `2_000_000` 纳美元

```rust
// 纳美元转美元
fn nanodollar_to_dollar(n: i64) -> f64 {
    n as f64 / 1_000_000_000.0
}

// 美元转纳美元
fn dollar_to_nanodollar(d: f64) -> i64 {
    (d * 1_000_000_000.0) as i64
}
```

---

## 3. 类型系统与领域建模 (Type System)

### 3.1 类型驱动设计 (Type-Driven Design)

不要使用基础类型（Primitives）来表示领域概念（Primitive Obsession）。

```rust
// 🛑 Bad
fn process_payment(user_id: String, amount: f64)

// ✅ Good
fn process_payment(user_id: UserId, amount: Money)
```

### 3.2 构造即合法 (Parse, Don't Validate)

利用类型系统保证数据的合法性，而不是到处写校验逻辑。

```rust
// ✅ Good: 定义 Email 结构体，其构造函数包含正则校验
// 一旦你拥有了一个 Email 实例，它必定是合法的
pub struct Email(String);

impl Email {
    pub fn new(s: String) -> Result<Self, ValidationError> {
        // 正则校验
        Ok(Email(s))
    }
}
```

### 3.3 类型定义模式

```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 价格信息（金额使用 i64 纳美元，详见 2.3 数值精度）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Price {
    pub id: i32,
    pub model: String,
    /// 每百万 token 输入价格（纳美元）
    pub input_price: i64,
    /// 每百万 token 输出价格（纳美元）
    pub output_price: i64,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// 创建/更新 Price 的输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceInput {
    pub model: String,
    pub input_price: i64,
    pub output_price: i64,
}

impl Default for PriceInput {
    fn default() -> Self {
        Self {
            model: String::new(),
            input_price: 0,
            output_price: 0,
        }
    }
}
```

---

## 4. 数据库交互规范 (Persistence Patterns)

### 4.1 兼容性抽象 (Polyglot Persistence)

代码必须同时兼容 SQLite 和 PostgreSQL。

- 🛑 禁止: 在业务逻辑中散落 `if db.kind() == Sqlite`
- ✅ 推荐: 使用 Repository Pattern 或 Query Builder

### 4.2 PostgreSQL 与 SQLite 差异处理

```rust
// 始终检查数据库类型
let is_postgres = db.kind() == "postgres";

// SQL 语句差异
let sql = if is_postgres {
    "SELECT * FROM table WHERE id = $1"  // PostgreSQL: $1, $2, ...
} else {
    "SELECT * FROM table WHERE id = ?"   // SQLite: ?, ?
};

// 关键字转义
let group_col = if is_postgres { "\"group\"" } else { "`group`" };
```

### 4.3 数据类型注意事项

| 类型 | PostgreSQL | SQLite | 推荐 |
|------|------------|--------|------|
| 布尔值 | BOOLEAN | INTEGER (0/1) | `i32` 或 `bool` + sqlx 转换 |
| 大整数 | BIGINT (signed) | INTEGER | 使用 `i64` 而非 `u64` |
| 时间戳 | BIGINT/i64 | INTEGER | `i64` Unix 时间戳 |

### 4.4 SQL 安全

- ✅ 必须使用参数化查询 (`sqlx::query("... $1 ...").bind(...)`)
- 🛑 严禁使用 `format!` 拼接 SQL 字符串（防止 SQL 注入）

### 4.5 Model 模式 (静态方法 + Database 参数)

```rust
use burncloud_database::{Database, Result};
use burncloud_common::types::Price;

pub struct PriceModel;

impl PriceModel {
    /// 获取单个价格
    pub async fn get(db: &Database, model: &str) -> Result<Option<Price>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = if is_postgres {
            "SELECT * FROM prices WHERE model = $1"
        } else {
            "SELECT * FROM prices WHERE model = ?"
        };

        let price = sqlx::query_as(sql)
            .bind(model)
            .fetch_optional(conn.pool())
            .await?;

        Ok(price)
    }

    /// 创建或更新 (Upsert)
    pub async fn upsert(db: &Database, input: &PriceInput) -> Result<()> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = if is_postgres {
            r#"
            INSERT INTO prices (model, input_price, output_price)
            VALUES ($1, $2, $3)
            ON CONFLICT(model) DO UPDATE SET
                input_price = EXCLUDED.input_price,
                output_price = EXCLUDED.output_price
            "#
        } else {
            r#"
            INSERT INTO prices (model, input_price, output_price)
            VALUES (?, ?, ?)
            ON CONFLICT(model) DO UPDATE SET
                input_price = excluded.input_price,
                output_price = excluded.output_price
            "#
        };

        sqlx::query(sql)
            .bind(&input.model)
            .bind(input.input_price)
            .bind(input.output_price)
            .execute(conn.pool())
            .await?;

        Ok(())
    }
}
```

---

## 5. 异步编程规范 (Async/Await)

### 5.1 避免阻塞 (Non-Blocking)

- 🛑 严禁在 async 函数中执行 CPU 密集型计算或同步 IO（如 `std::fs`, `std::thread::sleep`）
- ✅ 必须使用 `tokio::fs`, `tokio::time::sleep`
- ⚡ 重计算处理: 如果必须进行大量计算（如加密、图像处理），使用 `tokio::task::spawn_blocking`

### 5.2 锁的使用 (Locking)

- 🛑 禁止在跨越 `.await` 的地方持有 `std::sync::Mutex`
- ✅ 推荐: 使用 `tokio::sync::Mutex` 或 `RwLock`
- ⚡ 最佳实践: 尽量减少锁的粒度，或使用消息传递 (Channels) 代替共享内存

---

## 6. 工程化与依赖管理 (Engineering & Dependencies)

### 6.1 Workspace 依赖管理

- ✅ 强制: 所有第三方库版本必须在根 `Cargo.toml` 的 `[workspace.dependencies]` 中声明
- ✅ 强制: 子 Crate 必须引用 Workspace 版本：`serde = { workspace = true }`

**根 Cargo.toml 示例**:
```toml
[workspace.dependencies]
# 外部依赖
anyhow = "1.0"
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite", "postgres", "any"] }

# 内部 crate
burncloud-common = { path = "crates/common" }
burncloud-database = { path = "crates/database" }
burncloud-service = { path = "crates/service" }
```

### 6.2 模块可见性 (Visibility)

利用 Rust 的模块系统隐藏实现细节。

- ✅ 推荐: `pub(crate)` 用于同一个 Crate 内共享但不对外暴露的函数
- ✅ 推荐: `mod private` 模式保护关键 Trait 不被外部实现

### 6.3 避免 prelude 污染

- 🛑 禁止: 在库代码中使用 `use some_crate::prelude::*;`（除了标准库和非常通用的库如 tokio）
- 这会导致命名冲突并降低代码可读性

### 6.4 依赖选择指南

| 需求 | 推荐 crate |
|------|------------|
| 错误处理 (库) | `thiserror` |
| 错误处理 (应用) | `anyhow` |
| 序列化 | `serde` + `serde_json` |
| 异步运行时 | `tokio` |
| 数据库 | `sqlx` |
| HTTP 客户端 | `reqwest` |
| HTTP 服务端 | `axum` |
| 日志 | `log` / `tracing` |

---

## 7. 目录结构规范

### 7.1 Workspace 结构

```
burncloud/
├── Cargo.toml              # Workspace 根配置
├── crates/
│   ├── common/             # 共享类型和工具
│   ├── server/             # 控制平面
│   ├── router/             # 数据平面
│   │   └── crates/
│   │       └── router-aws/ # Router 子模块
│   ├── service/            # 业务逻辑聚合器
│   │   └── crates/
│   │       ├── service-inference/
│   │       ├── service-models/
│   │       ├── service-monitor/
│   │       └── service-user/
│   ├── database/           # 数据库聚合器
│   │   └── crates/
│   │       ├── database-user/
│   │       ├── database-models/
│   │       └── database-router/
│   ├── client/             # GUI 聚合器
│   │   └── crates/
│   │       ├── client-shared/
│   │       ├── client-api/
│   │       └── client-dashboard/
│   ├── cli/                # 命令行工具
│   ├── core/               # 核心功能
│   └── tests/              # E2E 测试
└── src/
    └── main.rs             # 应用入口
```

### 7.2 Crate 命名规范

| 类型 | 命名格式 | 示例 |
|------|----------|------|
| 顶层功能 | `burncloud-{name}` | `burncloud-router`, `burncloud-server` |
| Service 子 crate | `burncloud-service-{name}` | `burncloud-service-user` |
| Database 子 crate | `burncloud-database-{name}` | `burncloud-database-models` |
| Client 子 crate | `burncloud-client-{name}` | `burncloud-client-dashboard` |

### 7.3 文件组织

每个 crate 内部结构:

```
crates/xxx/
├── Cargo.toml
├── src/
│   ├── lib.rs          # 库入口（<100行，仅导出）
│   ├── error.rs        # 错误定义（如有）
│   ├── types.rs        # 类型定义（如有）
│   └── {module}/       # 子模块
│       └── mod.rs
├── examples/           # 示例（可选）
└── tests/              # 集成测试（可选）
```

**lib.rs 模板** (保持简洁):

```rust
//! Crate 简要描述
//!
//! 详细说明...

mod channel;
mod price;
mod error;

pub use channel::*;
pub use price::*;
pub use error::{Error, Result};
```

### 7.4 现有 Crate 状态监控

| Crate | lib.rs 行数 | 状态 | 行动 |
|-------|-------------|------|------|
| database-router | 938 | 🔴 超标 | 立即拆分 |
| client-register | 568 | 🔴 超标 | 立即拆分 |
| service-user | 445 | 🟡 警戒 | 短期拆分 |
| database-user | 405 | 🟡 警戒 | 短期拆分 |

> 💡 **建议**: 在 CI 中添加 `lib.rs` 行数检查，超过 300 行发出警告

---

## 8. 聚合器模式

### 8.1 模式说明 (pub use 重导出)

**service/Cargo.toml**:
```toml
[package]
name = "burncloud-service"
version = "0.1.0"
edition = "2021"

[dependencies]
burncloud-service-ip.workspace = true
burncloud-service-models.workspace = true
burncloud-service-monitor.workspace = true
```

**service/src/lib.rs**:
```rust
// 重新导出 service 子模块
pub use burncloud_service_ip as ip;
pub use burncloud_service_models as models;
pub use burncloud_service_monitor as monitor;
```

**使用方式**:
```rust
use burncloud_service::models::PriceModel;
use burncloud_service::monitor::MonitorService;
```

---

## 9. 测试规范 (Testing Strategy)

### 9.1 单元测试 (Unit Tests)

- 每个模块 (`mod.rs`) 下方应包含 `#[cfg(test)] mod tests`
- 测试核心业务逻辑的边缘情况

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_default() { /* ... */ }

    #[test]
    fn test_currency_symbol() { /* ... */ }

    #[tokio::test]
    async fn test_price_upsert() { /* ... */ }
}
```

### 9.2 集成测试 (Integration Tests)

- 放在 `crates/tests/` 目录下
- **黑盒测试**: 像使用者一样调用 Public API
- **自举环境**: 测试代码必须能够启动临时的 SQLite 内存数据库或 Docker 容器中的 Postgres

**测试文件组织**:
```
crates/tests/
├── Cargo.toml
├── tests/
│   ├── common/
│   │   └── mod.rs          # 测试工具函数
│   ├── api/
│   │   ├── mod.rs
│   │   ├── auth.rs         # 认证相关测试
│   │   └── channel.rs      # Channel API 测试
│   ├── api_tests.rs        # API 测试入口
│   └── ui_tests.rs         # UI 测试入口
└── src/
    └── lib.rs
```

**自举测试模式**:
```rust
use burncloud_database::Database;
use burncloud_server;

#[tokio::test]
async fn test_channel_create() {
    // 1. 创建临时数据库
    // 注: 测试代码中 .unwrap() 是允许的例外
    let db = Database::new_in_memory().await.unwrap();

    // 2. 启动测试服务器
    let server = burncloud_server::start_test_server(db).await;

    // 3. 执行测试
    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/api/channel", server.url()))
        .json(&json!({
            "name": "test-channel",
            "type": 1,
            "key": "sk-test",
        }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());

    // 4. 清理
    server.shutdown().await;
}
```

---

## 10. 新建 Crate 指南与代码模板

### 10.1 创建步骤

1. **创建目录结构**:
```bash
mkdir -p crates/new-crate/src
```

2. **创建 Cargo.toml**:
```toml
[package]
name = "burncloud-new-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow.workspace = true
thiserror.workspace = true
serde.workspace = true
burncloud-common.workspace = true
```

3. **创建 src/lib.rs**:
```rust
//! Crate 描述

mod error;
mod types;

pub use error::{Error, Result};
pub use types::*;
```

4. **注册到 workspace** (根 `Cargo.toml`):
```toml
[workspace]
members = [
    # ... 现有成员
    "crates/new-crate",
]

[workspace.dependencies]
burncloud-new-crate = { path = "crates/new-crate" }
```

### 10.2 子 Crate 创建 (聚合器模式)

1. **创建子 crate**:
```bash
mkdir -p crates/service/crates/service-xxx/src
```

2. **更新聚合器** (`crates/service/src/lib.rs`):
```rust
pub use burncloud_service_xxx as xxx;
```

3. **注册到根 workspace**:
```toml
[workspace]
members = ["crates/service/crates/service-xxx"]

[workspace.dependencies]
burncloud-service-xxx = { path = "crates/service/crates/service-xxx" }
```

### 10.3 Service Crate 模板

**src/lib.rs**:
```rust
//! XXX Service

mod error;
mod service;

pub use error::{Error, Result};
pub use service::XxxService;
```

**src/error.rs**:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("XXX operation failed: {0}")]
    OperationFailed(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, Error>;
```

**src/service.rs**:
```rust
use crate::{Error, Result};
use burncloud_database::Database;

pub struct XxxService;

impl XxxService {
    pub async fn do_something(db: &Database) -> Result<()> {
        // 业务逻辑
        Ok(())
    }
}
```

### 10.4 Database 子 Crate 模板

**src/model.rs**:
```rust
use burncloud_database::{Database, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct XxxRecord {
    pub id: i32,
    pub name: String,
    pub created_at: Option<i64>,
}

pub struct XxxModel;

impl XxxModel {
    pub async fn get(db: &Database, id: i32) -> Result<Option<XxxRecord>> {
        // 实现查询
    }

    pub async fn create(db: &Database, input: &XxxInput) -> Result<XxxRecord> {
        // 实现创建
    }
}
```

---

## 11. Git 提交规范

### 11.1 提交格式

```
<Icon> <Type>: <Summary>
```

### 11.2 图标与类型

| Icon | Type | 描述 |
|------|------|------|
| ✨ | feat | 新功能 |
| 🐛 | fix | Bug 修复 |
| 📚 | docs | 文档更新 |
| 🔨 | refactor | 代码重构 |
| 🚀 | perf | 性能优化 |
| 🧪 | test | 测试代码 |
| 🔧 | chore | 构建/工具 |

### 11.3 示例

```
✨ feat: add tiered pricing support for Qwen models
🐛 fix: resolve PostgreSQL connection pool leak
🔨 refactor: extract common database query patterns
🧪 test: add integration tests for price sync
```

---

## 12. AI 辅助生成代码的"红线" (Red Lines for AI)

当 AI 助手生成代码时，必须通过以下自我审查清单：

- ❌ 绝不为了图省事而让上层代码直接依赖底层实现（如 Server 层引用 SQLx）
- ❌ 绝不在代码中遗留 `TODO` 或 `unimplemented!()` 除非用户明确要求占位
- ❌ 绝不在循环中进行数据库查询（N+1 问题）。必须使用 `WHERE IN (...)` 批量查询
- ❌ 绝不使用 `unsafe` 代码块，除非有极致性能需求且经过人工审核
- ❌ 绝不在金额计算中使用 `f32`/`f64` 或 `rust_decimal::Decimal`，必须使用 `i64` 纳美元

---

## 13. 提交前检查清单 (Pre-Commit Checklist)

在提交代码前，请确保：

- [ ] `cargo fmt` 已运行
- [ ] `cargo clippy -- -D warnings` 无报错（将警告视为错误）
- [ ] `cargo test` 全部通过
- [ ] 没有引入新的循环依赖
- [ ] `Cargo.toml` 使用了 workspace 继承

### 13.1 新功能开发检查项

- [ ] 确定功能属于哪一层 (Client/Server/Router/Service/Database)
- [ ] 创建或修改正确的 crate
- [ ] 使用 workspace 依赖格式
- [ ] 遵循命名规范
- [ ] 实现正确的错误处理
- [ ] 支持 PostgreSQL 和 SQLite 双数据库
- [ ] 添加必要的测试
- [ ] 更新相关文档

### 13.2 Code Review 检查项

- [ ] 架构分层是否正确
- [ ] 依赖方向是否正确
- [ ] 错误处理是否完善
- [ ] 数据库查询是否兼容双数据库
- [ ] 代码是否符合现有模式
- [ ] 是否有硬编码的配置
- [ ] 测试覆盖是否充分
- [ ] 文档是否更新

---

## 14. 常见问题

### Q1: 何时创建新的子 crate？

当满足以下条件时考虑创建新子 crate:
- 功能独立，与现有子 crate 边界清晰
- 需要被多个其他 crate 复用
- 现有子 crate 已经过于庞大
- 新增独立业务领域（需同时创建 database-{domain} 和 service-{domain}）

### Q2: lib.rs 超过多少行必须拆分？

| 行数 | 状态 | 行动 |
|------|------|------|
| < 100 | ✅ 理想 | 保持现状 |
| 100-300 | 🟢 正常 | 可接受 |
| 300-500 | 🟡 警戒 | 计划拆分 |
| > 500 | 🔴 强制 | 必须立即拆分 |

**拆分步骤**:
1. 识别独立的实体/功能模块
2. 创建独立文件（如 `channel.rs`、`price.rs`）
3. 将相关函数和类型迁移到新文件
4. lib.rs 仅保留 `mod` 和 `pub use`

### Q3: 为什么金额必须用 i64 纳美元而不用 rust_decimal？

1. **性能**: i64 是原生类型，运算速度远超 Decimal
2. **兼容性**: PostgreSQL BIGINT 是有符号 i64，与无符号 u64 不兼容
3. **精度**: 纳美元提供 9 位小数精度（$0.000000001），足以满足 Token 计费需求
4. **一致性**: 统一使用 i64 避免类型转换带来的精度丢失

### Q4: 如何处理 UI 国际化？

- 使用 `dioxus` 的 i18n 功能
- 字符串资源放在配置文件中
- 支持中英文作为基准语言
- UI 代码中不硬编码字符串

### Q5: Database 和 Service 子 crate 不对齐怎么办？

**优先级**:
1. **立即修复**: 导致依赖混乱的不对齐
2. **短期修复**: 缺失的对应 crate（标记为技术债务）
3. **例外情况**: 纯外部服务（如 redis、ip）无需对应

**修复步骤**:
1. 创建缺失的 crate（如 `database-price` 对应 `service-price`）
2. 迁移相关代码
3. 更新聚合器的 `pub use`
4. 注册到 workspace

---

## 15. 参考资料

- [CLAUDE.md](./CLAUDE.md) - 项目核心文档
- [Cargo.toml](./Cargo.toml) - Workspace 配置
- [crates/common/src/types.rs](./crates/common/src/types.rs) - 核心类型定义
- [crates/database/src/error.rs](./crates/database/src/error.rs) - 错误处理示例
- [crates/service/src/lib.rs](./crates/service/src/lib.rs) - 聚合器模式示例
