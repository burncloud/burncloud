---
doc_id: ui.page-contract.admin-suppliers
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: admin
page: suppliers
---

# Admin Suppliers Page Contract v1.0

## Purpose

Suppliers 管理提供 GPU Capacity 的商业主体、等级、合约分成、可靠性和整体贡献。它不是 GPU inventory 页面。

## User Goal

> 我想知道哪些供应商值得依赖、贡献多少、商业条件是什么、哪里有风险。

## Primary Question

> 哪些 Supplier 是 BurnCloud 当前最重要、最可靠或最需要处理的合作方？

## Core Information

- Supplier profile / organization
- Level: Community / Verified / Professional / Strategic
- Revenue share
- Reliability summary
- Total/online resources summary
- Contribution
- Earnings / payable summary
- Settlement status
- Contract metadata
- Risk / verification status

## Primary Actions

- Review supplier
- Verify / change approved level according to process
- Manage commercial metadata / revenue share according to authority
- Place operational/financial hold when justified
- Open Supply / Reliability / Settlements details

高风险商业变更必须审计。

## Level vs Reliability vs Revenue Share

三者必须分离：

- Level = 合作/信任/规模层级
- Reliability = 运行表现
- Revenue Share = 商务协议

不能用一个“综合等级”偷偷决定所有商业结果。

## Self-service Supplier

自助加入默认低等级。升级应基于验证、稳定历史、规模和商务审批，不因为单次 Benchmark 自动成为 Strategic。

## Boundary

GPU 详情属于 Supply；逐 GPU health 只从 Supplier 详情链接；Settlement 执行属于 Settlements。

## Autopilot

系统可自动产生风险/升级建议，但商业等级和 Revenue Share 的重大变化不默认全自动执行。

## Success Condition

Admin 能从商业主体角度评估供应商价值和风险，并快速跳到真实资源、可靠性和财务依据。

## Verification Checklist

- [ ] Supplier 与 GPU inventory 分离
- [ ] Level / Reliability / Revenue Share 清楚分离
- [ ] 自助 Supplier 默认低信任等级
- [ ] 商业分成变更有权限和审计
- [ ] Supplier summary 可链接 Supply/Settlement evidence
- [ ] 不允许 Supplier 自改 Level/Share
- [ ] 风险状态有真实证据

## Product Gate

改变 Supplier Level 定义、Revenue Share 权限、自动升级/降级产生的商业后果，需要 Product/Business/Risk Gate。

## Final Rule

Suppliers 管的是“合作方”，Supply 管的是“资源”；不要把商业主体退化成一张 GPU 列表。