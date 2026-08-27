---
doc_id: ui.page-contract.admin-operations
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: admin
page: operations
---

# Admin Operations Page Contract v1.0

## Purpose

Operations 是 BurnCloud 的异常与人工介入中心，聚合 Autopilot 无法安全自动完成、需要人判断或需要跨域协调的事件。它不是所有后台功能的垃圾桶。

## User Goal

> 我想只看真正需要人处理的例外，并快速理解影响、系统已经做了什么、我应该做什么。

## Primary Question

> 哪些异常现在需要人介入？

## Incident Types

- Failed automation
- Capacity incidents
- Deployment failures
- Supplier outages
- Billing anomalies
- Settlement anomalies
- Security / abuse incidents
- External rental failures
- Data/control-plane degradation

## Core Information

每个事件默认展示：Severity、Status、Affected domain、Impact、Detected time、Autopilot actions already taken、Required human action（若有）、Owner/assignee when supported。

## Primary Action

针对最高风险事件提供明确 `Review / Approve / Resolve`，而不是通用“Fix”按钮。动作必须跳到真实所属领域或受控处置流程。

## Autopilot States

建议：Detected、Mitigating、Recovered Automatically、Needs Review、Waiting Approval、Resolved、Failed Mitigation。

如果已经自动恢复，默认不要求 Admin 再点一次“确认修复”；只保留事件记录和必要复盘。

## Human by Exception

只有以下类型默认进入人工：高财务影响、高安全风险、合同/分成改变、大规模外租、高风险 destructive action、系统置信不足。

## Boundary

正常供应管理回 Supply，容量规划回 Capacity，日常财务回 Revenue/Settlements，Supplier 管理回 Suppliers。Operations 只聚合异常和决策入口。

## Error Copy

每个事件必须回答：发生什么 → 影响什么 → BurnCloud 做了什么 → 还需要人做什么。

## Audit

所有人工 Approve/Reject/Override 应记录决策人、时间、输入、预期影响和最终结果。

## Success Condition

Admin 不需要巡查十几个页面找故障；真正需要人处理的例外集中出现，而已经自动解决的事件不会制造操作负担。

## Verification Checklist

- [ ] 只聚合异常/人工例外
- [ ] Incident 有 impact 和 owner/action
- [ ] 自动恢复不会伪装成待处理
- [ ] 高风险批准有审计
- [ ] 正常功能跳回所属 domain
- [ ] Severity 有真实规则，不靠颜色 alone
- [ ] 处置后验证最终结果
- [ ] 不出现无上下文原始日志洪流

## Product Gate

扩大 Autopilot 无需批准的高风险权限、改变 Severity/审批阈值、或让 Operations 承担新的正常业务域，需要 Product/Risk Gate。

## Final Rule

Operations 应让人“只处理机器不能安全决定的例外”，而不是让所有自动化最终都变成人工工单。