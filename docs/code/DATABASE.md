# Database 开发规范

## 命名三维统一规则

每个数据库实体的三个维度必须对齐，机械可推导：

| 维度 | 格式 | 示例 |
|------|------|------|
| SQL 表名 | `{domain}_{entities}` (snake_case, 复数) | `channel_abilities` |
| .rs 文件名 | `{domain}_{entity}.rs` (snake_case, 单数) | `channel_ability.rs` |
| 类型前缀 | `{Domain}{Entity}` (PascalCase, 单数) | `ChannelAbility` |

给定任意实体，三个名字可机械推导：

```
router_video_tasks  →  router_video_task.rs  →  RouterVideoTask / RouterVideoTaskModel
billing_prices      →  billing_price.rs      →  BillingPrice / BillingPriceModel
channel_abilities   →  channel_ability.rs    →  ChannelAbility / ChannelAbilityModel
```

## 六个业务域

| 域前缀 | 业务范围 | 示例表 |
|--------|---------|--------|
| `user_` | 账户、角色、权限、充值、API Key | `user_accounts`, `user_api_keys` |
| `channel_` | AI 渠道、路由能力、协议配置 | `channel_providers`, `channel_abilities` |
| `router_` | 请求日志、内部 Token、分组、上游、视频任务 | `router_logs`, `router_tokens` |
| `billing_` | 定价、分层定价、汇率 | `billing_prices`, `billing_exchange_rates` |
| `model_` | 模型能力元数据 | `model_capabilities` |
| `sys_` | 系统设置、安装记录、下载任务 | `sys_settings`, `sys_downloads` |

## 完整实体对齐矩阵（21 实体）

### user\_ 域

| 实体 | SQL 表名 | .rs 文件名 | 行类型 | 操作类型 | 输入类型 |
|------|---------|----------|--------|---------|---------|
| 用户账户 | `user_accounts` | `user_account.rs` | `UserAccount` | `UserAccountModel` | `UserAccountInput` |
| 角色定义 | `user_roles` | `user_role.rs` | `UserRole` | — | — |
| 用户-角色绑定 | `user_role_bindings` | `user_role_binding.rs` | `UserRoleBinding` | — | — |
| 充值记录 | `user_recharges` | `user_recharge.rs` | `UserRecharge` | — | — |
| 应用层 API Key | `user_api_keys` | `user_api_key.rs` | `UserApiKey` | `UserApiKeyModel` | `UserApiKeyInput`, `UserApiKeyUpdateInput` |

### channel\_ 域

| 实体 | SQL 表名 | .rs 文件名 | 行类型 | 操作类型 | 输入类型 |
|------|---------|----------|--------|---------|---------|
| AI 上游渠道 | `channel_providers` | `channel_provider.rs` | `ChannelProvider` | `ChannelProviderModel` | — |
| 路由能力矩阵 | `channel_abilities` | `channel_ability.rs` | `ChannelAbility` | `ChannelAbilityModel` | `ChannelAbilityInput` |
| 协议适配配置 | `channel_protocol_configs` | `channel_protocol_config.rs` | `ChannelProtocolConfig` | `ChannelProtocolConfigModel` | `ChannelProtocolConfigInput` |

### router\_ 域

| 实体 | SQL 表名 | .rs 文件名 | 行类型 | 操作类型 | 仓库类型 |
|------|---------|----------|--------|---------|---------|
| 请求日志 | `router_logs` | lib.rs (router-log) | `RouterLog` | `RouterLogModel` | — |
| 路由内部 Token | `router_tokens` | lib.rs (token) | `RouterToken` | `RouterTokenModel` | `RouterTokenRepository` |
| 路由分组 | `router_groups` | lib.rs (group) | `RouterGroup` | `RouterGroupModel` | `RouterGroupRepository` |
| 分组成员 | `router_group_members` | lib.rs (group) | `RouterGroupMember` | `RouterGroupMemberModel` | — |
| 上游配置 | `router_upstreams` | lib.rs (upstream) | `RouterUpstream` | `RouterUpstreamModel` | `RouterUpstreamRepository` |
| 视频任务 | `router_video_tasks` | `router_video_task.rs` | `RouterVideoTask` | `RouterVideoTaskModel` | — |

### billing\_ 域

| 实体 | SQL 表名 | .rs 文件名 | 行类型 | 操作类型 | 输入类型 |
|------|---------|----------|--------|---------|---------|
| 模型定价 | `billing_prices` | `billing_price.rs` | `BillingPrice` | `BillingPriceModel` | `BillingPriceInput` |
| 分层定价 | `billing_tiered_prices` | `billing_tiered_price.rs` | `BillingTieredPrice` | `BillingTieredPriceModel` | `BillingTieredPriceInput` |
| 汇率 | `billing_exchange_rates` | `billing_exchange_rate.rs` | `BillingExchangeRate` | `BillingExchangeRateModel` | — |

### model\_ 域

| 实体 | SQL 表名 | .rs 文件名 | 行类型 | 操作类型 |
|------|---------|----------|--------|---------|
| 模型能力描述 | `model_capabilities` | `model_capability.rs` | `ModelCapability` | `ModelCapabilityModel` |

> `ModelInfo`（HuggingFace 风格元数据）和 `ModelDatabase`（crate 控制器）保留原名，不属于行类型，不受三维命名规则约束。

### sys\_ 域

| 实体 | SQL 表名 | .rs 文件名 | 行类型 | 控制器 |
|------|---------|----------|--------|-------|
| 系统设置 | `sys_settings` | lib.rs (setting) | `SysSetting` | `SettingDatabase` |
| 软件安装记录 | `sys_installations` | lib.rs (installer) | `SysInstallation` | `InstallerDB` |
| 下载任务 | `sys_downloads` | lib.rs (download) | `SysDownload` | `DownloadDB` |

## 类型后缀规则

| 后缀 | 用途 | 示例 |
|------|------|------|
| 无后缀 | 数据库行类型 (FromRow) | `RouterToken`, `UserAccount` |
| `Model` | 操作类型（静态方法集合） | `RouterTokenModel`, `BillingPriceModel` |
| `Repository` | CRUD 仓库（实现 CrudRepository trait） | `RouterTokenRepository` |
| `Database`/`DB` | Crate 控制器（持有连接状态） | `UserDatabase`, `DownloadDB` |
| `Input` | 创建输入类型 | `ChannelAbilityInput` |
| `UpdateInput` | 更新输入类型 | `UserApiKeyUpdateInput` |

**禁止**: 行类型使用 `Db` 前缀（历史遗留，已全部清除）。

## Schema 权威来源规则

- `crates/database/src/schema.rs` 是所有核心业务表的唯一 CREATE TABLE 来源。
- Sub-crate 的 `init()` 只能执行 `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`。
- 只有独立 crate（`setting`, `installer`, `download`）可自行 CREATE TABLE，
  并在 `schema.rs` 末尾注释中登记。
- 禁止在多个 crate 中重复定义同一张表的 CREATE TABLE。

## 新建 Database Crate 检查清单

1. Crate 名称: `database-{domain}`（如 `database-billing`）
2. 表名先在 `schema.rs` 添加 CREATE TABLE
3. .rs 文件名与表名同步（`billing_price.rs` 对应 `billing_prices` 表）
4. SQL 占位符使用统一工具函数，禁止内联 `if is_postgres`
5. `src/lib.rs` 导出三类: 行类型、操作 Model、Crate 控制器
6. 至少一个 SQLite in-memory 集成测试覆盖基础 CRUD

---

# Repository 层规范（Database Model）

> 仅在编写 `crates/database/crates/*/src/` 下的代码时需要详细阅读。

---

## 核心约束

1. **每个 `database-*` crate 只负责一个业务域**。跨域数据访问通过 Service 层组合，不在 Database 层跨 crate 调用。
2. **Model 层只做 SQL 操作，不含业务逻辑**。条件判断、规则计算一律在 Service 层。
3. **多数据库兼容：不可硬编码 SQL 方言**。用 `db.kind()` 分支或 `adapt_sql()` / `ph()` / `phs()`。
4. **行类型定义在 `burncloud_common::types`**，Model 层只负责查询，不重复定义类型。

---

## 标准 Model 结构

```rust
use burncloud_common::types::XxxRow;
use burncloud_database::{Database, Result};

pub struct XxxModel;

impl XxxModel {
    pub async fn get_by_id(db: &Database, id: i64) -> Result<Option<XxxRow>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = if is_postgres {
            "SELECT * FROM xxx_table WHERE id = $1"
        } else {
            "SELECT * FROM xxx_table WHERE id = ?"
        };

        let row = sqlx::query_as(sql)
            .bind(id)
            .fetch_optional(conn.pool())
            .await?;

        Ok(row)
    }

    pub async fn create(db: &Database, input: &XxxInput) -> Result<i64> {
        let conn = db.get_connection()?;

        let sql = if db.kind() == "postgres" {
            "INSERT INTO xxx_table (col_a, col_b) VALUES ($1, $2) RETURNING id"
        } else {
            "INSERT INTO xxx_table (col_a, col_b) VALUES (?, ?)"
        };

        // PostgreSQL 用 RETURNING，SQLite 用 last_insert_rowid()
        if db.kind() == "postgres" {
            let row: (i64,) = sqlx::query_as(sql)
                .bind(&input.col_a)
                .bind(&input.col_b)
                .fetch_one(conn.pool())
                .await?;
            Ok(row.0)
        } else {
            let result = sqlx::query(sql)
                .bind(&input.col_a)
                .bind(&input.col_b)
                .execute(conn.pool())
                .await?;
            Ok(result.last_insert_id().unwrap_or(0))
        }
    }

    pub async fn get_all(db: &Database) -> Result<Vec<XxxRow>> {
        let conn = db.get_connection()?;
        let rows = sqlx::query_as("SELECT * FROM xxx_table ORDER BY id")
            .fetch_all(conn.pool())
            .await?;
        Ok(rows)
    }

    pub async fn delete(db: &Database, id: i64) -> Result<bool> {
        let conn = db.get_connection()?;

        let sql = if db.kind() == "postgres" {
            "DELETE FROM xxx_table WHERE id = $1"
        } else {
            "DELETE FROM xxx_table WHERE id = ?"
        };

        let result = sqlx::query(sql).bind(id).execute(conn.pool()).await?;
        Ok(result.rows_affected() > 0)
    }
}
```

---

## 多数据库 SQL 兼容规则

本项目同时支持 SQLite（开发/单机）和 PostgreSQL（生产）。SQL 需兼容两种方言。

### 占位符

| 场景 | 写法 |
|------|------|
| 短查询，1-3 个参数 | 用 `ph()` / `phs()` 构建 |
| 长查询，逻辑复杂 | 用 `if db.kind() == "postgres"` 分支 |

```rust
use burncloud_database::placeholder::{ph, phs};

let is_postgres = db.kind() == "postgres";

// ph(is_postgres, position) → "$1" 或 "?"
let sql = format!("WHERE id = {}", ph(is_postgres, 1));

// phs(is_postgres, count) → "$1, $2, $3" 或 "?, ?, ?"
let placeholders = phs(is_postgres, ids.len());
let sql = format!("WHERE id IN ({})", placeholders);
```

### 保留字转义

PostgreSQL 中 `group`、`type`、`order` 是保留字，需要双引号；SQLite 用反引号：

```rust
let group_col = if db.kind() == "postgres" { "\"group\"" } else { "`group`" };
```

### INSERT 返回 ID

```rust
if db.kind() == "postgres" {
    // RETURNING id 直接返回
    let row: (i64,) = sqlx::query_as("INSERT ... RETURNING id")...
} else {
    // SQLite: execute 后取 last_insert_rowid
    let result = sqlx::query("INSERT ...").execute(conn.pool()).await?;
    Ok(result.last_insert_id().unwrap_or(0))
}
```

---

## `adapt_sql()` 辅助函数

对于参数固定、只需切换占位符风格的简单查询，可用 `adapt_sql()`：

```rust
use burncloud_database::adapt_sql;

// adapt_sql 将 ? 替换为 $1, $2... (PostgreSQL) 或保持不变 (SQLite)
let is_postgres = db.kind() == "postgres";
let sql = adapt_sql(is_postgres, "SELECT * FROM users WHERE id = ? AND status = ?");
```

---

## 事务

批量操作或多步写入必须用事务：

```rust
let mut tx = conn.pool().begin().await?;

sqlx::query("INSERT INTO ...")
    .bind(val_a)
    .execute(&mut *tx)
    .await?;

sqlx::query("UPDATE ...")
    .bind(val_b)
    .execute(&mut *tx)
    .await?;

tx.commit().await?;
```

---

## `CrudRepository` Trait（Wave 3 规划）

`burncloud_common::CrudRepository` 是未来统一接口的基础 Trait：

```rust
use burncloud_common::CrudRepository;

#[async_trait::async_trait]
impl CrudRepository<XxxRow, i64, DatabaseError> for XxxModel {
    async fn find_by_id(&self, id: &i64) -> Result<Option<XxxRow>, DatabaseError> { ... }
    async fn list(&self) -> Result<Vec<XxxRow>, DatabaseError> { ... }
    async fn create(&self, input: &XxxRow) -> Result<XxxRow, DatabaseError> { ... }
    async fn update(&self, id: &i64, input: &XxxRow) -> Result<bool, DatabaseError> { ... }
    async fn delete(&self, id: &i64) -> Result<bool, DatabaseError> { ... }
}
```

现阶段（Wave 1-2）静态方法即可，新 crate 可选择实现 Trait。

---

## 反例

```rust
// ✗ 禁止：硬编码 PostgreSQL 方言
let sql = "SELECT * FROM users WHERE id = $1";

// ✗ 禁止：硬编码 SQLite 方言
let sql = "SELECT * FROM users WHERE id = ?";

// ✗ 禁止：跨 database-* crate 直接调用
use burncloud_database_user::UserAccountModel;
// （只能在 Service 层通过 Service 组合）

// ✗ 禁止：Model 层包含业务逻辑
pub async fn find_active_premium_users(db: &Database) -> Result<Vec<UserAccount>> {
    // 这是业务筛选，应在 Service 层
    if user.subscription == "premium" && user.balance > 0 { ... }
}
```
