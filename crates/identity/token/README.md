# burncloud-identity-token

Identity 域的 API Credential 能力。负责 Token 生命周期、安全策略、验证以及
per-credential spend quota 状态。

> #828 已将历史 `identity/service-token` 迁移为 `identity/token`。公开 API 与业务语义保持不变。

## Owner / Data Truth

Identity 是以下状态的唯一业务 Owner：

- `token` / `user_id` / `status`
- `expired_time` / `accessed_time`
- `key_version` / `old_key_hash` / `old_key_expires_at`
- `ip_whitelist` / `key_prefix`
- `created_at` / `last_rotated_at`
- `quota_limit` / `used_quota`

其中 quota 使用 nanodollar，但语义是 credential spend policy/state：
Commerce 负责计算请求成本，Identity 负责判断该 credential 是否还能消费以及累计已结算金额。

Traffic 的以下字段不属于 Identity：

- `order_type`
- `price_cap_nanodollars`

它们仍由 Traffic 作为 L1 routing/classifier projection 管理。

## 关键类型

| 类型 | 说明 |
|------|------|
| `TokenService` | Identity Token 对外公开业务入口 |
| `RouterToken` | 当前持久化 Token 记录；名称为历史兼容 |
| `RouterTokenModel` | Identity-owned persistence implementation |
| `RouterTokenValidationResult` | Valid / Invalid / Expired 验证结果 |
| `TokenRotationResult` | Key rotation 返回结果 |

## 依赖方向

允许：

```text
Traffic
  -> burncloud-identity-token public API
```

禁止：

```text
burncloud-identity-token
  -> traffic/database-router
```

因此 Identity 不再通过 Traffic 的 database crate 访问自己的 credential 数据。

## 数据库边界

生产 schema 仍由 Platform migrations 管理。

`TokenService::init` 只保留原有测试/兼容 bootstrap 行为，不替代 Platform migration。
#828 不修改历史 migration，也不移动 Traffic 的 routing projection SQL。

## 行为不变量

迁移必须保持：

- Token CRUD
- active / disabled / expired validation
- old-key transition validation
- accessed-time 更新
- quota check / atomic settlement
- over-cap 最后一笔仍记账的既有语义
- rotation / revoke-old
- IP whitelist
- SQLite / PostgreSQL 行为

不得通过 hidden skip、lint suppression 或修改失败预期来通过 CI。

## Domain Crate 10 问

1. **Owner 是谁？** Identity 域，API Credential（Token）能力；crate 为 `burncloud-identity-token`
   （历史 `identity/service-token`，#828 收敛为 `identity/token`）。

2. **负责什么？** `router_tokens` 的 credential 生命周期与持久化：`create` / `list` / `delete`、
   `status`（`active` / `disabled`）切换、有效性验证（`validate` / `validate_detailed`）、过期判断
   （`expired_time`）、`accessed_time` 记录、key rotation（`key_version` + old-key transition /
   `revoke_old_key`）、IP allowlist（`set_ip_whitelist` / `is_ip_allowed`）、per-credential spend quota
   预检（`check_quota`）与原子结算（`deduct_quota`，nanodollar），以及为路由消费方解析 active API key 身份
   （`active_api_key_identity` / `active_api_key_user_id`）。
   **不负责：** 请求成本计算（Commerce/Billing）、routing / classifier 投影、HTTP 展示、用户账户生命周期、
   数据库 schema / migration、`router_logs` 原始日志，以及 `order_type` / `price_cap_nanodollars`。

3. **Data Truth 是什么？** Identity-owned 的 `router_tokens` 行：`token`、`user_id`、`status`、
   `quota_limit`、`used_quota`、`expired_time`、`accessed_time`、`key_version`、`old_key_hash`、
   `old_key_expires_at`、`ip_whitelist`、`key_prefix`、`created_at`、`last_rotated_at`。其中
   `quota_limit` / `used_quota` 是 nanodollar 的 credential spend state；当 `router_tokens` 中无对应行时，
   legacy 路径读/写 `user_api_keys.remain_quota` / `used_quota`（`status = 1`），并通过 `JOIN user_accounts`
   解析 key 所有者（账户本身归 `identity/user`）。`order_type` / `price_cap_nanodollars` 不属 Identity，
   仍由 Traffic 作为 L1 routing/classifier projection 管理。生产 schema 由 Platform migrations 管理
   （`0009_router_tables.sql` 建表、`0016_token_rotation.sql` 加 rotation 列），本 crate 不改写历史 migration。

4. **公开什么契约？** `src/lib.rs` 是边界：
   - `TokenService` facade：`active_api_key_identity`、`active_api_key_user_id`、`init`、`list`、`create`、
     `delete`、`update_status`、`validate`、`validate_detailed`、`update_accessed_time`、`check_quota`、
     `deduct_quota`、`rotate`、`revoke_old_key`、`set_ip_whitelist`、`is_ip_allowed`。
   - `ActiveApiKeyIdentity { user_id, group, remain_quota, used_quota }`。
   - re-export：`RouterToken`（持久化行）、`RouterTokenValidationResult`（`Valid` / `Invalid` / `Expired`）、
     `TokenRotationResult`、`RouterTokenModel`。

   `RouterTokenModel` 是 implementation / compatibility 面，现有 Traffic 消费者仍直接使用；新代码应以
   `TokenService` 为准。该能力没有独立 contracts crate，跨域值对象即本 crate 的公开类型。

5. **可以依赖谁？** `burncloud-database`（`Database`、`adapt_sql`、`phs`、`DatabaseError`），实现库
   `sqlx`、`serde`、`rand`、`hex`、`md5`；dev-only `tokio`、`tempfile`。**不能依赖** `traffic/database-router`
   （Cargo.toml 中无此依赖边），也不依赖 Commerce / Traffic / interface 实现 crate。

6. **谁可以依赖它？** Traffic：`burncloud-router` 依赖 `burncloud-identity-token`，在 `storage` 复用
   `TokenService` 并 re-export `RouterToken` / `RouterTokenModel` / `RouterTokenValidationResult` /
   `TokenRotationResult`；以及明确的业务调用方 / interface。新增消费者需架构审查；不要因为
   `RouterTokenModel` 仍公开就绕过 `TokenService` 边界。

7. **内部如何组织？** 仅两个源文件：
   - `src/lib.rs` — `TokenService` facade、`ActiveApiKeyIdentity`、公开 re-export；`mod repository` 为私有。
   - `src/repository.rs` — 全部 SQL / row mapping 与实现类型（`RouterToken`、`RouterTokenModel`、
     `RouterTokenValidationResult`、`TokenRotationResult`），以及 `active_api_key_identity` /
     `active_api_key_user_id` / `init`。

   没有为模仿分层而设的 service / repository / factory 空层。

8. **错误 / 日志 / 安全？**
   - 错误：统一 `burncloud_database::DatabaseError`（`type Result<T>`）。`rotate` 对不存在的 credential 返回
     `DatabaseError::Query("Token not found")`（doc 注释写 `NotFound`，实现是 `Query`）。测试固化：数据库 /
     schema 损坏必须传播 `Err`，不得伪装成 `Invalid` / `None` / `false` / `true`。
   - 日志：该 crate 无 `tracing` / `log` 依赖，也没有日志语句；raw credential 不进入日志。
   - Hashing / rotation：rotation 生成新 key 后，旧 key 以**无盐 MD5 hex** 存入 `old_key_hash`
     （`format!("{:x}", md5::compute(token.as_bytes()))`）；`validate` / `validate_detailed` / `check_quota` /
     `deduct_quota` 同样以 MD5 匹配过渡期旧 key。`revoke_old = true` 时写入 `old_key_hash = NULL`、
     `old_key_expires_at = 0`，旧 key 立即失效。当前 key 明文存于 `router_tokens.token`（主键）。
   - Secret 生成：`rand::thread_rng().fill_bytes(&mut [u8; 32])` → `hex` 编码 → 前缀 `key_prefix`
     （如 `bc_live_`）；`TokenRotationResult.new_token` 文档标注 "shown only once"。
   - IP allowlist：逗号分隔**精确匹配**并 trim 两侧空白，源码明确不支持 CIDR；NULL / 空 allowlist，以及
     工作库中不存在的 credential 行，一律视为“无限制、允许”。
   - SQL：值均通过 `sqlx` `.bind()` 绑定；`active_api_key_identity` 的 `format!` 只拼接由 `db.kind()`
     决定的列名与占位符，token 仍为绑定参数。

9. **如何证明正确？** 实际执行的是两个 SQLite 集成测试文件，共 22 个 `#[tokio::test]`：
   - `tests/token_credentials.rs`（18 个）：active / disabled / expired / unknown 验证、`expired_time`
     的 -1 与 0 sentinel、quota 边界（含 `used + cost == limit`）与 unlimited（`-1`）、missing credential
     fail-closed、**over-cap 仍记账但返回 `false`**、additive 结算、credential 隔离、rotation + transition、
     `revoke_old` 立即失效、rotate unknown 返回错误、旧 alias 结算落到同一行、IP allowlist 允许 / 拒绝 / 设置。
   - `tests/token_service_entries.rs`（4 个）：13 个 service entry 在 schema 损坏（`DROP TABLE router_tokens`）
     时传播失败而不被吞成否定答案、`is_ip_allowed` / `revoke_old_key` 实测传播 `Err`、此前无测试的
     `create` / `delete` / `update_accessed_time` / `revoke_old_key` 的可见效果、delete 隔离；文件顶部含
     service-entry 对应表。

   两者都通过 `create_database_with_url("sqlite:///...")`（内部先跑 Platform versioned migrations）再加
   `TokenService::init` 建表，使用 `tempfile::NamedTempFile`；`TokenService::init` 只是兼容 / 测试 bootstrap，
   不替代 Platform migration。
   **PostgreSQL：本 crate 没有任何真实 PG 测试执行。** `db.kind() == "postgres"` 分支、`adapt_sql`、`phs`
   只在代码中按 dialect 选择；token 的测试全部跑 SQLite。既有 README 的“SQLite / PostgreSQL 行为”是迁移
   不变量，不是 PG 测试证据。

10. **正确 / 不正确调用？**

    ```text
    Correct:   Traffic -> burncloud-identity-token::TokenService -> router_tokens / user_api_keys (Identity-owned) -> platform Database
    Correct:   Caller -> TokenService::check_quota / deduct_quota (nanodollar); cost 由 Commerce/Billing 计算
    Incorrect: burncloud-identity-token -> traffic/database-router (Cargo.toml 无此依赖边)
    Incorrect: Traffic/Commerce -> 直接 SELECT/UPDATE router_tokens, 或把 order_type / price_cap_nanodollars 当作 Identity credential state
    ```

## 不拥有

- Commerce / Billing 的请求成本计算
- Traffic `router_logs` 原始日志 Data Truth
- Traffic `order_type` / `price_cap_nanodollars`（`0011_alter_router_mvp_columns.sql` 加在 `router_tokens` 上，
  但语义归 Traffic 的 L1 routing/classifier projection）
- Platform 数据库连接 / migration 基础设施
- 用户账户（`user_accounts`）生命周期；本 crate 只做只读 `JOIN` 解析 key 所有者
