---
doc_id: ui.index
doc_type: product-standard-index
truth: target
status: approved
version: 1.0
---

# BurnCloud UI Standards

本目录是 BurnCloud 目标产品 UI/UX 的权威工作台入口。这里的文档定义 **要做成什么样**，不代表 `burncloud/burncloud` 当前代码已经全部实现。

## Read Order

UI / Product Agent 开始非微小页面任务前，按顺序阅读：

1. [`product-standard.md`](product-standard.md) — 产品与 UI 最高原则
2. [`information-architecture.md`](information-architecture.md) — Buyer / Supplier / Admin 信息架构和页面边界
3. 对应 [`page-contracts/`](page-contracts/) — 页面级可执行契约
4. [`design-system.md`](design-system.md) — 视觉与组件系统规则
5. [`interaction-rules.md`](interaction-rules.md) — 交互、状态、危险操作和 Autopilot 行为
6. [`content-standard.md`](content-standard.md) — 文案与状态语言
7. [`review-checklist.md`](review-checklist.md) — UI 完成与评审门槛
8. [`agent-execution.md`](agent-execution.md) — Graph / Agent 执行协议

## Truth Model

本仓库多数 UI 文档标记为 `truth: target`：

- 它们是批准的目标产品规则。
- 它们不是当前代码/运行时事实。
- 实施前仍必须读取 `burncloud/burncloud` 当前源代码、可执行 Gate 和运行证据。
- 若目标规范与当前实现不���，应把差异作为迁移任务，不得在报告中声称目标已实现。

## Core Mental Models

```text
Buyer
Model → API → Usage → Billing

Supplier
GPU → Health → Contribution → Earnings

Admin
Supply → Capacity → Demand → Economics
```

## Golden Pages

第一阶段 Golden Pages：

- Buyer Overview
- Supplier Overview
- Admin Overview
- Buyer Marketplace
- Supplier Resources
- Admin Capacity

其它页面必须沿用 Golden Pages 已批准的 Typography、spacing、hierarchy、component usage、status language、CTA hierarchy 和 density。

## Change Authority

以下变化不得由 Coding/UI Agent 自行批准：

- 新增/删除一级菜单
- 修改三角色核心 Mental Model
- 改变 Buyer 购买对象
- 向 Buyer 默认暴露 GPU/Supplier/IDC 内部结构
- 让 Supplier 手工选择/部署模型或控制 Traffic
- 改变 Revenue Share / Contribution / Settlement 核心规则
- 扩大 Autopilot 的高财务/安全风险权限
- 建立新的全局 Design Pattern / 核心组件体系

这些进入 Product / Design System / High-Risk Gate。

## Promotion to Production Repo

本仓库是 workbench。规范稳定且对应实现开始后，应把批准文档按需要同步/迁移到 `burncloud/burncloud/docs/ui/`，同时保持目标规范与 source-derived/current implementation 文档的 truth 标签清晰分离。