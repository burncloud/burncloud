---
doc_id: ui.page-contract.supplier-settings
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: supplier
page: settings
---

# Supplier Settings Page Contract v1.0

## Purpose

Settings 管理 Supplier 自己的组织资料、结算资料、通知和 Node 接入配置。它不承载模型部署或调度策略。

## User Goal

> 我想维护供应商账户和资源接入所需的长期配置。

## Primary Question

> 我的 Supplier 资料、结算和接入配置是否完整有效？

## Sections

- Organization profile
- Contact information
- Settlement information
- Notifications
- Node installation / enrollment token management
- Allowed operational preferences
- Security / account settings when applicable

## Primary Action

按具体区域使用 `Save changes`，避免一个巨大的全页 Save 导致跨区误改。高风险设置单独确认。

## Node Enrollment

安装/注册 Token 应遵循最小权限、可撤销、必要时一次性显示。不能在 Settings 长期明文展示共享 Secret。

## Settlement Information

修改收款资料属于高风险动作，需要明确验证、审计与可能的人工/延迟保护。不能与普通联系方式更新同等处理。

## Boundary

禁止在 Settings 提供：

- Model deployment choice
- Traffic routing
- Scheduler weights
- Revenue share editing（Supplier 自己不可修改商务分成）
- Supplier level self-upgrade

Revenue Share 可只读展示或链接 Earnings/Settlements。

## Error / Unsaved State

保存失败必须明确哪些字段未生效。不能本地看似保存成功但服务器未接受。

## Intentionally Hidden

Internal finance credentials、global platform policies、other suppliers、scheduler secrets。

## Success Condition

Supplier 能安全维护账户和接入资料，并清楚哪些商业/调度配置由 BurnCloud 管理、自己不能修改。

## Verification Checklist

- [ ] Profile / settlement / notification 职责分区
- [ ] Secret 不长期明文显示
- [ ] Settlement detail change 有高风险保护
- [ ] Revenue Share 不可由 Supplier 自改
- [ ] Level 不可自助升级
- [ ] 不出现 deployment/routing 控件
- [ ] 保存结果反映真实服务器状态

## Product Gate

开放 Supplier 自改分成、等级、模型部署、调度权重或高风险结算自动变更，需要 Product/Finance/Security Gate。

## Final Rule

Supplier Settings 只管理“我的账户与接入资料”，不能成为绕过 BurnCloud Autopilot 和商业规则的后门。