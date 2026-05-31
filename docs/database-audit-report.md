# BurnCloud 数据库全面检查报告

**检查时间**: 2026-05-31
**检查范围**: 数据库迁移文件、表结构、数据模型一致性

---

## 1. 数据库概览

### 1.1 表统计

| 分类 | 数量 | 表名 |
|------|------|------|
| `channel_*` | 3 | channel_abilities, channel_protocol_configs, channel_providers |
| `user_*` | 5 | user_accounts, user_api_keys, user_recharges, user_role_bindings, user_roles |
| `router_*` | 6 | router_group_members, router_groups, router_logs, router_tokens, router_upstreams, router_video_tasks |
| `billing_*` | 5 | billing_exchange_rates, billing_plans, billing_prices, billing_subscriptions, billing_tiered_prices |
| `sys_*` | 3 | sys_downloads, sys_installations, sys_settings |
| `model_*` | 1 | model_capabilities |
| **总计** | **23** | |

### 1.2 迁移文件

- SQLite: 17 个迁移文件 (0001-0016，有重复)
- PostgreSQL: 16 个迁移文件 (0001-0016)

---

## 2. 发现的问题

### 🔴 P0: 迁移编号重复

**SQLite 有两个 `0013_` 开头的迁移文件**：
- `0013_alter_router_logs_add_cost_status.sql` (May 5)
- `0013_fix_bool_columns.sql` (May 10)

**影响**：迁移执行顺序不确定，可能导致数据不一致。

**修复方案**：
```
0013_alter_router_logs_add_cost_status.sql → 保持为 0013
0013_fix_bool_columns.sql → 重命名为 0017_fix_bool_columns.sql
```

---

### 🔴 P0: 旧表定义未清理

`0001_initial_schema.sql` 仍创建以下旧表名：
- `users` → 应使用 `user_accounts`
- `tokens` → 应使用 `user_api_keys`
- `channels` → 应使用 `channel_providers`
- `abilities` → 应使用 `channel_abilities`
- `prices` → 应使用 `billing_prices`
- `protocol_configs` → 应使用 `channel_protocol_configs`
- `tiered_pricing` → 应使用 `billing_tiered_prices`
- `exchange_rates` → 应使用 `billing_exchange_rates`
- `video_tasks` → 应使用 `router_video_tasks`

**影响**：
1. 新安装时同时创建新旧两套表
2. `rename.rs` 的重命名逻辑可能与新表冲突
3. 数据可能被写入错误的表

**根本原因**：`0001_initial_schema.sql` 是最早创建的，`0010_rename_tables.sql` 后来添加了重命名，但没有清理 `0001` 中的旧定义。

**修复方案**：
- 方案 A：在 `0001_initial_schema.sql` 中直接使用新表名
- 方案 B：添加迁移检查，如果新表已存在则跳过创建旧表

---

### 🟡 P1: router_groups 与 channel_abilities.group 功能重复

`0009_router_tables.sql` 创建了：
- `router_groups` (id, name, strategy, match_path)
- `router_group_members` (group_id, upstream_id, weight)

**冲突**：
- `channel_abilities.group` 字段已实现分组功能
- `model_router.rs` 基于 `channel_abilities` 做路由决策
- `router_groups.match_path` 与 `model_router.model` 是两种不同的路由策略

**影响**：Issue #280 的 Groups 功能与现有架构冲突。

**建议**：
1. 如果需要基于路径的路由，应扩展现有 `channel_abilities` 或 `router_upstreams` 表
2. 或者明确两套路由策略如何协调

---

### 🟡 P1: router_tokens 与 user_api_keys 概念重复

两个表都存储 API Token：
- `user_api_keys` - 用户 API Key（user 侧）
- `router_tokens` - Router Token（router 侧）

**问题**：
- 功能重叠
- 用户可能困惑该使用哪个

**建议**：统一为一个 Token 表，或明确两个表的职责分工。

---

### 🟡 P2: 迁移文件与 crate 定义分散

表定义存在于多处：
1. `migrations/sqlite/*.sql` - DDL 迁移
2. `migrations/postgres/*.sql` - DDL 迁移
3. `crates/database/crates/*/src/*.rs` - Model 定义
4. `crates/database/src/schema/` - 数据迁移逻辑

**问题**：分散定义容易导致不一致。

**建议**：
- DDL 只在迁移文件中定义
- Model 层只做 CRUD 操作，不定义表结构

---

## 3. 表关系图

```
user_accounts
    ├── user_api_keys (1:N)
    ├── user_recharges (1:N)
    └── user_role_bindings (M:N) ── user_roles

channel_providers
    └── channel_abilities (1:N) [group, model, channel_id]

router_upstreams
    └── router_group_members (M:N) ── router_groups

billing_prices
    └── billing_tiered_prices (1:N)
```

---

## 4. 修复建议

### 4.1 立即修复 (P0)

1. **修复迁移编号重复**：
   ```bash
   # 重命名 SQLite 迁移文件
   mv 0013_fix_bool_columns.sql 0017_fix_bool_columns.sql
   
   # 更新 PostgreSQL 对应文件
   # 如果 PostgreSQL 也有同样问题
   ```

2. **清理旧表定义**：
   - 修改 `0001_initial_schema.sql`，移除旧表定义
   - 或者添加条件判断：如果新表已存在则跳过

### 4.2 架构决策 (P1)

1. **router_groups 问题**：
   - 决策：保留还是移除？
   - 如果保留：如何与 `channel_abilities.group` 协调？
   - 建议：移除 `router_groups`，扩展现有 `channel_abilities`

2. **router_tokens 问题**：
   - 决策：与 `user_api_keys` 合并还是保留？
   - 如果保留：明确职责分工文档

### 4.3 长期改进 (P2)

1. **统一表命名规范**：
   - `{domain}_{entities}` 格式（已基本实现）
   
2. **集中化表定义管理**：
   - 所有 DDL 只在迁移文件中
   - Model 层不做表结构定义

---

## 5. 数据库 Crate 检查

| Crate | 负责表 | 状态 |
|-------|--------|------|
| `billing` | billing_prices, billing_tiered_prices, billing_plans, billing_subscriptions, billing_exchange_rates | ✅ 正常 |
| `channel` | channel_providers, channel_abilities, channel_protocol_configs | ✅ 正常 |
| `model` | model_capabilities | ✅ 正常 |
| `router` | router_logs, router_tokens, router_upstreams, router_video_tasks, router_groups?, router_group_members? | ⚠️ 有冲突 |
| `sys` | sys_settings, sys_downloads, sys_installations | ✅ 正常 |
| `user` | user_accounts, user_api_keys, user_recharges, user_roles, user_role_bindings | ✅ 正常 |

---

## 6. 总结

BurnCloud 数据库整体结构合理，采用了良好的命名规范（`{domain}_{entities}`），但存在以下问题需要解决：

| 优先级 | 问题 | 影响 |
|--------|------|------|
| 🔴 P0 | 迁移编号重复 | 数据迁移可能失败 |
| 🔴 P0 | 旧表定义未清理 | 新安装创建重复表 |
| 🟡 P1 | router_groups 与 channel_abilities 冲突 | 功能重复、路由混乱 |
| 🟡 P1 | router_tokens 与 user_api_keys 重叠 | 概念混乱 |
| 🟢 P2 | 定义分散 | 维护困难 |

建议优先修复 P0 问题，然后对 P1 问题做出架构决策。
