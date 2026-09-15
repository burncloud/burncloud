# burncloud-service-billing

多模态计费服务。管理 Token 计数、价格缓存和费用计算,支持标准/缓存/批量/优先级/音频多种计费模式。

## 关键类型

| 类型 | 说明 |
|------|------|
| `PriceCache` | 内存价格缓存,通过 `load(&db)` 初始化 |
| `CostCalculator` | 费用计算器,支持 preflight 检查和多币种结算 |
| `UnifiedTokenCounter` | 线程安全流式 Token 计数 |
| `UnifiedUsage` | 统一的 Token 使用量结构 |
| `CostBreakdown` | 费用明细 |

## 依赖

- `burncloud-database`, `burncloud-database-model` — 价格数据
- `burncloud-common` — 共享类型
