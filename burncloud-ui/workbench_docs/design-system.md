---
doc_id: ui.design-system
doc_type: design-standard
truth: target
status: approved
version: 1.0
parent: docs/ui/product-standard.md
---

# BurnCloud Design System v1.0

## 1. Direction

BurnCloud 的视觉目标是 **Apple-style restraint + Stripe-style professionalism**：克制、安静、精准、专业，不炫技，不制造视觉噪音。

设计系统的目标不是让每个页面“长得一样”，而是让相同语义始终用相同视觉和交互语言表达。

## 2. Priority

冲突时：

```text
User Goal
> Correctness
> Information Hierarchy
> Action Clarity
> State Completeness
> Consistency
> Accessibility
> Visual Beauty
```

## 3. Density

默认 Medium Density。Overview/Marketplace 可更松；Admin 专业页可更密，但不得退化成传统云控制台信息墙。

## 4. Color

主体：Black / White / Gray。

只保留一个克制的 BurnCloud Accent Color。状态色语义：Green=Healthy/Success，Yellow=Warning，Red=Critical/Danger，Blue=Informational。颜色不能作为状态唯一信号。

默认禁止大面积渐变、彩虹色指标、为装饰使用状态色。

## 5. Typography

建立稳定层级：Page title、Section title、Body、Secondary、Label/Caption、Numeric metric。数字与单位必须容易扫描；不要通过大量字号跳跃制造“高级感”。

## 6. Spacing / Layout

使用一致的 spacing scale 和页面最大宽度/栅格。页面层级靠 spacing、typography、divider、subtle surface 建立，不靠每块内容套 Card。

## 7. Cards

只用于有明确逻辑边界的信息组。禁止“每个指标一张彩色大卡 + 每个 section 再套卡”的 SaaS 模板化设计。

## 8. Tables

普通页面默认 Search / Filter / Sort。Admin 高级页按需支持 Export / Column Selection / Bulk Actions。表格列优先呈现任务结论，内部 ID 放 Advanced/Details。

## 9. Charts

每张图必须回答一个问题。优先 Line / Area / Bar / Sparkline。默认不用 Pie / Donut / Radar / Gauge / 3D。无趋势价值时用数字/表格比画图更好。

## 10. CTA Hierarchy

```text
Primary
Secondary
Tertiary
Danger
```

每个页面尽量只有一个明显 Primary CTA。Danger 永远不能伪装成普通 Primary。

## 11. Core Component Vocabulary

目标组件族：

```text
BCButton
BCInput
BCSelect
BCSearch
BCModal
BCCard
BCTable
BCBadge
BCAlert
BCMetric
BCPageHeader
BCEmptyState
BCSkeleton
BCChart
BCStatus
BCCommand
BCDangerZone
```

这些是目标组件语义，不代表当前代码都已存在。实现时先检查 `burncloud/burncloud` 当前共享组件，复用已有能力，避免为满足文档名称重复造组件。

## 12. Component Rule

Agent 在新增 Component 前必须回答：是否已有相同 Pattern��是否跨两个以上页面复用、是否值得成为 Design System API、是否会引入新的视觉语法。新的全局 Pattern 进入 Design System Gate。

## 13. Status Vocabulary

统一状态优先使用：Healthy、Ready、Running、Degraded、At Risk、Offline、Draining、Provisioning、Deploying、Scaling，以及各领域批准的财务状态。同一事实不得在页面 A 叫 Healthy、页面 B 叫 Normal、页面 C 叫 OK。

## 14. Responsive

窄屏优先保留：结论 → 状态 → 核心指标 → Primary Action → Needs Attention → Main Content。复杂表格应折叠、卡片化或横向滚动的选择必须基于可读性，不能挤压到不可读。

## 15. Accessibility

所有交互必须可键盘访问；焦点状态明确；状态不只靠颜色；图标有可理解标签；Alert/Loading 有语义；对比度满足适用标准；数字、币种、单位和错误信息可被辅助技术理解。

## 16. Golden Page Rule

Design System 的真实视觉语言最终由批准并实现的 Golden Pages 校准。文档给规则，Golden Pages 给组合范例，自动 Visual QA 防止漂移。