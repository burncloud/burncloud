---
doc_id: ui.page-contract.admin-demand
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: admin
page: demand
---

# Admin Demand Page Contract v1.0

## Purpose

Demand 解释 Buyer 正在购买和消耗什么模型能力，并为 Capacity Autopilot 提供需求趋势和预测视角。

## User Goal

> 我想知道哪些模型/Tier 需求上升、哪些下降、未来哪里会产生容量压力。

## Primary Question

> 客户现在真正需要什么，接下来可能需要什么？

## Core Information

- Requests
- Tokens
- Spend / revenue demand signal
- Model demand
- Tier demand
- Growth rate
- Time-of-day patterns
- Region when useful
- Forecast
- Forecast confidence / horizon

## Primary Conclusions

页面应主动回答：

- 哪些 Model/Tier demand 正在上涨？
- 哪些需求正在下降？
- 哪些容量可能闲置？
- 哪些需求即将超过 safe headroom？

## Primary Action

Demand 以分析/预测为主。发生明确 Capacity risk 时，CTA `Review Capacity`；不要在此页直接手工租 GPU。

## Charts

趋势图必须有时间轴和明确单位。允许 Line/Area/Bar；不要用大量 Pie/Gauge 代替趋势判断。

## Forecast Truth

预测必须与历史事实分开：Actual / Forecast 使用明确视觉和文字语义。低置信度不能当作确定事实。

## Autopilot

Demand signal 自动进入 Capacity Planner。Admin 看到预测与系统响应，而不是人工把图表读完后再逐机部署。

## Boundary

Demand 不负责：GPU inventory（Supply）、当前可售容量（Capacity）、Buyer 账户管理（Customers）、价格配置（Models）。

## Success Condition

Admin 能从需求变化提前理解容量压力，并跳到 Capacity 查看系统如何处理，而不是等服务出问题才发现需求上涨。

## Verification Checklist

- [ ] Actual / Forecast 明确区分
- [ ] Model / Tier demand 可下钻
- [ ] Growth 和时间范围语义完整
- [ ] Forecast 有 horizon/confidence
- [ ] 容量风险链接 Capacity
- [ ] 不在 Demand 页提供逐机调度
- [ ] 图表回答明确业务问题

## Product Gate

改变需求预测的核心业务用途、用预测直接触发高财务风险动作、或把 Demand 与 Customer profiling 混合，需要 Product/Risk Gate。

## Final Rule

Demand 的价值是提前告诉 BurnCloud“接下来客户会需要什么”，让 Autopilot 先于故障准备 Capacity。