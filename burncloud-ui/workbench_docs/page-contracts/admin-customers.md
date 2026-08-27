---
doc_id: ui.page-contract.admin-customers
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: admin
page: customers
---

# Admin Customers Page Contract v1.0

## Purpose

Customers 管理购买 BurnCloud Model API Capacity 的 Buyer 商业账户与账户状态，不把请求日志或底层基础设施详情塞进客户页。

## User Goal

> 我想知道谁在购买 BurnCloud、消费和余额情况怎样、哪些账户存在业务或风险问题。

## Primary Question

> 哪些 Customer 是重要客户，哪些账户现在需要运营或风险处理？

## Core Information

- Customer / organization profile
- Account status
- Balance
- Spend
- Usage summary
- API/service status
- Recent billing state
- Risk / abuse state
- Created / activity history

金额必须带币种；余额和消费口径不得混淆。

## Primary Actions

按权限：Review account、funding/balance action（若业务流程允许）、status management、open Usage/Logs/Billing evidence。涉及资金或账户停用必须明确确认和审计。

## Drill-down

Customer detail 可以汇总 Usage/Billing/Keys 状态，但深度请求诊断跳 Logs，完整 Usage 分析跳对应分析域，不复制所有功能。

## Risk

Risk/abuse 结论必须基于真实规则/证据。Unknown 不得默认 Safe；禁止展示伪造的“Security Score”。

## Boundary

Customers 不负责：Supplier、GPU、Deployment、Capacity management、request-level raw logs。

## Privacy

Buyer 数据仅向授权 Admin 显示，避免把请求内容、Secret 和不必要个人信息默认铺在列表。

## Success Condition

Admin 能从账户角度理解客户价值、余额/消费和风险，并能进入正确的 Billing/Usage/Logs 流程处理问题。

## Verification Checklist

- [ ] Balance / Spend / Usage 语义分离
- [ ] 币种清楚
- [ ] 资金与停用动作有确认/审计
- [ ] Risk 有真实证据，Unknown 不伪装 Safe
- [ ] 不在列表泄露 Secret/请求正文
- [ ] 深度 Debug 跳 Logs
- [ ] 不出现 GPU/Supplier infrastructure

## Product Gate

改变 Buyer 资金模型、账户停用规则、风险自动处置权限或跨客户数据可见性，需要 Product/Finance/Security Gate。

## Final Rule

Customers 管的是“谁在买、账户是否健康”，不是把客户详情做成 Billing + Logs + Infrastructure 的大杂烩。