---
doc_id: ui.page-contract.supplier-reliability
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: supplier
page: reliability
---

# Supplier Reliability Page Contract v1.0

## Purpose

Reliability 向 Supplier 解释资源稳定性表现以及它如何影响调度机会。Supplier 看到可理解的评级和原因；完整 Scheduler Resource Score 保留在内部/Admin 领域。

## User Goal

> 我怎样保持更稳定的调度机会，当前有哪些可靠性问题？

## Primary Question

> 我的资源为什么得到当前的调度机会？

## Primary Rating

Supplier 默认看到简化等级：

```text
Excellent
Good
Needs Attention
At Risk
```

等级不是装饰性分数，必须有实际证据和明确影响。

## Evidence Dimensions

- Availability
- Unexpected offline events
- Performance stability
- Network stability
- Benchmark history
- Task completion
- Hardware errors
- Operating history

默认优先给结论，再允许下钻时间线和证据。

## Primary Action

当存在可修复问题时，CTA 指向实际动作，例如 `Review unstable resources`、`Fix node connectivity`。无问题时不需要强 CTA。

## Graceful vs Unexpected Offline

正常 Graceful Offline 不应与突然掉线同等处罚。页面要清楚区分 planned drain 与 unexpected outage。

## Impact Explanation

允许解释评级可能影响：Scheduling Weight、Traffic Opportunity、Risk Threshold、Capacity Priority。不得把 Supplier Level 与 Revenue Share 混为一个评分结果。

## Supplier Level

可以显示 Community / Verified / Professional / Strategic 及升级所需条件，但等级与 Reliability、Revenue Share 必须概念分离。

## Autopilot

BurnCloud 自动检测异常、迁移工作负载、重新 Benchmark；Supplier 只负责可归因于自身的硬件/网络/可用性修复。

## Intentionally Hidden

- Exact global scheduler formula/weights when abuseable
- Other suppliers' scores
- Security-sensitive anti-gaming signals
- Buyer identities

## Success Condition

Supplier 能理解当前评级、近期扣分/风险原因和最有效修复动作，而不需要反向工程调度器。

## Verification Checklist

- [ ] 简化评级有真实证据
- [ ] Planned drain 与 unexpected outage 分离
- [ ] 每个警告尽量给出可执行动作
- [ ] Level / Reliability / Revenue Share 不混淆
- [ ] 不泄露其它 Supplier 或 anti-gaming internals
- [ ] Unknown evidence 不默认为 Excellent
- [ ] 自动恢复行为如实记录

## Product Gate

修改评级对收益/调度的实质影响、公开完整调度公式、或改变 Supplier Level 含义，需要 Product/Risk Gate。

## Final Rule

Reliability 应帮助 Supplier 变得更可靠，而不是给一个无法解释、容易被游戏化的神秘分数。