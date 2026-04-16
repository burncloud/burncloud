# burncloud-service-token

API Token 管理服务。Token CRUD、验证、配额检查和扣减。

## 关键类型

| 类型 | 说明 |
|------|------|
| `TokenService` | Token 业务逻辑(静态方法) |
| `TokenValidationResult` | 验证结果(含配额状态) |
| `DbToken` | Token 数据模型(重导出自 database 层) |

## 依赖

- `burncloud-database-token`, `burncloud-database` — 数据持久化
