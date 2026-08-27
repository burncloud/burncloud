---
doc_id: ui.page-contract.buyer-billing
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: buyer
page: billing
---

# Buyer Billing Page Contract v1.0

## Purpose

Billing 管理 Buyer 的预充值资金、余额和资金流水。第一阶段的核心模式是 Prepaid。

## User Goal

> 我想知道还有多少钱、钱去了哪里，并在余额不足前完成充值。

## Primary Question

> 我还有多少钱，可以继续使用多久，需要充值吗？

## Primary Information

- Current balance
- Low / critical balance state
- Recharge history
- Consumption statement summary
- Invoice / receipt status when supported

余额必须有币种和时间语义。未知余额不得显示成 0。

## Primary Action

正常 Billing 页：`Top Up / Recharge`。

当余额 Critical 时，该动作应升级为全页面最明显 CTA，并在 Overview 同步提示。

## Secondary Actions

- View transactions
- View statements
- Download supported invoice / receipt
- Manage supported billing profile

## Balance States

建议至少区分：Healthy、Low、Critical、Unavailable。Low/Critical 阈值应来自真实产品规则，不由前端自行猜测。

## Empty State

从未充值的新 Buyer 应明确说明需要充值后才能进行生产消费，并允许进入 Top Up；不要把余额 0 与系统错误混淆。

## Error

支付/充值状态必须区分 Pending / Succeeded / Failed / Reversed 等真实状态。不能因为页面刷新失败把 Pending 交易显示成失败或重复发起充值。

## Autopilot

系统可主动预测余额风险并通知 Buyer；第一阶段不默认自动扣外部支付方式，除非未来单独批准 Auto Top-up 产品能力。

## Boundary

Billing 不负责完整 Usage 分析，也不负责平台 Supplier Settlement。Buyer 只看自己的资金与消费关系。

## Intentionally Hidden

- Supplier revenue share
- Supplier settlement
- External GPU cost
- BurnCloud gross margin
- Other customers' balances

## Success Condition

Buyer 在余额不足前得到清晰预警，能安全充值并理解每笔资金变化，不需要通过 Usage 或客服推算余额。

## Verification Checklist

- [ ] Balance 来自真实账本/后端状态
- [ ] Unknown 不显示为 0
- [ ] Low/Critical 有明确且一致的阈值
- [ ] Critical 时 Top Up 提升为 Primary CTA
- [ ] 充值交易状态不会被误报
- [ ] 避免重复提交充值
- [ ] Billing 与 Usage 职责分离
- [ ] 不暴露 Supplier / margin 信息

## Product Gate

改变 Prepaid 模式、引入自动扣款/授信/后付费、改变余额扣减口径、或让 Buyer 参与 Supplier 结算，需要 Product/Finance Gate。

## Final Rule

Billing 永远优先让 Buyer 清楚“我的钱现在是什么状态”，而不是把资金页变成复杂的使用分析或平台财务后台。