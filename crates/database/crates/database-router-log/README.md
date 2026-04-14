# burncloud-database-router-log

路由日志持久化。记录请求日志、用量统计和余额扣减,支持详细的 Token/Cost 追踪。

## 关键类型

| 类型 | 说明 |
|------|------|
| `RouterLogModel` | 日志 CRUD(静态方法) |
| `DbRouterLog` | 路由日志记录(含 Token 明细和费用明细) |
| `BalanceModel` | 余额操作(USD/CNY 双币种) |
| `BillingSummary` | 账单汇总 |
| `UsageStats` / `ModelUsageStats` | 用量统计 |

## 依赖

- `burncloud-database` — 核心数据库抽象
