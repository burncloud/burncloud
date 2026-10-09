# burncloud-database-router

`router_` 业务域数据库 crate。管理 Traffic 路由投影、日志和视频任务持久化。\n`router_tokens` 的 credential 生命周期与额度状态由 Identity `service-token` 拥有；\n本 crate 仅保留兼容导出以及 Traffic 的 `order_type` / `price_cap` 投影读取。

## 关键类型

| 类型 | 说明 |
|------|------|
| `RouterDatabase`     | 聚合器，`init(&db)` 建表 + 跨模块委托 |
| `RouterUpstream` / `RouterUpstreamModel` | 上游配置 |
| `RouterToken` / `RouterTokenModel` | 兼容导出；真实实现归 Identity `service-token` |
| `RouterGroup` / `RouterGroupMember` / `RouterGroupModel` / `RouterGroupMemberModel` | 分组与成员 |
| `RouterLog` / `RouterLogModel` | 路由日志 |
| `RouterVideoTask` / `RouterVideoTaskModel` | 视频任务映射 |
| `BalanceModel` | 双币种余额扣费操作（操作 user_accounts 表） |

## 目录结构

```
src/
├── lib.rs               — 聚合器 + re-exports
├── upstream.rs          — RouterUpstream, RouterUpstreamModel, RouterUpstreamRepository
├── (token compatibility exports are inline in lib.rs; implementation lives in identity/service-token)\n├── group.rs             — RouterGroup(Member)(Model|Repository)
├── log.rs               — RouterLog, RouterLogModel, BalanceModel, usage stats
└── router_video_task.rs — RouterVideoTask, RouterVideoTaskModel
```

## 依赖

- `burncloud-database` — 数据库基础设施\n- `burncloud-identity-token` — Identity-owned credential public API
