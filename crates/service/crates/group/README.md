# burncloud-service-group

分组管理服务。管理路由分组和成员,用于负载均衡策略。

## 关键类型

| 类型 | 说明 |
|------|------|
| `GroupService` | 分组 CRUD(静态方法) |
| `GroupMemberService` | 分组成员管理 |
| `DbGroup` / `DbGroupMember` | 数据模型(重导出自 database 层) |

## 依赖

- `burncloud-database-router`, `burncloud-database` — 数据持久化
