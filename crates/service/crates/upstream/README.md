# burncloud-service-upstream

上游管理服务。管理 API 上游/Channel 配置的 CRUD。

## 关键类型

| 类型 | 说明 |
|------|------|
| `UpstreamService` | 上游业务逻辑(静态方法) |
| `DbUpstream` | 上游数据模型(重导出自 database 层) |

## 依赖

- `burncloud-database-router`, `burncloud-database` — 数据持久化
