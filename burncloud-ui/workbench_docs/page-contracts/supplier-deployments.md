---
doc_id: ui.page-contract.supplier-deployments
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: supplier
page: deployments
---

# Supplier Deployments Page Contract v1.0

## Purpose

Deployments 让 Supplier 只读了解 BurnCloud 当前如何使用其资源。该页面用于透明度，不授予调度控制权。

## User Goal

> BurnCloud 当前在我的资源上运行什么，这些部署是否正常、贡献如何？

## Primary Question

> BurnCloud 当前在我的资源上运行什么？

## Core Information

- Current model
- Deployment state
- Assigned cluster / resource summary
- Since when
- Current utilization
- Current contribution
- High-level deployment health

## States

建议统一：Provisioning、Deploying、Running、Draining、Switching、Degraded、Failed、Stopped。

状态必须来自真实编排过程，不得因为目标模型已选定就提前显示 Running。

## Actions

页面本身原则上没有部署修改 Primary CTA。允许：

- View deployment details
- Open related Resources
- Open Earnings contribution

若 Supplier 需要下线机器，跳转 Resources 发起 Graceful Offline。

## Autopilot

BurnCloud 可自动：部署、卸载、切换模型、重建 Worker Group、扩缩容、迁移流量。Supplier 只观察结果。

## Error

Deployment failure 应解释对 Supplier 的影响，以及 BurnCloud 是否正在自动恢复。例如 `Deployment failed; BurnCloud is retrying on healthy capacity. Your affected GPU remains under review.`

## Intentionally Forbidden

- Manual model switch
- Manual parallelism strategy
- Manual deployment delete
- Manual traffic assignment
- Customer selection
- Global scheduler override

## Success Condition

Supplier 能理解“我的资源正在被怎样利用以及是否产生贡献”，但无需学习或操作模型编排系统。

## Verification Checklist

- [ ] 页面只读
- [ ] Deployment states 与真实编排阶段一致
- [ ] 可关联到 Resources / Earnings
- [ ] 下线动作回到 Resources
- [ ] 不提供 model switch / traffic / parallelism 控件
- [ ] 自动切换过程有可理解状态
- [ ] 失败与恢复行为如实描述

## Product Gate

给 Supplier 增加任何改变 Deployment、模型选择、流量分配的能力必须 Product Gate。

## Final Rule

Deployments 是 BurnCloud 自动编排的透明窗口，不是 Supplier 的 Kubernetes/模型部署控制台。