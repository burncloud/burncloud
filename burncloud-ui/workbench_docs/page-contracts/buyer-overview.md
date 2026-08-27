---
doc_id: ui.page-contract.buyer-overview
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: buyer
page: overview
---

# Buyer Overview Page Contract v1.0

## 1. Purpose

本文档定义 BurnCloud Buyer Overview 的目标页面契约。

它不是视觉稿，也不是当前代码事实，而是后续 UI、前端实现、Agent 修改与自动验收共同遵循的产品契约。

Buyer Overview 的职责只有一个：

> 让消费用户在 5 秒内知道“今天用了多少、还能用多久、服务是否稳定、下一步是否需要处理什么”。

Overview 不负责解释 GPU、供应商、部署拓扑或模型调度细节。

---

## 2. User

主要用户：

- API 消费者
- 企业 API 使用方
- Buyer 账号管理员

用户可能是技术人员，也可能只是负责费用与运行状态的业务负责人。

页面不得假设用户理解 BurnCloud 底层 GPU 基础设施。

---

## 3. User Goal

Buyer 打开 Overview 时通常想快速回答：

1. 今天花了多少钱？
2. 账户还有多少钱？
3. API 现在稳定吗？
4. 今天用了多少 Token？
5. 有没有需要我马上处理的问题？

如果以上问题需要用户跳转多个页面才能回答，则 Overview 设计失败。

---

## 4. Primary Question

> 我的 BurnCloud API 今天使用情况是否正常，我现在是否需要采取行动？

Overview 必须先给结论，再给数据。

---

## 5. Primary Metrics

页面顶部固定四个核心指标：

```text
Today Spend
Balance
API Availability
Tokens Today
```

顺序默认保持不变。

### 5.1 Today Spend

回答：

> 今天截至当前已经消费多少？

默认展示：

- 今日累计消费金额
- 与昨日同期或昨日全天的轻量趋势信息（仅在数据可靠时）

不得在主指标中塞入模型拆分、供应商拆分或 GPU 成本。

### 5.2 Balance

回答：

> 当前预充值余额还有多少？

Balance 是 Buyer Overview 中最重要的可行动指标之一。

根据余额状态：

- Healthy：正常显示
- Low：显示明确提醒
- Critical：升级为页面最高优先级告警
- Exhausted：明确说明流量保护/暂停状态

当余额不足时，Primary CTA 自动切换为 `Top Up` / `Recharge`。

### 5.3 API Availability

回答：

> 当前 BurnCloud API 是否稳定可用？

默认展示平台对该 Buyer 实际可消费服务的可用性结论，而不是底层机器在线率。

推荐状态：

```text
Healthy
Degraded
At Risk
Unavailable
```

不要展示“43 台机器在线”“12 个 Supplier 正常”等底层信息。

### 5.4 Tokens Today

回答：

> 今天已经消费多少 Token？

默认显示累计 Token。

可以提供 input / output 细分，但只能作为次级信息或 Details，不应抢占主指标。

---

## 6. Primary Action

正常情况下，Overview 不强迫用户执行操作。

默认 Primary Action 可以是：

```text
Open Marketplace
```

或者在已有模型使用关系时保持弱化，不制造无意义 CTA。

系统出现明确用户责任时，Primary Action 应动态变化。

示例：

```text
Low balance      → Top Up
No API key       → Create API Key
No model in use  → Browse Marketplace
Service issue    → View Status / Inspect Logs
```

原则：

> 页面同一时刻尽量只有一个真正明显的 Primary Action。

---

## 7. Page Structure

推荐信息层级：

```text
Page Header
↓
System / Account Conclusion
↓
4 Primary Metrics
↓
Needs Attention（有问题时才出现）
↓
Models in Use
↓
Recent Activity
↓
Secondary links
```

不得为了填满页面而增加无明确问题的数据卡片。

---

## 8. Page Header

Header 建议包含：

```text
Overview
A concise account / usage summary
```

Header 不需要写长篇产品介绍。

右侧最多保留一个主要动作。

---

## 9. Account Conclusion

页面应该优先形成一句用户可理解的结论。

正常示例：

```text
Your API usage is healthy.
```

需要关注示例：

```text
Your balance is running low.
```

异常示例：

```text
Some requests are experiencing elevated failures.
```

结论应来自真实数据，不允许根据缺失数据推断“Healthy”。

如果状态未知，应明确：

```text
Usage health is temporarily unavailable.
```

---

## 10. Needs Attention

`Needs Attention` 不是固定占位区。

只有存在值得用户处理的问题时才显示。

适合进入该区域的事件：

- Balance low / exhausted
- API key missing / expired / disabled
- 异常失败率
- 模型不可用
- 明显的 Usage spike
- 充值失败
- 账户限制

每条告警必须回答：

```text
发生了什么
↓
有什么影响
↓
BurnCloud 已经做了什么
↓
用户还需要做什么
```

禁止：

```text
Error 503
Provider failed
Node offline
```

直接作为 Buyer 首页主要文案。

---

## 11. Models in Use

该区域回答：

> 我现在主要在使用哪些模型？

默认只展示少量高价值信息：

- Model name
- Tier（Economy / Standard / Performance）
- Today usage
- Today spend（可选）
- Status

示例：

```text
DeepSeek V3     Standard     Healthy
Qwen            Economy      Healthy
GLM             Performance  Degraded
```

点击模型可进入模型详情、Usage 或 Marketplace 对应页面。

不得展示：

- GPU type
- GPU count
- Supplier identity
- IDC name
- Deployment topology
- Worker group

---

## 12. Recent Activity

该区域回答：

> 最近发生了什么值得我知道？

只显示对 Buyer 有意义的事件，例如：

- Recharge completed
- New API key created
- Model usage started
- Significant failure incident
- Balance warning
- Service recovered

不要把所有 API Request 都塞入 Overview。

完整请求级数据属于 `Logs`。

---

## 13. Secondary Navigation

Overview 可以提供轻量入口：

```text
Marketplace
Playground
Usage
Billing
Logs
```

但这些入口不得形成第二套 Sidebar。

Overview 负责摘要与引导，不复制其它一级页面。

---

## 14. Empty State — New Buyer

新 Buyer 没有任何使用记录时，不应该看到大量 `0`。

推荐流程：

```text
Welcome to BurnCloud
↓
1. Add balance
2. Choose a model
3. Create an API key
4. Send your first request
```

但仍应保持极简，不做冗长 onboarding dashboard。

推荐 Primary CTA：

```text
Browse Marketplace
```

如果余额为零且平台要求先充值，则优先：

```text
Add Balance
```

---

## 15. Loading State

Loading 时必须保持布局稳定。

推荐：

- Page Header 立即显示
- 四个核心指标使用 Skeleton
- `Models in Use` 和 `Recent Activity` 独立加载

禁止整个页面只出现一个中央 Spinner。

如果部分数据加载失败，其他成功数据仍应展示。

---

## 16. Error State

### 16.1 Partial Failure

例如 Billing 正常、Usage API 失败：

- 保留可以确认的数据
- 对失败模块显示 `Unavailable`
- 不将整个 Overview 判定为失败

### 16.2 Critical Failure

如果无法确认余额、鉴权或服务可用性等关键数据：

- 明确状态未知
- 不显示虚假的 `0`
- 不显示虚假的 `Healthy`
- 提供 Retry 或相关入口

---

## 17. Balance States

建议至少定义：

```text
Healthy
Low
Critical
Exhausted
Unknown
```

阈值由 Billing / Product 配置决定，不写死在 UI。

UI 只消费标准化状态。

### Low

温和提醒，不抢占整个页面。

### Critical

显示 Needs Attention，并将 Top Up 提升为 Primary Action。

### Exhausted

清楚说明：

- 当前余额不足
- 哪些请求会受影响
- 如何恢复

---

## 18. Availability States

Buyer 看到的是服务级状态，而不是基础设施级状态。

推荐：

```text
Healthy
Degraded
At Risk
Unavailable
Unknown
```

如果 BurnCloud 已经通过自动调度恢复，应强调结果：

```text
Service recovered

BurnCloud moved traffic to healthy capacity automatically.
```

不要求 Buyer 理解具体调度动作。

---

## 19. Autopilot Behavior

Buyer Overview 是 BurnCloud Autopilot 的结果界面，不是基础设施操作界面。

系统可以主动完成：

- Route around unhealthy capacity
- Switch healthy model capacity
- Scale model capacity
- Restore availability

Buyer 看到的是：

```text
Problem detected
↓
BurnCloud action
↓
Current result
```

而不是：

```text
请选择 Supplier
请选择 GPU Pool
请选择 Worker
请重新配置 Route
```

---

## 20. Intentionally Hidden Information

Buyer Overview 明确禁止默认展示：

```text
GPU model
GPU count
GPU memory
Supplier name
Supplier revenue share
IDC name
External rental provider
AutoDL machine details
Deployment topology
Scheduler score
Supplier reliability score
Worker IDs
Internal route IDs
Raw infrastructure health
Internal margin
Supplier settlement
```

这些信息属于 Supplier / Admin / internal system domain。

---

## 21. Visual Rules

Buyer Overview 使用 Product Standard 中定义的：

- Medium density
- Apple-style restraint
- Stripe-style professionalism
- Black / White / Gray as base
- One restrained BurnCloud accent color

顶部四个指标不应该做成四张巨大彩色卡片。

允许通过：

- Typography
- spacing
- divider
- subtle surface

形成层级。

状态色仅用于状态表达。

---

## 22. Chart Rules

Overview 默认不需要大量图表。

如果加入趋势图，必须明确回答问题。

允许示例：

```text
Spend trend
Token usage trend
```

优先使用：

```text
Line
Area
Sparkline
```

不使用：

```text
Pie
Donut
Radar
Gauge
3D chart
```

除非未来有明确批准的产品理由。

---

## 23. Responsive Priority

窄屏优先级：

```text
1. Account conclusion
2. Balance / critical warning
3. Primary metrics
4. Primary action
5. Needs Attention
6. Models in Use
7. Recent Activity
```

不得为了保留桌面四列布局而压缩数字和标签到不可读。

---

## 24. Accessibility

必须满足：

- 状态不能只依赖颜色
- Metric 有可读 label
- CTA 可键盘访问
- Alert 可被辅助技术识别
- Loading 状态有语义
- 错误信息明确且可操作

---

## 25. Success Condition

Buyer Overview 成功的标准不是“页面看起来漂亮”。

成功条件：

> 一个第一次打开页面的 Buyer，可以在 5 秒内回答：今天花了多少、余额多少、API 是否稳定、今天用了多少 Token，以及自己是否需要采取行动。

---

## 26. Verification Checklist

实现或修改 Buyer Overview 后至少验证：

```text
[ ] 四个核心指标存在且顺序正确
[ ] Today Spend 语义正确
[ ] Balance 不把未知值显示成 0
[ ] API Availability 是 Buyer service-level status
[ ] Tokens Today 使用真实 Usage 数据
[ ] Low Balance 能触发明确提醒
[ ] Critical Balance 能提升 Top Up 为 Primary CTA
[ ] 新 Buyer 不出现一整页无意义的 0
[ ] Models in Use 不暴露 GPU / Supplier 信息
[ ] Recent Activity 不退化成完整 Logs
[ ] Partial API failure 不导致整页失败
[ ] Unknown 状态不会被伪装成 Healthy
[ ] 页面最多一个明显 Primary Action
[ ] Loading 不导致布局大幅跳动
[ ] 状态不只依赖颜色
[ ] 窄屏仍能优先看到结论与核心指标
```

---

## 27. Agent Change Rules

UI Agent 修改本页面前必须确认：

```text
User = Buyer
Primary question = Is my usage healthy and do I need to act?
Primary metrics = Today Spend / Balance / API Availability / Tokens Today
```

以下改变必须经过 Product Gate：

- 修改四个核心指标
- 增加新的 Buyer 一级概念
- 默认暴露 GPU / Supplier / IDC 信息
- 将 Overview 变成基础设施 Dashboard
- 将 Marketplace / Logs / Billing 的完整功能复制进 Overview
- 引入多个同等级 Primary CTA

---

## 28. Golden Layout

目标结构：

```text
Overview

Your API usage is healthy.

Today Spend     Balance        API Availability     Tokens Today
$...            $...           Healthy              ...

[Needs Attention — only when necessary]

Models in Use
DeepSeek V3     Standard     Healthy
Qwen            Economy      Healthy

Recent Activity
Recharge completed
Service recovered
...
```

这是结构参考，不是像素级视觉稿。

---

## 29. Final Rule

Buyer Overview 永远不应该回答：

> BurnCloud 后面到底有多少 GPU？

它应该回答：

> **我的模型 API 今天是否稳定、花了多少钱、还剩多少钱、用了多少，以及我现在是否需要做什么。**

这就是 Buyer Overview v1.0 的核心。