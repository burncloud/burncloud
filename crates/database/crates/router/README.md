# burncloud-database-router

路由数据库聚合器。初始化所有路由相关表(upstream, token, group, router_log),聚合 4 个子 crate 的操作。

## 关键类型

| 类型 | 说明 |
|------|------|
| `RouterDatabase` | 聚合器,提供 `init(&db)` 建表 |
| `DbUpstream` | 上游/Channel 配置 |
| `DbToken` | API Token |
| `DbGroup` / `DbGroupMember` | 分组和成员 |
| `DbRouterLog` | 路由日志 |

## 依赖

- `burncloud-database` — 核心数据库抽象
- `burncloud-database-upstream`, `burncloud-database-token`, `burncloud-database-group`, `burncloud-database-router-log` — 子 crate

## 注意

这是聚合器 crate,职责是统一 init。具体 CRUD 由各子 crate 提供。
