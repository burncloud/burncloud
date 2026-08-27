---
doc_id: ui.page-contract.supplier-earnings
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: supplier
page: earnings
---

# Supplier Earnings Page Contract v1.0

## Purpose

Earnings 解释 Supplier 的收入是如何由实际 Compute Contribution 形成的。它回答“赚了多少、为什么”，不负责“哪笔钱已经结算”。

## User Goal

> 我想知道自己赚了多少钱，以及哪些模型、Cluster、GPU 对收入贡献最大。

## Primary Question

> 我的收入从哪里来？

## Core Information

- Today earnings
- Period earnings
- Supplier revenue share
- Compute contribution
- Model contribution
- Resource contribution
- Adjustments that affect earned amount when applicable

## Drill-down

```text
Earnings
→ Model
→ Usage / Contribution
→ Cluster
→ GPU
```

允许从 GPU 回看相关健康/Deployment，但不要复制 Resources 页面。

## Revenue Model

所有 Supplier 可以共同组成 Compute Pool。先计算真实 Contribution，再应用该 Supplier 独立 Revenue Share（如 60% / 70% / 80%）。Contribution 与 Revenue Share 必须作为两个概念展示。

## Primary Action

页面主要用于理解，通常无强操作 CTA。可提供 `View settlement`、`Inspect contribution` 等上下文动作。

## Charts

只展示能回答收益变化原因的趋势：Earnings trend、Contribution trend、Earnings by model 等。避免装饰性仪表盘。

## Empty State

有资源但尚无贡献时解释原因：未通过 Benchmark、尚未被调度、没有有效工作负载等。不要简单显示 $0。

## Error / Unknown

Contribution 计算延迟、财务周期未封账等情况必须标为 Pending / Estimated / Unavailable，不能把估算值冒充最终结算值。

## Intentionally Hidden

- Other suppliers' revenue share / earnings
- BurnCloud platform margin unless explicitly approved
- Buyer private details
- Global scheduling formula internals

## Success Condition

Supplier 能从收入总额一直解释到 Model/Cluster/GPU 贡献，并理解自己的 Revenue Share 如何作用于收益。

## Verification Checklist

- [ ] Earnings 与全平台 Revenue 区分
- [ ] Contribution 与 Revenue Share 分开显示/计算
- [ ] 支持 Model → Cluster → GPU 下钻
- [ ] Estimated / final 状态区分
- [ ] 无贡献时解释原因
- [ ] 不把未结算 Earnings 显示成已支付
- [ ] 不泄露其它 Supplier 商业数据

## Product Gate

改变 Contribution 口径、Revenue Share 计算顺序、向 Supplier 暴露其它 Supplier 数据或平台毛利，需要 Product/Finance Gate。

## Final Rule

Earnings 解释“价值是怎样产生的”；Settlements 才解释“哪些价值已经进入付款流程”。