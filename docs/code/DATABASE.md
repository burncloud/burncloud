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
| 请求日志 | `router_logs` | lib.rs (database-router-log) | `RouterLog` | `RouterLogModel` | — |
| 路由内部 Token | `router_tokens` | lib.rs (database-token) | `RouterToken` | `RouterTokenModel` | `RouterTokenRepository` |
| 路由分组 | `router_groups` | lib.rs (database-group) | `RouterGroup` | `RouterGroupModel` | `RouterGroupRepository` |
| 分组成员 | `router_group_members` | lib.rs (database-group) | `RouterGroupMember` | `RouterGroupMemberModel` | — |
| 上游配置 | `router_upstreams` | lib.rs (database-upstream) | `RouterUpstream` | `RouterUpstreamModel` | `RouterUpstreamRepository` |
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
| 系统设置 | `sys_settings` | lib.rs (database-setting) | `SysSetting` | `SettingDatabase` |
| 软件安装记录 | `sys_installations` | lib.rs (database-installer) | `SysInstallation` | `InstallerDB` |
| 下载任务 | `sys_downloads` | lib.rs (database-download) | `SysDownload` | `DownloadDB` |

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
- 禁止在多个 crate 中重复定义同一张表的 CREATE TABLE。

## 新建 Database Crate 检查清单

1. Crate 名称: `database-{domain}`（如 `database-billing`）
2. 表名先在 `schema.rs` 添加 CREATE TABLE
3. .rs 文件名与表名同步（`billing_price.rs` 对应 `billing_prices` 表）
4. SQL 占位符使用统一工具函数，禁止内联 `if is_postgres`
5. `src/lib.rs` 导出三类: 行类型、操作 Model、Crate 控制器
6. 至少一个 SQLite in-memory 集成测试覆盖基础 CRUD
