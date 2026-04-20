# burncloud-database-router

`router_` 业务域数据库 crate。管理路由相关表：`router_upstreams`, `router_tokens`,
`router_groups`, `router_group_members`, `router_logs`, `router_video_tasks`。

## 关键类型

| 类型 | 说明 |
|------|------|
| `RouterDatabase`     | 聚合器，`init(&db)` 建表 + 跨模块委托 |
| `RouterUpstream` / `RouterUpstreamModel` | 上游配置 |
| `RouterToken` / `RouterTokenModel` | 路由内部 Token |
| `RouterGroup` / `RouterGroupMember` / `RouterGroupModel` / `RouterGroupMemberModel` | 分组与成员 |
| `RouterLog` / `RouterLogModel` | 路由日志 |
| `RouterVideoTask` / `RouterVideoTaskModel` | 视频任务映射 |
| `BalanceModel` | 双币种余额扣费操作（操作 user_accounts 表） |

## 目录结构

```
src/
├── lib.rs               — 聚合器 + re-exports
├── upstream.rs          — RouterUpstream, RouterUpstreamModel, RouterUpstreamRepository
├── token.rs             — RouterToken, RouterTokenModel, RouterTokenRepository
├── group.rs             — RouterGroup(Member)(Model|Repository)
├── log.rs               — RouterLog, RouterLogModel, BalanceModel, usage stats
└── router_video_task.rs — RouterVideoTask, RouterVideoTaskModel
```

## 依赖

- `burncloud-database`, `burncloud-common` — 核心抽象和共享类型
