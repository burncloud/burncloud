---
doc_id: ui.content-standard
doc_type: content-standard
truth: target
status: approved
version: 1.0
parent: docs/ui/product-standard.md
---

# BurnCloud UI Content Standard v1.0

## 1. Voice

BurnCloud 文案应克制、准确、直接、专业。不要用营销腔掩盖系统状态，不用拟人化语气让基础设施错误显得轻浮。

## 2. Conclusion First

优先写结论，再写数字和技术原因。

不好：`Utilization 93%`。

更好：`Capacity is running low. DeepSeek Standard is using 93% of safe capacity.`

## 3. Operational Message Shape

```text
What happened
→ Impact
→ What BurnCloud did
→ What the user needs to do
```

若用户无需操作，明确说明系统已处理，不要附一个多余 CTA。

## 4. Truthful Language

`Healthy / Recovered / Paid / Ready / Available` 等结论必须有对应真实状态。数据 Unknown 时写 Unknown/Unavailable，不得用默认值让 UI 看起来更完整。

## 5. Role Language

Buyer 使用 Model、Tier、API、Usage、Balance、Billing。

Supplier 使用 Resources、GPU、Health、Contribution、Earnings、Settlement。

Admin 使用 Supply、Capacity、Demand、Revenue、Margin、Settlement、Operations。

不要把内部代码名称、数据库枚举或跨角色术语无差别暴露。

## 6. Naming Consistency

同一概念全站使用同一名称。尤其统一：Economy / Standard / Performance；Healthy / Degraded / At Risk / Offline；Today Revenue / Gross Margin / Online GPU Capacity / API Availability。

## 7. Buttons

按钮用动作动词：`Use Model`、`Send Test Request`、`Create API Key`、`Top Up`、`Request Offline`、`Review Capacity`。避免 `OK`、`Submit`、`Execute` 等没有上下文的泛化动作。

## 8. Dangerous Buttons

明确对象和后果，例如 `Force Offline 8 GPUs`、`Delete API Key`。不要仅写 `Confirm`。

## 9. Empty States

说明原因和下一步。例如：

```text
No models in use yet.
Open Marketplace to choose your first model.
[Browse Marketplace]
```

## 10. Financial Copy

金额必须带币种/适用周期。区分 Estimate、Accruing、Eligible、Payable、Paid。`Earnings`、`Revenue`、`Balance`、`Settlement` 不可互换。

## 11. Autopilot Copy

优先描述系统结果：

```text
Capacity restored.
BurnCloud added temporary capacity after demand increased.
```

如果需要批准：

```text
Capacity risk detected.
Adding temporary capacity would increase available capacity by 18% and reduce projected margin by 1.8%.
[Review proposal]
```

## 12. Error Detail

用户层文案不以 raw error code 开头。技术 ID、trace、provider response 等放 `Details / Inspect`，同时保留稳定 request/incident reference 方便支持。

## 13. Avoid Overclaiming

不要把 Configured 写成 Connected、把部分检查通过写成 Healthy、把预测写成事实、把 Pending payment 写成 Paid、把本地表单成功写成服务器已生效。

## 14. Brevity

默认一句结论 + 一句必要解释。只有当决策需要时才展开更多上下文。极简不是删掉关键风险信息，而是删除重复和无决策价值内容。

## 15. Localization

产品概念、状态和金额格式必须有可本地化语义。不要在代码里用拼接碎片形成难翻译的状态句。

## 16. Content Gate

修改核心状态词、金融术语、Tier 名称、Buyer/Supplier/Admin Mental Model 词汇需要 Product/Content Gate，不能由单页 Agent 随意换同义词。