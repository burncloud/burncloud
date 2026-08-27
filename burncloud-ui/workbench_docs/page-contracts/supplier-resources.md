---
doc_id: ui.page-contract.supplier-resources
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: supplier
page: resources
---

# Supplier Resources Page Contract v1.0

## Purpose

Resources 是 Supplier 管理自己接入 BurnCloud 的物理/虚拟 GPU 资源与健康状态的页面。它管理“我提供了什么资源”，不管理“应该跑什么模型”。

## User Goal

> 我有哪些机器和 GPU 在线，它们是否健康，我如何安全接入或下线？

## Primary Question

> 我接入 BurnCloud 的 GPU 现在是什么状态？

## Core Information

- Cluster / machine / GPU inventory
- Online / Offline / Draining
- Utilization
- Hardware health
- Network health
- Benchmark status
- Last seen / uptime
- Current high-level workload indicator when useful

## Drill-down

```text
Resources
→ Cluster
→ Machine
→ GPU
```

默认列表提供 Search / Filter / Sort。不要默认塞 Admin 式列管理。

## Primary Actions

没有资源时：`Add Resource / Install Node`。

有资源时，新增资源可作为常规动作；对单个/一组资源允许 `Request Offline`，但必须走 Graceful Offline。

## Graceful Offline

```text
Request Offline
→ Drain
→ Stop New Tasks
→ Finish Existing Work
→ Release Deployment
→ Offline
```

UI 必须告诉 Supplier 当前阶段和预计影响。直接掉线/拔机不是正常下线流程，可能影响 Reliability 与 Revenue Opportunity。

## Error / Needs Attention

优先展示可行动问题：Node unreachable、benchmark failed、hardware error、network unstable。不要只给十六进制错误码。

## Autopilot

模型部署、重部署、模型切换、GPU 分组优化由 BurnCloud 完成。资源健康异常时 BurnCloud 可以自动迁移工作负载，Supplier 只处理硬件、网络和资源可用性问题。

## Intentionally Hidden / Forbidden Controls

- Select model to deploy
- Manual traffic weight
- Manual customer assignment
- Other suppliers' resources
- Global scheduling weights
- Internal Buyer routing

## Success Condition

Supplier 能准确知道自己每台资源的健康/在线状态，并能安全接入或优雅下线，而无需参与模型调度。

## Verification Checklist

- [ ] Inventory 可按 Cluster → Machine → GPU 下钻
- [ ] Online / Offline / Draining 状态明确
- [ ] Hardware / network / benchmark 状态有真实来源
- [ ] Request Offline 执行 Graceful Drain
- [ ] Unexpected offline 与正常下线区分
- [ ] 不出现模型选择与 traffic weight 控件
- [ ] Unknown 状态不伪装 Healthy
- [ ] 错误提供 Supplier 可执行下一步

## Product Gate

允许 Supplier 控制模型部署、改变 Graceful Offline 规则、隐藏异常掉线的收益/可靠性影响，需要 Product Gate。

## Final Rule

Resources 管 Supplier 的 GPU 是否可供 BurnCloud 使用；BurnCloud 自己决定这些 GPU 如何转化成模型能力。