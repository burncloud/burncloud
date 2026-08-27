---
doc_id: ui.page-contract.admin-settings
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: admin
page: settings
---

# Admin Settings Page Contract v1.0

## Purpose

Admin Settings 管平台级长期配置、自动化边界和全局政策。它必须把普通设置与高风险 Autopilot / Billing / Settlement / Security 设置清楚分区。

## User Goal

> 我想安全地修改平台长期规则，并清楚知道哪些改变会影响收入、供应、自动化或安全。

## Primary Question

> BurnCloud 当前有哪些全局策略，修改它们会产生什么影响？

## Sections

- Platform configuration
- Notification rules
- Billing defaults
- Settlement defaults
- External capacity providers
- Automation / Autopilot limits
- Approval thresholds
- Safety / security policy
- Maintenance / data controls when applicable

## Primary Actions

每个设置域独立 `Save`，避免一个全局 Save 混入高风险更改。高风险动作进入 `Advanced` / `Danger Zone` 并需要二次确认或审批。

## Autopilot Policy

明确配置：哪些动作可自动完成、成本/规模/风险阈值、何时必须 Human Gate、失败时 fallback。不要只提供一个模糊 `Enable AI` 开关。

## External Providers

外部 GPU/算力来源配置应包含可用性、成本/额度、权限和自动租赁上限；Secret 按安全规则处理，不长期明文显示。

## High-risk Changes

例如：提高自动外租上限、降低支付审批阈值、清空关键数据、关闭保护策略、改变 Settlement default。必须展示影响范围和恢复/回滚策略（若可行）。

## Truth Rule

前端只展示后端真实支持的设置。禁止展示保存到本地但系统实际上不读取的“假设置”。

## Audit

高风险设置改变应记录 Before/After、actor、time、reason/approval 和 rollout result。

## Success Condition

Admin 可以安全改变长期平台策略，同时很难因为误点一个普通设置就产生大规模财务或基础设施影响。

## Verification Checklist

- [ ] 设置按 domain 分区
- [ ] Autopilot 权限/阈值可解释
- [ ] 高风险项位于 Advanced/Danger Zone
- [ ] Secret 不长期明文
- [ ] 服务器不支持的设置不出现假控件
- [ ] 高风险改变有 impact preview / confirmation / audit
- [ ] 保存失败不会伪装成功
- [ ] 危险数据操作与普通设置分离

## Product Gate

改变高风险审批模型、Autopilot 最大财务权限、Settlement/Billing default 语义或核心安全保护，需要 Product/Finance/Security Gate。

## Final Rule

Settings 是平台规则的受控入口，不应该成为任何 Agent 或管理员绕过风险门槛的万能后门。