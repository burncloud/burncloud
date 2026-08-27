---
doc_id: ui.page-contract.admin-revenue
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: admin
page: revenue
---

# Admin Revenue Page Contract v1.0

## Purpose

Revenue 是 BurnCloud 的经营分析页，解释客户收入、供应/外租成本和 Gross Margin 如何形成。它不替代 Supplier Settlement。

## User Goal

> 我想知道今天卖了多少、赚了多少、哪些模型/Tier 最赚钱、成本压力在哪里。

## Primary Question

> BurnCloud 今天卖了多少钱，真实毛利如何形成？

## Core Information

- Gross revenue
- Gross margin / margin rate
- Revenue by model
- Revenue by tier
- Supplier revenue allocation estimate/final status
- External GPU rental cost
- Other approved direct serving costs
- Margin trend

所有金额必须明确币种、时间范围和 Estimate/Final 语义。

## Drill-down

```text
Revenue
→ Model
→ Tier
→ Cost Source
```

需要时可链接 Supplier/Settlement，但不要在 Revenue 页承担付款操作。

## Primary Conclusions

页面优先指出：高增长高毛利、增长但毛利被外租侵蚀、低利用率/低毛利模型、异常成本变化。

## Primary Action

通常以分析为主。出现严重 Margin pressure 可 `Review Capacity` / `Review cost source`，让 Admin 从原因页处理。

## Gross Margin Truth

Gross Margin 计算必须基于批准的成本口径。缺少外租成本或 Supplier allocation 时标为 Estimated/Incomplete，不得给虚假精确毛利。

## Charts

Revenue/Margin trend、Revenue by model、External cost trend。禁止用复杂图表掩盖口径。

## Boundary

Revenue ≠ Supplier Payables。Supplier 实际应付与状态属于 Settlements；Buyer 单账户资金属于 Customers/Billing domain。

## Success Condition

Admin 能解释收入与毛利变化的主要来源，并识别 Capacity/成本策略是否侵蚀利润。

## Verification Checklist

- [ ] Revenue / Margin 口径有定义
- [ ] Estimated / Final 明确区分
- [ ] Supplier allocation / external rental cost 被正确考虑
- [ ] 币种和时间范围清楚
- [ ] Model / Tier / cost source 可下钻
- [ ] Revenue 不承担 Settlement 支付动作
- [ ] 数据不完整时不输出虚假精确毛利

## Product Gate

改变 Gross Margin 口径、成本归属、收入确认时点或自动基于毛利调整高风险策略，需要 Product/Finance Gate。

## Final Rule

Revenue 回答“平台经济性是否健康以及为什么”，Settlements 回答“具体该给 Supplier 多少钱、付到哪一步”。