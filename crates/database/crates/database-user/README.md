# burncloud-database-user

用户和角色管理。支持双币种钱包(USD/CNY),金额用纳美元(i64)存储。

## 关键类型

| 类型 | 说明 |
|------|------|
| `UserDatabase` | 提供用户相关表的 `init(&db)` |
| `DbUser` | 用户(含 balance_usd, balance_cny 纳美元字段) |
| `DbRole` | 角色定义 |
| `DbUserRole` | 用户-角色关联 |
| `DbRecharge` | 充值记录(含币种) |

## 依赖

- `burncloud-database` — 核心数据库抽象
