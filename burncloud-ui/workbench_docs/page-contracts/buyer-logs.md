---
doc_id: ui.page-contract.buyer-logs
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: buyer
page: logs
---

# Buyer Logs Page Contract v1.0

## Purpose

Logs 是 Buyer 的请求级诊断页，用来找到失败、慢请求和异常用量的具体请求，不是平台基础设施日志浏览器。

## User Goal

> 我想找到哪一次请求出了问题，以及我能做什么。

## Primary Question

> 哪一次请求出了问题，为什么？

## Default Information

- Request time
- Request / trace reference
- Model
- Tier
- Outcome
- Latency
- Usage
- Error summary

默认优先突出 Failures / degraded outcomes，而不是原始日志文本。

## Primary Actions

以诊断为主，可提供：

- Inspect request
- Copy request reference
- Retry in Playground when safe/applicable
- Filter failures

不应出现“选择供应商重新路由”这类底层动作。

## Filters

时间、Model、Tier、Outcome、API Key（管理引用）等 Buyer 可理解维度。默认不提供 Worker / GPU / Supplier filter。

## Detail / Advanced

Inspect 可展示：

- Buyer-visible request metadata
- Error category
- Timing breakdown when safe
- Token usage
- Relevant response metadata

敏感请求内容的展示必须服从隐私、权限和日志保留策略。

## Error Copy

先给结论和影响，再给技术细节。例如：

```text
Request failed before inference completed.
No usage was charged for this attempt.
Try again or inspect the request details.
```

## Autopilot

如果 BurnCloud 已自动 failover/recover，应在日志中描述最终 outcome，不要求 Buyer 理解或手动操作底层路由。

## Intentionally Hidden

- Supplier credentials / identity by default
- Internal capability tokens
- Internal worker IDs unless explicitly approved for diagnostics
- Scheduler score / routing weights
- Internal infrastructure secrets
- Other tenants' data

## Success Condition

Buyer 能从失败现象定位到具体请求，理解 Outcome / charge / next action，并在需要支持时提供稳定的 request reference。

## Verification Checklist

- [ ] 默认列包含 Time / Model / Tier / Outcome / Latency / Usage
- [ ] Failures 容易筛选和识别
- [ ] Error summary 是 Buyer 可理解的结论
- [ ] 请求引用稳定可复制
- [ ] 不泄露 Supplier / internal secret
- [ ] 自动恢复时结果叙述真实
- [ ] 敏感请求内容遵循权限/保留策略
- [ ] Logs 不退化成基础设施原始日志控制台

## Product Gate

默认暴露 Supplier/Worker/内部拓扑、改变日志隐私策略、或让 Buyer 直接操作路由，需要 Product/Security Gate。

## Final Rule

Logs 回答“我的这次 API 请求发生了什么”，而不是“BurnCloud 的所有服务器内部发生了什么”。