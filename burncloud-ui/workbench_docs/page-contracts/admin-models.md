---
doc_id: ui.page-contract.admin-models
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: admin
page: models
---

# Admin Models Page Contract v1.0

## Purpose

Models 管理 BurnCloud 对外出售的 Model Product Catalog：版本、可用性、Tier、价格和生命周期。它不是 GPU Deployment 页面。

## User Goal

> 我想控制 BurnCloud 向 Buyer 出售哪些模型、以什么产品规格和价格出售。

## Primary Question

> BurnCloud 正在出售哪些模型产品，它们是否正确配置并可售？

## Core Information

- Model name / family
- Version
- Marketplace visibility
- Availability
- Pricing source
- Input / output pricing or applicable unit
- Supported tiers
- Capability/context summary
- Rollout / deprecation status
- Capacity link

## Pricing Rule

```text
有官方默认 API 定价
→ 跟随官方默认定价

没有官方默认定价
→ BurnCloud 统一定价
```

UI 必须标识价格来源和最近同步/确认时间。Supplier 不参与 Buyer 定价。

## Primary Actions

根据权限：Add model product、Edit product metadata/pricing policy、Publish/Unpublish。影响已在售模型的价格/停用属于高风险动作，应明确影响范围。

## Tier Configuration

Economy / Standard / Performance 是 Buyer 产品档位。Admin 可以配置某模型支持哪些 Tier 及产品承诺，但底层 GPU 选择仍由 Scheduler/Capacity 系统完成。

## Lifecycle

Draft、Active、Constrained、Deprecated、Retired 等状态必须与 Marketplace 行为一致。Deprecated 应提供迁移信息，而不是突然消失。

## Boundary

Models 不负责：逐 GPU Deployment、Supplier Contract、Demand 预测、Buyer Usage、Settlement。

## Autopilot

价格跟随官方规则可自动检测变更，但实际生效策略、异常差异和重大价格变化应遵循审批阈值。模型部署由 Autopilot 根据 Capacity 处理。

## Success Condition

Admin 能清楚知道每个模型对 Buyer 的商品定义、价格来源、Tier 和生命周期，并保证 Marketplace 展示与这里一致。

## Verification Checklist

- [ ] Model product 与 GPU deployment 分离
- [ ] 价格来源明确
- [ ] 官方价同步状态可追溯
- [ ] Supplier 无 Buyer pricing 权限
- [ ] Tier 与 Marketplace 一致
- [ ] 生命周期状态真实影响可售性
- [ ] 高影响价格/退役动作有保护
- [ ] Capacity 仅链接而非复制

## Product Gate

改变官方价跟随原则、允许 Supplier 自定 Buyer 价格、改变 Tier 模型、或大规模自动价格变更权限，需要 Product/Finance Gate。

## Final Rule

Models 定义“BurnCloud 卖什么模型产品”，不是“哪张 GPU 现在正在跑什么”。