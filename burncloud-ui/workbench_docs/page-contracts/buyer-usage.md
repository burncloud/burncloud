---
doc_id: ui.page-contract.buyer-usage
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: buyer
page: usage
---

# Buyer Usage Page Contract v1.0

## Purpose

Usage 是 Buyer 的模型 API 使用分析页，回答“资源消费发生在哪里”，不承担充值和单次请求深度 Debug。

## User Goal

> 我想知道自己的请求、Token 和费用主要花在哪些模型、Tier 和 API Key 上。

## Primary Question

> 我的 API 到底用在了哪里？

## Core Information

- Tokens
- Requests
- Spend
- Latency
- Success rate

默认允许时间范围选择，并按 Model / Tier / API Key 下钻。

推荐路径：

```text
Total Usage
→ Model
→ Tier
→ API Key
→ Time
```

## Primary Action

Usage 以理解为主，通常不需要强操作型 CTA。若发现异常费用，可提供 `View Logs` 或 `Review API Keys` 作为上下文动作，但不能抢占页面主结构。

## Filters

普通 Buyer 默认只提供 Search / Filter / Sort / Time range 中必要项。避免默认提供 Admin 式列管理和批量动作。

## Charts

只使用能回答问题的趋势图，例如 Spend trend、Token trend、Success rate trend。优先 Line / Area / Bar / Sparkline，不使用装饰性 Pie/Gauge。

## Empty State

新账户无使用量时说明尚未产生 API Traffic，并引导 Playground 或 Marketplace；不要展示一页无意义的 0 图表。

## Partial Failure

某个维度统计失败时保留其它已确认数据，并标识 Unknown / unavailable。不得把缺失统计显示为 0。

## Boundary

Usage 不负责：

- Recharge / balance management → Billing
- Request-level debugging → Logs
- Model purchasing → Marketplace
- GPU / Supplier performance analysis → Admin/Supplier domains

## Intentionally Hidden

GPU、Supplier identity、IDC、internal route IDs、supplier cost/revenue share、平台毛利。

## Success Condition

Buyer 能快速解释“本期为什么花了这些钱”，并从总量下钻到 Model/Tier/API Key，而无需导出数据自行拼接。

## Verification Checklist

- [ ] Tokens / Requests / Spend / Latency / Success rate 语义一致
- [ ] 支持合理时间范围
- [ ] Model / Tier / API Key 可下钻
- [ ] Missing 数据不显示为 0
- [ ] 无数据时不渲染无意义图表
- [ ] 不复制 Billing 和 Logs 的完整职责
- [ ] 图表每张都回答明确问题
- [ ] 不暴露内部供应链信息

## Product Gate

修改 Usage 的核心计量口径、把平台成本/供应商收益暴露给 Buyer、或将 Usage 与 Billing 合并，需要 Product Gate。

## Final Rule

Usage 回答“我是怎么用掉这些 API 资源的”，Billing 回答“我的钱发生了什么”，Logs 回答“哪次请求出了什么问题”。