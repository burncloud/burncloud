# BurnCloud 代码架构分析报告

> 执行日期: 2026-03-03
> 分析范围: 全项目代码库
> 参考: CLAUDE.md (v2.1 开发规范)

---

## 1. 执行摘要

本报告基于 CLAUDE.md v2.1 规范，对 BurnCloud 项目进行了全面的代码健康度审查。主要发现以下三个核心问题：

| 问题类型 | 严重程度 | 数量 |
|----------|----------|------|
| lib.rs 行数超标 (>=500) | 🔴 严重 | 4 个 |
| lib.rs 行数警戒 (300-500) | 🟡 警戒 | 4 个 |
| Database/Service 不对齐 | ⚠️ 需评估 | 2 个 |
| Server 层直接调用 Database | ⚠️ 架构违规 | 5+ 个文件 |
| 金额类型使用浮点数 | 🔴 严重 | 2 处 |

---

## 2. Database ↔ Service 对齐矩阵分析

### 2.1 当前状态

| 领域 | Database Crate | Service Crate | 状态 |
|------|----------------|---------------|------|
| User | `database-user` | `service-user` | 🟢 配对 |
| Models | `database-models` | `service-models` | 🟢 配对 |
| Setting | `database-setting` | `service-setting` | 🟢 配对 |
| Download | `database-download` | - | ⚠️ **缺失 service-download** |
| Router | `database-router` | - | ⚠️ **缺失 service-router** |
| Inference | - | `service-inference` | 🟡 例外: 复用 database-router |
| IP | - | `service-ip` | 🟢 例外: 纯外部服务 |
| Monitor | - | `service-monitor` | 🟢 例外: 纯计算服务 |
| Redis | - | `service-redis` | 🟢 例外: 纯外部服务 |

### 2.2 问题详情

#### ⚠️ 缺失 `service-router`

**当前状态**: `database-router` 存在 (1413 行)，但没有对应的 `service-router`

**影响**:
- Server 层 API 直接调用 `RouterDatabase` (见 `crates/server/src/api/token.rs`, `group.rs`, `log.rs`)
- Router 层直接调用 `RouterDatabase` (见 `crates/router/src/lib.rs`)
- 违反分层架构原则: Server/Router 应该只依赖 Service 层

**建议**:
1. 创建 `service-router` crate
2. 将 `database-router` 中的业务逻辑（如配额扣除、使用统计计算）上移到 `service-router`
3. `database-router` 只保留纯 SQLx 操作

#### ⚠️ 缺失 `service-download`

**当前状态**: `database-download` 存在 (173 行)，但没有对应的 `service-download`

**影响**:
- 下载逻辑可能散落在 `crates/download` 层
- 需要评估是否需要独立的 service 层

---

## 3. 代码臃肿度分析

### 3.1 超标文件列表 (lib.rs 行数)

根据规范 1.4 指标：
- 🔴 **严重** (>=500 行): 必须立即拆分
- 🟡 **警戒** (300-500 行): 计划拆分

| 文件 | 行数 | 风险 | 包含实体 |
|------|------|------|----------|
| `crates/router/src/lib.rs` | **1607** | 🔴 | 路由、限流、熔断、计费、价格同步、通知... |
| `crates/database/crates/database-router/src/lib.rs` | **1413** | 🔴 | DbUpstream, DbToken, DbGroup, DbRouterLog... |
| `crates/download/crates/download-aria2/src/lib.rs` | **752** | 🔴 | 下载逻辑过于集中 |
| `crates/client/crates/client-register/src/lib.rs` | **568** | 🔴 | 注册页面逻辑过于集中 |
| `crates/client/crates/client-access/src/lib.rs` | 478 | 🟡 | |
| `crates/database/crates/database-user/src/lib.rs` | 476 | 🟡 | |
| `crates/service/crates/service-user/src/lib.rs` | 447 | 🟡 | |
| `crates/client/crates/client-connect/src/lib.rs` | 443 | 🟡 | |

### 3.2 `database-router` 详细分析

该文件 (1413 行) 混合了以下职责：

```
database-router/src/lib.rs
├── DbUpstream (上游配置)
├── DbToken (令牌管理)
├── DbGroup (分组)
├── DbGroupMember (分组成员)
├── DbRouterLog (路由日志)
├── UsageStats (使用统计)
├── ModelUsageStats (模型使用统计)
└── 32+ 个公开方法
```

**违反原则**:
- "One Thing, One Crate" - 多个独立实体混在一起
- "Atomic Crates" - 1413 行远超 500 行强制拆分值

**建议拆分方案**:

```
crates/database/crates/
├── database-upstream/     # DbUpstream 相关
├── database-token/        # DbToken 相关 (配额管理)
├── database-group/        # DbGroup + DbGroupMember
├── database-router-log/   # DbRouterLog + 统计
└── database-router/       # 聚合器 (仅 pub use)
```

### 3.3 `crates/router/src/lib.rs` 详细分析

该文件 (1607 行) 混合了以下模块：

```rust
mod adaptive_limit;
mod adaptor;
mod balancer;
mod billing;        // 计费逻辑
mod channel_state;
mod circuit_breaker;
mod config;
mod exchange_rate;  // 汇率
mod limiter;
mod model_router;
mod notification;   // 通知
mod passthrough;
mod price_sync;     // 价格同步
mod pricing_loader;
mod response_parser;
mod state;
mod stream_parser;
mod token_counter;
```

**问题**:
- 计费 (`billing`) 和价格 (`price_sync`, `pricing_loader`) 逻辑应该在 Service 层
- 通知 (`notification`) 应该是独立的 Service

---

## 4. 架构违规分析

### 4.1 Server 层直接调用 Database 层

**违规文件**:

| 文件 | 直接使用的 Database 类型 |
|------|-------------------------|
| `crates/server/src/api/token.rs` | `RouterDatabase`, `DbToken` |
| `crates/server/src/api/group.rs` | `RouterDatabase`, `DbGroup`, `DbGroupMember` |
| `crates/server/src/api/log.rs` | `RouterDatabase` |
| `crates/server/src/api/user.rs` | `UserDatabase`, `DbUser`, `DbRecharge` |
| `crates/server/src/api/auth.rs` | `UserDatabase`, `DbUser` |

**示例** (来自 `token.rs:8`):
```rust
use burncloud_database_router::{DbToken, RouterDatabase};
// ...
match RouterDatabase::list_tokens(&state.db).await {
    // Server 直接调用 Database 层
}
```

**正确做法**:
```rust
// 应该通过 Service 层
use burncloud_service_token::TokenService;
// ...
match TokenService::list_tokens(&state.db).await {
}
```

### 4.2 Service 层缺失导致的问题

由于缺失 `service-router`，以下业务逻辑被放在了 Database 层：

1. **配额扣除逻辑** (`database-router/src/lib.rs:843-932`)
   - `deduct_quota()`
   - `deduct_usd()`
   - `deduct_cny()`
   - `deduct_dual_currency()`

2. **使用统计计算** (`database-router/src/lib.rs:1221-1365`)
   - `UsageStats` 结构体
   - `ModelUsageStats` 结构体
   - `get_usage_stats()`
   - `get_usage_stats_by_model()`

这些业务逻辑应该在 Service 层实现，Database 层只负责数据存取。

---

## 5. 金额类型使用问题

### 5.1 数据库 Schema 中的浮点数

**问题位置**: `database-router/src/lib.rs`

**SQLite Schema (行 278)**:
```sql
ALTER TABLE router_logs ADD COLUMN cost REAL NOT NULL DEFAULT 0
```

**PostgreSQL Schema (行 226)**:
```sql
cost DOUBLE PRECISION DEFAULT 0
```

**违反规范**: CLAUDE.md 2.3 规定所有金额必须使用 `BIGINT` (i64 纳美元)

### 5.2 代码中的 i64 使用

代码中 `DbRouterLog.cost` 字段已经定义为 `i64`:
```rust
pub struct DbRouterLog {
    // ...
    pub cost: i64,  // 正确: 使用 i64
}
```

**问题**: Schema 定义与代码不一致，需要数据库迁移

### 5.3 CLI 层的 f64 使用 (可接受)

`crates/cli/src/` 中使用 f64 进行金额转换是**可接受的**，因为：
- 仅用于显示目的 (纳美元 -> 美元)
- 不涉及金额计算或存储

---

## 6. 重构建议与优先级

### P0 - 立即执行

1. **修复数据库 Schema 中的金额类型**
   - 将 `cost` 列从 `REAL/DOUBLE PRECISION` 改为 `BIGINT`
   - 编写数据迁移脚本

2. **拆分 `database-router` (1413 行)**
   - 按实体拆分为独立子 crate
   - 保持每个 crate < 300 行

### P1 - 短期执行

3. **创建 `service-router`**
   - 将配额扣除、使用统计逻辑上移
   - 修改 Server 层调用 Service 而非 Database

4. **拆分 `crates/router/src/lib.rs` (1607 行)**
   - 将 `billing`, `price_sync`, `notification` 移至 Service 层
   - 或拆分为独立子模块

### P2 - 中期执行

5. **拆分其他超标文件**
   - `download-aria2` (752 行)
   - `client-register` (568 行)

6. **评估是否需要 `service-download`**

---

## 7. 具体行动步骤

### 7.1 拆分 `database-router` 步骤

```bash
# 1. 创建子 crate 目录
mkdir -p crates/database/crates/database-{token,upstream,group,router-log}/src

# 2. 迁移代码
# - database-token: DbToken + 相关方法
# - database-upstream: DbUpstream + 相关方法
# - database-group: DbGroup + DbGroupMember
# - database-router-log: DbRouterLog + UsageStats

# 3. 更新聚合器
# crates/database/src/lib.rs:
#   pub use burncloud_database_token as token;
#   pub use burncloud_database_upstream as upstream;
#   ...

# 4. 更新 Cargo.toml workspace
```

### 7.2 创建 `service-router` 步骤

```bash
# 1. 创建 crate
mkdir -p crates/service/crates/service-router/src

# 2. 迁移业务逻辑
# - 从 database-router 迁移: deduct_*, get_usage_stats

# 3. 更新 Server 层调用
# - token.rs: RouterDatabase::xxx -> TokenService::xxx
# - group.rs: RouterDatabase::xxx -> GroupService::xxx

# 4. 更新依赖链
# crates/service/crates/service-router/Cargo.toml:
#   burncloud-database-token = { workspace = true }
#   burncloud-database-group = { workspace = true }
```

---

## 8. 风险评估

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| 循环依赖 | 中 | 高 | 使用 ID 引用而非实体引用 |
| 引用路径变更 | 高 | 中 | 利用 `pub use` 保持兼容 |
| 数据迁移失败 | 低 | 高 | 先备份，测试环境验证 |
| 测试覆盖不足 | 中 | 中 | 重构前添加集成测试 |

---

## 9. 结论

BurnCloud 项目当前主要存在以下技术债务：

1. **架构分层不完整**: Service 层缺失导致业务逻辑下沉到 Database 层
2. **Crate 颗粒度过粗**: 多个 lib.rs 严重超标
3. **Schema 不一致**: 金额列使用浮点数

建议按照 P0 -> P1 -> P2 优先级逐步重构，预计需要 2-3 个开发周期完成。

---

*报告生成: 2026-03-03*
