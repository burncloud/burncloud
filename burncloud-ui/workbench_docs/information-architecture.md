---
doc_id: ui.information-architecture
doc_type: product-architecture
truth: target
status: approved
version: 1.0
parent: docs/ui/product-standard.md
---

# BurnCloud Information Architecture v1.0

## 1. Purpose

本文档定义 BurnCloud 目标产品的信息架构（Information Architecture, IA）。

它回答五个问题：

1. BurnCloud 有哪些主要角色？
2. 每个角色进入系统后应该看到什么？
3. 每个一级页面负责什么，不负责什么？
4. Buyer、Supplier、Admin 三个世界如何保持边界清晰？
5. 后续 UI Agent 在新增页面或功能时，应该把功能放在哪里？

本文档描述的是 **目标产品结构**，不是当前代码已经实现的事实。

若本文件与 `docs/ui/product-standard.md` 冲突，以 Product Standard 的最新批准版本为准。

---

# 2. Product Mental Model

BurnCloud 不应该被用户理解成“GPU 管理后台”。

不同角色应该看到完全不同的产品世界：

```text
Buyer
Model → API → Usage → Billing

Supplier
GPU → Health → Contribution → Earnings

Admin
Supply → Capacity → Demand → Economics
```

这是 BurnCloud 信息架构的最高原则。

复杂的底层基础设施可以存在，但不应该被无差别暴露给所有用户。

---

# 3. Role Model

BurnCloud 第一阶段定义三个主要角色：

```text
Buyer
Supplier
Admin
```

## 3.1 Buyer

Buyer 是模型 API 的消费用户。

Buyer 购买的是：

> Model API Capacity

Buyer 不购买：

- GPU
- Supplier Key
- IDC 机器
- GPU Worker
- Deployment
- Route

Buyer 的核心目标：

> 快速选择模型、充值、获得 BurnCloud API Key、稳定调用模型，并清楚知道自己的消费与服务状态。

---

## 3.2 Supplier

Supplier 提供 GPU Capacity。

Supplier 的核心目标：

> 把闲置 GPU 接入 BurnCloud，保持资源稳定在线，并持续获得收入。

Supplier 不需要：

- 手工选择部署模型
- 手工安装推理框架
- 手工分配流量
- 手工扩缩容
- 手工调模型

BurnCloud 负责自动完成模型部署与资源优化。

---

## 3.3 Admin

Admin 负责平台运营、容量、收入、供应商、客户和系统风险。

Admin 不应该以“逐台服务器运维人员”的方式使用 BurnCloud。

Admin 的核心管理模型：

```text
Supply
↓
Capacity
↓
Demand
↓
Economics
```

Admin 首先管理系统级结果，只有在需要诊断时才下钻到底层 GPU、Node 和 Deployment。

---

# 4. Multi-role Accounts

一个 BurnCloud 账号可以同时拥有多个角色。

例如：

```text
同一个账号
├── Buyer
└── Supplier
```

多角色账号不应该把三套菜单混合成一个巨大的 Sidebar。

推荐模式：

```text
BurnCloud
[ Buyer ▾ ]
```

用户通过 Role / Workspace Switcher 切换：

```text
Buyer
Supplier
Admin
```

切换角色后：

- Sidebar 完整切换
- Overview 完整切换
- 搜索范围切换
- 默认首页切换
- 权限上下文切换

禁止把 Buyer、Supplier、Admin 的所有菜单同时塞入同一侧边栏。

---

# 5. Global Navigation Principles

## 5.1 Navigation Is Task-based

一级菜单按照用户任务组织，而不是按照数据库表、后端模块或代码目录组织。

禁止直接把以下后端概念作为默认 Buyer 导航：

- Providers
- Channels
- Routes
- Workers
- GPU Pools
- Deployments
- Adapter
- Runtime

除非未来明确证明这些概念属于用户任务，否则不应暴露。

---

## 5.2 Stable First-level Navigation

一级菜单属于高成本产品决策。

Agent 不得因为新增功能就自动增加一级菜单。

新增一级入口必须经过 Product Gate。

优先顺序：

```text
现有页面内新增能力
>
二级页面
>
详情页
>
Advanced
>
最后才考虑新增一级菜单
```

---

## 5.3 Simple by Default

默认导航应该表达产品，而不是表达基础设施。

高级数据通过：

```text
Advanced
Details
Inspect
```

逐层下钻。

---

# 6. Buyer Information Architecture

Buyer 一级导航固定为：

```text
Overview
Playground
Marketplace
API Keys
Usage
Billing
Logs
```

顺序表达 Buyer 的主要使用逻辑：

```text
知道当前状态
↓
测试模型
↓
发现模型
↓
获得正式访问凭证
↓
观察使用量
↓
管理资金
↓
诊断请求
```

---

# 7. Buyer — Overview

## Primary Question

> 我今天用了多少？服务现在稳定吗？

## Primary Metrics

固定四项：

```text
Today Spend
Balance
API Availability
Tokens Today
```

## Main Sections

默认最多两个主要内容区域：

```text
Models in Use
Recent Activity
```

## Primary Actions

正常状态下：

- Continue using API
- Open Marketplace

余额不足时：

- Recharge / Top Up 成为最高优先级 CTA

## Must Not Become

Buyer Overview 不是：

- GPU 监控页
- Supplier 监控页
- 系统运维页
- 模型排行榜大杂烩
- 全平台经营 Dashboard

---

# 8. Buyer — Playground

## Primary Question

> 这个模型现在能不能满足我的需求？

Playground 是一级入口，不只是 Demo。

它负责：

```text
选择模型
↓
选择 Tier
↓
输入请求
↓
真实调用
↓
看到结果
↓
看到 Usage
↓
生成对应 API 示例
```

## Core Scope

- Model selection
- Tier selection
- Prompt / Request input
- Response preview
- Latency / usage summary
- API example generation

## Boundary

Playground 不负责：

- 模型采购
- GPU 选择
- Supplier 选择
- Deployment 管理
- Billing 全量���理

这些分别属于 Marketplace、BurnCloud Scheduler 和 Billing。

---

# 9. Buyer — Marketplace

## Primary Question

> BurnCloud 有哪些模型可以用？哪个适合我？

Marketplace 默认是一个 **Model Marketplace**，而不是 GPU Marketplace。

Buyer 默认只看到模型商品。

典型卡片：

```text
DeepSeek V3

Price
Availability
Performance Tier

[Use Model]
```

## Default Mode

默认模式追求极简。

允许展示：

- Model name
- Short description
- Official / BurnCloud pricing
- Availability
- Supported tiers
- Context / major capability summary
- Primary CTA

默认隐藏：

- GPU 数量
- GPU 型号
- Supplier 数量
- Supplier 商业身份
- IDC 名称
- Worker 数量
- Deployment topology
- Internal routing

---

# 10. Buyer — Marketplace Advanced

Marketplace 详情页可以提供 `Advanced`。

Advanced 可以展示：

- Real-time latency
- Historical availability
- Current load
- Context length
- Model version
- Benchmark
- Region
- Tier differences
- Compatibility notes

即使在 Advanced 中，也不默认暴露 Supplier 商业身份。

Buyer 可以理解基础设施质量，但不需要知道背后的供应商公司是谁。

---

# 11. Buyer — Performance Tiers

模型可提供：

```text
Economy
Standard
Performance
```

默认选择：

```text
Standard
```

专业 Buyer 可以手动切换。

## Economy

面向：

- 批量任务
- 成本敏感任务
- 可接受更高延迟的任务

## Standard

面向：

- 大多数生产场景
- 成本和性能平衡

## Performance

面向：

- 延迟敏感
- 高稳定性要求
- 更高 Capacity Headroom

Tier 是 Buyer 的产品概念。

GPU 型号不是 Buyer 的产品概念。

---

# 12. Buyer — API Keys

## Primary Question

> 我如何安全地访问 BurnCloud API？

API Keys 负责：

- Create
- Name
- View metadata
- Rotate
- Revoke / Delete
- Spend limits（如产品支持）
- Scope（如产品支持）

Buyer 拿到的是 BurnCloud Credential。

Buyer 永远不应该拿到 Supplier 的真实上游 Key 或内部凭证。

---

# 13. Buyer — Usage

## Primary Question

> 我的 API 到底用在了哪里？

Usage 是消费分析页。

推荐下钻层级：

```text
Total Usage
↓
Model
↓
Tier
↓
API Key
↓
Time
```

核心数据：

- Tokens
- Requests
- Spend
- Latency
- Success rate

Usage 不负责充值，也不负责深度请求 Debug。

充值属于 Billing。

单次请求诊断属于 Logs。

---

# 14. Buyer — Billing

## Primary Question

> 我还有多少钱？已经花了多少钱？

第一阶段以 Prepaid 为核心。

Billing 负责：

- Balance
- Recharge / Top Up
- Transaction history
- Consumption statements
- Invoice / receipt（按业务支持）
- Low-balance warning

Billing 不应该承担 Usage 的所有分析功能。

Billing 关心的是钱。

Usage 关心的是使用行为。

---

# 15. Buyer — Logs

## Primary Question

> 哪一次请求出了问题？为什么？

Logs 是诊断页。

默认展示：

- Request time
- Model
- Tier
- Outcome
- Latency
- Usage
- Error summary

Advanced / Inspect 可以展示更深诊断信息。

默认不得泄露：

- Supplier Secret
- Internal Credentials
- 不必要的内部拓扑
- 安全敏感基础设施信息

---

# 16. Supplier Information Architecture

Supplier 一级导航固定为：

```text
Overview
Resources
Deployments
Earnings
Settlements
Reliability
Settings
```

Supplier 的主流程：

```text
接入资源
↓
保持健康
↓
BurnCloud 自动部署
↓
产生贡献
↓
产生收益
↓
结算
```

---

# 17. Supplier — Overview

## Primary Question

> 我的机器正常吗？今天赚了多少钱？

首页顶部核心指标：

```text
Today Earnings
Online GPUs
GPU Utilization
Inference Today
```

核心区域：

```text
Needs Attention
Revenue Trend
Resource Health
```

新 Supplier 尚未接入资源时，Overview 变成 Onboarding：

```text
Start earning with your GPUs

1. Install BurnCloud Node
2. Connect machine
3. Run benchmark
4. Start earning
```

---

# 18. Supplier — Resources

## Primary Question

> 我接入 BurnCloud 的 GPU 现在是什么状态？

Resources 负责：

- Machine / Node inventory
- GPU inventory
- Cluster grouping
- Online / Offline
- Utilization
- Hardware health
- Network health
- Benchmark status
- Graceful offline request

允许下钻：

```text
Resources
↓
Cluster
↓
Machine
↓
GPU
```

Supplier 可以申请正常下线。

禁止 Supplier 在此页面决定部署哪个模型。

---

# 19. Supplier — Deployments

Deployments 是 **只读页面**。

## Primary Question

> BurnCloud 当前在我的资源上运行什么？

允许看到：

- Current model
- Deployment state
- Resource assignment
- Since when
- Current utilization
- Current contribution

Supplier 不可以：

- 手工更换模型
- 手工修改并行策略
- 手工重启调度流程
- 自己选择 Traffic

如需下线资源，应从 Resources 发起 Graceful Offline，而不是直接删除 Deployment。

---

# 20. Supplier — Earnings

## Primary Question

> 我的收入从哪里来？

Earnings 强调收入形成过程。

允许持续下钻：

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

需要展示：

- Today earnings
- Period earnings
- Contribution
- Revenue share
- Model contribution
- Resource contribution

Supplier 可以看到自己的 Revenue Share。

---

# 21. Supplier — Settlements

## Primary Question

> 哪些收入已经可以结算？

Settlements 负责：

- Revenue share
- Pending settlement
- Eligible settlement
- Settled history
- Settlement period
- Adjustment / penalty（如存在）

禁止把 Earnings 与 Settlements 混为一页。

Earnings = 赚了多少以及为什么。

Settlements = 哪些钱已经进入结算流程。

---

# 22. Supplier — Reliability

## Primary Question

> 我的资源为什么得到当前的调度机会？

Supplier 看到简化评级：

```text
Excellent
Good
Needs Attention
At Risk
```

允许查看评分原因：

- Availability
- Unexpected offline events
- Performance stability
- Network stability
- Benchmark history
- Task completion
- Hardware errors

内部完整 Resource Score 属于 Admin / Scheduler 领域。

不要求 Supplier 理解复杂内部权重公式。

---

# 23. Supplier Levels

Supplier 有四个主要等级：

```text
Level 1 — Community
Level 2 — Verified
Level 3 — Professional
Level 4 — Strategic
```

自助注册 Supplier 默认从低等级开始。

等级可以影响：

- Scheduling Weight
- Traffic Opportunity
- Risk Threshold
- Settlement Cycle
- Capacity Priority

等级与 Revenue Share 是两个概念。

Revenue Share 由商业协议独立配置，例如：

```text
60 / 40
70 / 30
80 / 20
```

Supplier Level 不应自动改写商业分成比例。

---

# 24. Supplier — Settings

Settings 负责 Supplier 自己的配置，例如：

- Organization profile
- Settlement information
- Contact information
- Notifications
- Node installation / token management
- Allowed operational preferences

Settings 不负责 Deployment 策略。

---

# 25. Admin Information Architecture

Admin 一级导航固定为：

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

顺序表达平台经营逻辑：

```text
先看经营结果
↓
确认有没有供给
↓
确认这些供给形成多少可卖容量
↓
确认客户需求在哪里
↓
管理最终出售的模型商品
↓
观察收入和毛利
↓
完成供应商分账
↓
管理供需双方
↓
处理异常
↓
管理系统
```

---

# 26. Admin — Overview

Admin Overview 是：

> Business + Infrastructure Command Center

顶部固定四项：

```text
Today Revenue
Gross Margin
Online GPU Capacity
API Availability
```

核心区域：

```text
Supply Health
Demand Pressure
Capacity Risk
Economics
Needs Attention
```

Admin Overview 必须优先输出结论，而不是堆原始数据。

例如不要只显示：

```text
DeepSeek Standard
Utilization 93%
```

优先显示：

```text
Capacity risk

DeepSeek Standard is approaching its safe capacity limit.
```

---

# 27. Admin — Supply

## Primary Question

> BurnCloud 现在拥有��少可用 GPU Supply？

Supply 管理“资源来源”。

包括：

- Supplier-provided resources
- IDC resources
- BurnCloud-owned resources（如存在）
- External rental resources
- AutoDL / external provider capacity

下钻路径：

```text
Supply
↓
Source
↓
Supplier / External Provider
↓
Cluster
↓
Machine
↓
GPU
```

Supply 不等于 Capacity。

GPU 在线不代表已经形成可售卖 Model Capacity。

---

# 28. Admin — Capacity

## Primary Question

> 当前这些 GPU 能向客户提供多少可售卖模型能力？

Capacity 是 BurnCloud 管理员最重要的页面之一。

它应该表达：

```text
Raw GPU Supply
↓
Deployable Capacity
↓
Active Model Capacity
↓
Headroom
↓
Risk
```

主要关注：

- Capacity by model
- Capacity by tier
- Current utilization
- Headroom
- Projected shortage
- External capacity in use
- Cost of added capacity

Capacity 页面不能退化成 GPU Inventory。

---

# 29. Admin — Demand

## Primary Question

> 客户现在真正需要什么？

Demand 负责：

- Requests
- Tokens
- Spend
- Growth
- Model demand
- Tier demand
- Time-of-day patterns
- Forecast

Demand 应帮助系统和 Admin 判断：

```text
哪些模型需求上涨？
哪些 Tier 供不应求？
哪些模型容量闲置？
下一步需要增加什么 Capacity？
```

---

# 30. Admin — Models

## Primary Question

> BurnCloud 正在出售哪些模型产品？

Models 是产品目录管理，而不是 GPU Deployment 页。

负责：

- Model catalog
- Model version
- Availability
- Pricing
- Official price tracking
- BurnCloud-defined price
- Supported tiers
- Marketplace visibility
- Deprecation / rollout

定价原则：

```text
存在官方默认定价
→ 跟随官方默认定价

没有官方默认定价
→ BurnCloud 统一定价
```

Supplier 不参与 Buyer 的模型定价。

---

# 31. Admin — Revenue

## Primary Question

> BurnCloud 今天卖了多少钱，赚了多少钱？

Revenue 负责：

- Gross revenue
- Gross margin
- Model revenue
- Tier revenue
- External GPU cost
- Supplier revenue allocation
- Margin trend

推荐下钻：

```text
Revenue
↓
Model
↓
Tier
↓
Cost source
```

Revenue 页面是平台经营分析页，不是 Supplier 单笔结算页。

---

# 32. Admin — Settlements

## Primary Question

> 平台现在应该向哪些 Supplier 分多少钱？

Settlements 负责：

- Supplier revenue share
- Contribution-based allocation
- Settlement periods
- Payable amount
- Adjustments
- Penalties
- Settlement status
- Settlement history

每个 Supplier 可以配置不同 Revenue Share。

例如：

```text
Supplier A 70%
Supplier B 60%
Supplier C 80%
```

Contribution 与 Revenue Share 必须分开计算。

---

# 33. Admin — Suppliers

## Primary Question

> 哪些供应商值得 BurnCloud 继续依赖？

Suppliers 负责供应商商业与运营视角：

- Supplier profile
- Level
- Revenue share
- Reliability
- Total resources
- Contribution
- Earnings
- Settlement status
- Contract metadata
- Risk status

Supplier 页面不是单纯 GPU 列表。

GPU 属于 Supply / Resource detail。

Supplier 是商业主体。

---

# 34. Admin — Customers

## Primary Question

> 谁在购买 BurnCloud 的模型能力？

Customers 负责：

- Customer profile
- Balance
- Spend
- Usage
- API status
- Risk / abuse state
- Account status

客户请求级 Debug 应跳转 Logs / Operations，而不是把 Customers 做成日志页。

---

# 35. Admin — Operations

## Primary Question

> 哪些异常需要人处理？

Operations 是异常与人工介入中心。

它应该聚合：

- Failed automation
- Capacity incidents
- Deployment failures
- Supplier outages
- Billing anomalies
- Settlement anomalies
- Security / abuse incidents
- External rental failures

Operations 不应该成为所有正常管理功能的垃圾桶。

正常业务仍然回到各自领域页面处理。

---

# 36. Admin — Settings

Settings 负责平台级配置，例如：

- Platform configuration
- Global policies
- Notification rules
- Billing defaults
- Settlement defaults
- External providers
- Automation limits
- Autopilot policies
- Safety / approval thresholds

高风险设置必须进入明确的 Advanced / Danger Zone。

---

# 37. Autopilot in the Information Architecture

BurnCloud 的核心能力不是“显示”，而是：

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

因此 IA 不应该为每个自动化动作都创建一个人工操作页面。

例如：

```text
Demand increases
↓
Capacity risk detected
↓
BurnCloud reallocates idle GPU
↓
Still insufficient
↓
BurnCloud rents temporary external GPU
↓
Deploys model
↓
Capacity restored
```

Admin 首先应该在 Overview / Capacity / Operations 看到结果与异常。

只有需要解释或审批时才下钻。

---

# 38. Human Intervention Model

BurnCloud 默认遵循：

> Human by exception

正常、低风险动作自动完成。

人工介入主要发生在：

- Large financial impact
- Large external rental cost
- Contract changes
- Supplier punishment / suspension
- Settlement disputes
- Security incidents
- Irreversible infrastructure operations
- Low-confidence automation decisions

UI 应明确区分：

```text
Auto-resolved
Action recommended
Approval required
Critical manual action
```

---

# 39. Buyer Visibility Boundary

Buyer 默认可以看到：

- Model
- Price
- Tier
- Availability
- Latency
- Usage
- Billing
- Request result

Buyer 默认不应该看到：

- Supplier name
- Supplier revenue share
- Supplier contract
- GPU inventory
- Exact machine count
- GPU pool topology
- Internal scheduler decision
- External rental source
- Internal infrastructure credentials

原则：

> Buyer 买模型能力，而不是研究 BurnCloud 的供应链。

---

# 40. Supplier Visibility Boundary

Supplier 可以看到：

- 自己的 GPU
- 自己的资源状态
- 自己的 Deployment（只读）
- 自己的 Contribution
- 自己的 Revenue Share
- 自己的 Earnings
- 自己的 Settlements
- 自己的 Reliability

Supplier 不应该看到：

- 其他 Supplier 的商业协议
- 其他 Supplier 的收益
- Buyer 私密数据
- 全平台内部 Scheduler 权重
- BurnCloud 内部利润规则
- 其他供应商的敏感基础设施信息

---

# 41. Admin Visibility Boundary

Admin 可以根据权限下钻完整平台信息。

但 Admin UI 仍然遵循：

```text
Conclusion
↓
Business impact
↓
System action
↓
Details
↓
Raw infrastructure
```

完整权限不等于首页必须显示全部信息。

---

# 42. Cross-page Ownership Rules

当一个功能不知道应该放在哪里时，使用以下判断：

```text
“我要买/选模型”
→ Marketplace

“我要试一下模型”
→ Playground

“我要创建正式访问凭证”
→ API Keys

“我要看用了多少”
→ Usage

���我要充值/看余额”
→ Billing

“我要查哪次请求失败”
→ Logs

“我要看 Supplier 的机器”
→ Supplier Resources / Admin Supply

“我要看这些机器现在跑什么”
→ Supplier Deployments / Admin Capacity detail

“我要看 GPU 最终形成多少模型能力”
→ Capacity

“我要看客户需求增长”
→ Demand

“我要看 BurnCloud 卖了多少钱”
→ Revenue

“我要看该给 Supplier 多少钱”
→ Settlements

“我要处理自动化失败”
→ Operations
```

---

# 43. Search Scope

全局搜索应该遵循当前角色。

Buyer 搜索：

- Models
- API Keys
- Logs
- Billing records

Supplier 搜索：

- Resources
- Clusters
- GPUs
- Deployments
- Settlement records

Admin 搜索：

- Suppliers
- Customers
- Models
- Resources
- Incidents
- Settlements

Role Switch 后搜索索引同步切换。

---

# 44. Detail Page Rule

不要为了每个对象创建一级菜单。

推荐关系：

```text
List Page
↓
Detail Page
↓
Advanced / Inspect
```

例如：

```text
Suppliers
↓
Supplier A
↓
Resources / Contribution / Settlement / Reliability
```

而不是创建：

```text
Suppliers
Supplier Resources
Supplier Earnings
Supplier Reliability
Supplier Contracts
```

多个一级菜单。

---

# 45. Information Density by Role

## Buyer

低到中等密度。

优先简单、明确、结论化。

## Supplier

中等密度。

首页极简，Resources / Earnings 可专业下钻。

## Admin

中等到较高密度。

但仍然优先结论，不允许退化成传统云控制台式信息堆积。

---

# 46. Mobile / Narrow Screen Priority

窄屏环境下优先保留：

1. 状态
2. 核心指标
3. Primary Action
4. Needs Attention
5. Main content

高级筛选、列管理、批量操作可以折叠或移入菜单。

不要为了保留桌面表格布局而强制横向挤压所有信息。

---

# 47. IA Change Gate

以下改动必须经过 Product Gate：

- 新增一级菜单
- 删除一级菜单
- 改变 Buyer / Supplier / Admin 的核心 Mental Model
- 将 GPU / Supplier 信息暴露给 Buyer
- 改变 Marketplace 商品单位
- 改变 Buyer 购买对象
- Supplier 获得 Deployment 控制权
- Admin 从 Capacity 视角退回纯 Infrastructure 视角
- 大幅改变角色之间的信息边界

Agent 不得自行批准这些改变。

---

# 48. Page Contract Requirement

每个一级页面必须有独立 Page Contract。

Page Contract 至少回答：

```text
User
User goal
Primary question
Primary information
Primary action
Secondary actions
Empty state
Loading state
Error state
Advanced information
Intentionally hidden information
Success condition
Verification
```

计划中的初始 Page Contracts：

```text
page-contracts/
��── buyer-overview.md
├── buyer-playground.md
├── buyer-marketplace.md
├── buyer-api-keys.md
├── buyer-usage.md
├── buyer-billing.md
├── buyer-logs.md
├── supplier-overview.md
├── supplier-resources.md
├── supplier-deployments.md
├── supplier-earnings.md
├── supplier-settlements.md
├── supplier-reliability.md
├── admin-overview.md
├── admin-supply.md
├── admin-capacity.md
├── admin-demand.md
├── admin-models.md
├── admin-revenue.md
├── admin-settlements.md
├── admin-suppliers.md
├── admin-customers.md
└── admin-operations.md
```

---

# 49. Golden User Flows

## Buyer Golden Flow

```text
Overview
↓
Marketplace
↓
Model Detail
↓
Playground
↓
API Key
↓
Production API
↓
Usage
↓
Billing
↓
Logs when needed
```

## Supplier Golden Flow

```text
Create Supplier account
↓
Install BurnCloud Node
↓
Resource detected
↓
Benchmark
↓
Resource accepted
↓
BurnCloud deploys automatically
↓
Contribution starts
↓
Earnings
↓
Settlement
```

## Admin Golden Flow

```text
Overview detects pressure
↓
Demand explains why
↓
Capacity shows shortage
↓
Autopilot adds / reallocates supply
↓
Capacity recovers
↓
Revenue shows economics
↓
Settlements allocate supplier share
```

---

# 50. Final IA Rule

如果一个 UI 设计让：

- Buyer 开始思考 GPU
- Supplier 开始手工部署模型
- Admin 开始逐台机器做日常调度

那么这个信息架构方向就是错误的。

正确的 BurnCloud 应该让三类用户分别感受到：

> **Buyer：我在购买稳定的大模型能力。**

> **Supplier：我的 GPU 接上 BurnCloud 就能持续产生收益。**

> **Admin：BurnCloud 正在自动经营 Supply、Capacity、Demand 和 Economics，我只处理真正重要的例外。**

这就是 BurnCloud Information Architecture v1.0 的核心。