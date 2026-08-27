---
doc_id: ui.page-contract.supplier-settlements
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: supplier
page: settlements
---

# Supplier Settlements Page Contract v1.0

## Purpose

Settlements 管理 Supplier 已赚收入进入结算、应付、调整和已支付流程的状态。

## User Goal

> 哪些收入已经确认、什么时候能结算、最后会收到多少钱？

## Primary Question

> 哪些收入已经可以结算？

## Core Information

- Current revenue share
- Settlement period
- Pending amount
- Eligible / payable amount
- Paid amount/history
- Adjustments / penalties / credits
- Settlement status
- Payment reference when supported

## States

建议：Accruing、Pending Review、Eligible、Processing、Paid、Adjusted、On Hold、Failed。

不得把 Earnings 的实时估算直接标成 Payable。

## Primary Action

若产品支持 Supplier 发起结算：`Request Settlement`；若平台按周期自动结算，则页面以状态和资料完善为主，不虚构按钮。

## Adjustment Transparency

任何因异常掉线、合同调整或其它规则导致的扣减，应显示原因、金额、依据时间范围和可申诉/联系渠道（若业务支持）。

## Empty State

没有可结算收入时说明当前收益仍在 Accruing / Pending，或尚未产生贡献。

## Error

支付失败不得改写为 Paid。外部支付渠道状态 Unknown 时保留上一确认状态并显示正在核对。

## Boundary

Earnings 解释收入来源；Settlements 只解释财务结算状态。Buyer Billing 与 Supplier Settlements 必须完全隔离。

## Intentionally Hidden

- Other suppliers' settlements
- Buyer balances
- Platform bank/payment secrets
- Internal finance credentials

## Success Condition

Supplier 可以对每个结算周期回答：应得多少、调整多少、当前状态、何时/是否已支付。

## Verification Checklist

- [ ] Earnings / Payable 概念分离
- [ ] Revenue Share 显示当前适用比例
- [ ] Adjustment 有原因和金额
- [ ] Paid 必须有已确认支付事实
- [ ] Pending/Unknown 不伪装 Paid
- [ ] Settlement periods 可追溯
- [ ] 不泄露其它 Supplier/Buyer 财务数据

## Product Gate

改变结算周期、Revenue Share 应用方式、自动提现/支付规则或处罚机制，需要 Product/Finance Gate。

## Final Rule

Settlements 是 Supplier 的应收账款状态页，不是收入分析页，也不能用乐观状态掩盖尚未确认的付款。