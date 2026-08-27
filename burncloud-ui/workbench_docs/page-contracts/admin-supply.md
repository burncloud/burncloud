---
doc_id: ui.page-contract.admin-supply
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: admin
page: supply
---

# Admin Supply Page Contract v1.0

## Purpose

Supply 管理 BurnCloud 的 GPU 资源来源与可用性，回答“我们拥有什么资源”，但不等同于这些资源已经形成可售卖模型 Capacity。

## User Goal

> 我想知道 BurnCloud 的算力来自哪里、多少在线、质量如何，以及供应是否存在结构性风险。

## Primary Question

> BurnCloud 现在拥有多少可靠可用的 GPU Supply？

## Supply Sources

- Supplier-provided resources
- IDC resources
- BurnCloud-owned resources（如存在）
- External rental resources
- AutoDL / other external providers

## Core Information

- Online / total resources by source
- GPU class / normalized capability
- Reliability
- Region / network when operationally useful
- Cost model / contract class at Admin level
- External rental status
- Resource aging / failure signals

## Drill-down

```text
Supply
→ Source
→ Supplier / External Provider
→ Cluster
→ Machine
→ GPU
```

## Primary Actions

以管理来源和异常为主：Inspect source、Review unhealthy supplier、Add/Configure external source（权限允许时）。逐 GPU 操作应位于详情并受到风险控制。

## Supply vs Capacity

页面必须明确：`GPU Online ≠ Model Capacity Available`。要看可售卖模型能力应进入 Capacity。

## External Rental

外租资源应标识来源、租赁成本、生命周期、是否由 Autopilot 临时拉起。不要与长期 Supplier Supply 混成一种成本语义。

## Autopilot

BurnCloud 可自动发现、Benchmark、归池、隔离不健康资源；管理员主要处理来源风险、合同/成本和异常。

## Intentionally Not Primary

- Model pricing
- Buyer traffic analytics
- Per-model headroom → Capacity
- Supplier settlement → Settlements

## Success Condition

Admin 能解释 BurnCloud 的资源供应构成、质量与风险，并能从来源下钻到底层 GPU，而不会误把“在线资源”当成“已可出售容量”。

## Verification Checklist

- [ ] 各 Supply source 清楚区分
- [ ] Online GPU 与 Capacity 概念分离
- [ ] Supplier/External rental 成本语义可区分
- [ ] 可下钻到 Cluster/Machine/GPU
- [ ] Reliability/health 有真实来源
- [ ] 不复制 Capacity/Demand 页面
- [ ] Autopilot 隔离/恢复结果可追溯

## Product Gate

改变 Supply 来源类型、开放危险批量资源操作、或把 Supply 与 Capacity 合并成单一模糊指标，需要 Product/Operations Gate。

## Final Rule

Supply 回答“原材料从哪里来、是否可靠”；Capacity 才回答“这些原材料现在能卖出多少模型服务”。