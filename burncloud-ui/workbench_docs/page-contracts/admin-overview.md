---
doc_id: ui.page-contract.admin-overview
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: admin
page: overview
---

# Admin Overview Page Contract v1.0

## Purpose

Admin Overview 是 BurnCloud 的 Business + Infrastructure Command Center。它不让 Admin 逐台运维 GPU，而是告诉管理者 Supply、Capacity、Demand、Economics 是否处于健康状态，以及 Autopilot 已做/需要做什么。

## User Goal

> 我想在最短时间内知道平台今天经营如何、服务是否健康、哪里存在容量/成本风险、是否有例外需要我处理。

## Primary Question

> BurnCloud 现在是否在健康地经营和提供模型服务？

## Primary Metrics

固定四项：

```text
Today Revenue
Gross Margin
Online GPU Capacity
API Availability
```

Unknown 不得显示为 0 或 Healthy。

## Core Sections

- Supply Health
- Demand Pressure
- Capacity Risk
- Economics
- Needs Attention

首屏优先结论，不把 Raw GPU inventory、所有模型表和日志同时塞进来。

## Needs Attention

只聚合需要关注的异常或高价值建议：容量短缺、供应商大面积掉线、外租成本异常、自动化失败、API 退化等。正常状态不制造警报。

## Intelligent Copy

不要只显示 `DeepSeek Standard 93%`，应表达：

```text
Capacity risk
DeepSeek Standard is approaching its safe capacity limit.
BurnCloud is adding temporary capacity.
```

## Autopilot

目标行为：Observe → Predict → Decide → Act → Verify。低风险动作自动完成，Overview 报告结果；高财务/安全/合同风险动作才出现 Approve/Review。

## Primary Action

正常健康状态通常无需强 CTA。存在最高优先例外时，唯一 Primary CTA 指向对应 Capacity / Operations / Settlements 等页面。

## Charts

只保留能回答经营问题的 Revenue/Margin/Capacity/Demand 趋势。禁止装饰性 Gauge/Pie 堆叠。

## Intentionally Not Primary

- Per-GPU tuning
- Raw worker logs
- Manual deployment controls
- Full supplier/customer tables
- All settings

## Success Condition

Admin 10 秒内能回答：今天卖多少、毛利怎样、有多少可用容量、API 是否稳定、最大的风险是什么、系统是否已经自动处理。

## Verification Checklist

- [ ] 四个核心指标固定且语义正确
- [ ] Gross Margin 使用真实成本/收入定义
- [ ] Online GPU Capacity 不与 GPU count 混淆
- [ ] Needs Attention 只显示真实事项
- [ ] 结论优先于原始数据
- [ ] 自动恢复显示结果和影响
- [ ] 高风险动作才请求人工批准
- [ ] 不退化成服务器监控墙

## Product Gate

修改四个核心指标、把 Admin Mental Model 从 Supply→Capacity→Demand→Economics 改回逐机管理、或扩大自动化高风险权限，需要 Product Gate。

## Final Rule

Admin Overview 应让管理者看到“这家 AI 基础设施公司是否被 Autopilot 正常经营”，而不是要求管理者自己充当调度器。