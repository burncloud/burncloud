---
doc_id: ui.product-standard
doc_type: product-standard
truth: target
status: approved
version: 1.0
---

# BurnCloud Product & UI Standard v1.0

**Status:** Approved for implementation  
**Product:** BurnCloud  
**Positioning:** P2P GPU Supply → Automated Model Capacity → Unified API Marketplace  
**UI Direction:** Apple-style restraint + Stripe-style professionalism

## 1. Product Definition

BurnCloud 是一个分布式 AI 算力与模型服务平台。

底层供应商提供闲置 GPU、IDC GPU 或其他算力资源。

BurnCloud 自动完成：

- GPU 识别
- Benchmark
- 节点评级
- GPU Pool 编排
- 模型选择
- 模型部署
- 模型切换
- 自动扩缩容
- 外部算力补充
- API 服务
- Usage Metering
- Billing
- Revenue Allocation
- Supplier Settlement
- Reliability Management

客户购买的不是 GPU，也不是供应商 Key。

客户购买的是：**Model API Capacity**。

## 2. Core Business Flow

```text
GPU Supplier
    ↓
BurnCloud Node
    ↓
Hardware Discovery
    ↓
Benchmark
    ↓
Reliability Score
    ↓
Global Compute Pool
    ↓
BurnCloud Scheduler
    ↓
Automatic Model Deployment
    ↓
Model Capacity
    ↓
Economy / Standard / Performance
    ↓
Model Marketplace
    ↓
Unified BurnCloud API
    ↓
Buyer Prepaid Balance
    ↓
API Consumption
    ↓
Usage Metering
    ↓
BurnCloud Revenue
    ↓
Contribution Engine
    ↓
Supplier Revenue Share
    ↓
Settlement
```

当自有资源不足时：

```text
Demand ↑
    ↓
Capacity Prediction
    ↓
Capacity Shortage
    ↓
Automatic External Rental
    ↓
AutoDL / External GPU Capacity
    ↓
Temporary Compute Pool
    ↓
Deploy Model
    ↓
Restore Capacity
```

## 3. Fundamental Product Principles

### 3.1 Outcome Before Infrastructure

首先告诉用户：**现在是什么情况。**

然后才告诉用户：**为什么。**

最后才展示底层技术细节。

禁止首页首先展示大量 GPU、节点、驱动、VRAM、Worker 等基础设施数据。

### 3.2 Hide Unnecessary Complexity

Buyer 不需要理解：

- GPU 型号
- GPU 数量
- IDC
- Supplier
- CUDA
- NPU
- Worker Group
- Deployment topology

Buyer 首先看到：

- Model
- Price
- Availability
- Latency
- Usage
- Balance

### 3.3 Supplier Should Not Operate AI Infrastructure

Supplier 的目标不是成为 AI 运维工程师。

Supplier 应该：

```text
安装 BurnCloud Node
→ 接入机器
→ 通过检测
→ 保持在线
→ 获得收入
```

不要求 Supplier：

- 选择模型
- 部署模型
- 选择推理框架
- 调整并行参数
- 手动迁移模型
- 手动分配流量

这些全部由 BurnCloud 自动完成。

### 3.4 BurnCloud Is an Autopilot

BurnCloud 不只是展示问题。

BurnCloud 应该逐步做到：

```text
发现问题
    ↓
理解问题
    ↓
预测影响
    ↓
制定动作
    ↓
自动执行
    ↓
验证结果
```

正常、低风险操作尽量自动完成。

### 3.5 Human by Exception

人主要处理：

- 高风险操作
- 商业合同
- 大规模资金操作
- 异常供应商争议
- 重大基础设施变化
- 安全问题
- 系统置信度不足的决策

原则：**正常情况自动运行，异常情况才呼叫人。**

## 4. Role Model

BurnCloud 有三种主要角色：

```text
Buyer
Supplier
Admin
```

一个账号未来可以拥有多个角色。

## 5. Buyer Mental Model

Buyer 的世界只有：

```text
钱
↓
模型
↓
API
↓
使用情况
```

Buyer 不应该感受到复杂 GPU Marketplace。

Buyer 购买的是：**模型服务。**

## 6. Buyer Navigation

一级菜单固定为：

```text
Overview
Playground
Marketplace
API Keys
Usage
Billing
Logs
```

禁止继续把以下概念重新暴露成 Buyer 一级菜单：

```text
Providers
Channels
Routes
Workers
Deployments
GPU Pools
```

## 7. Buyer Overview Contract

Buyer 打开 Overview 后，5 秒内必须知道：

1. 今天花了多少钱
2. 还有多少钱可以使用
3. API 是否稳定
4. 今天用了多少 Token

顶部只展示四个核心指标：

```text
Today Spend
Balance
API Availability
Tokens Today
```

下方最多两个主要区域：

```text
Models in Use
Recent Activity
```

当余额不足时：**Recharge / Top Up 必须成为页面最高优先级 CTA。**

## 8. Buyer Marketplace

Marketplace 默认应该像模型商店，而不是基础设施控制台。

### Default Mode

模型卡片只展示必要信息：

```text
DeepSeek V3

Price
Availability
Performance Tier

[Use Model]
```

默认禁止出现：

- GPU 数
- Supplier 数
- Supplier 名称
- IDC 名称
- Worker 数
- Pool 拓扑

## 9. Marketplace Advanced Mode

点击 Advanced 后才允许展示：

- Real-time latency
- Historical availability
- Current load
- Context length
- Benchmark
- Version
- Tier details
- Regional information

仍然原则上不暴露供应商商业身份。

## 10. Performance Tiers

每个支持的模型可以提供：

```text
Economy
Standard
Performance
```

### Economy

- 成本优先
- 允许更高延迟
- 适合批处理
- 使用更经济的算力组合

### Standard

- 默认推荐
- 成本和性能平衡
- 大多数用户默认使用

### Performance

- 低延迟
- 高可用
- 更大 Capacity Headroom
- 优先调度高质量资源

默认：**Standard**。

专业用户可以主动切换。

底层 GPU 选择永远由 BurnCloud 完成。

## 11. Buyer Playground

Playground 是一级入口。

作用不是 Demo，而是：**让用户快速完成第一次有效 API 使用。**

用户应该能够：

```text
选择模型
↓
选择 Tier
↓
输入 Prompt
↓
Send
↓
看到结果
↓
看到 Usage
↓
生成对应 API 示例
```

Playground 必须和真实生产能力保持一致。

禁止存在：“Playground 能调用，但 API 实际不可用。”

## 12. Supplier Mental Model

Supplier 的世界应该只有：

```text
机器
↓
健康状态
↓
贡献
↓
收入
```

Supplier 最重要的问题：

- 我的机器正常吗？
- 今天赚了多少钱？

## 13. Supplier Navigation

```text
Overview
Resources
Deployments
Earnings
Settlements
Reliability
Settings
```

Deployments：**只读。**

Supplier 可以知道 BurnCloud 当前在自己的机器上运行什么。

Supplier 不可以直接改变 Deployment。

## 14. Supplier Overview Contract

顶部建议四个指标：

```text
Today Earnings
Online GPUs
GPU Utilization
Inference Today
```

下面优先：

```text
Needs Attention
Revenue Trend
Resource Health
```

首页不展示大量底层参数。

## 15. Supplier Onboarding

Supplier 可以自助加入。

首次体验应该极其简单：

```text
Start earning with your GPUs

1. Install BurnCloud Node
2. Connect machine
3. Run benchmark
4. Start earning
```

供应商第一次加入默认属于较低信任等级。

## 16. Supplier Levels

### Level 1 — Community

- 自助加入
- 基础验证
- 低调度优先级
- 风险控制最严格

### Level 2 — Verified

- 身份验证
- 机器验证
- 网络验证
- 有一定稳定运行历史

### Level 3 — Professional

- 大规模资源
- 长期稳定
- 良好 SLA
- 更高调度机会

### Level 4 — Strategic

- IDC
- 大型合作伙伴
- 商务签约
- 高稳定度
- 核心 Capacity Provider

等级可以影响：

- Scheduling Weight
- Traffic Opportunity
- Risk Threshold
- Settlement Cycle
- Capacity Priority

等级不直接决定 Revenue Share。

## 17. Supplier Revenue Share

每个 Supplier 可以有独立商业比例，例如：

```text
Supplier A
70%

Supplier B
60%

Supplier C
80%
```

Supplier UI 可以明确显示：

```text
Your Revenue Share
70%
```

禁止隐藏核心收益计算规则。

## 18. Contribution Model

所有 Supplier 的 GPU 可以共同组成 Compute Pool。

收入不简单按照“这个 Buyer 属于哪个 Supplier”，而是：

```text
Model Revenue
    ↓
Actual Compute Contribution
    ↓
Contribution Weight
    ↓
Supplier Revenue Share
    ↓
Final Supplier Earnings
```

Earnings 页面允许一直下钻：

```text
Revenue
↓
Model
↓
Usage
↓
Cluster
↓
GPU
```

## 19. Supplier Graceful Shutdown

Supplier 可以申请下线资源。

正常流程：

```text
Request Offline
↓
Drain
↓
Stop New Tasks
↓
Finish Existing Work
↓
Release Deployment
↓
Offline
```

异常断电、直接拔机器、频繁掉线等行为：

```text
Reliability ↓
Scheduling Weight ↓
Revenue Opportunity ↓
```

必要时可以产生收益处罚。

## 20. Reliability

Supplier 看到简化等级。

Admin 可以看到完整评分。

评分可以综合：

- Availability
- Unexpected Offline Rate
- Performance Stability
- Network Stability
- Benchmark History
- Task Completion
- Hardware Errors
- Operating History

禁止把一个复杂内部评分直接作为唯一真相展示给 Supplier。

Supplier 应优先看到：

```text
Excellent
Good
Needs Attention
At Risk
```

再允许查看原因。

## 21. Admin Mental Model

Admin 不应该以“服务器管理员”的方式理解系统。

Admin 应该以：

```text
Supply
↓
Capacity
↓
Demand
↓
Economics
```

理解 BurnCloud。

## 22. Admin Navigation

固定一级菜单：

```text
Overview

Supply
Capacity
Demand
Models

Revenue
Settlements

Suppliers
Customers

Operations
Settings
```

## 23. Admin Overview

Admin 首页是：**Business + Infrastructure Command Center**。

顶部固定四个核心指标：

```text
Today Revenue
Gross Margin
Online GPU Capacity
API Availability
```

## 24. Admin Overview Sections

### Supply Health

回答：GPU Supply 健康吗？

### Demand Pressure

回答：哪些模型需求正在快速上涨？

### Capacity Risk

回答：哪些模型快没有足够容量？

### Economics

回答：

- 现在赚多少钱？
- 外部算力成本是否开始侵蚀毛利？

## 25. Needs Attention

Admin Overview 必须具备主动判断能力。

不能只是：

```text
DeepSeek Utilization
91%
```

而应该：

```text
Capacity risk

DeepSeek Standard capacity is approaching its safe limit.

Demand increased 24% during the last hour.
```

更成熟时进一步：

```text
BurnCloud has detected a capacity shortage.

8 temporary GPUs can be rented automatically.

Expected impact:
Capacity +18%
Margin -1.8%

[Approve]
```

最终目标：对于低风险动作甚至无需 Approve。

## 26. Autopilot Principle

BurnCloud 最终应该做到：

```text
Observe
↓
Predict
↓
Decide
↓
Act
↓
Verify
```

例如：

```text
DeepSeek Demand ↑
↓
Capacity Prediction
↓
Capacity不足
↓
寻找供应商闲置资源
↓
不够
↓
租 External GPU
↓
自动部署
↓
加入 Pool
↓
Capacity恢复
```

Admin 收到的是：

```text
Capacity restored

BurnCloud added temporary capacity to DeepSeek Standard.

Expected incremental cost:
$X/hour
```

而不是：请管理员自己去租机器。

## 27. Model Deployment

模型由 BurnCloud 全自动部署。

Supplier 不参与部署决策。

Scheduler 应根据：

- GPU type
- VRAM
- GPU count
- model architecture
- expected demand
- measured TPS
- latency
- cost
- power efficiency
- network
- reliability
- current utilization
- external rental price
- expected margin

自动计算最优方案。

## 28. Dynamic Model Switching

GPU 不固定绑定某个模型。

BurnCloud 可以根据 Demand 自动：

```text
Qwen Capacity ↓
DeepSeek Demand ↑

GPU Pool
↓
Drain Qwen
↓
Unload
↓
Deploy DeepSeek
↓
Benchmark
↓
Join DeepSeek Pool
```

UI 不应该要求管理员手工完成这套流程。

## 29. Pricing Rules

模型存在官方 API 标准价格时：**优先跟随官方公开标准定价。**

没有官方价格时：**由 BurnCloud 统一制定价格。**

Supplier 不能自己向 Buyer 定价。

## 30. Buyer Billing

第一阶段：**Prepaid**。

流程：

```text
Recharge
↓
Balance
↓
API Consumption
↓
Real-time Usage
↓
Balance Deduction
```

余额不足：

```text
Warning
↓
Critical
↓
Traffic Protection
```

页面必须提前提醒。

## 31. Visual Philosophy

总体：**极简 + 专业**。

参考精神：

```text
Apple
+
Stripe
```

不是照抄视觉。

原则：

- 克制
- 安静
- 精准
- 专业
- 不炫技
- 不制造视觉噪音

## 32. Density

默认：**Medium Density**，接近 Stripe。

首页可以更松。

专业管理页可以适度增加信息密度。

禁止：

- 大量无意义留白
- 超高密度传统云控制台
- 每个区域都塞数据

## 33. Color System

主体：

```text
Black
White
Gray
```

品牌颜色：只使用一个克制的 BurnCloud Accent Color。

状态颜色：

```text
Green  = Healthy
Yellow = Warning
Red    = Critical
Blue   = Informational
```

禁止为了视觉效果大量使用渐变和彩色卡片。

## 34. Cards

Card 只用于：**有明确逻辑边界的信息组。**

禁止所有东西都放卡片，避免形成普通 SaaS Dashboard Template 风格。

## 35. Tables

普通页面默认提供：

```text
Search
Filter
Sort
```

Admin 高级页面根据需要允许：

```text
Export
Column Selection
Bulk Actions
```

禁止默认给普通用户复杂 Excel 式操作体验。

## 36. Chart Rules

每个 Chart 必须回答一个明确问题。

推荐：

```text
Line
Area
Bar
Sparkline
```

谨慎或默认禁止：

```text
Pie
Donut
Radar
Gauge
3D Chart
```

禁止为了“让 Dashboard 看起来高级”而增加图表。

## 37. Page Hierarchy

标准页面结构：

```text
Page Header

Primary Summary

Primary Content

Secondary Detail

Empty / Loading / Error

Danger / Advanced
```

每个页面必须明确：

```text
用户为什么来到这里？
第一眼应该知道什么？
最重要动作是什么？
成功以后发生什么？
```

## 38. CTA Hierarchy

每个页面尽量只有一个明显 Primary Action。

层级：

```text
Primary
Secondary
Tertiary
Danger
```

禁止满屏同等级按钮。

## 39. Content Standard

文案必须优先表达：

```text
发生了什么
↓
有什么影响
↓
系统做了什么
↓
用户需要做什么
```

错误示例：

```text
Node Error
Status: Offline
```

推荐：

```text
12 GPUs went offline unexpectedly.

Traffic has already been moved to healthy capacity.

Review the affected supplier if the outage continues.
```

## 40. Intelligent Copy

由于 BurnCloud 是 Autopilot，系统文案应该强调：**结论，而不是原始数据。**

不要：

```text
Utilization 93%
```

应该：

```text
Capacity is running low

DeepSeek Standard is using 93% of available capacity.
```

如果已经自动解决：

```text
Capacity restored

BurnCloud added temporary capacity after demand increased.
```

## 41. Status Design

统一状态语言。

推荐：

```text
Healthy
Ready
Running
Degraded
At Risk
Offline
Draining
Provisioning
Deploying
Scaling
```

相同状态不得在不同页面使用不同名称。

## 42. Empty State

Empty State 不应该只写：

```text
No data
```

必须解释：**为什么为空 + 下一步做什么**。

例如：

```text
No models in use yet

Open Marketplace to choose your first model.

[Browse Marketplace]
```

## 43. Loading

Loading 应：

- 保持布局稳定
- 尽量使用 Skeleton
- 避免整个页面闪烁
- 长任务展示真实阶段

例如：

```text
Benchmarking GPU
↓
Preparing runtime
↓
Deploying model
↓
Verifying inference
```

## 44. Error States

### Recoverable

系统可以自动处理。优先自动恢复。

### User Action Required

明确告诉用户下一步。

### Critical

必须突出：

- 风险
- 影响
- 建议
- 是否已经保护流量

## 45. Advanced Mode

复杂能力原则上采用：

```text
Simple by default
Powerful when needed
```

专业参数统一进入：

```text
Advanced
Details
Inspect
```

而不是默认显示。

## 46. Design System

BurnCloud 应逐步统一核心组件：

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

Agent 优先使用已有组件。

禁止重复创造相同 Pattern。

## 47. UI Engineering Priority

遇到冲突时，优先级为：

```text
User Goal
>
Correctness
>
Information Hierarchy
>
Action Clarity
>
State Completeness
>
Consistency
>
Accessibility
>
Visual Beauty
```

漂亮永远不能压过可理解性。

## 48. AI Agent UI Rules

任何 UI Agent 在修改页面前必须依次��答：

```text
1. Who is the user?
2. Why did they open this page?
3. What must they know first?
4. What is the primary action?
5. What existing component should be reused?
6. What states must exist?
7. What is intentionally hidden?
8. How will the result be verified?
```

未回答完成，不进入 Coding。

## 49. UI Graph

正式 UI Graph：

```text
Task
 ↓
Discovery
 ↓
User Goal Analysis
 ↓
Page Contract
 ↓
UX Plan
 ↓
Component Resolution
 ↓
Implementation
 ↓
Compile
 ↓
Functional Verification
 ↓
UI Convention Check
 ↓
Product UX Check
 ↓
Visual QA
 ↓
Accessibility
 ↓
Reviewer
 ↓
FAIL ─→ Fix ─→ Verify
 ↓
PASS
 ↓
PR
```

## 50. Human Gates

原则上只保留三个主要人工 Gate。

### Product Gate

涉及：

- 一级 IA
- 核心业务逻辑
- 定价模式
- 新核心用户流程

### Design System Gate

涉及：

- 新的全局 Pattern
- 新核心组件
- 大规模视觉体系改变

### High-Risk Release Gate

涉及：

- Billing
- Settlement
- Supplier Revenue
- Model Marketplace
- 自动租 GPU
- 大规模自动化执行
- 危险基础设施操作

## 51. Golden Pages

第一阶段优先建立三个 Golden Page：

```text
Buyer Overview
Supplier Overview
Admin Overview
```

之后建立：

```text
Marketplace
Supplier Resources
Admin Capacity
```

所有后续页面应以 Golden Page 的以下属性作为参考：

- Typography
- Spacing
- Hierarchy
- Component usage
- Status representation
- CTA hierarchy
- Density

## 52. Product North Star

Buyer 的目标：

> 打开 BurnCloud，快速找到模型，充值，获得 API，然后稳定使用。

Supplier 的目标：

> 安装 BurnCloud Node，把闲置 GPU 接进来，然后稳定赚钱。

Admin 的目标：

> 不需要亲自管理每张 GPU，而是管理 Supply、Capacity、Demand 和 Economics。

BurnCloud 的目标：

> 自动把世界上分散的 GPU，持续转化成稳定、统一、可购买的大模型 API Capacity。

## 53. Final UI Philosophy

BurnCloud 不应该让用户感觉：

> 这里有很多 GPU 和复杂基础设施。

应该让 Buyer 感觉：

> **我随时可以买到稳定的大模型能力。**

让 Supplier 感觉：

> **我的 GPU 接上去就能持续产生收益。**

让 Admin 感觉：

> **BurnCloud 正在自动运行这家 AI 基础设施公司，我只需要处理真正重要的事情。**

这就是 BurnCloud UI v1.0 的核心。
