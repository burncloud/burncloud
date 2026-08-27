---
doc_id: ui.page-contract.admin-capacity
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: admin
page: capacity
---

# Admin Capacity Page Contract v1.0

## Purpose

Capacity 是 BurnCloud 最核心的 Admin 页面之一，把 Raw GPU Supply 转换成“当前能向 Buyer 出售多少模型能力”的经营视图。

## User Goal

> 我想知道每个 Model/Tier 有多少可售容量、Headroom 是否安全、哪里将短缺，以及 Autopilot 怎样补足。

## Primary Question

> 当前这些 GPU 能向客户提供多少安全可售卖的模型能力？

## Mental Model

```text
Raw GPU Supply
→ Deployable Capacity
→ Active Model Capacity
→ Utilization
→ Headroom
→ Forecast Risk
```

## Core Information

- Capacity by model
- Capacity by tier
- Current utilization
- Safe headroom
- Projected shortage / time-to-risk
- Deployment readiness
- External capacity in use
- Incremental external cost
- Capacity recovery status

单位必须清楚，可根据产品采用 tokens/s、requests/s、concurrency、normalized capacity 等；不要把不同单位混成一个没有解释的“capacity score”。

## Primary Action

正常状态由 Autopilot 自动调度，无需手动按钮。遇到超出自动权限的高成本扩容时，可出现 `Approve capacity action`，并展示预期 Capacity +X、Cost +Y、Margin impact。

## Autopilot Flow

```text
Demand rises
→ Predict shortage
→ Reallocate idle supply
→ If insufficient, evaluate external rental
→ Deploy / warm model
→ Verify capacity
→ Report recovered state
```

## Risk Presentation

优先结论：`Healthy headroom`、`Capacity tightening`、`At risk`、`Shortage`，再下钻模型/Tier 数据。

## Boundary

Capacity 不等于 GPU Inventory（Supply），不等于历史请求分析（Demand），也不负责模型价格（Models）。

## Advanced

允许：deployment topology summary、benchmark-derived throughput、region breakdown、external rental scenarios、forecast confidence。默认首屏不展示所有底层 Worker。

## Success Condition

Admin 能在短时间内知道哪些 Model/Tier 有风险、何时可能不足、BurnCloud 正在做什么、是否需要批准额外成本。

## Verification Checklist

- [ ] Capacity 与 raw GPU count 分离
- [ ] Model/Tier headroom 可解释
- [ ] Forecast 有时间范围/置信语义
- [ ] External cost 与 margin impact 可见
- [ ] 低风险扩缩容默认 Autopilot
- [ ] 高风险扩容才请求批准
- [ ] Recovery 必须验证后才显示 Restored
- [ ] 不退化成 GPU inventory 表

## Product Gate

改变 safe headroom 策略、外租自动审批阈值、Capacity 核心单位或 Autopilot 财务权限，需要 Product/Finance/Operations Gate。

## Final Rule

Capacity 页管理的是“可出售的模型供给”，不是“有多少张 GPU”；它是 BurnCloud 自动供需平衡的核心驾驶舱。