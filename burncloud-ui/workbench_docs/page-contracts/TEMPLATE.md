---
doc_id: ui.page-contract.<role>-<page>
doc_type: page-contract
truth: target
status: draft
version: 0.1
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: <buyer|supplier|admin>
page: <page>
---

# <Role> <Page> Page Contract

## 1. Purpose

说明该页面为什么存在。不要写视觉细节，不要把未来目标描述成当前实现事实。

## 2. User Goal

> 用户来到这里真正想完成什么？

## 3. Primary Question

> 页面必须在最短时间内回答的一个问题是什么？

## 4. Primary Information

列出默认首屏需要出现的信息或指标，并说明语义和排序。

## 5. Primary Action

页面原则上只有一个最明显 Primary CTA。说明在不同状态下 CTA 是否变化。

## 6. Secondary Actions

列出必要但不能与 Primary CTA 同等级竞争的操作。

## 7. Main Content / Drill-down

定义主要区域以及允许的下钻路径。

## 8. Empty State

解释为什么为空、用户下一步做什么。禁止仅显示 `No data`。

## 9. Loading / Partial Failure

保持布局稳定；部分 API 失败不得无条件拖垮整页；Unknown 不得伪装成 0 或 Healthy。

## 10. Error / Needs Attention

优先表达：发生了什么 → 有什么影响 → BurnCloud 已做什么 → 用户是否需要行动。

## 11. Autopilot Behavior

明确哪些事情由 BurnCloud 自动完成，页面只报告结果；哪些情况才请求人工操作或批准。

## 12. Advanced Information

默认保持简单，专业信息通过 `Advanced / Details / Inspect` 下钻。

## 13. Intentionally Hidden Information

列出本角色不应该看到的内部概念、敏感字段和跨角色信息。

## 14. Success Condition

用用户能否完成目标来定义成功，不用“页面漂亮”定义成功。

## 15. Verification Checklist

- [ ] Primary Question 可以被页面快速回答
- [ ] Primary Action 唯一且明确
- [ ] Empty / Loading / Error 状态完整
- [ ] Unknown 没有被伪装成正常值
- [ ] 没有跨越 IA 角色边界
- [ ] 没有重复其它一级页面的完整职责
- [ ] Advanced 信息没有污染默认首屏
- [ ] 窄屏仍保留结论、状态和 Primary Action
- [ ] 状态不只依赖颜色

## 16. Product Gate

列出 Agent 不能自行批准的改变，例如：修改一级 Mental Model、暴露跨角色数据、改变购买对象、改变结算规则、增加新的一级概念等。

## 17. Final Rule

用一句话描述：该页面永远应该回答什么、永远不应该退化成什么。