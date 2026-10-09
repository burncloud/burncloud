# burncloud-service-token

Identity 域的 API Credential 能力。负责 Token 生命周期、安全策略、验证以及
per-credential spend quota 状态。

> 当前 crate 名仍保留历史 `service-token`。是否进一步收敛为 `identity/token`
> 属于 #734 Phase F，不在 #811 中做机械重命名。

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
| `RouterTokenRepository` | CRUD compatibility repository |
| `RouterTokenValidationResult` | Valid / Invalid / Expired 验证结果 |
| `TokenRotationResult` | Key rotation 返回结果 |

## 依赖方向

允许：

```text
Traffic
  -> burncloud-service-token public API
```

禁止：

```text
burncloud-service-token
  -> traffic/database-router
```

因此 Identity 不再通过 Traffic 的 database crate 访问自己的 credential 数据。

## 数据库边界

生产 schema 仍由 Platform migrations 管理。

`TokenService::init` 只保留原有测试/兼容 bootstrap 行为，不替代 Platform migration。
#811 不修改历史 migration，也不移动 Traffic 的 routing projection SQL。

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
