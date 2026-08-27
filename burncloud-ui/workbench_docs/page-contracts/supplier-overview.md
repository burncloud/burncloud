---
doc_id: ui.page-contract.supplier-overview
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: supplier
page: overview
---

# Supplier Overview Page Contract v1.0

## Purpose

Supplier Overview 让 GPU 供应商快速知道机器是否正常、今天赚了多少，以及是否有必须处理的问题。它也是尚未接入资源的新 Supplier 的 onboarding 首页。

## User Goal

> 我的 GPU 正常赚钱吗？有没有需要我处理的问题？

## Primary Question

> 我的机器正常吗？今天赚了多少钱？

## Primary Metrics

固定优先：

```text
Today Earnings
Online GPUs
GPU Utilization
Inference Today
```

首页显示结论，不默认展示大量 GPU 参数。

## Main Sections

- Needs Attention
- Revenue Trend
- Resource Health

没有异常时不要强行制造 Needs Attention 卡片。

## New Supplier State

没有已接入资源时，Overview 转成极简 onboarding：

```text
Start earning with your GPUs
1. Install BurnCloud Node
2. Connect machine
3. Run benchmark
4. Start earning
```

Primary CTA：`Add Resources / Install BurnCloud Node`。

## Active Supplier Primary Action

正常状态以观察为主；若存在资源异常，Primary CTA 根据最高优先问题变为 `Review affected resources`。不要同时放多个强 CTA。

## Reliability Summary

首页只显示简化评级与影响，例如 `Good` / `Needs Attention`。完整原因进入 Reliability。

## Earnings Summary

展示 Supplier 实际可理解收益，不把 gross model revenue 与 Supplier earnings 混淆。Revenue Share 可以显示，但细分计算进入 Earnings/Settlements。

## Autopilot

模型选择、部署、切换、流量分配由 BurnCloud 自动完成。Overview 应报告结果，例如：`BurnCloud moved workload away from 2 unstable GPUs`，而不是要求 Supplier 手工迁移。

## Intentionally Hidden / Not Primary

- Buyer identity/details unless explicitly needed and approved
- Other suppliers
- BurnCloud margin
- Global scheduler internals
- Manual model deployment controls
- Manual traffic routing

## Success Condition

Supplier 5 秒内能回答：今天赚多少、多少 GPU 在线、利用率怎样、是否有异常需要自己处理。

## Verification Checklist

- [ ] 四个核心指标语义正确
- [ ] 新 Supplier 自动进入 onboarding
- [ ] Earnings 不与全平台 Revenue 混淆
- [ ] Needs Attention 只有真实异常时出现
- [ ] Reliability 使用简化等级
- [ ] 不提供模型部署/路由控制
- [ ] Unknown 不显示为 0/Healthy
- [ ] 异常 CTA 指向 Resources/Reliability 等正确页面

## Product Gate

修改四个指标、让 Supplier 直接控制 Deployment/Traffic、暴露平台毛利或其它 Supplier 数据，需要 Product Gate。

## Final Rule

Supplier Overview 应让供应商感觉“接上 GPU、保持健康、持续赚钱”，而不是让他成为 BurnCloud 的模型运维工程师。