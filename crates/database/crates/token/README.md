# burncloud-database-token

API Token 持久化。Token 验证、配额追踪和过期管理。

## 关键类型

| 类型 | 说明 |
|------|------|
| `TokenModel` | Token CRUD(静态方法) |
| `DbToken` | Token 数据模型(含 quota, used_quota 纳美元字段) |
| `TokenValidationResult` | 验证结果(含配额状态) |

## 依赖

- `burncloud-database` — 核心数据库抽象
