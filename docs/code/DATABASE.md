# Database 开发规范

## 命名三维统一规则

每个数据库实体的三个维度必须对齐，机械可推导：

| 维度 | 格式 | 示例 |
|------|------|------|
| SQL 表名 | `{domain}_{entities}` (snake_case, 复数) | `channel_abilities` |
| .rs 文件名 | `{domain}_{entity}.rs` (snake_case, 单数) | `channel_ability.rs` |
| 类型前缀 | `{Domain}{Entity}` (PascalCase, 单数) | `ChannelAbility` |

## 六个业务域

| 域前缀 | 业务范围 | 示例表 |
|--------|---------|--------|
| `user_` | 账户、角色、权限、充值、API Key | `user_accounts`, `user_api_keys` |
| `channel_` | AI渠道、路由能力、协议配置 | `channel_providers`, `channel_abilities` |
| `router_` | 请求日志、内部Token、分组、上游、视频任务 | `router_logs`, `router_tokens` |
| `billing_` | 定价、分层定价、汇率 | `billing_prices`, `billing_exchange_rates` |
| `model_` | 模型能力元数据 | `model_capabilities` |
| `sys_` | 系统设置、安装记录、下载任务 | `sys_settings`, `sys_downloads` |

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
- 只有独立 crate（`database-setting`, `database-installer`, `database-download`）可自行 CREATE TABLE，
  并在 `schema.rs` 末尾注释中登记。

## 新建 Database Crate 检查清单

1. Crate 名称: `database-{domain}`（如 `database-billing`）
2. 表名先在 `schema.rs` 添加 CREATE TABLE
3. .rs 文件名与表名同步（`billing_price.rs` 对应 `billing_prices` 表）
4. SQL 占位符使用统一工具函数，禁止内联 `if is_postgres`
5. `src/lib.rs` 导出三类: 行类型、操作 Model、Crate 控制器
6. 至少一个 SQLite in-memory 集成测试覆盖基础 CRUD
