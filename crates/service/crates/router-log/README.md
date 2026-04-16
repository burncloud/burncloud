# burncloud-service-router-log

路由日志服务。管理请求日志、用量统计和双币种余额扣减。

## 关键类型

| 类型 | 说明 |
|------|------|
| `RouterLogService` | 路由日志业务逻辑 |
| `UsageStatsService` | 用量统计聚合 |
| `BalanceService` | 余额操作(USD/CNY 双币种) |
| `DbRouterLog` | 路由日志数据模型 |
| `UsageStats` | 聚合用量统计 |

## 依赖

- `burncloud-database-router-log`, `burncloud-database` — 数据持久化
