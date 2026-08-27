---
doc_id: ui.agent-execution
doc_type: agent-execution-standard
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
---

# BurnCloud UI Agent Execution Protocol v1.0

## 1. Goal

本协议把 BurnCloud UI Standard 转成可用于 LangGraph / Coding Agent / Review Agent 的执行流程。Agent 的任务不是“把页面做漂亮”，而是在真实代码约束下实现已批准的产品契约。

## 2. Master Graph

```text
Task
→ Discovery
→ User Goal Analysis
→ Page Contract
→ UX Plan
→ Component Resolution
→ Implementation
→ Compile / Functional Check
→ Product / UI Gates
→ Visual QA
→ Accessibility
→ Reviewer
→ FAIL → Fix → Verify
→ PASS → PR
```

## 3. Shared UI State

推荐 Graph State：

```text
UIState
├── task: goal, role, page, scope
├── product: user_goal, primary_question, contract
├── current: routes, files, components, behavior, screenshots
├── design: hierarchy, CTA, states, advanced/hidden
├── implementation: changed_files, diff
├── verification: compile, functional, conventions, visual, a11y
└── decision: risk, confidence, human_gate
```

每个 Node 更新共享 State，不依赖聊天历史猜测关键产品事实。

## 4. Discovery Node

在改代码前确认：真实 Route、当前页��、数据来源、现有组件、现有 UX Gates、相关 Page Contract、最小执行路径。搜索只用于发现候选，不把文件名或旧文档当运行事实。

## 5. User Goal Node

必须回答：

```text
Who is the user?
Why did they open this page?
What must they know first?
What is the Primary Action?
What should stay hidden?
```

无法回答则不能进入 Coding。

## 6. UX Plan Node

只设计信息层级、状态、动作和下钻，不在此阶段大规模改 CSS。计划必须指出：Default、Loading、Empty、Partial Error、Critical、Advanced。

## 7. Component Resolver

优先复用当前 `burncloud/burncloud` 已存在共享组件和 tokens。只有当现有 Pattern 不足且新 Pattern 有跨页价值时才提出 Design System proposal；Agent 不得自行创建另一套按钮/卡片语言。

## 8. Implementation Node

一个页面/一个明确 scope 由一个 Owner Agent 实现。避免多个 Coding Agent 同时无协调修改同一 UI tree/CSS cascade。遵循 Smallest Correct Change。

## 9. Verification Nodes

至少分开：

- Compile/type/build
- Functional behavior
- Existing executable UI conventions
- Page Contract / Product UX
- Visual QA
- Accessibility
- Final diff inspection

Build green 不等于 UX contract green。

## 10. Reviewer Node

Reviewer 不重新设计页面，只根据证据判断：Product Contract、state completeness、role boundary、visual consistency、security/privacy、regression risk 是否满足。

## 11. Fix Loop

Reviewer 输出结构化问题：severity、contract rule、evidence、expected correction。Fix Agent 只修这些问题，再跑受影响验证；避免借机大重构。

## 12. Human Gates

### Product Gate

- IA 一级菜单
- Buyer/Supplier/Admin Mental Model
- 定价/商品/结算核心规则
- 新核心用户流程

### Design System Gate

- 新全局 Pattern
- 新核心 Component API
- 大范围视觉体系改变

### High-Risk Gate

- Billing / Settlement
- Supplier Revenue Share
- 自动租 GPU 的高成本阈值
- 大规模 infrastructure action
- Security / destructive data action

## 13. Autonomy Rule

低风险、可验证、可恢复 UI/实现决策由 Graph 自己完成；Agent 不应因为 spacing、已定义状态、组件复用等每个小问题都呼叫人。只有超出已批准规则的决策才 Gate。

## 14. Evidence Rule

每次完成报告至少包含：Goal、Contract used、Files changed、Behavior changed、Verification run、Known gaps、Product Gate decisions（如有）。不能只说“UI optimized”。

## 15. Golden Page Migration

优先顺序：

```text
Buyer Overview
Supplier Overview
Admin Overview
Buyer Marketplace
Supplier Resources
Admin Capacity
```

先把 Golden Pages 做��标准，再通过 Graph 逐页迁移。不要同时让 Agent 随机重做所有页面。

## 16. Current vs Target

Workbench 文档是 `truth: target`。Agent 必须在任务中明确区分：Target Contract、Current Source Truth、Gap。未经实现和验证，不得把 target 写成“BurnCloud 已支持”。

## 17. Completion Rule

只有对应 Page Contract Success Condition、Review Checklist、适用 executable gates 和最终 diff inspection 都通过，才能从 `PASS` 进入 PR/交付。