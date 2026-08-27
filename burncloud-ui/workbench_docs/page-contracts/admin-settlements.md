---
doc_id: ui.page-contract.admin-settlements
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: admin
page: settlements
---

# Admin Settlements Page Contract v1.0

## Purpose

Admin Settlements 计算并管理 BurnCloud 应向各 Supplier 支付的收入分成、调整和结算状态。

## User Goal

> 我想知道每个结算周期该给哪些 Supplier 分多少钱，计算是否可信，哪些异常需要处理。

## Primary Question

> 平台现在应该向哪些 Supplier 分多少钱，哪些已经处理完成？

## Calculation Model

```text
Model Revenue / Eligible Revenue
→ Actual Compute Contribution
→ Supplier Contribution Allocation
→ Supplier-specific Revenue Share
→ Adjustments / Penalties
→ Payable Amount
```

Contribution 与 Revenue Share 必须分开计算并可审计。

## Core Information

- Supplier
- Settlement period
- Contribution basis
- Revenue share (60/70/80% etc.)
- Gross allocated amount
- Adjustments / penalties
- Payable
- Status
- Payment reference / history

## States

Accruing、Pending Calculation、Pending Review、Approved/Eligible、Processing、Paid、On Hold、Failed、Adjusted。

## Primary Actions

按权限和流程提供 Review / Approve / Hold / Resolve anomaly / Mark or initiate payment only when backed by real finance action。批量操作属于 Admin 专业功能，必须有预览、总金额和确认。

## Anomaly Handling

以下优先进入 Needs Attention：Contribution mismatch、Revenue Share missing/change、large adjustment、payment failure、supplier settlement info changed、duplicate risk。

## Revenue Share

每个 Supplier 比例可独立配置，但实际修改属于 Supplier contract/business metadata，不应在结算执行时悄悄改比例。

## Autopilot

常规计算、匹配、周期封账可自动；大金额支付、异常调整、分成比例变更按风险阈值进入 Human Gate。

## Auditability

每个 Payable 必须能追溯到：周期、Contribution、适用 Revenue Share、调整、审批/支付状态。不得只显示一个无法解释的最终金额。

## Success Condition

Admin 能对任一 Supplier 的任一周期解释“为什么是这个金额”，并避免重复、错误或未经确认的支付。

## Verification Checklist

- [ ] Contribution 与 Revenue Share 分步计算
- [ ] 每个 Supplier 使用正确的合同比例
- [ ] 调整有原因、操作者/规则和审计记录
- [ ] Paid 必须对应真实确认
- [ ] 批量动作有金额预览和确认
- [ ] 重复支付风险有保护
- [ ] 结算资料变更有风险提示
- [ ] 可从 Payable 追溯原始计算依据

## Product Gate

修改分成计算顺序、自动支付权限、处罚机制、结算周期或大额审批阈值，需要 Product/Finance Gate。

## Final Rule

Settlements 必须做到“每一分钱都可解释、可追溯、不会因为自动化而失去财务控制”。