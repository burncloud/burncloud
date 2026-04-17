# burncloud-database-group

分组管理持久化。管理路由分组和成员,用于负载均衡策略(round-robin / 加权)。

## 关键类型

| 类型 | 说明 |
|------|------|
| `GroupModel` | 分组 CRUD(静态方法) |
| `GroupMemberModel` | 分组成员 CRUD |
| `DbGroup` | 分组数据模型 |
| `DbGroupMember` | 分组成员(含权重) |

## 依赖

- `burncloud-database` — 核心数据库抽象
