---
doc_id: ui.page-contracts.index
doc_type: page-contract-index
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
---

# BurnCloud Page Contracts

本目录把 BurnCloud 的 Product Standard 与 Information Architecture 落成逐页可执行契约。

Page Contract 不是像素稿，也不声明当前代码已经实现目标行为。它定义页面在目标产品中必须回答什么、负责什么、隐藏什么、何时需要人工 Gate，以及如何验证。

## Authority

冲突时按以下顺序处理目标产品决策：

`approved product decision > product-standard.md > information-architecture.md > page contract > implementation proposal`

当前代码事实仍必须从真实源码、运行证据和可执行测试确认，不能因为 Page Contract 写了某个目标就把未实现行为当作事实。

## Required Contract Shape

每个契约至少必须明确：

- User / Role
- User Goal
- Primary Question
- Primary Information / Metrics
- Primary Action
- Secondary Actions
- Empty / Loading / Error states
- Advanced information
- Intentionally Hidden information
- Autopilot / Human Intervention behavior
- Success Condition
- Verification Checklist
- Product Gate conditions

新增一级页面前必须先通过 IA Product Gate，再创建 Page Contract。Agent 不得因为实现方便自行新增一级入口。

## Buyer

- [Buyer Overview](buyer-overview.md)
- [Buyer Playground](buyer-playground.md)
- [Buyer Marketplace](buyer-marketplace.md)
- [Buyer API Keys](buyer-api-keys.md)
- [Buyer Usage](buyer-usage.md)
- [Buyer Billing](buyer-billing.md)
- [Buyer Logs](buyer-logs.md)

Buyer 的 Mental Model：`Model → API → Usage → Billing`。

## Supplier

- [Supplier Overview](supplier-overview.md)
- [Supplier Resources](supplier-resources.md)
- [Supplier Deployments](supplier-deployments.md)
- [Supplier Earnings](supplier-earnings.md)
- [Supplier Settlements](supplier-settlements.md)
- [Supplier Reliability](supplier-reliability.md)
- [Supplier Settings](supplier-settings.md)

Supplier 的 Mental Model：`GPU → Health → Contribution → Earnings`。

## Admin

- [Admin Overview](admin-overview.md)
- [Admin Supply](admin-supply.md)
- [Admin Capacity](admin-capacity.md)
- [Admin Demand](admin-demand.md)
- [Admin Models](admin-models.md)
- [Admin Revenue](admin-revenue.md)
- [Admin Settlements](admin-settlements.md)
- [Admin Suppliers](admin-suppliers.md)
- [Admin Customers](admin-customers.md)
- [Admin Operations](admin-operations.md)
- [Admin Settings](admin-settings.md)

Admin 的 Mental Model：`Supply → Capacity → Demand → Economics`。

## Cross-role Rule

如果一个设计让 Buyer 开始思考 GPU、Supplier 开始手工部署模型、Admin 开始逐台机器做日常调度，则默认视为 IA 退化，需要 Product Gate。

## Template

新契约必须从 [TEMPLATE.md](TEMPLATE.md) 开始，不要复制某个具体页面后只替换标题。