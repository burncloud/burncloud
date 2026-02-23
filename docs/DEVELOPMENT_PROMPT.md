# BurnCloud 开发规范提示词

本文档为 AI 助手提供 BurnCloud 项目的开发规范，确保新功能开发符合项目架构标准和代码质量要求。

---

## 1. 架构概览

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

### 1.2 依赖方向规则

```
Client  →  Server  →  Router  →  Service  →  Database  →  Common
           ↓           ↓           ↓            ↓
           └───────────┴───────────┴────────────┘
                          ↓
                      Common (共享基础类型)
```

**规则**:
- 依赖只能向下流动，不能反向依赖
- Common 是所有层的基础，不依赖其他内部 crate
- 同层模块之间可以相互依赖（谨慎使用）

---

## 2. 目录结构规范

### 2.1 Workspace 结构

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
│   │       ├── service-user/
│   │       └── ...
│   ├── database/           # 数据库聚合器
│   │   └── crates/
│   │       ├── database-user/
│   │       ├── database-models/
│   │       ├── database-router/
│   │       └── ...
│   ├── client/             # GUI 聚合器
│   │   └── crates/
│   │       ├── client-shared/   # 共享组件
│   │       ├── client-api/      # API 客户端
│   │       ├── client-dashboard/
│   │       └── ...
│   ├── cli/                # 命令行工具
│   ├── core/               # 核心功能
│   ├── tests/              # E2E 测试
│   └── ...
└── src/
    └── main.rs             # 应用入口
```

### 2.2 Crate 命名规范

| 类型 | 命名格式 | 示例 |
|------|----------|------|
| 顶层功能 | `burncloud-{name}` | `burncloud-router`, `burncloud-server` |
| Service 子 crate | `burncloud-service-{name}` | `burncloud-service-user` |
| Database 子 crate | `burncloud-database-{name}` | `burncloud-database-models` |
| Client 子 crate | `burncloud-client-{name}` | `burncloud-client-dashboard` |

### 2.3 文件组织

每个 crate 内部结构:

```
crates/xxx/
├── Cargo.toml
├── src/
│   ├── lib.rs          # 库入口
│   ├── error.rs        # 错误定义（如有）
│   ├── types.rs        # 类型定义（如有）
│   └── {module}/       # 子模块
│       └── mod.rs
├── examples/           # 示例（可选）
└── tests/              # 集成测试（可选）
```

---

## 3. 新建 Crate 指南

### 3.1 创建步骤

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
# 使用 workspace 依赖
anyhow.workspace = true
thiserror.workspace = true
serde.workspace = true

# 内部 crate 依赖
burncloud-common.workspace = true
```

3. **创建 src/lib.rs**:
```rust
//! Crate 描述
//!
//! 详细说明...

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
# ... 现有依赖
burncloud-new-crate = { path = "crates/new-crate" }
```

### 3.2 子 Crate 创建 (聚合器模式)

如果需要创建新的子 crate (如 `service-xxx`):

1. **创建子 crate**:
```bash
mkdir -p crates/service/crates/service-xxx/src
```

2. **创建 Cargo.toml**:
```toml
[package]
name = "burncloud-service-xxx"
version = "0.1.0"
edition = "2021"

[dependencies]
burncloud-common.workspace = true
burncloud-database.workspace = true
```

3. **更新聚合器** (`crates/service/Cargo.toml`):
```toml
[dependencies]
burncloud-service-xxx.workspace = true
```

4. **更新聚合器** (`crates/service/src/lib.rs`):
```rust
pub use burncloud_service_xxx as xxx;
```

5. **注册到根 workspace** (`Cargo.toml`):
```toml
[workspace]
members = [
    # ...
    "crates/service/crates/service-xxx",
]

[workspace.dependencies]
burncloud-service-xxx = { path = "crates/service/crates/service-xxx" }
```

---

## 4. 代码模式

### 4.1 类型定义模式 (common/src/types.rs)

```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 价格信息 - 使用 i64 纳美元存储 (9位小数精度)
/// 注意: 使用 i64 而非 u64 以兼容 PostgreSQL BIGINT
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

### 4.2 错误处理模式

**库级别** (`error.rs`):
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

**应用级别**:
```rust
// 使用 anyhow 处理顶层错误
use anyhow::{Context, Result};

async fn handle_request() -> Result<()> {
    let data = fetch_data()
        .await
        .context("Failed to fetch data")?;
    Ok(())
}
```

### 4.3 Model 模式 (静态方法 + Database 参数)

```rust
use burncloud_database::{Database, Result};
use burncloud_common::types::Price;
use sqlx::FromRow;

/// 输入类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceInput {
    pub model: String,
    pub input_price: i64,
    pub output_price: i64,
}

/// Model - 静态方法集合
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

    /// 列表查询（分页）
    pub async fn list(
        db: &Database,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Price>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = if is_postgres {
            "SELECT * FROM prices ORDER BY model LIMIT $1 OFFSET $2"
        } else {
            "SELECT * FROM prices ORDER BY model LIMIT ? OFFSET ?"
        };

        let prices = sqlx::query_as(sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(conn.pool())
            .await?;

        Ok(prices)
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

    /// 删除
    pub async fn delete(db: &Database, model: &str) -> Result<bool> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "DELETE FROM prices WHERE model = $1"
        } else {
            "DELETE FROM prices WHERE model = ?"
        };

        let result = sqlx::query(sql)
            .bind(model)
            .execute(conn.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
```

### 4.4 聚合器模式 (pub use 重导出)

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

### 4.5 共享组件模式 (client-shared)

```
client/crates/client-shared/
├── src/
│   ├── lib.rs
│   ├── components/       # 可复用 UI 组件
│   │   ├── mod.rs
│   │   ├── button.rs
│   │   └── table.rs
│   ├── api/              # API 服务层
│   │   ├── mod.rs
│   │   └── client.rs
│   └── utils/            # 工具函数
│       └── mod.rs
```

---

## 5. 依赖管理规范

### 5.1 Workspace 依赖声明 (根 Cargo.toml)

```toml
[workspace.dependencies]
# 外部依赖
anyhow = "1.0"
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite", "postgres", "any"] }

# 内部 crate
burncloud-common = { path = "crates/common" }
burncloud-database = { path = "crates/database" }
burncloud-service = { path = "crates/service" }
burncloud-router = { path = "crates/router" }
```

### 5.2 子 Crate 引用

```toml
[package]
name = "burncloud-service-user"
version = "0.1.0"
edition = "2021"

[dependencies]
# 外部依赖 - 使用 workspace
anyhow.workspace = true
thiserror.workspace = true
serde.workspace = true

# 内部依赖 - 使用 workspace
burncloud-common.workspace = true
burncloud-database.workspace = true

# 特定 feature 需要重复声明
tokio = { workspace = true, features = ["full", "test-util"] }
```

### 5.3 依赖选择指南

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

## 6. 测试规范

### 6.1 测试文件组织

```
crates/tests/
├── Cargo.toml
├── tests/
│   ├── common/
│   │   └── mod.rs          # 测试工具函数
│   ├── api/
│   │   ├── mod.rs
│   │   ├── auth.rs         # 认证相关测试
│   │   ├── channel.rs      # Channel API 测试
│   │   └── relay.rs        # 转发 API 测试
│   ├── ui/
│   │   ├── mod.rs
│   │   └── basic_render.rs # UI 渲染测试
│   ├── api_tests.rs        # API 测试入口
│   └── ui_tests.rs         # UI 测试入口
└── src/
    └── lib.rs
```

### 6.2 自举测试模式

测试必须是自举的（自己启动服务）:

```rust
// tests/api/channel_test.rs
use burncloud_database::Database;
use burncloud_server;

#[tokio::test]
async fn test_channel_create() {
    // 1. 创建临时数据库
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

### 6.3 测试命名约定

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_default() { /* ... */ }

    #[test]
    fn test_currency_symbol() { /* ... */ }

    #[test]
    fn test_currency_from_str() { /* ... */ }

    #[tokio::test]
    async fn test_price_upsert() { /* ... */ }
}
```

---

## 7. 代码模板

### 7.1 新 Service Crate 模板

**crates/service/crates/service-xxx/Cargo.toml**:
```toml
[package]
name = "burncloud-service-xxx"
version = "0.1.0"
edition = "2021"
description = "XXX service for BurnCloud"

[dependencies]
anyhow.workspace = true
thiserror.workspace = true
serde.workspace = true
async-trait.workspace = true

burncloud-common.workspace = true
burncloud-database.workspace = true

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

**crates/service/crates/service-xxx/src/lib.rs**:
```rust
//! XXX Service
//!
//! 提供 XXX 功能的业务逻辑

mod error;
mod service;

pub use error::{Error, Result};
pub use service::XxxService;
```

**crates/service/crates/service-xxx/src/error.rs**:
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

**crates/service/crates/service-xxx/src/service.rs**:
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

### 7.2 新 Database 子 Crate 模板

**crates/database/crates/database-xxx/Cargo.toml**:
```toml
[package]
name = "burncloud-database-xxx"
version = "0.1.0"
edition = "2021"

[dependencies]
sqlx = { workspace = true, features = ["sqlite", "postgres"] }
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true

burncloud-database.workspace = true
burncloud-common.workspace = true

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
tempfile.workspace = true
```

**crates/database/crates/database-xxx/src/lib.rs**:
```rust
//! XXX 数据库操作

mod error;
mod model;

pub use error::{Error, Result};
pub use model::*;
```

**crates/database/crates/database-xxx/src/model.rs**:
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XxxInput {
    pub name: String,
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

### 7.3 新 Client 组件模板

**crates/client/crates/client-xxx/Cargo.toml**:
```toml
[package]
name = "burncloud-client-xxx"
version = "0.1.0"
edition = "2021"

[dependencies]
dioxus.workspace = true
dioxus-router.workspace = true

burncloud-client-api.workspace = true
burncloud-client-shared.workspace = true
burncloud-common.workspace = true
```

**crates/client/crates/client-xxx/src/lib.rs**:
```rust
//! XXX Client Module

mod page;
mod components;

pub use page::XxxPage;
```

**crates/client/crates/client-xxx/src/page.rs**:
```rust
use dioxus::prelude::*;

pub fn XxxPage() -> Element {
    rsx! {
        div {
            class: "p-4",
            h1 { "XXX Page" }
        }
    }
}
```

---

## 8. 数据库兼容性

### 8.1 PostgreSQL 与 SQLite 差异处理

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

### 8.2 数据类型注意事项

| 类型 | PostgreSQL | SQLite | 推荐 |
|------|------------|--------|------|
| 布尔值 | BOOLEAN | INTEGER (0/1) | `i32` 或 `bool` + sqlx 转换 |
| 大整数 | BIGINT (signed) | INTEGER | 使用 `i64` 而非 `u64` |
| 时间戳 | BIGINT/i64 | INTEGER | `i64` Unix 时间戳 |

---

## 9. Git 提交规范

### 9.1 提交格式

```
<Icon> <Type>: <Summary>
```

### 9.2 图标与类型

| Icon | Type | 描述 |
|------|------|------|
| ✨ | feat | 新功能 |
| 🐛 | fix | Bug 修复 |
| 📚 | docs | 文档更新 |
| 🔨 | refactor | 代码重构 |
| 🚀 | perf | 性能优化 |
| 🧪 | test | 测试代码 |
| 🔧 | chore | 构建/工具 |

### 9.3 示例

```
✨ feat: add tiered pricing support for Qwen models
🐛 fix: resolve PostgreSQL connection pool leak
🔨 refactor: extract common database query patterns
🧪 test: add integration tests for price sync
```

---

## 10. 检查清单

### 10.1 新功能开发检查项

- [ ] 确定功能属于哪一层 (Client/Server/Router/Service/Database)
- [ ] 创建或修改正确的 crate
- [ ] 使用 workspace 依赖格式
- [ ] 遵循命名规范
- [ ] 实现正确的错误处理
- [ ] 支持 PostgreSQL 和 SQLite 双数据库
- [ ] 添加必要的测试
- [ ] 更新相关文档

### 10.2 Code Review 检查项

- [ ] 架构分层是否正确
- [ ] 依赖方向是否正确
- [ ] 错误处理是否完善
- [ ] 数据库查询是否兼容双数据库
- [ ] 代码是否符合现有模式
- [ ] 是否有硬编码的配置
- [ ] 测试覆盖是否充分
- [ ] 文档是否更新

---

## 11. 常见问题

### Q1: 何时创建新的子 crate？

当满足以下条件时考虑创建新子 crate:
- 功能独立，与现有子 crate 边界清晰
- 需要被多个其他 crate 复用
- 现有子 crate 已经过于庞大

### Q2: 价格存储为什么要用 i64 纳美元？

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

### Q3: 如何处理 UI 国际化？

- 使用 `dioxus` 的 i18n 功能
- 字符串资源放在配置文件中
- 支持中英文作为基准语言
- UI 代码中不硬编码字符串

---

## 12. 参考资料

- [CLAUDE.md](./CLAUDE.md) - 项目核心文档
- [Cargo.toml](./Cargo.toml) - Workspace 配置
- [crates/common/src/types.rs](./crates/common/src/types.rs) - 核心类型定义
- [crates/database/src/error.rs](./crates/database/src/error.rs) - 错误处理示例
- [crates/service/src/lib.rs](./crates/service/src/lib.rs) - 聚合器模式示例
